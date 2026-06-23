//! The `read` optimizer (AST code compaction + edit-safety) — the Rust escape hatch.
//!
//! Read compaction lives in [`read_optimizer`]; the edit-safety helpers
//! ([`correct`], [`fuzzy`], [`unit`], [`write_guard`], [`dedupe`]) support the
//! Edit/Write surfaces and the `snip resolve` recovery command.

pub mod correct;
mod dedupe;
mod diff;
mod edit_write;
mod fuzzy;
pub mod read_optimizer;
mod single_line_correct;
mod unit;
mod write_guard;

pub use read_optimizer::ReadOptimizer;
