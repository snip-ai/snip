//! Block byte-ranges that medium/high modes collapse onto one line.
//!
//! High collapses every outermost multi-line brace block onto its header line
//! (keeping item structure — one function per line); medium collapses only blocks
//! holding a single statement. Both feed [`crate::compaction::single_line`]'s
//! collapse ranges, which keeps an origin map for edit-fix.

use tree_sitter::{Node, Tree};

use crate::config::CompactMode;

/// Collapse ranges for `mode`: outermost blocks (High), single-statement blocks
/// (Medium), or none (Soft). `block_kinds` are the language's brace-block kinds.
#[must_use]
pub fn collapse_ranges_for(
    mode: CompactMode,
    tree: &Tree,
    block_kinds: &[&str],
) -> Vec<(usize, usize)> {
    match mode {
        CompactMode::High => outermost_blocks(tree, block_kinds),
        CompactMode::Medium => single_statement_blocks(tree, block_kinds),
        CompactMode::Soft => Vec::new(),
    }
}

/// Outermost multi-line brace blocks — high mode collapses each onto its header
/// line, keeping the file's item structure instead of one whole-file line.
fn outermost_blocks(tree: &Tree, block_kinds: &[&str]) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut cursor = tree.walk();
    'outer: loop {
        let node = cursor.node();
        let matched = block_kinds.contains(&node.kind())
            && node.end_position().row > node.start_position().row;
        if matched {
            ranges.push((node.start_byte(), node.end_byte()));
        }
        // Don't descend into a collapsed block: nested blocks are already covered.
        if !matched && cursor.goto_first_child() {
            continue;
        }
        if cursor.goto_next_sibling() {
            continue;
        }
        loop {
            if !cursor.goto_parent() {
                break 'outer;
            }
            if cursor.goto_next_sibling() {
                continue 'outer;
            }
        }
    }
    ranges
}

/// Multi-line brace blocks containing exactly one statement (`if (x) {\n y;\n }`),
/// which medium mode collapses onto one line.
fn single_statement_blocks(tree: &Tree, block_kinds: &[&str]) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut cursor = tree.walk();
    'outer: loop {
        let node = cursor.node();
        if is_single_statement_block(node, block_kinds)
            && node.end_position().row > node.start_position().row
        {
            ranges.push((node.start_byte(), node.end_byte()));
        }
        if cursor.goto_first_child() {
            continue;
        }
        if cursor.goto_next_sibling() {
            continue;
        }
        loop {
            if !cursor.goto_parent() {
                break 'outer;
            }
            if cursor.goto_next_sibling() {
                continue 'outer;
            }
        }
    }
    ranges
}

/// `true` if `node` is a brace block whose only child (ignoring braces and
/// comments) is a single statement.
fn is_single_statement_block(node: Node<'_>, block_kinds: &[&str]) -> bool {
    if !block_kinds.contains(&node.kind()) {
        return false;
    }
    let mut count = 0usize;
    let mut c = node.walk();
    if c.goto_first_child() {
        loop {
            let k = c.node().kind();
            if k != "{" && k != "}" && !k.contains("comment") {
                count += 1;
            }
            if !c.goto_next_sibling() {
                break;
            }
        }
    }
    count == 1
}

#[cfg(test)]
#[path = "../../tests/unit/compaction/collapse.tests.rs"]
mod tests;
