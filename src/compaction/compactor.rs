//! AST compaction via tree-sitter — the read optimizer's engine.
//!
//! Soft mode removes comment nodes while leaving every code byte identical (so
//! Edit `old_string`s still match). Medium/high additionally collapse code:
//! single-line-safe languages via [`compact_collapse`] (with an origin map),
//! others via [`compact_whitespace`].

use tree_sitter::{Node, Parser, Tree, TreeCursor};

use crate::compaction::collapse::collapse_ranges_for;
use crate::compaction::single_line::compact_collapse;
use crate::compaction::splice::{full_line_expanded, splice_out};
use crate::compaction::whitespace::compact_whitespace;
use crate::config::CompactMode;
use crate::languages::LanguageSpec;

/// Compacts source for one [`LanguageSpec`].
pub struct Compactor<'a> {
    spec: &'a LanguageSpec,
}

/// Parsed source with comment + string node byte-ranges (one DFS pass).
struct Scan {
    tree: Tree,
    comments: Vec<(usize, usize)>,
    strings: Vec<(usize, usize)>,
}

impl<'a> Compactor<'a> {
    /// Create a compactor bound to `spec`.
    #[must_use]
    pub const fn new(spec: &'a LanguageSpec) -> Self {
        Self { spec }
    }

    /// Soft-compact `source`: remove comment nodes, keep code byte-for-byte.
    /// Returns `None` if parsing fails or there is nothing to strip.
    #[must_use]
    pub fn compress(&self, source: &str) -> Option<String> {
        let scan = self.scan(source)?;
        if scan.comments.is_empty() {
            return None;
        }
        Some(splice_out(
            source,
            &full_line_expanded(source, &scan.comments),
        ))
    }

    /// Compact `source` for `mode`. Soft strips comments only; medium/high also
    /// collapse code (single-line view for safe langs, else whitespace-normalize).
    /// `None` if parsing fails, or (Soft) there were no comments to strip.
    #[must_use]
    pub fn compress_mode(&self, source: &str, mode: CompactMode) -> Option<String> {
        if mode == CompactMode::Soft {
            return self.compress(source);
        }
        let scan = self.scan(source)?;
        if self.spec.is_single_line_safe {
            let ranges = collapse_ranges_for(mode, &scan.tree, self.spec.block_kinds);
            let comments = full_line_expanded(source, &scan.comments);
            let (bytes, _) = compact_collapse(source.as_bytes(), &comments, &scan.strings, &ranges);
            Some(String::from_utf8_lossy(&bytes).into_owned())
        } else {
            let stripped = splice_out(source, &full_line_expanded(source, &scan.comments));
            let out = compact_whitespace(stripped.as_bytes(), self.spec.indent_based);
            Some(String::from_utf8_lossy(&out).into_owned())
        }
    }

    /// The compacted view + byte origin-map (`origin[i]` = source byte of output
    /// byte `i`) for `mode`. `None` for Soft or a language that can't single-line
    /// safely — callers then use the line-based fuzzy matcher.
    #[must_use]
    pub fn view_for_mode(&self, source: &str, mode: CompactMode) -> Option<(String, Vec<usize>)> {
        if mode == CompactMode::Soft || !self.spec.is_single_line_safe {
            return None;
        }
        let scan = self.scan(source)?;
        let ranges = collapse_ranges_for(mode, &scan.tree, self.spec.block_kinds);
        let comments = full_line_expanded(source, &scan.comments);
        let (bytes, origin) =
            compact_collapse(source.as_bytes(), &comments, &scan.strings, &ranges);
        Some((String::from_utf8_lossy(&bytes).into_owned(), origin))
    }

    /// Sorted comment node byte-ranges (used by `code_lines`). `None` on parse fail.
    pub(crate) fn comment_ranges(&self, source: &str) -> Option<Vec<(usize, usize)>> {
        Some(self.scan(source)?.comments)
    }

    /// Parse `source` and collect comment + string node ranges in one DFS.
    ///
    /// The parse is wall-clock-bounded ([`super::parse::parse_bounded`]): a source
    /// too large to parse within the hot-path budget yields `None`, so the caller
    /// passes the file through unchanged rather than stalling the Read.
    fn scan(&self, source: &str) -> Option<Scan> {
        let mut parser = Parser::new();
        parser.set_language(&self.spec.grammar()).ok()?;
        let tree = super::parse::parse_bounded(&mut parser, source)?;
        let mut comments = Vec::new();
        let mut strings = Vec::new();
        self.collect(&tree.root_node(), &mut comments, &mut strings);
        comments.sort_unstable();
        strings.sort_unstable();
        Some(Scan {
            tree,
            comments,
            strings,
        })
    }

    /// Iterative DFS (one cursor) collecting comment and string node byte-ranges;
    /// never descends into a comment or a string (their contents are atomic).
    fn collect(
        &self,
        root: &Node<'_>,
        comments: &mut Vec<(usize, usize)>,
        strings: &mut Vec<(usize, usize)>,
    ) {
        let mut cursor: TreeCursor = root.walk();
        'dfs: loop {
            let node = cursor.node();
            let kind = node.kind();
            let stop = if self.spec.is_comment(kind) {
                comments.push((node.start_byte(), node.end_byte()));
                true
            } else if self.spec.is_string(kind) {
                strings.push((node.start_byte(), node.end_byte()));
                true
            } else {
                false
            };
            if !stop && cursor.goto_first_child() {
                continue;
            }
            loop {
                if cursor.goto_next_sibling() {
                    continue 'dfs;
                }
                if !cursor.goto_parent() {
                    break 'dfs;
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/compaction/compactor.tests.rs"]
mod tests;
