//! Shared e2e harness: drive the built `snip` binary the way Claude Code does.
//!
//! Each [`Snip`] roots the binary at a throwaway `SNIP_HOME`, so on-disk config,
//! the stats DB, and the session cache never touch the real data dir or another
//! test's. Tests feed a hook's JSON on stdin and assert the JSON on stdout plus
//! the always-exit-0 invariant — the exact contract from `hook-protocol.md`.

use std::path::Path;
use std::process::Output;

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

/// A `snip` invocation rooted at an isolated, auto-cleaned `SNIP_HOME`.
pub struct Snip {
    home: TempDir,
}

impl Snip {
    /// A fresh isolated snip environment (its own data root).
    #[must_use]
    pub fn fresh() -> Self {
        Self {
            home: tempfile::tempdir().expect("a temp SNIP_HOME"),
        }
    }

    /// This invocation's data root — assert on-disk side effects through it.
    #[must_use]
    pub fn home(&self) -> &Path {
        self.home.path()
    }

    /// A `snip` command with the isolated `SNIP_HOME` and every `SNIP_*` / plugin
    /// override cleared, so only what a test sets is in effect.
    #[must_use]
    pub fn command(&self) -> Command {
        let mut cmd = Command::cargo_bin("snip").expect("the built snip binary");
        cmd.env("SNIP_HOME", self.home.path())
            .env_remove("SNIP_ENABLED")
            .env_remove("SNIP_DEBUG")
            .env_remove("SNIP_CONFIG_PATH")
            .env_remove("SNIP_WRAPPED")
            .env_remove("SNIP_SESSION")
            .env_remove("CLAUDE_PLUGIN_ROOT");
        cmd
    }

    /// Run `snip <args…>` feeding `stdin`, returning the captured output.
    #[must_use]
    pub fn run(&self, args: &[&str], stdin: &str) -> Output {
        self.command()
            .args(args)
            .write_stdin(stdin.to_owned())
            .output()
            .expect("snip runs")
    }
}

/// Parse a captured stdout as a single JSON value (panics if it is not JSON).
#[must_use]
pub fn stdout_json(out: &Output) -> Value {
    serde_json::from_slice(&out.stdout).expect("stdout is one JSON value")
}

/// A captured stdout as a lossy `String`.
#[must_use]
pub fn stdout_str(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Whether a real POSIX `sh` is on `PATH` — the command-runtime e2e cases no-op
/// where it is absent so the suite stays CI-safe on a bare Windows runner.
#[must_use]
pub fn sh_available() -> bool {
    std::process::Command::new("sh")
        .arg("-c")
        .arg(":")
        .status()
        .is_ok_and(|s| s.success())
}
