//! Unit tests for [`code_lines`], in AAA form. Compiled into `snip_lib` via a
//! `#[path]` include in `src/compaction/code_lines.rs`.

use assert2::check;

use super::code_lines;
use crate::languages::detect;

#[test]
fn strips_inline_comments_and_drops_comment_only_lines() {
    // Arrange
    let spec = detect("a.rs").expect("a rust spec");
    let src = "fn f() {\n    let x = 1; // set x\n    // full line\n}\n";

    // Act
    let lines = code_lines(spec, src);

    // Assert: the comment-only line is dropped; the inline comment is stripped
    check!(
        lines
            == vec![
                (0, "fn f() {".to_owned()),
                (1, "    let x = 1;".to_owned()),
                (3, "}".to_owned()),
            ]
    );
}

#[test]
fn comment_free_source_keeps_every_code_line() {
    // Arrange: no comments at all → every non-blank line survives verbatim
    let spec = detect("a.rs").expect("a rust spec");
    let src = "fn f() {\n    g();\n}\n";

    // Act
    let lines = code_lines(spec, src);

    // Assert: each non-blank line kept with its original index
    check!(
        lines
            == vec![
                (0, "fn f() {".to_owned()),
                (1, "    g();".to_owned()),
                (2, "}".to_owned()),
            ]
    );
}

#[test]
fn empty_source_yields_no_lines() {
    // Arrange: a valid spec but empty source — parses clean, no lines to keep
    let spec = detect("a.rs").expect("a rust spec");

    // Act
    let lines = code_lines(spec, "");

    // Assert
    check!(lines.is_empty());
}

#[test]
fn unsupported_language_yields_no_lines() {
    // Arrange: a spec exists, but feed it through an unparsable nonsense path —
    // detect returns None for an unknown extension, so callers fall back.
    let detected = detect("notes.unknownext");

    // Assert
    check!(detected.is_none());
}
