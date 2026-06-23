//! Unit tests for [`fold_diff`] diff-hunk pruning, in AAA form. Compiled into
//! `snip_lib` via a `#[path]` include in `src/spec/diff_fold.rs`.

use assert2::check;

use super::{DiffFoldCfg, fold_diff};

fn lines(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn folds_a_long_context_run_keeping_changes_and_headers() {
    // Arrange: a 5-line context run between a hunk header and a change
    let records = lines(&[
        "@@ -1,8 +1,8 @@",
        " a",
        " b",
        " c",
        " d",
        " e",
        "-old",
        "+new",
    ]);

    // Act: defaults (enabled, min_run = 4, context = 0)
    let out = fold_diff(records, &DiffFoldCfg::default());

    // Assert: header + change lines verbatim; the run folds to one marker
    check!(out == lines(&["@@ -1,8 +1,8 @@", "… (5 unchanged)", "-old", "+new"]));
}

#[test]
fn leaves_a_short_run_verbatim() {
    // Arrange: a 3-line context run (≤ min_run 4) — git's default -U3 shape
    let records = lines(&["@@ -1,4 +1,4 @@", " a", " b", " c", "-old", "+new"]);

    // Act
    let out = fold_diff(records.clone(), &DiffFoldCfg::default());

    // Assert: byte-identical (nothing folded)
    check!(out == records);
}

#[test]
fn keeps_context_lines_on_each_side() {
    // Arrange: a 7-line run with context = 2 → keep 2 + marker(3) + 2
    let records = lines(&["@@", "-x", " 1", " 2", " 3", " 4", " 5", " 6", " 7", "+y"]);
    let cfg = DiffFoldCfg {
        context: 2,
        ..DiffFoldCfg::default()
    };

    // Act
    let out = fold_diff(records, &cfg);

    // Assert
    check!(out == lines(&["@@", "-x", " 1", " 2", "… (3 unchanged)", " 6", " 7", "+y"]));
}

#[test]
fn never_folds_a_combined_diff_change_line() {
    // Arrange: a `" -"` line (combined diff: changed vs the 2nd parent) sits
    // between two foldable context runs and must be treated as a change
    let records = lines(&[
        " a", " b", " c", " d", " e", " -x", " f", " g", " h", " i", " j",
    ]);

    // Act
    let out = fold_diff(records, &DiffFoldCfg::default());

    // Assert: the `" -x"` line is kept verbatim, flushing the runs around it
    check!(out == lines(&["… (5 unchanged)", " -x", "… (5 unchanged)"]));
}

#[test]
fn treats_a_blank_line_as_context() {
    // Arrange: a blank line inside the run counts as context (is_context's None arm)
    let records = lines(&["@@", " a", "", " b", " c", " d", "+x"]);

    // Act
    let out = fold_diff(records, &DiffFoldCfg::default());

    // Assert: the 5-line run (blank included) folds
    check!(out == lines(&["@@", "… (5 unchanged)", "+x"]));
}

#[test]
fn disabled_returns_input_unchanged() {
    // Arrange
    let records = lines(&["@@", " a", " b", " c", " d", " e", " f"]);
    let cfg = DiffFoldCfg {
        enabled: false,
        ..DiffFoldCfg::default()
    };

    // Act
    let out = fold_diff(records.clone(), &cfg);

    // Assert
    check!(out == records);
}
