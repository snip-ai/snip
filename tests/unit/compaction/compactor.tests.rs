//! Unit tests for the soft-mode [`Compactor`], in AAA form. Compiled into
//! `snip_lib` via a `#[path]` include in `src/compaction/compactor.rs`.

use assert2::check;

use super::Compactor;
use crate::config::CompactMode;
use crate::languages::detect;

#[test]
fn strips_rust_comments_keeps_code_and_strings() {
    // Arrange
    let src = "// header\nfn main() {\n    let x = 5; // trailing\n    /* block */\n    let s = \"hi // not a comment\";\n}\n";
    let spec = detect("x.rs").unwrap();

    // Act
    let out = Compactor::new(spec).compress(src).unwrap();

    // Assert: comments gone; code (and the string's `//`) preserved byte-for-byte
    check!(!out.contains("header"));
    check!(!out.contains("trailing"));
    check!(!out.contains("/* block */"));
    check!(out.contains("fn main() {"));
    check!(out.contains("let x = 5;"));
    check!(out.contains("hi // not a comment"));
}

#[test]
fn soft_mode_drops_indentation_only_comment_residue() {
    // Arrange: a full-line comment with indentation, an inline comment, and a
    // genuine blank line. Only the indentation-only residue left by the full-line
    // comment should vanish; code bytes and the author's blank line stay.
    let src = "fn main() {\n    // full-line\n    let x = 1; // inline\n\n}\n";
    let spec = detect("x.rs").unwrap();

    // Act
    let out = Compactor::new(spec).compress(src).unwrap();

    // Assert
    check!(!out.contains("\n    \n")); // the `    ` residue line is gone
    check!(out.contains("    let x = 1;\n")); // inline comment + its gap gone, no trailing space
    check!(!out.contains("let x = 1; ")); // no leftover trailing space before the stripped comment
    check!(out.contains("fn main() {"));
    check!(out.contains("\n\n}")); // the genuine blank line survives
}

#[test]
fn no_comments_returns_none() {
    // Arrange
    let spec = detect("x.rs").unwrap();

    // Act
    let out = Compactor::new(spec).compress("fn main() {}\n");

    // Assert
    check!(out.is_none());
}

#[test]
fn strips_comments_across_languages() {
    // Arrange: (path, source with a `COMMENTMARK` comment, code that must remain)
    let cases = [
        ("x.py", "# COMMENTMARK\nx = 1  # tail\n", "x = 1"),
        ("x.js", "// COMMENTMARK\nlet a = 1; /* b */\n", "let a = 1;"),
        (
            "x.ts",
            "// COMMENTMARK\nconst a: number = 1;\n",
            "const a: number = 1;",
        ),
        ("x.tsx", "// COMMENTMARK\nconst a = 1;\n", "const a = 1;"),
        ("x.go", "// COMMENTMARK\npackage main\n", "package main"),
        (
            "x.c",
            "/* COMMENTMARK */\nint main(){ return 0; }\n",
            "int main()",
        ),
        (
            "x.cpp",
            "// COMMENTMARK\nint main(){ return 0; }\n",
            "int main()",
        ),
        ("x.java", "// COMMENTMARK\nclass A {}\n", "class A"),
        ("x.rb", "# COMMENTMARK\nx = 1\n", "x = 1"),
        ("x.sh", "# COMMENTMARK\necho hi\n", "echo hi"),
        ("x.cs", "// COMMENTMARK\nclass A {}\n", "class A"),
        ("x.php", "<?php // COMMENTMARK\n$a = 1;\n", "$a = 1;"),
        (
            "x.css",
            "/* COMMENTMARK */\na { color: red; }\n",
            "color: red",
        ),
        ("x.lua", "-- COMMENTMARK\nlocal a = 1\n", "local a = 1"),
        (
            "x.ex",
            "# COMMENTMARK\ndefmodule A do\nend\n",
            "defmodule A",
        ),
        ("x.kt", "// COMMENTMARK\nfun main() {}\n", "fun main()"),
        ("x.scala", "// COMMENTMARK\nobject A {}\n", "object A"),
        ("x.yml", "# COMMENTMARK\nkey: value\n", "key: value"),
        ("x.toml", "# COMMENTMARK\nkey = \"v\"\n", "key ="),
        ("x.sql", "-- COMMENTMARK\nSELECT 1;\n", "SELECT 1"),
        ("x.html", "<!-- COMMENTMARK -->\n<p>hi</p>\n", "<p>hi</p>"),
        ("x.swift", "// COMMENTMARK\nfunc f() {}\n", "func f()"),
        ("x.dart", "// COMMENTMARK\nvoid main() {}\n", "void main()"),
        ("x.R", "# COMMENTMARK\nx <- 1\n", "x <- 1"),
        ("x.zig", "// COMMENTMARK\nconst x = 1;\n", "const x = 1"),
        ("x.jl", "# COMMENTMARK\nx = 1\n", "x = 1"),
        ("x.hs", "-- COMMENTMARK\nx = 1\n", "x = 1"),
        (
            "x.m",
            "// COMMENTMARK\nint main(){ return 0; }\n",
            "int main()",
        ),
    ];

    for (path, src, code) in cases {
        // Act
        let spec = detect(path).unwrap_or_else(|| panic!("no spec for {path}"));
        let out = Compactor::new(spec)
            .compress(src)
            .unwrap_or_else(|| panic!("no compaction for {path}"));

        // Assert (std assert! names the failing language)
        assert!(
            !out.contains("COMMENTMARK"),
            "comment not stripped in {path}: {out:?}"
        );
        assert!(out.contains(code), "code lost in {path}: {out:?}");
    }
}

