//! Unit tests for the `Project` transform, in AAA form. Compiled into `snip_lib`
//! via a `#[path]` include in `src/spec/project.rs`.

use assert2::check;

use super::project;

#[test]
fn keeps_selected_columns_in_order() {
    // Arrange
    let records = vec!["a b c d".to_owned(), "1 2 3 4".to_owned()];

    // Act
    let out = project(records, &[0, 2], " ");

    // Assert
    check!(out == vec!["a c".to_owned(), "1 3".to_owned()]);
}

#[test]
fn reorders_fields_and_uses_the_separator() {
    // Arrange
    let records = vec!["x y z".to_owned()];

    // Act
    let out = project(records, &[2, 0], " | ");

    // Assert
    check!(out == vec!["z | x".to_owned()]);
}

#[test]
fn keeps_a_record_without_the_requested_columns_verbatim() {
    // Arrange: one field, but column 5 doesn't exist
    let records = vec!["solo".to_owned()];

    // Act
    let out = project(records, &[5], " ");

    // Assert: never destroys an unexpected line
    check!(out == vec!["solo".to_owned()]);
}
