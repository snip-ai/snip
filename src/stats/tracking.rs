//! Stats store: a hot-path append-log drained into `SQLite` off the hot path.
//!
//! A tool hook never opens `SQLite` and never forks: `record_*` append one line to
//! `<data_dir>/events.log` — a single `O_APPEND` write, so concurrent sessions
//! don't interleave and the per-call cost is one tiny file write (no process
//! spawn). [`Tracker::drain`] folds the log into `SQLite`; it runs only off the hot
//! path — from `gain`/`status` (via `summary`/`load`) and the `SessionStart`
//! update-check. `insert` is the synchronous single-row write used by the drain,
//! the `stat-record` utility, and tests.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

use super::db::connect;
use super::event::{Kind, StatEvent};
use super::summary::{Aggregate, Summary};
use crate::clock::now_secs;
use crate::paths::data_dir;

/// Records and loads optimization events.
pub struct Tracker;

impl Tracker {
    /// Record tokens saved by an optimization (hot-path append, best-effort).
    pub fn record_saved(optimizer: &str, surface: &str, before: usize, after: usize) {
        append(Kind::Saved, optimizer, surface, before, after);
    }

    /// Record the induced cost of re-reading a spilled output (hot-path append).
    pub fn record_induced(optimizer: &str, surface: &str, cost: usize) {
        append(Kind::Induced, optimizer, surface, cost, 0);
    }

    /// Insert one event synchronously (used by the drain, `stat-record`, and tests).
    ///
    /// # Errors
    /// Returns an error if the DB can't be opened or written.
    pub fn insert(event: &StatEvent) -> anyhow::Result<()> {
        let conn = connect()?;
        conn.execute(
            "INSERT INTO events (ts, optimizer, surface, kind, before, after) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                i64::try_from(event.ts).unwrap_or(i64::MAX),
                event.optimizer,
                event.surface,
                event.kind.as_str(),
                i64::try_from(event.before).unwrap_or(i64::MAX),
                i64::try_from(event.after).unwrap_or(i64::MAX),
            ],
        )?;
        Ok(())
    }

    /// Fold the hot-path append-log into `SQLite`. Off the hot path, best-effort
    /// (any error is swallowed). Idempotent: a missing/empty log is a no-op.
    pub fn drain() {
        let _ = drain_log();
    }

    /// Load all recorded events (best-effort: a DB error yields an empty list).
    /// Drains the append-log first so the view is current.
    #[must_use]
    pub fn load() -> Vec<StatEvent> {
        Self::drain();
        load_events().unwrap_or_default()
    }

    /// The NET [`Summary`] via a single SQL `GROUP BY` — so `gain`/`status` never
    /// materialize every row. Drains the append-log first; best-effort.
    #[must_use]
    pub fn summary() -> Summary {
        Self::drain();
        Summary::from_aggregates(&aggregate().unwrap_or_default())
    }
}

/// Append one event line to the hot-path log:
/// `<ts> <kind> <optimizer> <surface> <before> <after>`.
///
/// Fork-free and best-effort — a single `O_APPEND` write so concurrent hook
/// processes don't interleave. Optimizer/surface names are a closed set with no
/// whitespace, so the space-separated line round-trips through [`parse_line`].
fn append(kind: Kind, optimizer: &str, surface: &str, before: usize, after: usize) {
    let Some(path) = events_log_path() else {
        return;
    };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let line = format!(
        "{} {} {optimizer} {surface} {before} {after}\n",
        now_secs(),
        kind.as_str(),
    );
    if let Ok(mut file) = OpenOptions::new().append(true).create(true).open(&path) {
        let _ = file.write_all(line.as_bytes());
    }
}

/// The hot-path append-log path: `<data_dir>/events.log`.
fn events_log_path() -> Option<PathBuf> {
    Some(data_dir()?.join("events.log"))
}

/// Atomically claim the append-log (rename to a per-process temp), fold its lines
/// into `SQLite` in one transaction, then delete the temp. The rename means events
/// appended during the drain land in a fresh log and aren't lost; the per-process
/// temp name means concurrent drainers never clobber each other's batch — the loser
/// of the rename simply finds no log and no-ops. Malformed lines (e.g. a torn write
/// on crash) are skipped, not fatal.
fn drain_log() -> anyhow::Result<()> {
    let Some(log) = events_log_path() else {
        return Ok(());
    };
    let claimed = log.with_extension(format!("draining.{}", std::process::id()));
    if std::fs::rename(&log, &claimed).is_err() {
        return Ok(()); // no log, or another drainer claimed it first
    }
    let text = std::fs::read_to_string(&claimed).unwrap_or_default();
    let events: Vec<StatEvent> = text.lines().filter_map(parse_line).collect();
    if !events.is_empty() {
        let mut conn = connect()?;
        let tx = conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO events (ts, optimizer, surface, kind, before, after) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )?;
            for event in &events {
                stmt.execute(rusqlite::params![
                    i64::try_from(event.ts).unwrap_or(i64::MAX),
                    event.optimizer,
                    event.surface,
                    event.kind.as_str(),
                    i64::try_from(event.before).unwrap_or(i64::MAX),
                    i64::try_from(event.after).unwrap_or(i64::MAX),
                ])?;
            }
        }
        tx.commit()?;
    }
    let _ = std::fs::remove_file(&claimed);
    Ok(())
}

/// Parse one append-log line back into a [`StatEvent`]; `None` on a malformed line.
fn parse_line(line: &str) -> Option<StatEvent> {
    let mut fields = line.split_whitespace();
    let ts = fields.next()?.parse().ok()?;
    let kind = Kind::parse(fields.next()?)?;
    let optimizer = fields.next()?.to_owned();
    let surface = fields.next()?.to_owned();
    let before = fields.next()?.parse().ok()?;
    let after = fields.next()?.parse().ok()?;
    Some(StatEvent {
        ts,
        optimizer,
        surface,
        kind,
        before,
        after,
    })
}

/// Aggregate the store into one row per `(optimizer, surface, kind)` in SQL.
fn aggregate() -> anyhow::Result<Vec<Aggregate>> {
    let conn = connect()?;
    let mut stmt = conn.prepare(
        "SELECT optimizer, surface, kind, SUM(before), SUM(after), COUNT(*) \
         FROM events GROUP BY optimizer, surface, kind",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            Kind::parse(&r.get::<_, String>(2)?).unwrap_or(Kind::Saved),
            r.get::<_, i64>(3)?,
            r.get::<_, i64>(4)?,
            usize::try_from(r.get::<_, i64>(5)?).unwrap_or(0),
        ))
    })?;
    Ok(rows.filter_map(Result::ok).collect())
}

/// Query every event from the DB, in insertion order.
fn load_events() -> anyhow::Result<Vec<StatEvent>> {
    let conn = connect()?;
    let mut stmt = conn
        .prepare("SELECT ts, optimizer, surface, kind, before, after FROM events ORDER BY rowid")?;
    let rows = stmt.query_map([], |r| {
        Ok(StatEvent {
            ts: u64::try_from(r.get::<_, i64>(0)?).unwrap_or(0),
            optimizer: r.get(1)?,
            surface: r.get(2)?,
            kind: Kind::parse(&r.get::<_, String>(3)?).unwrap_or(Kind::Saved),
            before: usize::try_from(r.get::<_, i64>(4)?).unwrap_or(0),
            after: usize::try_from(r.get::<_, i64>(5)?).unwrap_or(0),
        })
    })?;
    Ok(rows.filter_map(Result::ok).collect())
}

#[cfg(test)]
#[path = "../../tests/unit/stats/tracking.tests.rs"]
mod tests;
