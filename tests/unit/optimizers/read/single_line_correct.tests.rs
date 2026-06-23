//! Unit tests for [`single_line_correct`] (origin-map mapping), in AAA form.
//! Compiled into `snip_lib` via a `#[path]` include in
//! `src/optimizers/read/single_line_correct.rs`.

use assert2::check;

use super::{find_unique, single_line_correct};
use crate::config::CompactMode;
use crate::languages::detect;

#[test]
fn maps_a_collapsed_needle_back_to_real_file_bytes() {
    // Arrange
    let spec = detect("x.rs").unwrap();
    let file = "fn compute() {\n    let total = a + b;\n}\n";

    // Act: a needle taken from the High collapsed view
    let got = single_line_correct(file, "let total = a + b;", spec, CompactMode::High);

    // Assert: maps back to a real substring of the file
    assert2::assert!(let Some(text) = got);
    check!(file.contains(&text));
    check!(text.contains("total = a + b"));
}

#[test]
fn too_short_needle_is_rejected() {
    // Arrange
    let spec = detect("x.rs").unwrap();

    // Act: below the MIN_SINGLE_LINE_CHARS floor
    let got = single_line_correct("fn f() {\n    x();\n}\n", "x", spec, CompactMode::High);

    // Assert
    check!(got.is_none());
}

#[test]
fn find_unique_rejects_an_empty_needle() {
    // Arrange + Act: the empty-needle guard inside `find_unique` (line 38). It is
    // unreachable through `single_line_correct` (the MIN_SINGLE_LINE_CHARS floor
    // rejects an empty needle first), so test the helper directly.
    let got = find_unique("haystack with words", "");

    // Assert
    check!(got.is_none());
}

#[test]
fn ambiguous_needle_in_collapsed_view_is_rejected() {
    // Arrange: a needle that occurs twice in the view → not unique, so the
    // origin-map mapping returns None.
    let spec = detect("x.rs").unwrap();
    let file = "fn f() {\n    let v = a + b;\n    let v = a + b;\n}\n";

    // Act
    let got = single_line_correct(file, "let v = a + b;", spec, CompactMode::High);

    // Assert
    check!(got.is_none());
}
