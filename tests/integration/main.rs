//! Integration tests — `snip_lib`'s public API, black-box (a separate crate).
//!
//! Each submodule mirrors a `src/` area and is wired in by `#[path]` so the tree
//! stays parallel to `src/`. Declared as a single `[[test]]` target in
//! `Cargo.toml` (with `autotests = false`), so nothing here is auto-discovered.

#[path = "read_pipeline.tests.rs"]
mod read_pipeline;

#[path = "search_pipeline.tests.rs"]
mod search_pipeline;

#[path = "command_pipeline.tests.rs"]
mod command_pipeline;

#[path = "exec_runtime.tests.rs"]
mod exec_runtime;

#[path = "edit_correction.tests.rs"]
mod edit_correction;

#[path = "overflow_spill.tests.rs"]
mod overflow_spill;

#[path = "config_layering.tests.rs"]
mod config_layering;

#[path = "stats_net.tests.rs"]
mod stats_net;
