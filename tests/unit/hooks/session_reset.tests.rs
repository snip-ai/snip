//! Unit tests for the `session-reset` hook, in AAA form. Compiled into
//! `snip_lib` via a `#[path]` include in `src/hooks/session_reset.rs`, so these
//! reach the private `remove_session` / `older_than` helpers.

// clippy::pedantic suggests `Duration::from_mins`, which needs the unstable
// `duration_constructors` feature (not on stable 1.96), so `from_secs` stays.
#![allow(clippy::duration_suboptimal_units)]

use std::env;
use std::fs;
use std::time::{Duration, SystemTime};

use assert2::check;

use super::{PRUNE_AFTER, is_stale, older_than, prune_orphans, remove_session};

#[test]
fn older_than_is_false_for_fresh_and_skewed_clocks() {
    // Arrange
    let now = SystemTime::now();

    // Act + Assert: equal mtime is fresh; an mtime in the future is never stale
    check!(!older_than(now, now, PRUNE_AFTER));
    check!(!older_than(now + Duration::from_secs(60), now, PRUNE_AFTER));
}

#[test]
fn older_than_is_true_past_the_horizon() {
    // Arrange: an mtime well beyond the prune window
    let now = SystemTime::now();
    let modified = now - PRUNE_AFTER - Duration::from_secs(60);

    // Act
    let stale = older_than(modified, now, PRUNE_AFTER);

    // Assert
    check!(stale);
}

#[test]
fn prune_orphans_keeps_fresh_session_dirs() {
    // Arrange: a just-created session cache under an isolated root
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = env::temp_dir().join(format!("snip-prune-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    let fresh = home.join("session-cache").join("sess-fresh");
    fs::create_dir_all(&fresh).unwrap();

    temp_env::with_var("SNIP_HOME", Some(&home), || {
        // Act: pruning walks the cache and drops only stale (>7d) dirs
        prune_orphans();

        // Assert: a fresh dir survives
        check!(fresh.exists());
    });

    // Cleanup
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn prune_orphans_is_a_noop_without_a_cache_root() {
    // Arrange: an isolated data root that has no session-cache dir at all
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = env::temp_dir().join(format!("snip-prune-empty-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);

    temp_env::with_var("SNIP_HOME", Some(&home), || {
        // Act: read_dir on a missing root early-returns; this must not panic
        prune_orphans();

        // Assert: still nothing there, no directories were created
        check!(!home.join("session-cache").exists());
    });

    // Cleanup
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn is_stale_is_false_for_a_fresh_dir_entry() {
    // Arrange: a real, just-created session-cache dir read back as a DirEntry
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = env::temp_dir().join(format!("snip-is-stale-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    let cache = home.join("session-cache");
    fs::create_dir_all(cache.join("sess-fresh")).unwrap();
    let entry = fs::read_dir(&cache).unwrap().next().unwrap().unwrap();

    // Act: a freshly-modified directory is well within the prune horizon
    let stale = is_stale(&entry, SystemTime::now());

    // Assert
    check!(!stale);

    // Cleanup
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn remove_session_clears_dedupe_but_keeps_spills() {
    // Arrange: one session holding both a dedupe map and a spill file
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = env::temp_dir().join(format!("snip-reset-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    let dir = home.join("session-cache").join("sess-A");
    fs::create_dir_all(&dir).unwrap();
    let dedupe = dir.join("read-dedupe.json");
    let spill = dir.join("spill-command-deadbeef.txt");
    fs::write(&dedupe, "{}").unwrap();
    fs::write(&spill, "full output").unwrap();

    temp_env::with_var("SNIP_HOME", Some(&home), || {
        // Act
        remove_session("sess-A");

        // Assert: dedupe state gone (full views served again), spill survives
        // so a breadcrumb in context stays resolvable across the compaction.
        check!(!dedupe.exists());
        check!(spill.exists());
    });

    // Cleanup
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn remove_session_removes_the_dir_when_no_spills_remain() {
    // Arrange: a session with only dedupe state (no spills)
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = env::temp_dir().join(format!("snip-reset-empty-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    let dir = home.join("session-cache").join("sess-A");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("read-dedupe.json"), "{}").unwrap();

    temp_env::with_var("SNIP_HOME", Some(&home), || {
        // Act
        remove_session("sess-A");

        // Assert: with nothing left to keep, the empty dir is reaped
        check!(!dir.exists());
    });

    // Cleanup
    let _ = fs::remove_dir_all(&home);
}
