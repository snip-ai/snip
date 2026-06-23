//! The command pipeline through the public API, in AAA form. This is the
//! **differential harness** (`ARCHITECTURE.md` §4.7): for any line snip rewrites,
//! the visible output and exit code must match raw `sh -c` exactly when nothing
//! is optimized, and recognized output must shrink without changing the code.
//! Every test no-ops where `sh` is unavailable, so it stays CI-safe on Windows.

use std::process::{Command, Stdio};

use assert2::check;
use serial_test::serial;
use snip_lib::optimizers::command::exec::run_capture;

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

/// Lines where snip recognizes nothing — must be perfectly transparent.
const PASSTHROUGH: &[&str] = &[
    "echo hello",
    "printf 'x\ty\n'",
    "echo a && echo b",
    "echo a; echo b; echo c",
    "echo 'quoted; not | split'",
    "true && echo ok",
    "false || echo recovered",
    "echo $((1 + 2))",
    "echo a | tr a A",
];

// `run_capture` reads the process-global `SNIP_ENABLED`, which the env-mutating
// integration tests (`exec_runtime`, `config_layering`) toggle under `#[serial]`.
// These cases must join that serialization domain or they race and see snip
// disabled mid-run (no compaction).
#[test]
#[serial]
fn passthrough_is_byte_identical_to_raw_sh() {
    // Arrange
    if !sh_available() {
        return;
    }

    // Act + Assert: stdout and exit code match `sh -c` for every line
    for &cmd in PASSTHROUGH {
        let (got_out, got_code) = run_capture(cmd).expect("sh runs");
        let (want_out, want_code) = raw(cmd);
        assert!(got_out == want_out, "stdout differs for {cmd:?}");
        assert!(got_code == want_code, "exit code differs for {cmd:?}");
    }
}

#[test]
#[serial]
fn recognized_output_is_compacted_and_exit_code_preserved() {
    // Arrange: a directory of many files so `ls` overflows the truncate cap
    if !sh_available() {
        return;
    }
    let dir = std::env::temp_dir().join(format!("snip-cmd-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for i in 0..150 {
        std::fs::write(dir.join(format!("f{i:03}.rs")), "").unwrap();
    }
    let cmd = format!("ls '{}'", dir.to_string_lossy().replace('\\', "/"));

    // Act
    let (out, code) = run_capture(&cmd).expect("sh runs");
    let (raw_out, _) = raw(&cmd);

    // Assert: compacted (tagged + smaller) but the command still succeeded
    let text = String::from_utf8_lossy(&out);
    check!(text.contains("[snip: ls |"));
    check!(out.len() < raw_out.len());
    check!(code == 0);

    // Cleanup
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[serial]
fn recognized_command_with_small_output_is_transparent() {
    // Arrange: a few files → the transform can't shrink it → verbatim (no-inflation)
    if !sh_available() {
        return;
    }
    let dir = std::env::temp_dir().join(format!("snip-cmd-small-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for name in ["a.rs", "b.rs", "c.rs"] {
        std::fs::write(dir.join(name), "").unwrap();
    }
    let cmd = format!("ls '{}'", dir.to_string_lossy().replace('\\', "/"));

    // Act
    let (out, code) = run_capture(&cmd).expect("sh runs");
    let (raw_out, raw_code) = raw(&cmd);

    // Assert: identical to raw — the no-inflation guard left it untouched
    check!(out == raw_out);
    check!(code == raw_code);

    // Cleanup
    let _ = std::fs::remove_dir_all(&dir);
}
