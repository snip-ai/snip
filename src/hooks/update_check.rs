//! `SessionStart` hook: keep the managed binary in lockstep with the plugin.
//!
//! snip is installed and updated **only** through the Claude Code plugin. This
//! hook reconciles the running binary's version against the plugin's declared
//! version and, on drift, re-runs the bootstrap **detached** so the next session
//! uses the matching binary. It also drains the hot-path stats append-log into
//! `SQLite` (off the hot path — `SessionStart` is not a tool hook), keeping the log
//! bounded between `gain`/`status` reads. It never blocks startup and writes
//! nothing to stdout.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::clock::now_secs;

/// Once-a-day throttle so a session never stats/spawns more than necessary.
const THROTTLE_SECS: u64 = 24 * 3600;

/// Run the update-check hook. In production always succeeds (exit-0 invariant):
/// any failure degrades to a no-op. Strict debug mode
/// ([`crate::panic_guard::strict`]) surfaces a panic as a non-zero exit.
///
/// # Errors
/// Only under `SNIP_DEBUG` (strict mode); otherwise never.
pub fn run() -> anyhow::Result<()> {
    // Defense-in-depth: this maintenance hook bypasses the Dispatcher, so guard it
    // here too — a panic must never break exit-0 in production.
    crate::panic_guard::guarded("update-check", || {
        // Off the hot path (SessionStart is not a tool hook): fold the stats
        // append-log into SQLite so it stays bounded even if `gain`/`status` are
        // never run. Best-effort.
        crate::stats::Tracker::drain();
        let _ = reconcile();
        Ok(())
    })
}

/// Re-bootstrap the managed binary when its version drifts from the plugin's.
fn reconcile() -> Option<()> {
    let plugin_root = std::env::var("CLAUDE_PLUGIN_ROOT").ok()?;
    if throttled() {
        return Some(());
    }
    touch_state();
    let want = plugin_version(&plugin_root)?;
    if want == env!("CARGO_PKG_VERSION") {
        return Some(());
    }
    let script = PathBuf::from(&plugin_root)
        .join("scripts")
        .join("snip-bootstrap.sh");
    let home = crate::paths::data_dir()?;
    if script.exists() {
        spawn_bootstrap(&script, &want, &home);
    }
    Some(())
}

fn state_path() -> Option<PathBuf> {
    Some(crate::paths::data_dir()?.join(".update-check"))
}

/// Whether the last check was within [`THROTTLE_SECS`].
fn throttled() -> bool {
    let Some(path) = state_path() else {
        return false;
    };
    let Ok(text) = fs::read_to_string(&path) else {
        return false;
    };
    let Ok(last) = text.trim().parse::<u64>() else {
        return false;
    };
    now_secs().saturating_sub(last) < THROTTLE_SECS
}

/// Record "checked now" so the throttle holds until the next window.
fn touch_state() {
    let Some(path) = state_path() else {
        return;
    };
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    let _ = fs::write(&path, now_secs().to_string());
}

/// Read the plugin's declared version from `<root>/.claude-plugin/plugin.json`.
fn plugin_version(plugin_root: &str) -> Option<String> {
    let path = PathBuf::from(plugin_root)
        .join(".claude-plugin")
        .join("plugin.json");
    let text = fs::read_to_string(path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    value.get("version")?.as_str().map(str::to_owned)
}

/// Spawn the bootstrap script detached (null stdio, no console) so the
/// `SessionStart` hook returns immediately.
fn spawn_bootstrap(script: &Path, version: &str, home: &Path) {
    let mut cmd = Command::new("bash");
    cmd.arg(script)
        .arg(version)
        .arg(home)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    detach(&mut cmd);
    let _ = cmd.spawn();
}

#[cfg(windows)]
fn detach(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    // DETACHED_PROCESS | CREATE_NO_WINDOW
    cmd.creation_flags(0x0800_0008);
}

#[cfg(not(windows))]
#[allow(clippy::missing_const_for_fn)] // no-op, parity with the Windows arm (which can't be const)
fn detach(_cmd: &mut Command) {}

#[cfg(test)]
#[path = "../../tests/unit/hooks/update_check.tests.rs"]
mod tests;
