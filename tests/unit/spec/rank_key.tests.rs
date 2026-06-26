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

#[test]
fn path_sorts_lexicographically_so_same_dir_paths_become_consecutive() {
    // Arrange: mtime-interleaved paths from two directories
    let records = lines(&["src/a/x.rs", "src/b/y.rs", "src/a/z.rs", "src/b/w.rs"]);

    // Act
    let out = RankKey::Path.rank(records);

    // Assert: each directory's files are now adjacent, ready for group(dir)
    check!(out == lines(&["src/a/x.rs", "src/a/z.rs", "src/b/w.rs", "src/b/y.rs"]));
}
