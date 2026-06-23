//! Read-optimizer compaction aggressiveness (soft/medium/high).

use serde::{Deserialize, Serialize};

/// How aggressively the `read` optimizer rewrites code.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompactMode {
    /// Comments/docstrings removed only; code stays byte-identical (Edit-safe).
    #[default]
    Soft,
    /// Soft + whitespace normalization and single-statement block collapsing.
    Medium,
    /// Medium + each code block collapsed onto its header (single-line-safe langs).
    High,
}

impl CompactMode {
    /// The canonical lowercase name (`soft` / `medium` / `high`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Soft => "soft",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/config/compact_mode.tests.rs"]
mod tests;
