//! Unit tests for the detached stats [`run`] recorder, in AAA form. Compiled into
//! `snip_lib` via a `#[path]` include in `src/stats/recorder.rs`.

use std::env;
use std::fs;

use assert2::check;

use super::run;
use crate::stats::Tracker;

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn records_an_event_from_cli_args() {
    // Arrange: isolated data root
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = env::temp_dir().join(format!("snip-recorder-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    temp_env::with_var("SNIP_HOME", Some(&home), || {
        // Act: the exact args the detached spawn would pass
        let ok = run(&args(&["saved", "command", "bash", "200", "80"]));
        let events = Tracker::load();

        // Assert
        check!(ok.is_ok());
        check!(events.len() == 1);
        check!(events[0].optimizer == "command");
        check!(events[0].before == 200 && events[0].after == 80);
    });
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn rejects_malformed_args() {
    // Arrange + Act + Assert: wrong arity / bad kind never panics, returns Err
    check!(run(&args(&["saved", "x"])).is_err());
    check!(run(&args(&["nope", "o", "s", "1", "0"])).is_err());
}

#[test]
fn rejects_unparseable_token_counts() {
    // Arrange: correct arity and kind, but the numeric fields aren't integers
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = env::temp_dir().join(format!("snip-recorder-badnum-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    temp_env::with_var("SNIP_HOME", Some(&home), || {
        // Act + Assert: a non-numeric before/after is rejected (Err), never persisted
        check!(run(&args(&["saved", "o", "s", "nan", "0"])).is_err());
        check!(run(&args(&["saved", "o", "s", "10", "oops"])).is_err());
        check!(Tracker::load().is_empty());
    });
    let _ = fs::remove_dir_all(&home);
}
