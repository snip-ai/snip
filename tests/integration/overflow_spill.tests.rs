//! The overflow/spill service through the public API, in AAA form: an over-budget
//! body is elided with a recovery breadcrumb while the full body is written to a
//! session-scoped spill file under an isolated `SNIP_HOME`. Mirrors the unit test
//! but black-box. Serialized on the process-global `SNIP_HOME` via `serial_test`.

use std::fs;

use assert2::check;
use serial_test::serial;
use snip_lib::overflow::{OverflowCfg, Spill};

#[test]
fn under_budget_returns_the_body_unchanged() {
    // Arrange: comfortably within budget ⇒ no spill, no I/O
    let cfg = OverflowCfg::default();
    let body = "a small body\n".to_owned();

    // Act
    let out = Spill::apply(body.clone(), Some("s1"), "read", &cfg);

    // Assert
    check!(out == body);
}

#[test]
#[serial]
fn over_budget_elides_spills_and_leaves_a_breadcrumb() {
    // Arrange: a tiny budget forces a spill; an isolated SNIP_HOME keeps the spill
    // file off the real data root.
    let home = tempfile::tempdir().unwrap();
    temp_env::with_var("SNIP_HOME", Some(home.path()), || {
        let cfg = OverflowCfg {
            max_tokens: 4,
            ..OverflowCfg::default()
        };
        let body = "a match line\n".repeat(200);

        // Act
        let out = Spill::apply(body.clone(), Some("sess-1"), "search", &cfg);

        // Assert: the view shrank, carries the breadcrumb, and the full body is on disk
        check!(out.len() < body.len());
        check!(out.contains("output truncated"));
        let spill = fs::read_dir(home.path().join("session-cache").join("sess-1"))
            .unwrap()
            .filter_map(Result::ok)
            .find(|e| e.file_name().to_string_lossy().starts_with("spill-search-"));
        assert2::assert!(let Some(entry) = spill);
        check!(fs::read_to_string(entry.path()).unwrap() == body);
    });
}
