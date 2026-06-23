//! The `Parse` transform's format selector — dispatches to [`super::formats`].
//!
//! A `Parse` step interprets a known machine format and re-emits compact lines,
//! staying within the `Vec<String>` pipeline (no regex). Unrecognized lines are
//! kept verbatim so output is never dropped.

use serde::{Deserialize, Serialize};

use super::formats;

/// A machine output format the `Parse` transform can compact.
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParseFormat {
    /// `git status --porcelain=v2 --branch` → the familiar short form.
    GitStatusV2,
    /// `cargo --message-format=json` → one compact line per diagnostic.
    CargoJson,
    /// `eslint --format=json` → one compact line per lint message.
    EslintJson,
    /// `ruff check --output-format=json` → one compact line per violation.
    RuffJson,
    /// `go test -json` → failing tests + a per-package pass/fail/skip tally.
    GoTestJson,
    /// `jest --json` → failed assertions + a pass/fail tally.
    JestJson,
    /// Pretty-printed JSON → minified (insignificant whitespace stripped).
    JsonMinify,
    /// A uniform JSON array of objects → a header + value rows (TOON).
    JsonArrayTable,
    /// A whitespace-delimited table → drop constant columns + a `[const …]` note.
    TableCollapse,
}

impl ParseFormat {
    /// Parse `records` in this format, returning compacted lines.
    #[must_use]
    pub fn apply(self, records: &[String]) -> Vec<String> {
        match self {
            Self::GitStatusV2 => formats::git_status::git_status_v2(records),
            Self::CargoJson => formats::cargo_json::cargo_json(records),
            Self::EslintJson => formats::eslint_json::eslint_json(records),
            Self::RuffJson => formats::ruff_json::ruff_json(records),
            Self::GoTestJson => formats::go_test_json::go_test_json(records),
            Self::JestJson => formats::jest_json::jest_json(records),
            Self::JsonMinify => formats::json::json_minify(records),
            Self::JsonArrayTable => formats::json::json_array_table(records),
            Self::TableCollapse => formats::table_collapse::table_collapse(records),
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/spec/parse_format.tests.rs"]
mod tests;
