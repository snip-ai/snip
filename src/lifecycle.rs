//! The binary-lifecycle banner: a one-shot, user-visible `SessionStart` notice.
//!
//! The detached bootstrap (`snip-bootstrap.sh`) records the outcome of a download
//! — install, update, or failure — in a `.lifecycle` sentinel under the data dir.
//! The next `SessionStart` reads it and surfaces a one-line `systemMessage` to the
//! user (never the model — `systemMessage` is user-only; `additionalContext` would
//! reach the model and is deliberately unused), then consumes the sentinel so the
//! notice fires exactly once. This is the only feedback channel for work that
//! finishes after the spawning hook has already exited.

use std::fs;
use std::path::Path;

/// The `.lifecycle` sentinel filename under the data dir.
/// KEEP IN SYNC with `scripts/snip-bootstrap.sh` (the writer).
const LIFECYCLE_FILE: &str = ".lifecycle";

/// A binary-lifecycle event left by the bootstrap for the next `SessionStart`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lifecycle {
    /// First successful install, at the given version.
    Installed(String),
    /// Self-update applied: `from` → `to`.
    Updated {
        /// The version that was running before the update.
        from: String,
        /// The version now installed.
        to: String,
    },
    /// A download or checksum step failed; nothing was installed.
    DownloadFailed,
    /// No prebuilt binary exists for this platform.
    UnsupportedPlatform,
}

impl Lifecycle {
    /// Read and remove the `.lifecycle` sentinel under `data_dir`, returning the
    /// parsed event. Returns `None` when the file is absent or malformed; removal
    /// is best-effort (a leftover only risks re-announcing the notice once).
    #[must_use]
    pub fn consume(data_dir: &Path) -> Option<Self> {
        let path = data_dir.join(LIFECYCLE_FILE);
        let text = fs::read_to_string(&path).ok()?;
        let _ = fs::remove_file(&path);
        Self::parse(text.lines().next().unwrap_or_default())
    }

    /// Parse one sentinel line — `"<state> [args…]"` — into an event.
    fn parse(line: &str) -> Option<Self> {
        let mut parts = line.split_whitespace();
        match (parts.next(), parts.next(), parts.next()) {
            (Some("installed"), Some(v), None) => Some(Self::Installed(v.to_owned())),
            (Some("updated"), Some(from), Some(to)) => Some(Self::Updated {
                from: from.to_owned(),
                to: to.to_owned(),
            }),
            (Some("download-failed"), None, _) => Some(Self::DownloadFailed),
            (Some("unsupported-platform"), None, _) => Some(Self::UnsupportedPlatform),
            _ => None,
        }
    }

    /// The user-facing one-line message for this event.
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::Installed(v) => {
                format!("snip: installed v{v} — now optimizing Reads, Bash, Grep, and Glob.")
            }
            Self::Updated { from, to } => format!("snip: updated {from} → {to}."),
            Self::DownloadFailed => "snip: could not download the optimizer binary; it will \
                                     retry next session (or run /snip update)."
                .to_owned(),
            Self::UnsupportedPlatform => {
                "snip: no prebuilt binary for this platform — optimization is inactive.".to_owned()
            }
        }
    }

    /// Render the `SessionStart` hook JSON that shows [`Self::message`] to the user
    /// without adding anything to the model's context: a `systemMessage` (user-only)
    /// inside the canonical `SessionStart` envelope, never `additionalContext`.
    #[must_use]
    pub fn banner_json(&self) -> String {
        serde_json::json!({
            "hookSpecificOutput": { "hookEventName": "SessionStart" },
            "systemMessage": self.message(),
        })
        .to_string()
    }
}

#[cfg(test)]
#[path = "../tests/unit/lifecycle.tests.rs"]
mod tests;
