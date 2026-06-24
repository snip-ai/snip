//! `SessionStart` update-check: drain stats and flag whether a self-update is due.
//!
//! The actual fetch is spawned by `snip-run.sh` (bash), **never from here**: a
//! native binary can't spawn a detached shell that survives its own exit on
//! Windows (the whole MSYS subtree dies with the non-cygwin root), so the
//! bootstrap must run from the bash hook. This subcommand drains the hot-path
//! stats append-log into `SQLite` (off the hot path — `SessionStart` is not a
//! tool hook) and, when the once-a-day throttle has elapsed, drops a `.fetch-due`
//! sentinel that `snip-run.sh` consumes to spawn the bootstrap. It never blocks
//! startup, writes nothing to stdout, and always exits 0.

use std::fs;
use std::path::PathBuf;

use crate::clock::now_secs;

/// Once-a-day throttle so a session never re-checks the release more than needed.
const THROTTLE_SECS: u64 = 24 * 3600;

/// Run the update-check hook: drain stats, then flag a self-update when due.
///
/// When the once-a-day throttle has elapsed, records "checked now" and drops the
/// `.fetch-due` sentinel for `snip-run.sh` to act on. In production always succeeds
/// (exit-0 invariant); any failure degrades to a no-op.
///
/// # Errors
/// Only under `SNIP_DEBUG` strict mode ([`crate::panic_guard::strict`]); otherwise
/// never.
pub fn run() -> anyhow::Result<()> {
    // Defense-in-depth: this maintenance hook bypasses the Dispatcher, so guard it
    // here too — a panic must never break exit-0 in production.
    crate::panic_guard::guarded("update-check", || {
        crate::stats::Tracker::drain();
        if !throttled() {
            touch_state();
            flag_fetch_due();
        }
        Ok(())
    })
}

fn state_path() -> Option<PathBuf> {
    Some(crate::paths::data_dir()?.join(".update-check"))
}

/// The sentinel `snip-run.sh` consumes to spawn the bootstrap (bash can spawn a
/// surviving shell; the native binary can't). KEEP IN SYNC with `scripts/snip-run.sh`.
fn fetch_due_path() -> Option<PathBuf> {
    Some(crate::paths::data_dir()?.join(".fetch-due"))
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

/// Drop the `.fetch-due` sentinel so `snip-run.sh` spawns the bootstrap next.
fn flag_fetch_due() {
    let Some(path) = fetch_due_path() else {
        return;
    };
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    let _ = fs::write(&path, b"");
}

#[cfg(test)]
#[path = "../../tests/unit/hooks/update_check.tests.rs"]
mod tests;
