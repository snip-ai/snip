//! Unit tests for the [`Transform`] vocabulary, in AAA form. Compiled into
//! `snip_lib` via a `#[path]` include in `src/spec/transform.rs`.

use assert2::check;

use super::Transform;
use crate::spec::{RankKey, StrPred};

fn lines(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn dedupe_collapses_consecutive_identicals() {
    // Arrange
    let records = lines(&["a", "a", "a", "b", "a"]);

    // Act
    let out = Transform::Dedupe.apply(records);

    // Assert: only consecutive runs collapse; the trailing lone "a" stays
    check!(out == lines(&["a (×3)", "b", "a"]));
}

#[test]
fn truncate_elides_the_middle() {
    // Arrange
    let records = lines(&["1", "2", "3", "4", "5", "6"]);

    // Act
    let out = Transform::Truncate { head: 2, tail: 1 }.apply(records);

    // Assert
    check!(out == lines(&["1", "2", "… (3 lines elided)", "6"]));
}

#[test]
fn truncate_keeps_short_input_intact() {
    // Arrange: len (3) <= head + tail + 1 (4) → nothing to elide
    let records = lines(&["1", "2", "3"]);

    // Act
    let out = Transform::Truncate { head: 2, tail: 1 }.apply(records);

    // Assert
    check!(out == lines(&["1", "2", "3"]));
}

#[test]
fn keep_retains_only_matching_records() {
    // Arrange
    let records = lines(&["error: a", "ok", "error: b"]);
    let pred = StrPred::Contains {
        value: "error".to_owned(),
    };

    // Act
    let out = Transform::Keep { pred }.apply(records);

    // Assert
    check!(out == lines(&["error: a", "error: b"]));
}

#[test]
fn drop_removes_matching_records() {
    // Arrange
    let records = lines(&["keep", "DEBUG noise", "keep too"]);
    let pred = StrPred::StartsWith {
        value: "DEBUG".to_owned(),
    };

    // Act
    let out = Transform::Drop { pred }.apply(records);

    // Assert
    check!(out == lines(&["keep", "keep too"]));
}

#[test]
fn rank_surfaces_errors_first() {
    // Arrange
    let records = lines(&["ok", "error: x"]);

    // Act
    let out = Transform::Rank {
        by: RankKey::ErrorsFirst,
    }
    .apply(records);

    // Assert
    check!(out == lines(&["error: x", "ok"]));
}

#[test]
fn strip_ansi_removes_color() {
    // Arrange
    let records = lines(&["\u{1b}[31mERR\u{1b}[0m x"]);

    // Act
    let out = Transform::StripAnsi.apply(records);

    // Assert
    check!(out == lines(&["ERR x"]));
}

#[test]
fn squeeze_collapses_blank_runs_and_trims() {
    // Arrange
    let records = lines(&["a", "", "", "b  ", ""]);

    // Act
    let out = Transform::Squeeze.apply(records);

    // Assert
    check!(out == lines(&["a", "", "b", ""]));
}

#[test]
fn fingerprint_variant_deserializes_and_folds() {
    // Arrange: the internally-tagged newtype variant round-trips from JSON
    let transform: Transform = serde_json::from_str(r#"{"op":"fingerprint"}"#).unwrap();
    let records = lines(&["item 1 ok", "item 2 ok"]);

    // Act
    let out = transform.apply(records);

    // Assert
    check!(out == lines(&["item 1 ok (×2)"]));
}

#[test]
fn fold_frames_variant_deserializes_and_folds() {
    // Arrange: the internally-tagged newtype variant round-trips from JSON with
    // all-default config (enabled, default markers, keep_top = 1).
    let transform: Transform = serde_json::from_str(r#"{"op":"fold_frames"}"#).unwrap();
    let records = lines(&[
        "  File \"app/main.py\", line 10, in run",
        "  File \"x/site-packages/lib.py\", line 1, in a",
        "  File \"x/site-packages/lib.py\", line 2, in b",
        "  File \"x/site-packages/lib.py\", line 3, in c",
    ]);

    // Act
    let out = transform.apply(records);

    // Assert: app frame kept; the framework run keeps keep_top (1) then folds 2
    check!(
        out == lines(&[
            "  File \"app/main.py\", line 10, in run",
            "  File \"x/site-packages/lib.py\", line 1, in a",
            "… (2 framework frames)",
        ])
    );
}

#[test]
fn test_rollup_collapses_a_passing_run() {
    // Arrange
    let records = lines(&["test a ... ok", "test b ... ok", "test c ... ok"]);
    let pred = StrPred::EndsWith {
        value: " ... ok".to_owned(),
    };

    // Act
    let out = Transform::TestRollup { pred }.apply(records);

    // Assert
    check!(out == lines(&["… (3 passed)"]));
}

#[test]
fn test_rollup_keeps_a_lone_pass_verbatim() {
    // Arrange: a run of 1 must NOT inflate to a longer marker
    let records = lines(&["test a ... ok", "test x ... FAILED"]);
    let pred = StrPred::EndsWith {
        value: " ... ok".to_owned(),
    };

    // Act
    let out = Transform::TestRollup { pred }.apply(records);

    // Assert: the lone pass stays verbatim; the failure is untouched
    check!(out == lines(&["test a ... ok", "test x ... FAILED"]));
}

#[test]
fn test_rollup_splits_runs_around_a_failure() {
    // Arrange
    let records = lines(&[
        "a ... ok",
        "b ... ok",
        "x ... FAILED",
        "c ... ok",
        "d ... ok",
    ]);
    let pred = StrPred::EndsWith {
        value: " ... ok".to_owned(),
    };

    // Act
    let out = Transform::TestRollup { pred }.apply(records);

    // Assert: each pass run collapses; the failure stays verbatim in place
    check!(out == lines(&["… (2 passed)", "x ... FAILED", "… (2 passed)"]));
}

#[test]
fn test_rollup_leaves_the_summary_line() {
    // Arrange: the cargo summary line does not end in " ... ok"
    let records = lines(&[
        "a ... ok",
        "b ... ok",
        "test result: ok. 2 passed; 0 failed; finished in 0.01s",
    ]);
    let pred = StrPred::EndsWith {
        value: " ... ok".to_owned(),
    };

    // Act
    let out = Transform::TestRollup { pred }.apply(records);

    // Assert
    check!(
        out == lines(&[
            "… (2 passed)",
            "test result: ok. 2 passed; 0 failed; finished in 0.01s",
        ])
    );
}

#[test]
fn test_rollup_variant_deserializes_and_applies() {
    // Arrange: the struct variant round-trips from JSON with its predicate
    let transform: Transform = serde_json::from_str(
        r#"{"op":"test_rollup","pred":{"match":"ends_with","value":"PASSED"}}"#,
    )
    .unwrap();
    let records = lines(&["t1 PASSED", "t2 PASSED"]);

    // Act
    let out = transform.apply(records);

    // Assert
    check!(out == lines(&["… (2 passed)"]));
}

#[test]
fn fold_diff_variant_deserializes_and_folds() {
    // Arrange: the internally-tagged newtype variant round-trips with all-default
    // config (enabled, min_run = 4, context = 0)
    let transform: Transform = serde_json::from_str(r#"{"op":"fold_diff"}"#).unwrap();
    let records = lines(&["@@", " a", " b", " c", " d", " e", "+new"]);

    // Act
    let out = transform.apply(records);

    // Assert: the 5-line context run folds; header + change line kept
    check!(out == lines(&["@@", "… (5 unchanged)", "+new"]));
}

#[test]
fn parse_arm_dispatches_to_the_format() {
    // Arrange: the Parse arm routes to ParseFormat (json_minify here)
    let transform: Transform =
        serde_json::from_str(r#"{"op":"parse","format":"json_minify"}"#).unwrap();
    let records = lines(&["{", "  \"a\": 1", "}"]);

    // Act
    let out = transform.apply(records);

    // Assert
    check!(out == lines(&["{\"a\":1}"]));
}

#[test]
fn project_arm_defaults_the_separator_to_a_space() {
    // Arrange: no `sep` in the JSON ⇒ default_sep() supplies a single space
    let transform: Transform = serde_json::from_str(r#"{"op":"project","cols":[0,2]}"#).unwrap();
    let records = lines(&["a b c d"]);

    // Act
    let out = transform.apply(records);

    // Assert: columns 0 and 2 joined by the default space separator
    check!(out == lines(&["a c"]));
}

#[test]
fn dedupe_arm_on_empty_input_is_empty() {
    // Arrange: the Dedupe transform on no records hits its empty early-return
    let records: Vec<String> = Vec::new();

    // Act
    let out = Transform::Dedupe.apply(records);

    // Assert
    check!(out.is_empty());
}

#[test]
fn template_wraps_each_record() {
    // Arrange
    let records = lines(&["a.rs", "b.rs"]);

    // Act
    let out = Transform::Template {
        each: "- {}".to_owned(),
    }
    .apply(records);

    // Assert
    check!(out == lines(&["- a.rs", "- b.rs"]));
}
