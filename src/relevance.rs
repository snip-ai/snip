//! Shared "relevance" heuristic: whether a line reads like an error/failure.
//!
//! The single source the `Rank` transform, the `RelevanceFirst` overflow
//! strategy, and the `Fingerprint` protect default all draw from, so their notion
//! of "relevant" cannot drift apart (it previously did — one list dropped
//! `exception`).

/// Substrings (matched case-insensitively) that mark a line as an error/failure
/// worth surfacing first and protecting from lossy folds.
pub(crate) const ERROR_MARKERS: [&str; 5] = ["error", "fail", "panic", "exception", "warning"];

/// Whether `line` contains an [`ERROR_MARKERS`] substring (case-insensitive).
#[must_use]
pub(crate) fn contains_error_marker(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    ERROR_MARKERS.iter().any(|m| lower.contains(m))
}
