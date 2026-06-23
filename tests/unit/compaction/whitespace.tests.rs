//! Unit tests for [`compact_whitespace`], in AAA form. Compiled into `snip_lib`
//! via a `#[path]` include in `src/compaction/whitespace.rs`.

use assert2::check;

use super::compact_whitespace;

fn s(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes).unwrap()
}

#[test]
fn collapses_blank_runs_and_trims_trailing_ws() {
    // Arrange: trailing spaces and a double blank line (not indent-based)
    let src = b"a   \n\n\n b \n";

    // Act
    let out = compact_whitespace(src, false);

    // Assert
    check!(s(out) == "a\n\n b\n");
}

#[test]
fn python_collapses_a_single_statement_block() {
    // Arrange
    let src = b"if x:\n    return y\nz = 1\n";

    // Act
    let out = compact_whitespace(src, true);

    // Assert
    check!(s(out) == "if x: return y\nz = 1\n");
}

#[test]
fn python_keeps_multi_statement_blocks() {
    // Arrange: two statements in the block → not collapsed
    let src = b"if x:\n    a()\n    b()\n";

    // Act
    let out = compact_whitespace(src, true);

    // Assert: structure (and indentation) preserved when more than one statement
    check!(s(out) == "if x:\n    a()\n    b()\n");
}

#[test]
fn drops_trailing_blank_lines() {
    // Arrange: trailing blank lines must be trimmed (not just collapsed)
    let src = b"a\n\n\n";

    // Act
    let out = compact_whitespace(src, false);

    // Assert: the trailing blank run is removed, leaving one terminated line
    check!(s(out) == "a\n");
}

#[test]
fn python_does_not_merge_a_non_indented_next_line() {
    // Arrange: a `:` header whose next line is at the same indent — not a body,
    // so nothing merges (next_indent > curr_indent is false)
    let src = b"if x:\ny = 1\n";

    // Act
    let out = compact_whitespace(src, true);

    // Assert: both lines kept verbatim
    check!(s(out) == "if x:\ny = 1\n");
}
