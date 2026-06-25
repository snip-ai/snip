//! Unit tests for [`fingerprint`] log folding, in AAA form. Compiled into
//! `snip_lib` via a `#[path]` include in `src/spec/log_fold.rs`.

use assert2::check;

use super::{FingerprintCfg, fingerprint};

fn lines(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn folds_consecutive_masked_equal_lines() {
    // Arrange: lines differing only by an id collapse to one template + count
    let records = lines(&[
        "Processing item 1041 ... ok",
        "Processing item 1042 ... ok",
        "Processing item 1043 ... ok",
    ]);

    // Act
    let out = fingerprint(records, &FingerprintCfg::default());

    // Assert: the first concrete line is kept as the sample
    check!(out == lines(&["Processing item 1041 ... ok (×3)"]));
}

#[test]
fn protects_error_lines_from_folding() {
    // Arrange: an error line between two similar lines
    let records = lines(&[
        "Connecting to 10.0.0.1",
        "error: failed to reach 10.0.0.2",
        "Connecting to 10.0.0.3",
    ]);

    // Act
    let out = fingerprint(records, &FingerprintCfg::default());

    // Assert: error verbatim; the two non-consecutive lines stay separate
    check!(out.len() == 3);
    check!(out[1] == "error: failed to reach 10.0.0.2");
}

#[test]
fn folds_lines_differing_only_by_a_uuid() {
    // Arrange: two log lines whose only difference is a UUID — its short 4-hex
    // segments would otherwise stay verbatim and block the collapse
    let records = lines(&[
        "request 550e8400-e29b-41d4-a716-446655440000 served",
        "request 7c9e6679-7425-40de-944b-e07fc1f90ae7 served",
    ]);

    // Act
    let out = fingerprint(records, &FingerprintCfg::default());

    // Assert: both collapse to one templated line with a ×2 count
    check!(out.len() == 1);
    check!(out[0].contains("(×2)"));
}

#[test]
fn whole_window_folds_across_the_output() {
    // Arrange: whole-output grouping, first position kept
    let cfg: FingerprintCfg = serde_json::from_str(r#"{"window":"whole"}"#).unwrap();
    let records = lines(&["req 1 ok", "other", "req 2 ok", "req 3 ok"]);

    // Act
    let out = fingerprint(records, &cfg);

    // Assert
    check!(out == lines(&["req 1 ok (×3)", "other"]));
}

#[test]
fn disabled_returns_input_unchanged() {
    // Arrange: a disabled config must pass records straight through
    let cfg = FingerprintCfg {
        enabled: false,
        ..FingerprintCfg::default()
    };
    let records = lines(&["item 1 ok", "item 2 ok"]);

    // Act
    let out = fingerprint(records.clone(), &cfg);

    // Assert
    check!(out == records);
}

#[test]
fn whole_window_keeps_a_protected_line_in_place() {
    // Arrange: whole-output grouping with an error line that must never fold
    let cfg: FingerprintCfg = serde_json::from_str(r#"{"window":"whole"}"#).unwrap();
    let records = lines(&["req 1 ok", "error: boom", "req 2 ok"]);

    // Act
    let out = fingerprint(records, &cfg);

    // Assert: the error is pushed verbatim; the two reqs collapse around it
    check!(out == lines(&["req 1 ok (×2)", "error: boom"]));
}

#[test]
fn masks_a_dash_token_that_is_not_a_uuid() {
    // Arrange: lines with a hyphenated token (contains '-' but no UUID) that
    // differ only by a digit run → the non-UUID dash walk + number masking fold
    let records = lines(&["build-123 done", "build-456 done"]);

    // Act
    let out = fingerprint(records, &FingerprintCfg::default());

    // Assert: both collapse to one templated line
    check!(out == lines(&["build-123 done (×2)"]));
}

#[test]
fn masks_a_long_hex_token() {
    // Arrange: lines differing only by a long non-numeric hex run → `<x>` mask
    let records = lines(&["hash cafef00d ok", "hash deadbeef ok"]);

    // Act
    let out = fingerprint(records, &FingerprintCfg::default());

    // Assert: both collapse via the hex placeholder
    check!(out == lines(&["hash cafef00d ok (×2)"]));
}

#[test]
fn does_not_fold_a_pure_numeric_template() {
    // Arrange: distinct numbers (a `seq`) mask to the trivial template `<n>`. Folding
    // them to `1 (×3)` would misrepresent three distinct values as a repeat.
    let records = lines(&["1", "2", "3"]);

    // Act
    let out = fingerprint(records.clone(), &FingerprintCfg::default());

    // Assert: left verbatim, not collapsed
    check!(out == records);
}

#[test]
fn does_not_fold_a_pure_id_column_in_the_whole_window() {
    // Arrange: a column of distinct hashes masks to the trivial template `<x>`; the
    // whole-window must not collapse them into one false `(×N)` row either.
    let cfg: FingerprintCfg = serde_json::from_str(r#"{"window":"whole"}"#).unwrap();
    let records = lines(&["cafef00d", "deadbeef", "0badf00d"]);

    // Act
    let out = fingerprint(records.clone(), &cfg);

    // Assert
    check!(out == records);
}

#[test]
fn still_folds_a_template_with_literal_content_around_numbers() {
    // Arrange: numbers embedded in a line with real words still fold — the literal
    // text makes the template non-trivial, so the numbers are incidental noise.
    let records = lines(&["worker 1 idle", "worker 2 idle", "worker 3 idle"]);

    // Act
    let out = fingerprint(records, &FingerprintCfg::default());

    // Assert
    check!(out == lines(&["worker 1 idle (×3)"]));
}
