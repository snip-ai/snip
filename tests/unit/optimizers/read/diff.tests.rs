//! Unit tests for the dedupe diff-on-change view, in AAA form. Compiled into
//! `snip_lib` via a `#[path]` include in `src/optimizers/read/diff.rs`.

use assert2::check;

use super::changed_notice;

/// Build an `n`-line body of `prefix {i}` lines.
fn lines(prefix: &str, n: usize) -> String {
    (0..n)
        .map(|i| format!("{prefix} {i}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn small_change_in_large_file_yields_a_compact_diff() {
    // Arrange: 100 identical lines with a single changed line in the middle
    let old = lines("line", 100);
    let mut rows: Vec<String> = (0..100).map(|i| format!("line {i}")).collect();
    rows[50] = "line 50 CHANGED".to_owned();
    let new = rows.join("\n");

    // Act
    let got = changed_notice("/x/foo.rs", &old, &new);

    // Assert: a worthwhile diff naming the file, the change, and the context gaps
    assert2::assert!(let Some(body) = got);
    check!(body.contains("foo.rs"));
    check!(body.contains("- line 50"));
    check!(body.contains("+ line 50 CHANGED"));
    check!(body.contains("@@"));
}

#[test]
fn wholesale_rewrite_is_not_worthwhile() {
    // Arrange: same-size old/new sharing no prefix or suffix
    let old = lines("alpha", 50);
    let new = lines("zulu", 50);

    // Act: the diff would be ~both files, larger than the size gate allows
    let got = changed_notice("/x/foo.rs", &old, &new);

    // Assert: caller falls back to the normal compacted view
    check!(got.is_none());
}

#[test]
fn identical_content_yields_no_diff() {
    // Arrange
    let same = lines("line", 20);

    // Act: prefix consumes everything, leaving an empty changed middle
    let got = changed_notice("/x/foo.rs", &same, &same);

    // Assert
    check!(got.is_none());
}
