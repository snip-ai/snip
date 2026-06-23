//! Map a medium/high `old_string` back to source bytes via the origin map.

use crate::compaction::Compactor;
use crate::config::CompactMode;
use crate::languages::LanguageSpec;

/// Minimum non-whitespace length of `old_string` for a single-line correction —
/// guards against degenerate matches on a collapsed one-line view.
const MIN_SINGLE_LINE_CHARS: usize = 3;

/// Map a single-line `old_str` (a substring of the collapsed view) back to the
/// original file region via the origin map. `None` if it isn't found, isn't
/// unique, or is too short — the Edit then fails naturally (and `resolve` reports
/// ambiguity vs not-found).
#[must_use]
pub fn single_line_correct(
    file_content: &str,
    old_str: &str,
    spec: &LanguageSpec,
    mode: CompactMode,
) -> Option<String> {
    let needle = old_str.trim();
    if needle.chars().filter(|c| !c.is_whitespace()).count() < MIN_SINGLE_LINE_CHARS {
        return None;
    }
    let (view, origin) = Compactor::new(spec).view_for_mode(file_content, mode)?;
    let c0 = find_unique(&view, needle)?;
    let c1 = c0 + needle.len();
    // Uniqueness in the view implies uniqueness in the original.
    let start = *origin.get(c0)?;
    let end = origin.get(c1 - 1).map(|&o| o + 1)?;
    file_content.get(start..end).map(str::to_owned)
}

/// Byte offset of `needle` in `haystack` iff it occurs exactly once.
fn find_unique(haystack: &str, needle: &str) -> Option<usize> {
    if needle.is_empty() {
        return None;
    }
    let mut hits = haystack.match_indices(needle);
    let first = hits.next()?.0;
    hits.next().is_none().then_some(first) // reject ambiguous (≥ 2) matches
}

#[cfg(test)]
#[path = "../../../tests/unit/optimizers/read/single_line_correct.tests.rs"]
mod tests;
