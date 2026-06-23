//! The `search` optimizer (Grep/Glob) — a declarative [`crate::optimizers::SpecOptimizer`].
//!
//! Search has no bespoke Rust: it's the built-in `search` specs (dedupe →
//! group-by-file/dir → truncate) run through `SpecOptimizer`, dispatched by the
//! registry on the Grep/Glob surfaces. This module is the documented home; the
//! behavior is entirely data — see `spec/builtin/specs/search.json`.
