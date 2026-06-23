//! Unit tests for the NET [`Summary`] aggregation, in AAA form. Compiled into
//! `snip_lib` via a `#[path]` include in `src/stats/summary.rs`.

use assert2::check;

use super::Summary;
use crate::stats::{Kind, StatEvent};

fn event(optimizer: &str, surface: &str, kind: Kind, before: usize, after: usize) -> StatEvent {
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
fn aggregates_gross_induced_and_net() {
    // Arrange: two savings and one induced re-read
    let events = vec![
        event("read", "read", Kind::Saved, 100, 40),     // +60
        event("search", "grep", Kind::Saved, 50, 20),    // +30
        event("overflow", "read", Kind::Induced, 25, 0), // -25
    ];

    // Act
    let summary = Summary::from_events(&events);

    // Assert
    check!(summary.gross_saved == 90);
    check!(summary.induced == 25);
    check!(summary.net == 65);
    check!(summary.events == 3);
}

#[test]
fn breaks_down_per_optimizer_and_surface() {
    // Arrange
    let events = vec![
        event("read", "read", Kind::Saved, 100, 40),
        event("search", "grep", Kind::Saved, 50, 20),
        event("overflow", "read", Kind::Induced, 25, 0),
    ];

    // Act
    let summary = Summary::from_events(&events);

    // Assert: sorted by name; the read surface nets 60 saved − 25 re-read = 35
    check!(
        summary.per_optimizer
            == vec![
                ("overflow".to_owned(), -25),
                ("read".to_owned(), 60),
                ("search".to_owned(), 30),
            ]
    );
    check!(summary.per_surface == vec![("grep".to_owned(), 30), ("read".to_owned(), 35)]);
}

#[test]
fn empty_log_is_all_zero() {
    // Arrange + Act
    let summary = Summary::from_events(&[]);

    // Assert
    check!(summary.net == 0);
    check!(summary.events == 0);
}

#[test]
fn from_aggregates_matches_from_events() {
    // Arrange: the same data, once per-event and once pre-summed per group (what
    // the SQL `GROUP BY` produces).
    let events = vec![
        event("read", "read", Kind::Saved, 100, 40),
        event("read", "read", Kind::Saved, 80, 30),
        event("overflow", "read", Kind::Induced, 25, 0),
    ];
    let groups = vec![
        (
            "read".to_owned(),
            "read".to_owned(),
            Kind::Saved,
            180,
            70,
            2,
        ),
        (
            "overflow".to_owned(),
            "read".to_owned(),
            Kind::Induced,
            25,
            0,
            1,
        ),
    ];

    // Act
    let per_event = Summary::from_events(&events);
    let per_group = Summary::from_aggregates(&groups);

    // Assert: the SQL-aggregated path is identical to the per-event path
    check!(per_group.gross_saved == per_event.gross_saved);
    check!(per_group.induced == per_event.induced);
    check!(per_group.net == per_event.net);
    check!(per_group.events == per_event.events);
    check!(per_group.per_optimizer == per_event.per_optimizer);
    check!(per_group.per_surface == per_event.per_surface);
}
