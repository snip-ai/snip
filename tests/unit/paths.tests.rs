//! Unit tests for [`crate::paths`] session-id sanitization, in AAA form.
//! Compiled into `snip_lib` via a `#[path]` include in `src/paths.rs`.

use assert2::check;

use super::sanitize;

#[test]
fn empty_or_missing_id_collapses_to_a_shared_bucket() {
    // Arrange + Act + Assert: a spill always needs a home
    check!(sanitize(None) == "no-session");
    check!(sanitize(Some("")) == "no-session");
    check!(sanitize(Some("   ")) == "no-session");
}

#[test]
fn alphanumeric_and_dash_pass_through_unchanged() {
    // Arrange + Act + Assert
    check!(sanitize(Some("abc-123-DEF")) == "abc-123-DEF");
}

#[test]
fn path_separators_and_traversal_are_neutralized() {
    // Arrange: a hostile id must not escape the cache root
    // Act
    let out = sanitize(Some("a/b\\c..d:e"));

    // Assert: every non-[A-Za-z0-9-] byte becomes '-'
    check!(out == "a-b-c--d-e");
}
