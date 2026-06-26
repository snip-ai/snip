//! What the `Rank` transform stably orders records by.

use serde::{Deserialize, Serialize};

use crate::relevance::contains_error_marker;

/// What the `Rank` transform reorders records by. The reorder is **stable**, so
/// records sharing a rank keep their original relative order.
#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RankKey {
    /// Surface error/failure lines first; the rest follow, order preserved. Pairs
    /// with the `RelevanceFirst` overflow strategy so the useful lines survive a cap.
    ErrorsFirst,
    /// Lexicographic by full line. Used before `group(dir)` on Glob output so
    /// same-directory paths — interleaved by Claude Code's mtime ordering — become
    /// consecutive and actually fold. The full set of paths is kept, just reordered.
    Path,
}

impl RankKey {
    /// Stably reorder `records` by this key.
    #[must_use]
    pub fn rank(self, mut records: Vec<String>) -> Vec<String> {
        match self {
            Self::ErrorsFirst => {
                records.sort_by_key(|r| usize::from(!contains_error_marker(r)));
                records
            }
            Self::Path => {
                records.sort();
                records
            }
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/spec/rank_key.tests.rs"]
mod tests;
