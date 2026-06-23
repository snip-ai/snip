//! snip — unified token/context optimizer for Claude Code (library root).
//!
//! Declares the module tree so integration tests can link against the crate.
//! Layered: `domain` (model) ← `engine` (dispatch) ← adapters (`optimizers`,
//! `spec`, `cli`, `hooks`, `commands`) over infrastructure (`config`,
//! `languages`, `compaction`, `overflow`, `stats`). Dependency-light leaf utils
//! (`paths`, `tokens`, `relevance`, `clock`) sit beneath everything, depend on nothing.

pub mod cli;
pub mod clock;
pub mod commands;
pub mod compaction;
pub mod config;
pub mod domain;
pub mod engine;
pub mod hooks;
pub mod languages;
pub mod optimizers;
pub mod overflow;
pub mod panic_guard;
pub mod paths;
pub mod relevance;
pub mod spec;
pub mod stats;
pub mod tokens;
