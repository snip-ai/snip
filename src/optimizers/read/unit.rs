//! A normalized code unit: comment-free text plus its original line span.

use crate::compaction::code_lines;
use crate::languages::LanguageSpec;

/// One normalized code unit from the file: canonical (comment-free, trimmed) text
/// plus the inclusive range of original line indices it covers. In soft mode each
/// non-blank, non-comment line is its own unit.
pub struct Unit {
    /// Comment-free, trimmed line text used for matching.
    pub text: String,
    /// First original line index this unit covers.
    pub first: usize,
    /// Last original line index this unit covers.
    pub last: usize,
}

/// Build the file's normalized code units, anchored to the AST.
///
/// [`code_lines`] gives the comment-free residue of each non-blank, non-comment
/// line (tree-sitter knows comments from strings). Comment-only lines are dropped
/// and blank lines skipped, exactly as soft compaction renders the view. With no
/// grammar, every non-blank line is treated as code (graceful fallback).
///
/// `collapse_blocks` (medium/high on an indent-based language, e.g. Python) merges
/// each single-statement block onto its header — mirroring [`compact_whitespace`]
/// so the merged view line (`def f(): return x`) resolves back to its two original
/// lines. Off (soft mode, or a non-indent language) → one unit per code line.
///
/// [`compact_whitespace`]: crate::compaction::whitespace::compact_whitespace
#[must_use]
pub fn file_units(
    spec: Option<&LanguageSpec>,
    file: &str,
    file_lines: &[&str],
    collapse_blocks: bool,
) -> Vec<Unit> {
    use std::collections::HashMap;

    let residues = spec.map(|s| code_lines(s, file)).unwrap_or_default();
    let have_ast = !residues.is_empty();
    let code: HashMap<usize, String> = residues.into_iter().collect();

    let units: Vec<Unit> = file_lines
        .iter()
        .enumerate()
        .filter_map(|(i, line)| match code.get(&i) {
            Some(text) => Some(Unit {
                text: text.trim().to_owned(),
                first: i,
                last: i,
            }),
            // Blank line, or (with an AST) a comment-only line → drop it.
            None if line.trim().is_empty() || have_ast => None,
            // No AST available: fall back to treating non-blank lines as code.
            None => Some(Unit {
                text: line.trim().to_owned(),
                first: i,
                last: i,
            }),
        })
        .collect();

    if collapse_blocks && spec.is_some_and(|s| s.indent_based) {
        merge_indent_blocks(&units, file_lines)
    } else {
        units
    }
}

/// Merge each single-statement indented block onto its header (`def f():` + one
/// deeper line → `def f(): <stmt>`), spanning both original lines, so the merged
/// medium/high view line maps back to its source region. Mirrors
/// [`crate::compaction::whitespace::compact_whitespace`]'s Python collapse.
fn merge_indent_blocks(units: &[Unit], file_lines: &[&str]) -> Vec<Unit> {
    let indent = |line_ix: usize| {
        let l = file_lines.get(line_ix).copied().unwrap_or("");
        l.len() - l.trim_start().len()
    };
    let mut out = Vec::with_capacity(units.len());
    let mut i = 0;
    while i < units.len() {
        let u = &units[i];
        // A single-statement block: a `:` header, a more-indented next unit, and
        // the unit after it (if any) dedenting back to the header — exactly one
        // statement, just as `compact_whitespace` requires before it merges.
        let single_stmt = u.text.ends_with(':')
            && units
                .get(i + 1)
                .is_some_and(|s| indent(s.first) > indent(u.first))
            && units
                .get(i + 2)
                .is_none_or(|n| indent(n.first) <= indent(u.first));
        if let Some(s) = units.get(i + 1).filter(|_| single_stmt) {
            out.push(Unit {
                text: format!("{} {}", u.text, s.text),
                first: u.first,
                last: s.last,
            });
            i += 2;
        } else {
            out.push(Unit {
                text: u.text.clone(),
                first: u.first,
                last: u.last,
            });
            i += 1;
        }
    }
    out
}

#[cfg(test)]
#[path = "../../../tests/unit/optimizers/read/unit.tests.rs"]
mod tests;