#[test]
fn high_mode_collapses_a_rust_block_onto_its_header() {
    // Arrange
    let src = "// c\nfn main() {\n    let x = 1;\n    let y = 2;\n}\n";
    let spec = detect("x.rs").unwrap();

    // Act
    let out = Compactor::new(spec)
        .compress_mode(src, CompactMode::High)
        .unwrap();

    // Assert: comment gone; body folded onto the header → fewer lines
    check!(!out.contains("// c"));
    check!(out.contains("fn main() {"));
    check!(out.lines().count() < src.lines().count());
}

#[test]
fn view_for_mode_maps_a_view_byte_back_to_source() {
    // Arrange
    let src = "fn main() {\n    let x = 1;\n}\n";
    let spec = detect("x.rs").unwrap();

    // Act
    let view = Compactor::new(spec).view_for_mode(src, CompactMode::High);

    // Assert: a collapsed view + origin map exists for a single-line-safe lang
    assert2::assert!(let Some((text, origin)) = view);
    check!(text.len() == origin.len());
    let xv = text.find('x').expect("x in the view");
    check!(src.as_bytes()[origin[xv]] == b'x');
}

#[test]
fn view_for_mode_round_trips_across_single_line_safe_languages() {
    // Arrange: every single-line-safe language, each with a `UNIQUEMARK` identifier
    // inside a collapsible block. The origin map must map each marker byte back to
    // its exact source byte — the invariant `snip resolve`/`edit-fix` rely on.
    let cases = [
        ("x.rs", "fn f() {\n    let UNIQUEMARK = 1;\n}\n"),
        ("x.c", "int f(){\n    int UNIQUEMARK = 1;\n}\n"),
        ("x.cpp", "int f(){\n    int UNIQUEMARK = 1;\n}\n"),
        ("x.java", "class A {\n    int UNIQUEMARK = 1;\n}\n"),
        ("x.cs", "class A {\n    int UNIQUEMARK = 1;\n}\n"),
        ("x.css", "a {\n    color: UNIQUEMARK;\n}\n"),
        ("x.swift", "func f() {\n    let UNIQUEMARK = 1\n}\n"),
        ("x.dart", "void f() {\n    var UNIQUEMARK = 1;\n}\n"),
        ("x.R", "f <- function() {\n    UNIQUEMARK <- 1\n}\n"),
        ("x.zig", "fn f() void {\n    const UNIQUEMARK = 1;\n}\n"),
        ("x.jl", "function f()\n    UNIQUEMARK = 1\nend\n"),
        ("x.m", "int f(){\n    int UNIQUEMARK = 1;\n}\n"),
    ];

    for (path, src) in cases {
        // Act
        let spec = detect(path).unwrap_or_else(|| panic!("no spec for {path}"));
        let view = Compactor::new(spec).view_for_mode(src, CompactMode::High);

        // Assert: a collapsed view + a 1:1 origin map whose marker bytes round-trip
        assert!(view.is_some(), "no collapsed view for {path}");
        let (text, origin) = view.unwrap();
        assert!(
            text.len() == origin.len(),
            "origin map length mismatch for {path}"
        );
        let pos = text
            .find("UNIQUEMARK")
            .unwrap_or_else(|| panic!("marker missing in {path}: {text:?}"));
        for (k, ch) in "UNIQUEMARK".bytes().enumerate() {
            assert!(
                src.as_bytes()[origin[pos + k]] == ch,
                "origin map byte {k} maps wrong for {path}: {text:?}"
            );
        }
    }
}

#[test]
fn view_for_mode_is_none_for_a_non_single_line_lang() {
    // Arrange: Python is not single-line-safe → no collapsed view/origin map
    let spec = detect("x.py").unwrap();

    // Act
    let view = Compactor::new(spec).view_for_mode("if x:\n    y\n", CompactMode::High);

    // Assert
    check!(view.is_none());
}

#[test]
fn medium_mode_folds_a_single_statement_python_block() {
    // Arrange: Python uses the whitespace path
    let spec = detect("x.py").unwrap();
    let src = "# c\nif x:\n    return y\n";

    // Act
    let out = Compactor::new(spec)
        .compress_mode(src, CompactMode::Medium)
        .unwrap();

    // Assert
    check!(!out.contains("# c"));
    check!(out.contains("if x: return y"));
}
