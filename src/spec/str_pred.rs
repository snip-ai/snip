//! A closed, regex-free string predicate for the `Keep`/`Drop` transforms.

use serde::{Deserialize, Serialize};

/// A closed string predicate evaluated against a record (a line by default).
///
/// Matching is plain `std` string ops only — the spec vocabulary forbids regex
/// and scripting. Negation is expressed by choosing `Keep` vs `Drop`, so the
/// predicate itself stays a single positive test.
#[derive(Clone, Deserialize, Serialize)]
#[serde(tag = "match", rename_all = "snake_case")]
pub enum StrPred {
    /// The record contains `value` as a substring.
    Contains {
        /// The substring to look for.
        value: String,
    },
    /// The record starts with `value`.
    StartsWith {
        /// The required prefix.
        value: String,
    },
    /// The record ends with `value`.
    EndsWith {
        /// The required suffix.
        value: String,
    },
    /// The record equals `value` exactly.
    Equals {
        /// The exact text required.
        value: String,
    },
}

impl StrPred {
    /// Whether `record` satisfies this predicate.
    #[must_use]
    pub fn matches(&self, record: &str) -> bool {
        match self {
            Self::Contains { value } => record.contains(value.as_str()),
            Self::StartsWith { value } => record.starts_with(value.as_str()),
            Self::EndsWith { value } => record.ends_with(value.as_str()),
            Self::Equals { value } => record == value,
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/spec/str_pred.tests.rs"]
mod tests;
