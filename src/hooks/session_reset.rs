//! `PreCompact` hook: reset the session's dedupe state, but keep overflow spills.
//!
//! After a context compaction, re-reads must serve the full view again, so the
//! dedupe state must never outlive a compaction. Spill files are deliberately
//! **kept**: a recovery breadcrumb already in the model's context must stay
//! resolvable across the compaction boundary (the spills are LRU-capped per
//! session and orphan-pruned after a week, so keeping them is still bounded).
//! Runs even when snip is disabled. Also prunes orphaned session caches left by
//! sessions that never compacted.

use std::fs::{self, DirEntry};
use std::io::Read;
use std::time::{Duration, SystemTime};

use serde_json::Value;

use crate::overflow::Spill;
use crate::paths::{SESSION_CACHE_DIRNAME, data_dir, session_cache_dir};

/// Orphan session caches older than this are pruned on any `PreCompact`.
// clippy::pedantic suggests `Duration::from_days`, which needs the unstable
// `duration_constructors` feature (not on stable 1.96), so `from_secs` stays.
#[allow(clippy::duration_suboptimal_units)]
const PRUNE_AFTER: Duration = Duration::from_secs(7 * 24 * 3600);

/// Run the session-reset hook. In production always succeeds (exit-0 invariant):
/// any failure degrades to a no-op. Strict debug mode
/// ([`crate::panic_guard::strict`]) surfaces a panic as a non-zero exit.
///
/// # Errors
/// Only under `SNIP_DEBUG` (strict mode); otherwise never.
pub fn run() -> anyhow::Result<()> {
    // This maintenance hook bypasses the Dispatcher, so guard it here too — a panic
    // must never break exit-0 in production (strict mode surfaces it for debugging).
    crate::panic_guard::guarded("session-reset", || {
        reset();
        Ok(())
    })
}

/// Remove this session's cache dir, then prune stale orphans.
fn reset() {
    if let Some(id) = read_session_id() {
        remove_session(&id);
    }
    prune_orphans();
}

/// Clear one session's dedupe state while keeping its spill files, best-effort.
///
/// Every non-spill file (the `read-dedupe.json` map and any cached previous-read
/// content) is removed so re-reads serve full views again; spill files survive so
/// breadcrumbs already in context stay resolvable. The now-(possibly-)empty dir is
/// removed only if nothing is left — a spill keeps it alive for the orphan pruner.
fn remove_session(session_id: &str) {
    let Some(dir) = session_cache_dir(Some(session_id)) else {
        return;
    };
    let Ok(entries) = fs::read_dir(&dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let is_spill = path.to_str().is_some_and(Spill::is_spill_path);
        if !is_spill && entry.file_type().is_ok_and(|t| t.is_file()) {
            let _ = fs::remove_file(&path);
        }
    }
    // Succeeds only if no spill files remain; otherwise the pruner reaps it later.
    let _ = fs::remove_dir(&dir);
}

/// Read the `PreCompact` hook's `session_id` from stdin (best-effort).
fn read_session_id() -> Option<String> {
    let mut raw = String::new();
    std::io::stdin().read_to_string(&mut raw).ok()?;
    let hook: Value = serde_json::from_str(&raw).ok()?;
    hook.get("session_id")?.as_str().map(str::to_owned)
}

/// Remove session-cache subdirs whose mtime is older than [`PRUNE_AFTER`].
fn prune_orphans() {
    let Some(root) = data_dir().map(|d| d.join(SESSION_CACHE_DIRNAME)) else {
        return;
    };
    let Ok(entries) = fs::read_dir(&root) else {
        return;
    };
    let now = SystemTime::now();
    for entry in entries.filter_map(Result::ok) {
        if is_stale(&entry, now) {
            let _ = fs::remove_dir_all(entry.path());
        }
    }
}

/// Whether `entry` is a directory last modified before the prune horizon.
fn is_stale(entry: &DirEntry, now: SystemTime) -> bool {
    let Ok(meta) = entry.metadata() else {
        return false;
    };
    meta.is_dir()
        && meta
            .modified()
            .is_ok_and(|m| older_than(m, now, PRUNE_AFTER))
}

/// Whether `modified` is more than `max` before `now` (clock skew ⇒ `false`).
fn older_than(modified: SystemTime, now: SystemTime, max: Duration) -> bool {
    now.duration_since(modified).is_ok_and(|age| age > max)
}

#[cfg(test)]
#[path = "../../tests/unit/hooks/session_reset.tests.rs"]
mod tests;
