//! The command-runtime core (`run_capture`) through the public API, in AAA form:
//! the per-optimizer-disabled bypass and the JSON / repetitive-log auto-detect
//! fallback — branches the e2e tier can't observe (the real `run` exits via
//! `process::exit`). Each test no-ops where `sh` is unavailable, so it stays
//! CI-safe on Windows.

use std::env;
use std::fs;
use std::process::{Command, Stdio};

use assert2::check;
use serial_test::serial;
use snip_lib::optimizers::command::exec::run_capture;

/// True when a POSIX `sh` is on PATH (the runtime shells out to `sh -c`).
fn sh_available() -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(":")
        .status()
        .is_ok_and(|s| s.success())
}

/// Raw `sh -c cmd` baseline: `(stdout, exit code)`.
fn raw(cmd: &str) -> (Vec<u8>, i32) {
    let out = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .stderr(Stdio::inherit())
        .output()
        .expect("sh runs");
    (out.stdout, out.status.code().unwrap_or(-1))
}

/// An isolated `SNIP_HOME` so no stray on-disk config perturbs `Config::load`.
fn isolated_home(tag: &str) -> std::path::PathBuf {
    let home = env::temp_dir().join(format!("snip-exec-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    home
}

#[test]
#[serial]
fn disabled_optimizer_runs_verbatim_and_preserves_exit_code() {
    // Arrange: master switch off via env, hermetic SNIP_HOME. A command that
    // would be auto-detected when enabled, returning a non-zero exit code.
    if !sh_available() {
        return;
    }
    let home = isolated_home("disabled");
    let cmd = concat!(
        "printf '[\\n'; ",
        "for i in $(seq 1 30); do printf '  {\"a\":1},\\n'; done; ",
        "printf ']\\n'; exit 3",
    );

    temp_env::with_vars(
        [
            ("SNIP_HOME", Some(home.to_string_lossy().into_owned())),
            ("SNIP_ENABLED", Some("0".to_owned())),
        ],
        || {
            // Act
            let (out, code) = run_capture(cmd).expect("sh runs");
            let (raw_out, raw_code) = raw(cmd);

            // Assert: identical to raw — nothing optimized — and the exit code survives
            check!(out == raw_out);
            check!(code == raw_code);
            check!(code == 3);
        },
    );

    let _ = fs::remove_dir_all(&home);
}

#[test]
#[serial]
fn unrecognized_json_array_is_autodetected_and_shrunk() {
    // Arrange: an unrecognized command whose output is a uniform JSON array of
    // objects over the 20-line auto-detect floor — the columnar TOON view is much
    // smaller than the pretty array. Enabled, hermetic SNIP_HOME.
    if !sh_available() {
        return;
    }
    let home = isolated_home("autodetect");
    let cmd = concat!(
        "printf '[\\n'; ",
        "for i in $(seq 1 24); do printf '  {\"a\":%d,\"b\":%d},\\n' \"$i\" \"$i\"; done; ",
        "printf '  {\"a\":99,\"b\":99}\\n]\\n'",
    );

    temp_env::with_vars(
        [
            ("SNIP_HOME", Some(home.to_string_lossy().into_owned())),
            ("SNIP_ENABLED", None),
        ],
        || {
            // Act
            let (out, code) = run_capture(cmd).expect("sh runs");
            let (raw_out, _) = raw(cmd);

            // Assert: recognized as JSON and rewritten strictly smaller; exit 0 preserved
            let text = String::from_utf8_lossy(&out);
            check!(text.contains("a,b"));
            check!(out.len() < raw_out.len());
            check!(code == 0);
        },
    );

    let _ = fs::remove_dir_all(&home);
}

#[test]
#[serial]
fn unrecognized_nonrepetitive_output_stays_byte_identical() {
    // Arrange: an unrecognized command whose output is distinct non-JSON lines, so
    // fingerprinting collapses nothing and the buffer passes through untouched.
    if !sh_available() {
        return;
    }
    let home = isolated_home("plain");
    let cmd = "printf '%s\\n' spring summer winter morning evening meadow glimmer \
        thunder willow pioneer voyage lantern compass horizon gravel kindling murmur \
        twilight prism quill ripple tundra unwind vortex whisper";

    temp_env::with_vars(
        [
            ("SNIP_HOME", Some(home.to_string_lossy().into_owned())),
            ("SNIP_ENABLED", None),
        ],
        || {
            // Act
            let (out, code) = run_capture(cmd).expect("sh runs");
            let (raw_out, raw_code) = raw(cmd);

            // Assert: byte-identical to raw — the no-collapse path is transparent
            check!(out == raw_out);
            check!(code == raw_code);
        },
    );

    let _ = fs::remove_dir_all(&home);
}

#[test]
#[serial]
fn unrecognized_repetitive_log_is_folded_and_shrunk() {
    // Arrange: an unrecognized command whose output is a repetitive log over the
    // 20-line floor — each line differs only in a digit run, so the fingerprinter
    // collapses them to one counted template. Enabled, hermetic SNIP_HOME.
    if !sh_available() {
        return;
    }
    let home = isolated_home("replog");
    let cmd = "for i in $(seq 1 30); do echo \"GET /api/v1/users/$i 200 OK\"; done";

    temp_env::with_vars(
        [
            ("SNIP_HOME", Some(home.to_string_lossy().into_owned())),
            ("SNIP_ENABLED", None),
        ],
        || {
            // Act
            let (out, code) = run_capture(cmd).expect("sh runs");
            let (raw_out, _) = raw(cmd);

            // Assert: folded to a counted template, strictly smaller; exit 0 preserved
            let text = String::from_utf8_lossy(&out);
            check!(text.contains("(×30)"));
            check!(out.len() < raw_out.len());
            check!(code == 0);
        },
    );

    let _ = fs::remove_dir_all(&home);
}
