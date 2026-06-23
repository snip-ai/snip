//! Unit tests for the language registry (extension lookup + grammar ABI), in AAA
//! form. Compiled into `snip_lib` via a `#[path]` include in
//! `src/languages/registry.rs`, so these can reach the private `SPECS` table.

use assert2::check;

use super::{SPECS, detect};

#[test]
fn detects_languages_by_extension() {
    // Act + Assert (pure lookups; no shared state to arrange)
    check!(detect("/a/b/main.rs").map(|l| l.name) == Some("rust"));
    check!(detect("app.py").map(|l| l.name) == Some("python"));
    check!(detect("a.tsx").map(|l| l.name) == Some("tsx"));
    check!(detect("Program.cs").map(|l| l.name) == Some("csharp"));
    check!(detect("Main.kt").map(|l| l.name) == Some("kotlin"));
    check!(detect("build.scala").map(|l| l.name) == Some("scala"));
    check!(detect("ci.yml").map(|l| l.name) == Some("yaml"));
    check!(detect("Cargo.toml").map(|l| l.name) == Some("toml"));
    check!(detect("schema.sql").map(|l| l.name) == Some("sql"));
    check!(detect("index.html").map(|l| l.name) == Some("html"));
    check!(detect("App.swift").map(|l| l.name) == Some("swift"));
    check!(detect("widget.dart").map(|l| l.name) == Some("dart"));
    check!(detect("model.R").map(|l| l.name) == Some("r"));
    check!(detect("main.zig").map(|l| l.name) == Some("zig"));
    check!(detect("calc.jl").map(|l| l.name) == Some("julia"));
    check!(detect("Lib.hs").map(|l| l.name) == Some("haskell"));
    check!(detect("View.m").map(|l| l.name) == Some("objc"));
    check!(detect("legacy.h").map(|l| l.name) == Some("c")); // .h stays with C, not objc
    check!(detect("/a/b/readme.md").is_none()); // markdown intentionally not a read language
    check!(detect("noext").is_none());
}

#[test]
fn metadata_is_internally_consistent() {
    // Act + Assert: single-line-safe iff it has brace blocks to collapse; every
    // spec names at least one comment and string kind; indent-based langs
    // (Python) are never single-line-safe.
    for spec in SPECS {
        assert!(
            spec.is_single_line_safe != spec.block_kinds.is_empty(),
            "{}: single_line_safe must match having block_kinds",
            spec.name
        );
        assert!(
            !spec.comment_kinds.is_empty(),
            "{}: no comment kinds",
            spec.name
        );
        assert!(
            !spec.string_kinds.is_empty(),
            "{}: no string kinds",
            spec.name
        );
        assert!(
            !(spec.indent_based && spec.is_single_line_safe),
            "{}: indent-based langs are not single-line-safe",
            spec.name
        );
    }
}

#[test]
fn every_grammar_loads() {
    // Arrange
    let mut parser = tree_sitter::Parser::new();

    // Act + Assert: every grammar is ABI-compatible with the pinned tree-sitter
    for spec in SPECS {
        assert!(
            parser.set_language(&spec.grammar()).is_ok(),
            "grammar failed to load: {}",
            spec.name
        );
    }
}
