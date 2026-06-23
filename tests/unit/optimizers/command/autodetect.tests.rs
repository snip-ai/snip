//! Unit tests for [`compact`] auto-detection, in AAA form. Compiled into
//! `snip_lib` via a `#[path]` include in `src/optimizers/command/autodetect.rs`.

use std::fmt::Write as _;

use assert2::check;

use super::{compact, fold_is_lossy, is_json_shaped};
use crate::config::AutodetectCfg;

#[test]
fn json_shape_detection_distinguishes_json_from_logs() {
    // Arrange + Act + Assert: leading `{`/`[` (after whitespace) is JSON; else a log
    check!(is_json_shaped("{\"a\":1}"));
    check!(is_json_shaped("  \n[1,2,3]"));
    check!(!is_json_shaped("GET /api/v1/users/1 200 OK"));
    check!(!is_json_shaped("/src/module_0/file_0.rs"));
}

#[test]
fn fold_is_lossy_flags_distinct_lines_but_not_identical_ones() {
    // Arrange: a fold of 25 paths differing in a digit run drops 24 distinct lines;
    // a fold of 25 byte-identical lines loses nothing (the count reconstructs them).
    let distinct = (0..25)
        .map(|i| format!("/src/module_{i}/file_{i}.rs"))
        .collect::<Vec<_>>()
        .join("\n");
    let identical = "tick\n".repeat(25);

    // Act + Assert: only the distinct-line fold is lossy and must be spilled
    check!(fold_is_lossy("/src/module_0/file_0.rs (×25)", &distinct));
    check!(!fold_is_lossy("tick (×25)", &identical));
}

#[test]
fn minifies_a_large_pretty_json_object() {
    // Arrange: a pretty JSON object over the min_lines threshold
    let mut src = String::from("{\n");
    for i in 0..30 {
        let _ = writeln!(src, "  \"k{i}\": {i},");
    }
    src.push_str("  \"end\": true\n}\n");

    // Act
    let out = compact(&src, &AutodetectCfg::default(), false);

    // Assert: minified to one line, strictly smaller
    assert2::assert!(let Some(view) = out);
    check!(!view.contains("\n  "));
    check!(view.len() < src.len());
}

#[test]
fn ndjson_objects_are_autodetected_as_a_columnar_table() {
    // Arrange: 30 line-delimited JSON objects (NDJSON) sharing a uniform key set.
    // Joined, they are not one document (so minify can't help), but they re-shape
    // into a TOON header + value rows that drops the repeated keys.
    let mut src = String::new();
    for i in 0..30 {
        let _ = writeln!(
            src,
            "{{\"method\":\"GET\",\"path\":\"/api/users/{i}\",\"status\":200}}"
        );
    }

    // Act
    let out = compact(&src, &AutodetectCfg::default(), false);

    // Assert: a value row in table form (keys dropped), strictly smaller
    assert2::assert!(let Some(view) = out);
    check!(view.contains("GET,/api/users/0,200"));
    check!(!view.contains("\"method\""));
    check!(view.len() < src.len());
}

#[test]
fn skips_short_output() {
    // Arrange: JSON, but under min_lines
    let out = compact("{\n  \"a\": 1\n}\n", &AutodetectCfg::default(), false);

    // Assert
    check!(out.is_none());
}

#[test]
fn folds_a_repetitive_plain_log() {
    // Arrange: 30 non-JSON lines differing only in a digit run — the fingerprinter
    // masks the digits and collapses them to one template with an occurrence count.
    let mut src = String::new();
    for i in 0..30 {
        let _ = writeln!(src, "GET /api/v1/users/{i} 200 OK");
    }

    // Act
    let out = compact(&src, &AutodetectCfg::default(), false);

    // Assert: collapsed to a counted template, strictly smaller
    assert2::assert!(let Some(view) = out);
    check!(view.contains("(×30)"));
    check!(view.len() < src.len());
}

#[test]
fn keeps_non_repetitive_plain_output_verbatim() {
    // Arrange: 25 distinct non-JSON lines (no maskable run repeats), so
    // fingerprinting collapses nothing and the output must pass through untouched.
    const WORDS: [&str; 25] = [
        "spring", "summer", "winter", "morning", "evening", "meadow", "glimmer", "thunder",
        "willow", "pioneer", "voyage", "lantern", "compass", "horizon", "gravel", "kindling",
        "murmur", "twilight", "prism", "quill", "ripple", "tundra", "unwind", "vortex", "whisper",
    ];
    let src = format!("{}\n", WORDS.join("\n"));

    // Act
    let out = compact(&src, &AutodetectCfg::default(), false);

    // Assert: nothing collapsed → verbatim (None)
    check!(out.is_none());
}

#[test]
fn log_disabled_keeps_repetitive_output_verbatim() {
    // Arrange: log auto-detect off, with output that would otherwise fold
    let cfg: AutodetectCfg = serde_json::from_str(r#"{"log":false}"#).unwrap();
    let mut src = String::new();
    for i in 0..30 {
        let _ = writeln!(src, "GET /api/v1/users/{i} 200 OK");
    }

    // Act + Assert: the log toggle gates the fold
    check!(compact(&src, &cfg, false).is_none());
}

#[test]
fn disabled_config_returns_none() {
    // Arrange
    let cfg: AutodetectCfg = serde_json::from_str(r#"{"enabled":false}"#).unwrap();
    let src = format!("[\n{}  {{\"a\":1}}\n]\n", "  {\"a\":1},\n".repeat(30));

    // Act + Assert
    check!(compact(&src, &cfg, false).is_none());
}

#[test]
fn masks_secrets_and_never_falls_back_to_unmasked_when_secret_safe() {
    // Arrange: a JSON array over min_lines whose values carry credentials
    let mut src = String::from("[\n");
    for _ in 0..30 {
        src.push_str("  {\"token\": \"ghp_0123456789abcdefABCDEF0123456789abcd\"},\n");
    }
    src.push_str("  {\"token\": \"ghp_0123456789abcdefABCDEF0123456789abcd\"}\n]\n");

    // Act: same input with secret_safe off vs on
    let plain = compact(&src, &AutodetectCfg::default(), false);
    let safe = compact(&src, &AutodetectCfg::default(), true);

    // Assert: the secret leaks without secret_safe but is masked with it, and the
    // masked view is emitted (never None → the unmasked verbatim buffer).
    assert2::assert!(let Some(plain) = plain);
    check!(plain.contains("ghp_0123456789abcdefABCDEF0123456789abcd"));
    assert2::assert!(let Some(safe) = safe);
    check!(!safe.contains("ghp_0123456789abcdefABCDEF0123456789abcd"));
}

#[test]
fn masks_secrets_on_the_log_path() {
    // Arrange: non-JSON repetitive lines, each carrying a credential and differing
    // only in a trailing digit run (so they fold to one template).
    let mut src = String::new();
    for i in 0..30 {
        let _ = writeln!(
            src,
            "auth token=ghp_0123456789abcdefABCDEF0123456789abcd for user {i}"
        );
    }

    // Act: secret_safe off (leaks) vs on (masked)
    let plain = compact(&src, &AutodetectCfg::default(), false);
    let safe = compact(&src, &AutodetectCfg::default(), true);

    // Assert: the fold leaks the credential without secret_safe but masks it with it
    assert2::assert!(let Some(plain) = plain);
    check!(plain.contains("ghp_0123456789abcdefABCDEF0123456789abcd"));
    assert2::assert!(let Some(safe) = safe);
    check!(!safe.contains("ghp_0123456789abcdefABCDEF0123456789abcd"));
}
