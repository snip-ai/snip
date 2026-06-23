//! `snip stat-record <kind> <optimizer> <surface> <before> <after>` — a direct
//! single-row `SQLite` insert utility (manual backfill / scripted use).
//!
//! The hot path no longer spawns this: `record_*` append to the events log and a
//! drain folds it into `SQLite` off the hot path (see [`super::tracking`]). This
//! subcommand remains a stable way to insert one event synchronously.

use anyhow::{Result, anyhow};

use super::event::{Kind, StatEvent};
use super::tracking::Tracker;
use crate::clock::now_secs;

/// Insert one event from the CLI args, synchronously.
///
/// # Errors
/// Returns an error on malformed args or a DB write failure.
pub fn run(args: &[String]) -> Result<()> {
    let [kind, optimizer, surface, before, after] = args else {
        return Err(anyhow!(
            "usage: stat-record <kind> <optimizer> <surface> <before> <after>"
        ));
    };
    let event = StatEvent {
        ts: now_secs(),
        optimizer: optimizer.clone(),
        surface: surface.clone(),
        kind: Kind::parse(kind).ok_or_else(|| anyhow!("bad kind: {kind}"))?,
        before: before
            .parse()
            .map_err(|_| anyhow!("bad before: {before}"))?,
        after: after.parse().map_err(|_| anyhow!("bad after: {after}"))?,
    };
    Tracker::insert(&event)
}

#[cfg(test)]
#[path = "../../tests/unit/stats/recorder.tests.rs"]
mod tests;
