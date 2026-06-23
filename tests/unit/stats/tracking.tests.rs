//! Unit tests for the [`Tracker`] `SQLite` store, in AAA form. Compiled into
//! `snip_lib` via a `#[path]` include in `src/stats/tracking.rs`. Uses the
//! synchronous `insert` (the detached `record_*` path spawns a child).

use std::env;
use std::fs;

use assert2::check;

use super::Tracker;
use crate::stats::Kind;
use crate::stats::event::StatEvent;

fn ev(kind: Kind, optimizer: &str, surface: &str, before: usize, after: usize) -> StatEvent {
    StatEvent {
        ts: 0,
        optimizer: optimizer.to_owned(),
        surface: surface.to_owned(),
        kind,
        before,
        after,
    }
}

#[test]
fn inserts_then_loads_events_in_order() {
    // Arrange: a unique data root so the DB never touches the real one
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = env::temp_dir().join(format!("snip-stats-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    temp_env::with_var("SNIP_HOME", Some(&home), || {
        // Act
        Tracker::insert(&ev(Kind::Saved, "read", "read", 100, 40)).unwrap();
        Tracker::insert(&ev(Kind::Induced, "overflow", "read", 25, 0)).unwrap();
        let events = Tracker::load();

        // Assert: both round-trip, insertion order preserved
        check!(events.len() == 2);
        check!(events[0].optimizer == "read");
        check!(events[0].kind == Kind::Saved);
        check!(events[0].before == 100 && events[0].after == 40);
        check!(events[1].kind == Kind::Induced);
        check!(events[1].before == 25);
    });
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn summary_reports_net_saved_minus_induced() {
    // Arrange: an isolated store with one saving and one induced cost
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = env::temp_dir().join(format!("snip-stats-net-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    temp_env::with_var("SNIP_HOME", Some(&home), || {
        Tracker::insert(&ev(Kind::Saved, "read", "read", 100, 40)).unwrap();
        Tracker::insert(&ev(Kind::Induced, "overflow", "bash", 25, 0)).unwrap();

        // Act: aggregation runs in SQL via Tracker::summary
        let summary = Tracker::summary();

        // Assert: gross 60, induced 25, NET 35 over 2 events
        check!(summary.gross_saved == 60);
        check!(summary.induced == 25);
        check!(summary.net == 35);
        check!(summary.events == 2);
    });
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn summary_is_empty_for_a_fresh_store() {
    // Arrange: an isolated, never-written store
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = env::temp_dir().join(format!("snip-stats-empty-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    temp_env::with_var("SNIP_HOME", Some(&home), || {
        // Act
        let summary = Tracker::summary();
        let events = Tracker::load();

        // Assert: zeroed totals and no events
        check!(summary.net == 0);
        check!(summary.events == 0);
        check!(events.is_empty());
    });
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn record_helpers_append_then_drain_into_the_store() {
    // Arrange: record_* append to the hot-path log (no fork); load() drains the
    // log into SQLite and reads it back, in append order.
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = env::temp_dir().join(format!("snip-stats-record-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    temp_env::with_var("SNIP_HOME", Some(&home), || {
        // Act
        Tracker::record_saved("read", "read", 100, 40);
        Tracker::record_induced("overflow", "bash", 25);
        let events = Tracker::load();

        // Assert: both persisted through the append → drain → SQLite round-trip
        check!(events.len() == 2);
        check!(events[0].kind == Kind::Saved);
        check!(events[0].optimizer == "read");
        check!(events[0].before == 100 && events[0].after == 40);
        check!(events[1].kind == Kind::Induced);
        check!(events[1].optimizer == "overflow");
        check!(events[1].before == 25);
    });
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn drain_is_idempotent_and_safe_on_an_empty_store() {
    // Arrange: an isolated, never-written store
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = env::temp_dir().join(format!("snip-stats-drain-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    temp_env::with_var("SNIP_HOME", Some(&home), || {
        // Act: draining with no append-log, twice, must not error or fabricate events
        Tracker::drain();
        Tracker::drain();

        // Assert
        check!(Tracker::load().is_empty());
    });
    let _ = fs::remove_dir_all(&home);
}
