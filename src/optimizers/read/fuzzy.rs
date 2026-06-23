//! Soft-mode fuzzy matching: map a compacted `old_string` back to file bytes.
//!
//! The compacted view differs from the file only by stripped comments, so we
//! normalize both (comment-free, trimmed lines), slide the needle over the file's
//! code units by LCS similarity, and return the **original** bytes (comments and
//! CRLF intact) of the best window — the text Claude Code matches verbatim.

use super::unit::{Unit, file_units};
use crate::languages::LanguageSpec;

/// Similarity at or above which a window is accepted (safety invariant — do not
/// lower; see `CLAUDE.md`).
const MATCH_THRESHOLD: f64 = 0.85;
/// Length-ratio floor (shorter/longer of the two normalized texts) — a second
/// anti-false-positive guard alongside [`MATCH_THRESHOLD`].
const LENGTH_RATIO_FLOOR: f64 = 0.80;
/// Minimum normalized line count to attempt fuzzy scoring; shorter needles match
/// too many windows, so they require an exact unique match instead.
const MIN_CODE_LINES: usize = 3;
/// Upper bound on LCS cell-updates (`windows × n²`) before the fuzzy scan is
/// abandoned. The scan is `O((units − n + 1) · n²)`, i.e. cubic when the needle
/// `n ≈ units/2`; an unbounded scan stalls for seconds on the model-blocking
/// `Edit`/`snip resolve` path (which, unlike `apply_read`, has no byte cap). Past
/// this bound, degrade to verbatim — the model re-reads — rather than block. A
/// needle this large is an unusual edit; ~20M cells ≈ low tens of ms on commodity
/// hardware (well under any worst case the budget cares about).
const MAX_LCS_CELLS: usize = 20_000_000;

/// Map the compacted `old_str` back to the original region of `file_content`.
///
/// `spec` drives AST comment removal so the file normalizes the same way the
/// compacted view did. `collapse_blocks` (medium/high on an indent-based language)
/// merges single-statement blocks so a merged view line resolves to its source
/// span. Returns `None` below [`MIN_CODE_LINES`] unless an exact unique match
/// exists, or when no window clears both thresholds.
#[must_use]
#[allow(clippy::cast_precision_loss)] // line/byte counts are always << 2^53
pub fn fuzzy_match(
    spec: Option<&LanguageSpec>,
    file_content: &str,
    old_str: &str,
    collapse_blocks: bool,
) -> Option<String> {
    let file_lines: Vec<&str> = file_content.lines().collect();
    let units = file_units(spec, file_content, &file_lines, collapse_blocks);
    let norm_old: Vec<String> = old_str
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
        .collect();

    let n = norm_old.len();
    if n == 0 || units.len() < n {
        return None;
    }
    if n < MIN_CODE_LINES {
        return short_exact_match(file_content, &file_lines, &units, &norm_old);
    }
    // Bound the O(windows · n²) LCS work so a pathological `old_str` can't stall
    // this synchronous, model-blocking path; degrade to a verbatim re-read instead.
    let windows = units.len() - n + 1;
    if windows.saturating_mul(n).saturating_mul(n) > MAX_LCS_CELLS {
        return None;
    }

    let old_hashes: Vec<u64> = norm_old.iter().map(|s| hash_str(s)).collect();
    let unit_hashes: Vec<u64> = units.iter().map(|u| hash_str(&u.text)).collect();
    let old_len: usize = norm_old.iter().map(String::len).sum();
    let mut prev = vec![0usize; n + 1];
    let mut curr = vec![0usize; n + 1];

    let mut best: Option<usize> = None;
    let mut best_score = 0.0_f64;
    for start in 0..=unit_hashes.len() - n {
        let window = &unit_hashes[start..start + n];
        let score = if window == old_hashes.as_slice() {
            1.0
        } else {
            lcs_len(&old_hashes, window, &mut prev, &mut curr) as f64 / n as f64
        };
        if score > best_score
            && score >= MATCH_THRESHOLD
            && length_ok(old_len, &units[start..start + n])
        {
            best_score = score;
            best = Some(start);
            if (score - 1.0).abs() < f64::EPSILON {
                break; // can't beat an exact match
            }
        }
    }
    let start = best?;
    Some(slice_units(file_content, &file_lines, &units, start, n))
}

/// Whether the matched window's normalized length is within the ratio floor of
/// the needle — rejects windows of wildly different size despite a high LCS.
#[allow(clippy::cast_precision_loss)]
fn length_ok(old_len: usize, window: &[Unit]) -> bool {
    let win_len: usize = window.iter().map(|u| u.text.len()).sum();
    let (lo, hi) = (old_len.min(win_len), old_len.max(win_len));
    hi != 0 && (lo as f64 / hi as f64) >= LENGTH_RATIO_FLOOR
}

/// Slice the original bytes spanning the matched window's first..last lines,
/// preserving exact line endings (CRLF/LF) so the verbatim Edit match succeeds.
fn slice_units(file: &str, file_lines: &[&str], units: &[Unit], start: usize, n: usize) -> String {
    let first_line = file_lines[units[start].first];
    let last_line = file_lines[units[start + n - 1].last];
    let start_off = byte_offset(file, first_line);
    let end_off = byte_offset(file, last_line) + last_line.len();
    file[start_off..end_off].to_owned()
}

/// Byte offset of subslice `sub` within `parent` (both from the same `&str`).
fn byte_offset(parent: &str, sub: &str) -> usize {
    sub.as_ptr() as usize - parent.as_ptr() as usize
}

/// Exact-and-unique window match for needles shorter than [`MIN_CODE_LINES`].
fn short_exact_match(
    file: &str,
    file_lines: &[&str],
    units: &[Unit],
    norm_old: &[String],
) -> Option<String> {
    let n = norm_old.len();
    let mut found: Option<usize> = None;
    for start in 0..=units.len() - n {
        if (0..n).all(|i| units[start + i].text == norm_old[i]) {
            if found.is_some() {
                return None; // ambiguous — refuse to guess
            }
            found = Some(start);
        }
    }
    let start = found?;
    Some(slice_units(file, file_lines, units, start, n))
}

/// Hash a normalized line to a `u64` for fast equality in the LCS loop.
fn hash_str(s: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

/// Length of the longest common subsequence of `a` and `b` (rolling-row DP,
/// caller-provided buffers reused across windows to avoid per-window allocation).
fn lcs_len(a: &[u64], b: &[u64], prev: &mut [usize], curr: &mut [usize]) -> usize {
    let n = a.len();
    prev[..=n].iter_mut().for_each(|x| *x = 0);
    for &bv in b {
        curr[0] = 0;
        for i in 0..n {
            curr[i + 1] = if a[i] == bv {
                prev[i] + 1
            } else {
                curr[i].max(prev[i + 1])
            };
        }
        prev[..=n].copy_from_slice(&curr[..=n]);
    }
    prev[n]
}

#[cfg(test)]
#[path = "../../../tests/unit/optimizers/read/fuzzy.tests.rs"]
mod tests;
