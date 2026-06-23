//! One recorded optimization event — the unit of the stats store.

use serde::{Deserialize, Serialize};

/// Whether an event records tokens **saved** by an optimization or the induced
/// **cost** of recovering spilled output (a re-read) — the NET subtrahend.
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// Input tokens an optimization removed (`before` − `after`).
    Saved,
    /// Induced cost: tokens the model spent re-reading a spilled output.
    Induced,
}

impl Kind {
    /// The stored/argument name (`saved` / `induced`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Saved => "saved",
            Self::Induced => "induced",
        }
    }

    /// Parse a stored/argument name back into a [`Kind`].
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "saved" => Some(Self::Saved),
            "induced" => Some(Self::Induced),
            _ => None,
        }
    }
}

/// A single optimization event, one row in the `SQLite` store.
#[derive(Clone, Serialize, Deserialize)]
pub struct StatEvent {
    /// Unix seconds when recorded.
    pub ts: u64,
    /// The optimizer name (`read`, `search`, `command`, …) or `overflow`.
    pub optimizer: String,
    /// The surface (`read`, `grep`, `glob`, `bash`).
    pub surface: String,
    /// Whether this is a saving or an induced cost.
    pub kind: Kind,
    /// Tokens before optimization (or the re-read cost for `Induced`).
    pub before: usize,
    /// Tokens after optimization (`0` for `Induced`).
    pub after: usize,
}

impl StatEvent {
    /// The net token delta this event contributes: `+saved` or `−cost`.
    #[must_use]
    pub fn net(&self) -> i64 {
        let before = i64::try_from(self.before).unwrap_or(i64::MAX);
        let after = i64::try_from(self.after).unwrap_or(i64::MAX);
        match self.kind {
            Kind::Saved => before - after,
            Kind::Induced => -before,
        }
    }
}
