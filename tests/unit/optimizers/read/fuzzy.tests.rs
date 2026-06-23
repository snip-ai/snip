//! Unit tests for soft-mode [`fuzzy_match`], in AAA form. Compiled into
//! `snip_lib` via a `#[path]` include in `src/optimizers/read/fuzzy.rs`.

use assert2::check;

use super::fuzzy_match;
use crate::languages::detect;

#[test]
fn recovers_a_block_with_an_inline_comment() {
    // Arrange: the view stripped the inline comment; map old_string back to bytes
    let file = "fn main() {\n    let x = 1; // set x\n    let y = 2;\n}\n";
    let old = "fn main() {\n    let x = 1;\n    let y = 2;\n}\n";

    // Act
    let got = fuzzy_match(detect("a.rs"), file, old, false);

    // Assert: the corrected text matches the real file verbatim (comment intact)
    check!(got.as_deref() == Some("fn main() {\n    let x = 1; // set x\n    let y = 2;\n}"));
}

#[test]
fn short_needle_matches_a_unique_line_with_comment() {
    // Arrange: a single-line needle below the fuzzy floor → exact-unique match
    let file = "let a = 0;\nlet z = 9; // nine\nlet b = 0;\n";
    let old = "let z = 9;";

    // Act
    let got = fuzzy_match(detect("a.rs"), file, old, false);

    // Assert
    check!(got.as_deref() == Some("let z = 9; // nine"));
}

#[test]
fn no_confident_match_returns_none() {
    // Arrange: a needle absent from the file
    let file = "fn a() {\n    let x = 1;\n    let y = 2;\n}\n";
    let old = "fn totally() {\n    different();\n    code();\n}\n";

    // Act
    let got = fuzzy_match(detect("a.rs"), file, old, false);

    // Assert
    check!(got.is_none());
}

#[test]
fn needle_longer_than_the_file_returns_none() {
    // Arrange: the file has one code unit but the needle normalizes to three lines,
    // so `units.len() < n` short-circuits (line 44).
    let file = "fn a() {}\n";
    let old = "first_call();\nsecond_call();\nthird_call();";

    // Act
    let got = fuzzy_match(detect("a.rs"), file, old, false);

    // Assert
    check!(got.is_none());
}

#[test]
fn exact_multiline_match_breaks_early() {
    // Arrange: a 3+ line needle that matches the file verbatim — score hits 1.0 and
    // the search breaks (lines 71-73).
    let file = "fn a() {\n    one();\n    two();\n}\n";
    let old = "fn a() {\n    one();\n    two();\n}";

    // Act
    let got = fuzzy_match(detect("a.rs"), file, old, false);

    // Assert: returns the exact original span
    check!(got.as_deref() == Some("fn a() {\n    one();\n    two();\n}"));
}

#[test]
fn pathological_large_needle_is_capped_to_none() {
    // Arrange: a 400-line needle over an 800-unit file. Without the work cap this
    // is a valid block (exact match at start 0), but `windows × n²`
    // (401 × 400² ≈ 64M) exceeds MAX_LCS_CELLS, so the scan must bail to None
    // (→ verbatim re-read) rather than risk a multi-second stall.
    let file = (0..800)
        .map(|i| format!("let v{i} = {i};"))
        .collect::<Vec<_>>()
        .join("\n");
    let old = (0..400)
        .map(|i| format!("let v{i} = {i};"))
        .collect::<Vec<_>>()
        .join("\n");

    // Act
    let got = fuzzy_match(detect("a.rs"), &file, &old, false);

    // Assert: over the cap → None (degraded), not a stall
    check!(got.is_none());
}

#[test]
fn collapsed_python_block_maps_back_to_its_two_source_lines() {
    // Arrange: medium/high merges `def f():` + its one indented statement onto one
    // line in the view; with collapse_blocks the merged `old_string` must resolve to
    // the original TWO lines (BUG-3 — soft/fuzzy alone could not reverse the merge).
    let file = "x = 0\ndef trap():\n    return x\ny = 1\n";
    let old = "def trap(): return x";

    // Act
    let got = fuzzy_match(detect("a.py"), file, old, true);

    // Assert: the original two-line block (real indentation intact)
    check!(got.as_deref() == Some("def trap():\n    return x"));
}

#[test]
fn collapsed_off_a_python_merge_does_not_match_in_soft_mode() {
    // Arrange: in soft mode the view is byte-identical (no merge), so the merged
    // one-liner is NOT a view line and must NOT resolve — collapse_blocks=false.
    let file = "x = 0\ndef trap():\n    return x\ny = 1\n";
    let old = "def trap(): return x";

    // Act
    let got = fuzzy_match(detect("a.py"), file, old, false);

    // Assert
    check!(got.is_none());
}

#[test]
fn ambiguous_short_needle_returns_none() {
    // Arrange: a one-line needle (below the fuzzy floor) that occurs twice → the
    // exact-unique matcher refuses to guess (line 116).
    let file = "let x = 1;\nlet y = 2;\nlet x = 1;\n";
    let old = "let x = 1;";

    // Act
    let got = fuzzy_match(detect("a.rs"), file, old, false);

    // Assert
    check!(got.is_none());
}
