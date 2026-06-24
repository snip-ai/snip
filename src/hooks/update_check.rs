//! `SessionStart` hook: keep the managed binary on the latest release.
//!
//! snip is installed and updated **only** through the Claude Code plugin. This
//! hook re-runs the bootstrap **detached** so it resolves the latest GitHub
//! release and, when the running binary is older, fetches the matching binary
//! for the next session. The fetch target is the release, **not** the plugin
//! manifest version: a third-party marketplace does not auto-refresh the
//! manifest, so the binary must self-heal independently of it. It also drains the
//! hot-path stats append-log into `SQLite` (off the hot path — `SessionStart` is
//! not a tool hook), keeping the log bounded between `gain`/`status` reads. It
//! never blocks startup and writes nothing to stdout.

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
/// `force` skips the once-a-day throttle — set by the manual `/snip-update`
/// command so a user can pull a fresh release immediately; the `SessionStart`
/// hook leaves it `false`.
///
/// # Errors
/// Only under `SNIP_DEBUG` (strict mode); otherwise never.
pub fn run(force: bool) -> anyhow::Result<()> {
    // Defense-in-depth: this maintenance hook bypasses the Dispatcher, so guard it
    // here too — a panic must never break exit-0 in production.
    crate::panic_guard::guarded("update-check", || {
        // Off the hot path (SessionStart is not a tool hook): fold the stats
        // append-log into SQLite so it stays bounded even if `gain`/`status` are
        // never run. Best-effort.
        crate::stats::Tracker::drain();
        let _ = reconcile(force);
        Ok(())
    })
}

/// Re-bootstrap so the managed binary tracks the latest release.
///
/// The fetch target is the latest GitHub release (resolved by the bootstrap),
/// not the plugin manifest version: a third-party marketplace does not
/// auto-refresh the manifest, so trusting it would pin the binary to whatever
/// version was first installed. The bootstrap is handed the running binary's
/// version and skips the download when it already matches the latest.
fn reconcile(force: bool) -> Option<()> {
    let plugin_root = std::env::var("CLAUDE_PLUGIN_ROOT").ok()?;
    if !force && throttled() {
        return Some(());
    }
    touch_state();
    let script = PathBuf::from(&plugin_root)
        .join("scripts")
        .join("snip-bootstrap.sh");
    let home = crate::paths::data_dir()?;
    if script.exists() {
        spawn_bootstrap(&script, &home);
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

/// Spawn the bootstrap script detached (null stdio, no console) so the
/// `SessionStart` hook returns immediately. The empty version argument tells the
/// bootstrap to resolve the latest release; the current binary version lets it
/// skip the download when already up to date.
fn spawn_bootstrap(script: &Path, home: &Path) {
    let mut cmd = Command::new("bash");
    cmd.arg(script)
        .arg("")
        .arg(home)
        .arg(env!("CARGO_PKG_VERSION"))
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
