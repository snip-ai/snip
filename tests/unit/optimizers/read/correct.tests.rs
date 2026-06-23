//! Unit tests for [`correct_old_string`], in AAA form. Compiled into `snip_lib`
//! via a `#[path]` include in `src/optimizers/read/correct.rs`.

use assert2::check;

use super::correct_old_string;
use crate::config::CompactMode;

#[test]
fn corrects_a_stripped_old_string_to_real_bytes() {
    // Arrange
    let file = "fn main() {\n    do_it(); // go\n    done();\n}\n";
    let old = "fn main() {\n    do_it();\n    done();\n}\n";

    // Act
    let got = correct_old_string("a.rs", file, old, CompactMode::Soft);

    // Assert
    check!(got.as_deref() == Some("fn main() {\n    do_it(); // go\n    done();\n}"));
}

#[test]
fn returns_none_when_nothing_matches() {
    // Arrange
    let file = "fn a() {\n    one();\n    two();\n}\n";
    let old = "fn b() {\n    nope();\n    never();\n}\n";

    // Act
    let got = correct_old_string("a.rs", file, old, CompactMode::Soft);

    // Assert
    check!(got.is_none());
}

#[test]
fn high_mode_maps_a_collapsed_needle_via_origin_map() {
    // Arrange: rust is single-line-safe, so High routes through the origin map
    // (lines 24-31) rather than the soft fuzzy matcher.
    let file = "fn compute() {\n    let total = a + b;\n}\n";
    let old = "let total = a + b;";

    // Act
    let got = correct_old_string("a.rs", file, old, CompactMode::High);

    // Assert: maps back to a real, verbatim substring of the file
    assert2::assert!(let Some(text) = got);
    check!(file.contains(&text));
    check!(text.contains("total = a + b"));
}

#[test]
fn high_mode_maps_a_multiline_crlf_needle_against_an_lf_file() {
    // Arrange: an LF file, but the model pastes a multi-line old_string with CRLF
    // endings (the previously-failing direction — the retry only normalized LF→CRLF).
    let file = "use a::Reader;\nuse a::Writer;\nuse a::Debug;\n\nfn run() {\n    go();\n}\n";
    let old = "use a::Reader;\r\nuse a::Writer;";

    // Act
    let got = correct_old_string("a.rs", file, old, CompactMode::High);

    // Assert: resolved to the file's verbatim (LF) bytes
    check!(got.as_deref() == Some("use a::Reader;\nuse a::Writer;"));
}

#[test]
fn high_mode_maps_a_multiline_lf_needle_against_a_crlf_file() {
    // Arrange: the reverse — a CRLF file with an LF-typed old_string
    let file =
        "use a::Reader;\r\nuse a::Writer;\r\nuse a::Debug;\r\n\r\nfn run() {\r\n    go();\r\n}\r\n";
    let old = "use a::Reader;\nuse a::Writer;";

    // Act
    let got = correct_old_string("a.rs", file, old, CompactMode::High);

    // Assert: resolved to the file's verbatim (CRLF) bytes
    check!(got.as_deref() == Some("use a::Reader;\r\nuse a::Writer;"));
}

#[test]
fn high_mode_returns_none_when_collapsed_needle_is_absent() {
    // Arrange: a needle that appears nowhere in the collapsed view or its CRLF form.
    let file = "fn compute() {\n    let total = a + b;\n}\n";
    let old = "let nonexistent = q * z;";

    // Act
    let got = correct_old_string("a.rs", file, old, CompactMode::High);

    // Assert
    check!(got.is_none());
}
