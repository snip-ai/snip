//! AST-anchored code lines: each non-blank, non-comment line, comments stripped.

use crate::compaction::Compactor;
use crate::languages::LanguageSpec;

/// Each non-blank, non-comment line with its comment byte-ranges removed.
///
/// Returns `(line_index, code_residue)` per kept line (leading indentation kept,
/// trailing whitespace trimmed); `line_index` matches `source.lines()` enumeration.
///
/// Tree-sitter draws the comment/string line exactly, so a `//` inside a string
/// is never mistaken for a comment. Returns empty when the grammar is
/// unavailable, so callers can fall back to a text heuristic.
#[must_use]
pub fn code_lines(spec: &LanguageSpec, source: &str) -> Vec<(usize, String)> {
    let Some(comment_ranges) = Compactor::new(spec).comment_ranges(source) else {
        return Vec::new();
    };
    let base = source.as_ptr() as usize;
    source
        .lines()
        .enumerate()
        .filter_map(|(idx, line)| {
            let line_start = line.as_ptr() as usize - base;
            let residue = strip_ranges(line, line_start, &comment_ranges);
            let residue = residue.trim_end();
            (!residue.trim().is_empty()).then(|| (idx, residue.to_owned()))
        })
        .collect()
}

/// Return `line` with the byte spans in `comment_ranges` (absolute offsets)
/// removed. `line_start` is `line`'s absolute byte offset within the source.
///
/// Uses `str::get` for every slice so a range that (defensively) lands off a
/// char boundary is skipped rather than panicking.
fn strip_ranges(line: &str, line_start: usize, comment_ranges: &[(usize, usize)]) -> String {
    let line_end = line_start + line.len();
    let mut out = String::with_capacity(line.len());
    let mut cursor = 0usize; // relative to line start
    for &(rs, re) in comment_ranges {
        if re <= line_start || rs >= line_end {
            continue; // range does not overlap this line
        }
        let start_rel = rs.saturating_sub(line_start).max(cursor).min(line.len());
        if start_rel > cursor {
            out.push_str(line.get(cursor..start_rel).unwrap_or(""));
        }
        let end_rel = (re - line_start).min(line.len());
        cursor = cursor.max(end_rel);
    }
    if cursor < line.len() {
        out.push_str(line.get(cursor..).unwrap_or(""));
    }
    out
}

#[cfg(test)]
#[path = "../../tests/unit/compaction/code_lines.tests.rs"]
mod tests;
