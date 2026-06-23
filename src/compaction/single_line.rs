//! Whitespace collapse with a byte origin-map (the single-line-safe path).
//!
//! Removes comments, copies string literals verbatim, and collapses whitespace to
//! single spaces **within `collapse_ranges`** (everything else keeps its layout).
//! The origin map (`origin[i]` = source byte offset of output byte `i`) lets the
//! edit-fix path map a substring of the compacted view back to exact source bytes.
//! A removed comment still separates tokens (`a/*c*/b` → `a b`, never `ab`).

/// Punctuation that should not have a space inserted *before* it.
const NO_SPACE_BEFORE: [u8; 6] = [b')', b']', b'}', b',', b'.', b';'];
/// Punctuation that should not have a space inserted *after* it.
const NO_SPACE_AFTER: [u8; 3] = [b'(', b'[', b'.'];

/// Compact `src`, collapsing whitespace only inside `collapse_ranges`.
///
/// Comments (`comment_ranges`) are removed everywhere; `string_ranges` are copied
/// verbatim; whitespace outside `collapse_ranges` is preserved. Returns the bytes
/// (source bytes + ASCII spaces → always valid UTF-8) and their origin map.
#[must_use]
pub fn compact_collapse(
    src: &[u8],
    comment_ranges: &[(usize, usize)],
    string_ranges: &[(usize, usize)],
    collapse_ranges: &[(usize, usize)],
) -> (Vec<u8>, Vec<usize>) {
    let mut comments = sorted_nonempty(comment_ranges);
    let mut strings = sorted_nonempty(string_ranges);
    let mut collapse = sorted_nonempty(collapse_ranges);
    comments.sort_unstable();
    strings.sort_unstable();
    collapse.sort_unstable();

    let mut out = Vec::with_capacity(src.len());
    let mut origin = Vec::with_capacity(src.len());
    let mut ws_pending: Option<usize> = None;
    let (mut ci, mut si, mut coli) = (0usize, 0usize, 0usize);
    let mut i = 0usize;

    while i < src.len() {
        while ci < comments.len() && comments[ci].1 <= i {
            ci += 1;
        }
        while si < strings.len() && strings[si].1 <= i {
            si += 1;
        }
        while coli < collapse.len() && collapse[coli].1 <= i {
            coli += 1;
        }

        // Comment → skip entirely, but it still separates the tokens around it.
        if ci < comments.len() && i >= comments[ci].0 {
            if ws_pending.is_none() {
                ws_pending = Some(i);
            }
            i = comments[ci].1;
            continue;
        }
        // String → copy verbatim (after flushing a pending separator space).
        if si < strings.len() && i >= strings[si].0 {
            let (s, e) = strings[si];
            flush_ws(&mut out, &mut origin, &mut ws_pending, src[s]);
            for (k, &b) in src[s..e].iter().enumerate() {
                out.push(b);
                origin.push(s + k);
            }
            i = e;
            continue;
        }

        // Code byte.
        let in_collapse = coli < collapse.len() && i >= collapse[coli].0;
        let b = src[i];
        if b.is_ascii_whitespace() {
            if in_collapse {
                if ws_pending.is_none() {
                    ws_pending = Some(i);
                }
            } else {
                ws_pending = None;
                out.push(b);
                origin.push(i);
            }
        } else {
            flush_ws(&mut out, &mut origin, &mut ws_pending, b);
            out.push(b);
            origin.push(i);
        }
        i += 1;
    }

    (out, origin)
}

/// Copied, sorted, non-empty ranges.
fn sorted_nonempty(ranges: &[(usize, usize)]) -> Vec<(usize, usize)> {
    ranges.iter().copied().filter(|&(s, e)| s < e).collect()
}

/// Emit a single separating space if one is pending and the context allows it.
fn flush_ws(out: &mut Vec<u8>, origin: &mut Vec<usize>, ws_pending: &mut Option<usize>, next: u8) {
    if let Some(off) = ws_pending.take() {
        // The separator space exists so `a/*c*/b` stays `a b`. After a newline it
        // is never needed — a removed whole-line comment used to leak it as a
        // leading space, breaking Edit old_strings copied from the view.
        let last_ok = out
            .last()
            .is_none_or(|&b| !NO_SPACE_AFTER.contains(&b) && b != b'\n' && b != b'\r');
        if !out.is_empty() && last_ok && !NO_SPACE_BEFORE.contains(&next) {
            out.push(b' ');
            origin.push(off);
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/compaction/single_line.tests.rs"]
mod tests;
