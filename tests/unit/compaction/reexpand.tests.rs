//! Unit tests for [`reexpand`], in AAA form. Compiled into `snip_lib` via a
//! `#[path]` include in `src/compaction/reexpand.rs`.

use assert2::check;

use super::reexpand;
use crate::languages::detect;

fn nows(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

#[test]
fn expands_a_flattened_rust_block() {
    // Arrange
    let spec = detect("x.rs").unwrap();
    let flat = "fn f() { let x = 1; g(); }";

    // Act
    let out = reexpand(spec, flat);

    // Assert: now multi-line, with identical non-whitespace content
    check!(out.contains('\n'));
    check!(nows(&out) == nows(flat));
}

#[test]
fn leaves_multiline_input_unchanged() {
    // Arrange
    let spec = detect("x.rs").unwrap();
    let already = "fn f() {\n    x();\n}";

    // Act + Assert: already multi-line → untouched
    check!(reexpand(spec, already) == already);
}

#[test]
fn leaves_a_non_single_line_language_unchanged() {
    // Arrange: Python is not single-line-safe → never re-expanded
    let spec = detect("x.py").unwrap();

    // Act + Assert
    check!(reexpand(spec, "a;{") == "a;{");
}

#[test]
fn leaves_a_fragment_that_parses_with_errors_unchanged() {
    // Arrange: contains `;`/`{` (passes the cheap guard) but is invalid Rust, so
    // the parse has an error node and the input must be returned verbatim
    let spec = detect("x.rs").unwrap();
    let broken = "{ ) ] ; (";

    // Act + Assert
    check!(reexpand(spec, broken) == broken);
}

#[test]
fn an_array_repeat_semicolon_does_not_break_the_line() {
    // Arrange: `[0; 4]`'s `;` is an array separator, not a statement terminator,
    // so it must stay inline (exercises is_statement_semicolon's array arm)
    let spec = detect("x.rs").unwrap();
    let flat = "fn f() { let a = [0; 4]; g(a); }";

    // Act
    let out = reexpand(spec, flat);

    // Assert: re-expanded (multi-line), content preserved, and the array `;` did
    // NOT force a newline — `0`, `;` and `4` stay on one line (unlike a statement
    // `;`, which would put `4` on its own indented line)
    check!(out.contains('\n'));
    check!(nows(&out) == nows(flat));
    check!(out.contains("0; 4") || out.contains("0;4"));
}
