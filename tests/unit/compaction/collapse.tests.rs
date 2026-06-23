//! Unit tests for [`collapse_ranges_for`] block detection, in AAA form. Compiled
//! into `snip_lib` via a `#[path]` include in `src/compaction/collapse.rs`.

use assert2::check;
use tree_sitter::Parser;

use super::collapse_ranges_for;
use crate::config::CompactMode;

fn rust_tree(src: &str) -> tree_sitter::Tree {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .expect("rust grammar");
    parser.parse(src, None).expect("parse")
}

#[test]
fn high_collapses_the_outermost_block() {
    // Arrange
    let src = "fn f() {\n    let x = 1;\n}\n";
    let tree = rust_tree(src);

    // Act
    let ranges = collapse_ranges_for(CompactMode::High, &tree, &["block"]);

    // Assert: exactly the multi-line body block is a collapse range
    check!(ranges.len() == 1);
    let (s, e) = ranges[0];
    check!(&src[s..e] == "{\n    let x = 1;\n}");
}

#[test]
fn medium_collapses_a_single_statement_block() {
    // Arrange: the body holds exactly one statement
    let src = "fn f() {\n    do_it();\n}\n";
    let tree = rust_tree(src);

    // Act
    let ranges = collapse_ranges_for(CompactMode::Medium, &tree, &["block"]);

    // Assert
    check!(ranges.len() == 1);
}

#[test]
fn medium_skips_a_multi_statement_block() {
    // Arrange: the body holds two statements ⇒ not a single-statement block
    let src = "fn f() {\n    a();\n    b();\n}\n";
    let tree = rust_tree(src);

    // Act
    let ranges = collapse_ranges_for(CompactMode::Medium, &tree, &["block"]);

    // Assert: medium only collapses single-statement blocks, so none here
    check!(ranges.is_empty());
}

#[test]
fn soft_collapses_nothing() {
    // Arrange
    let tree = rust_tree("fn f() {\n    let x = 1;\n}\n");

    // Act
    let ranges = collapse_ranges_for(CompactMode::Soft, &tree, &["block"]);

    // Assert
    check!(ranges.is_empty());
}
