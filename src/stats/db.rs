//! `SQLite` connection for the stats store.
//!
//! Off the hot path by construction: only the detached recorder (writes) and the
//! `gain`/`status` commands (reads) open it — never a tool hook. WAL mode plus a
//! busy-timeout let concurrent sessions write without corrupting each other.

use std::time::Duration;

use anyhow::{Context, Result};
use rusqlite::Connection;

use crate::paths::data_dir;

/// Open (creating if needed) the stats DB at `<data_dir>/stats.db`, in WAL mode
/// with the `events` table ensured.
///
/// # Errors
/// Returns an error if the data dir can't be resolved or the DB can't be opened.
pub fn connect() -> Result<Connection> {
    let dir = data_dir().context("no data dir for the stats DB")?;
    std::fs::create_dir_all(&dir).ok();
    let conn = Connection::open(dir.join("stats.db")).context("opening stats.db")?;
    // WAL: concurrent readers + a single writer without blocking; busy-timeout so a
    // contended write waits briefly rather than failing.
    let _ = conn.pragma_update(None, "journal_mode", "WAL");
    conn.busy_timeout(Duration::from_secs(2))?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS events (
            ts        INTEGER NOT NULL,
            optimizer TEXT    NOT NULL,
            surface   TEXT    NOT NULL,
            kind      TEXT    NOT NULL,
            before    INTEGER NOT NULL,
            after     INTEGER NOT NULL
        )",
        [],
    )
    .context("ensuring the events table")?;
    // Supports the `gain`/`status` GROUP BY aggregation so they never full-scan.
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_events_opt_surface ON events(optimizer, surface)",
        [],
    )
    .context("ensuring the events index")?;
    Ok(conn)
}
