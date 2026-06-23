//! Byte-range removal for comment stripping, with full-line residue cleanup.
//!
//! [`splice_out`] drops sorted, non-overlapping byte ranges from a source.
//! [`full_line_expanded`] first widens each comment range that occupies a whole
//! line so it also takes that line's leading indentation and single trailing
//! newline — leaving no residue. This is what keeps soft mode byte-identical for
//! code even when a grammar's doc-comment node (e.g. Rust `///`) swallows its own
//! trailing newline, which a line-delimited residue pass could not see.

/// Build a new string with the (sorted, non-overlapping) byte ranges removed.
#[must_use]
pub fn splice_out(source: &str, ranges: &[(usize, usize)]) -> String {
    let mut out = String::with_capacity(source.len());
    let mut pos = 0;
    for &(start, end) in ranges {
        if start >= pos {
            out.push_str(&source[pos..start]);
            pos = end;
        }
    }
    out.push_str(&source[pos..]);
    out
}

/// Widen each full-line comment range to also take its indent and trailing newline.
///
/// A comment occupies a whole line when only whitespace precedes it and nothing
/// but whitespace (or its own newline) follows it; such a range is expanded to
/// cover that leading indentation and single trailing newline, leaving no
/// residue. A *trailing* comment (code before it on the line) also takes the
/// whitespace gap before it, so no trailing spaces are left on the code line —
/// the code bytes themselves are untouched. Input ranges are sorted and
/// non-overlapping; the output stays sorted and at worst adjacent, which
/// [`splice_out`] tolerates.
#[must_use]
pub fn full_line_expanded(source: &str, ranges: &[(usize, usize)]) -> Vec<(usize, usize)> {
    let bytes = source.as_bytes();
    let blank = |s: &str| s.bytes().all(|b| matches!(b, b' ' | b'\t' | b'\r'));
    ranges
        .iter()
        .map(|&(start, end)| {
            let line_start = source[..start].rfind('\n').map_or(0, |i| i + 1);
            if !blank(&source[line_start..start]) {
                // Trailing comment (code precedes it): drop the comment AND the
                // whitespace gap before it, so soft mode leaves no trailing spaces on
                // the code line. The fuzzy matcher trims lines, so no code byte is lost.
                let kept = source[line_start..start]
                    .trim_end_matches([' ', '\t'])
                    .len();
                return (line_start + kept, end);
            }
            if bytes.get(end.wrapping_sub(1)) == Some(&b'\n') {
                return (line_start, end); // node already swallowed its newline
            }
            let rest = &source[end..];
            match rest.find('\n') {
                Some(off) if blank(&rest[..off]) => (line_start, end + off + 1),
                None if blank(rest) => (line_start, source.len()),
                _ => (start, end), // code follows on the same line
            }
        })
        .collect()
}

#[cfg(test)]
#[path = "../../tests/unit/compaction/splice.tests.rs"]
mod tests;
