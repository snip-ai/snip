//! Shared filesystem locations for snip's own state (never user source files).
//!
//! Centralizes data-dir resolution so config, the session dedupe cache, overflow
//! spills, and the update-check throttle agree on one root. Honors `SNIP_HOME`
//! (tests/CI) before the platform data dir.

use std::path::PathBuf;

/// The directory name (under [`data_dir`]) holding every session's cache —
/// dedupe map + overflow spills. A single source of truth so the engine, the
/// spill service, and the `session-reset` hook agree on one literal.
pub(crate) const SESSION_CACHE_DIRNAME: &str = "session-cache";

/// snip's data root: `$SNIP_HOME`, else `<platform data dir>/snip`.
///
/// All of snip's state (config, stats DB, session cache, spills, the managed
/// binary under `bin/`) lives under here — never in user source files.
#[must_use]
pub fn data_dir() -> Option<PathBuf> {
    if let Ok(home) = std::env::var("SNIP_HOME") {
        return Some(PathBuf::from(home));
    }
    Some(dirs::data_dir()?.join("snip"))
}

/// The session-scoped cache dir (dedupe cache + overflow spills):
/// `<data_dir>/session-cache/<sanitized session id>`.
///
/// A missing/empty id collapses to a shared bucket so a spill always has a home.
#[must_use]
pub fn session_cache_dir(session_id: Option<&str>) -> Option<PathBuf> {
    Some(
        data_dir()?
            .join(SESSION_CACHE_DIRNAME)
            .join(sanitize(session_id)),
    )
}

/// Sanitize a session id to `[A-Za-z0-9-]`, mapping every other byte to `-`, so
/// it can never escape the cache root (no `/`, `\`, `..`).
fn sanitize(session_id: Option<&str>) -> String {
    let raw = session_id.unwrap_or("").trim();
    if raw.is_empty() {
        return "no-session".to_owned();
    }
    raw.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

/// Serializes the env-mutating tests (`SNIP_HOME`) across this module and
/// [`crate::overflow`] / [`crate::hooks::session_reset`] so the process-global
/// var can never race under parallel test execution.
#[cfg(test)]
pub(crate) static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
#[path = "../tests/unit/paths.tests.rs"]
mod tests;
