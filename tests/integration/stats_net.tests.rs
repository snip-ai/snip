//! NET stats accounting through the public API, in AAA form: a Saved event and an
//! Induced event under an isolated `SNIP_HOME` aggregate to NET = `gross_saved` −
//! `induced`. Mirrors the `gain` unit-test setup but black-box. Serialized on the
//! process-global `SNIP_HOME` via `serial_test`.

use assert2::check;
use serial_test::serial;
use snip_lib::stats::{Kind, StatEvent, Tracker};

#[test]
#[serial]
fn summary_reports_net_as_gross_saved_minus_induced() {
    // Arrange: a fresh DB under an isolated SNIP_HOME, one saving (100→40 ⇒ +60)
    // and one induced re-read cost (10 ⇒ −10).
    let home = tempfile::tempdir().unwrap();
    temp_env::with_var("SNIP_HOME", Some(home.path()), || {
        Tracker::insert(&StatEvent {
            ts: 0,
            optimizer: "read".to_owned(),
            surface: "read".to_owned(),
            kind: Kind::Saved,
            before: 100,
            after: 40,
        })
        .unwrap();
        Tracker::insert(&StatEvent {
            ts: 0,
            optimizer: "overflow".to_owned(),
            surface: "read".to_owned(),
            kind: Kind::Induced,
            before: 10,
            after: 0,
        })
        .unwrap();

        // Act
        let summary = Tracker::summary();

        // Assert: gross 60, induced 10, NET 50 over two events
        check!(summary.gross_saved == 60);
        check!(summary.induced == 10);
        check!(summary.net == 50);
        check!(summary.events == 2);
    });
}
