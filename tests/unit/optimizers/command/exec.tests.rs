//! Unit tests for the `exec` runtime, in AAA form. Compiled into `snip_lib` via
//! a `#[path]` include in `src/optimizers/command/exec.rs`. The process-spawning tests
//! no-op where `sh` is unavailable so they stay CI-safe on Windows.

use std::process::Command;

use assert2::check;

use super::{decode, run_capture, strip_token};
use crate::optimizers::command::b64;

fn sh_available() -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(":")
        .status()
        .is_ok_and(|s| s.success())
}

#[test]
fn decode_reads_the_payload_after_the_dashes() {
    // Arrange
    let args = vec!["--".to_owned(), b64::encode(b"echo hi")];

    // Act
    assert2::assert!(let Ok(cmd) = decode(&args));

    // Assert
    check!(cmd == "echo hi");
}

#[test]
fn decode_errs_without_a_payload() {
    // Arrange + Act + Assert
    check!(decode(&["--".to_owned()]).is_err());
}

#[test]
fn strip_token_removes_every_marker() {
    // Arrange + Act + Assert
    let stripped = strip_token(b"\x01M\x01a\x01M\x01b", "\u{1}M\u{1}");
    check!(stripped == b"ab".to_vec());
}

#[test]
fn passthrough_is_transparent_and_preserves_exit_codes() {
    // Arrange: unrecognized commands run verbatim
    if !sh_available() {
        return;
    }

    // Act
    let (out, code) = run_capture("printf 'a\nb\n'").expect("sh runs");
    let (_, fail) = run_capture("exit 3").expect("sh runs");

    // Assert
    check!(out == b"a\nb\n");
    check!(code == 0);
    check!(fail == 3);
}

#[test]
fn multi_segment_sequence_runs_each_segment_in_order() {
    // Arrange
    if !sh_available() {
        return;
    }

    // Act: `&&` chaining is preserved by the real shell
    let (out, code) = run_capture("echo one && echo two").expect("sh runs");

    // Assert
    check!(out == b"one\ntwo\n");
    check!(code == 0);
}

#[test]
fn lossy_autodetect_fold_spills_the_dropped_lines_but_a_lossless_fold_does_not() {
    // Arrange: sh + an isolated data root + a session so any spill has a home
    if !sh_available() {
        return;
    }
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = std::env::temp_dir().join(format!("snip-exec-fold-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    temp_env::with_vars(
        [
            ("SNIP_HOME", Some(home.to_string_lossy().into_owned())),
            ("SNIP_SESSION", Some("sess-fold".to_owned())),
            ("SNIP_ENABLED", Some("1".to_owned())),
        ],
        || {
            // Act: a lossy fold (25 paths differing in a digit run, POSIX sh — no
            // `seq` dependency) then a lossless one (25 byte-identical lines)
            let (lossy, _) = run_capture(
                "i=0; while [ $i -lt 25 ]; do echo \"/src/module_$i/file_$i.rs\"; i=$((i+1)); done",
            )
            .expect("sh runs");
            let _ = run_capture("i=0; while [ $i -lt 25 ]; do echo same-line; i=$((i+1)); done")
                .expect("sh runs");

            // Assert: the lossy view folds, and exactly one spill (the lossy one) holds
            // every dropped distinct line — the lossless fold spilled nothing.
            check!(String::from_utf8_lossy(&lossy).contains("(×25)"));
            let dir = home.join("session-cache").join("sess-fold");
            let spills: Vec<_> = std::fs::read_dir(&dir)
                .expect("session cache dir exists")
                .filter_map(Result::ok)
                .filter(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .starts_with("spill-command-autodetect-")
                })
                .collect();
            check!(spills.len() == 1);
            let body = std::fs::read_to_string(spills[0].path()).unwrap();
            check!(body.contains("/src/module_0/file_0.rs"));
            check!(body.contains("/src/module_24/file_24.rs"));
            check!(!body.contains("same-line"));
        },
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn large_unrecognized_output_is_capped_and_spilled() {
    if !sh_available() {
        return;
    }
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = std::env::temp_dir().join(format!("snip-exec-cap-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    temp_env::with_vars(
        [
            ("SNIP_HOME", Some(home.to_string_lossy().into_owned())),
            ("SNIP_SESSION", Some("sess-cap".to_owned())),
            ("SNIP_ENABLED", Some("1".to_owned())),
        ],
        || {
            // ~6000 lines of distinct alpha-word content: not JSON, not losslessly
            // foldable — the exact "unrecognized, non-repetitive" case that used to
            // pass through raw and flood context (~280 KB) with no cap.
            let (out, code) = run_capture(
                "awk 'BEGIN{for(i=0;i<6000;i++){s=\"\";n=i+1000000;\
                 while(n>0){s=s sprintf(\"%c\",97+(n%26));n=int(n/26)};\
                 print s\" \"s\" \"s\" unique structural payload row\"}}'",
            )
            .expect("sh runs");
            let shown = String::from_utf8_lossy(&out);

            check!(code == 0);
            // snip rewrote it (left a recoverable [snip: …] breadcrumb) …
            check!(shown.contains("[snip:"));
            // … capped the shown view far below the ~280 KB it generated …
            check!(out.len() < 60_000);
            // … and spilled the full output recoverably.
            let dir = home.join("session-cache").join("sess-cap");
            let spills = std::fs::read_dir(&dir)
                .expect("session cache dir exists")
                .filter_map(Result::ok)
                .filter(|e| e.file_name().to_string_lossy().starts_with("spill-command"))
                .count();
            check!(spills >= 1);
        },
    );
    let _ = std::fs::remove_dir_all(&home);
}
