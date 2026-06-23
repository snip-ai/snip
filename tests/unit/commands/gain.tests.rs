//! Unit tests for `snip gain`, in AAA form. Compiled into `snip_lib` via a
//! `#[path]` include in `src/commands/gain.rs`.

use std::env;
use std::fs;

use assert2::check;

use super::run;

#[test]
fn run_is_ok_on_an_empty_log() {
    // Arrange: an isolated, empty data root
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = env::temp_dir().join(format!("snip-gain-empty-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    temp_env::with_var("SNIP_HOME", Some(&home), || {
        // Act + Assert: the "nothing recorded yet" branch
        check!(run().is_ok());
    });

    // Cleanup
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn run_is_ok_with_recorded_events() {
    // Arrange: a log with a saving and an induced re-read drives the full report
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = env::temp_dir().join(format!("snip-gain-full-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    temp_env::with_var("SNIP_HOME", Some(&home), || {
        crate::stats::Tracker::insert(&crate::stats::StatEvent {
            ts: 0,
            optimizer: "read".to_owned(),
            surface: "read".to_owned(),
            kind: crate::stats::Kind::Saved,
            before: 100,
            after: 40,
        })
        .unwrap();
        crate::stats::Tracker::insert(&crate::stats::StatEvent {
            ts: 0,
            optimizer: "overflow".to_owned(),
            surface: "read".to_owned(),
            kind: crate::stats::Kind::Induced,
            before: 10,
            after: 0,
        })
        .unwrap();

        // Act + Assert: the breakdown branch (per optimizer + per surface)
        check!(run().is_ok());
    });

    // Cleanup
    let _ = fs::remove_dir_all(&home);
}
