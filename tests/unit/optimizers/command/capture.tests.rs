//! Unit tests for the bounded stdout drain (`drain_capped`).

use std::io::Cursor;

use assert2::check;

use super::drain_capped;

#[test]
fn keeps_all_bytes_and_reports_untruncated_when_under_cap() {
    // Arrange
    let mut input = Cursor::new(b"hello".to_vec());

    // Act
    let (buf, truncated) = drain_capped(&mut input, 100);

    // Assert
    check!(buf == b"hello");
    check!(!truncated);
}

#[test]
fn keeps_only_the_prefix_and_reports_truncated_when_over_cap() {
    // Arrange
    let mut input = Cursor::new(b"hello world".to_vec());

    // Act
    let (buf, truncated) = drain_capped(&mut input, 4);

    // Assert
    check!(buf == b"hell");
    check!(truncated);
}

#[test]
fn reports_untruncated_when_input_exactly_fills_the_cap() {
    // Arrange
    let mut input = Cursor::new(b"abcd".to_vec());

    // Act
    let (buf, truncated) = drain_capped(&mut input, 4);

    // Assert
    check!(buf == b"abcd");
    check!(!truncated);
}
