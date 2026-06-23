//! Unit tests for [`StrPred`] matching, in AAA form. Compiled into `snip_lib`
//! via a `#[path]` include in `src/spec/str_pred.rs`.

use assert2::check;

use super::StrPred;

#[test]
fn contains_matches_a_substring() {
    // Arrange
    let pred = StrPred::Contains {
        value: "err".to_owned(),
    };

    // Act
    let hit = pred.matches("an error line");

    // Assert
    check!(hit);
}

#[test]
fn equals_requires_an_exact_match() {
    // Arrange
    let pred = StrPred::Equals {
        value: "done".to_owned(),
    };

    // Act
    let exact = pred.matches("done");
    let loose = pred.matches("done now");

    // Assert
    check!(exact);
    check!(!loose);
}

#[test]
fn starts_with_anchors_the_prefix() {
    // Arrange
    let pred = StrPred::StartsWith {
        value: "warn".to_owned(),
    };

    // Act
    let at_start = pred.matches("warning: x");
    let mid = pred.matches("a warning");

    // Assert
    check!(at_start);
    check!(!mid);
}

#[test]
fn ends_with_anchors_the_suffix() {
    // Arrange
    let pred = StrPred::EndsWith {
        value: ".rs".to_owned(),
    };

    // Act
    let at_end = pred.matches("src/main.rs");
    let not_end = pred.matches("main.rs.bak");

    // Assert
    check!(at_end);
    check!(!not_end);
}
