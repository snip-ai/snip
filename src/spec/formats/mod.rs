//! Format-aware parsers backing the `Parse` transform — one per machine format.
//!
//! Each parser turns a known machine format into compact, familiar lines, keeping
//! unrecognized lines verbatim so output is never dropped. No regex.

pub mod cargo_json;
pub mod eslint_json;
pub mod git_status;
pub mod go_test_json;
pub mod jest_json;
pub mod json;
pub mod ruff_json;
pub mod table_collapse;
