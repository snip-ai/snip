//! Unit tests for [`RankKey`] stable reordering, in AAA form. Compiled into
//! `snip_lib` via a `#[path]` include in `src/spec/rank_key.rs`.

use assert2::check;

use super::RankKey;

fn lines(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn errors_first_moves_relevant_lines_up_stably() {
    // Arrange: error/panic lines interleaved with plain ones
    let records = lines(&["ok one", "error: boom", "ok two", "panic at x", "ok three"]);

    // Act
    let out = RankKey::ErrorsFirst.rank(records);

    // Assert: relevant first in original order, the rest stable behind
    check!(out == lines(&["error: boom", "panic at x", "ok one", "ok two", "ok three"]));
}

#[test]
fn errors_first_is_a_noop_without_relevant_lines() {
    // Arrange
    let records = lines(&["ok one", "ok two"]);

    // Act
    let out = RankKey::ErrorsFirst.rank(records);

    // Assert
    check!(out == lines(&["ok one", "ok two"]));
}
