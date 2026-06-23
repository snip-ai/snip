//! End-to-end tests — drive the built `snip` binary on stdin exactly as Claude
//! Code invokes the plugin hooks: a hook's JSON in, the JSON Claude Code consumes
//! out, and the always-exit-0 invariant. Black-box at the process boundary; the
//! one tier that exercises `main` → `cli::run` → dispatch end-to-end.
//!
//! Declared as the `e2e` `[[test]]` target in `Cargo.toml` (with
//! `autotests = false`); each submodule mirrors a surface and is wired in by
//! `#[path]` so the tree stays parallel to `src/`. The shared harness lives in
//! `support.rs`.

#[path = "support.rs"]
mod support;

#[path = "read_hook.tests.rs"]
mod read_hook;

#[path = "search_hooks.tests.rs"]
mod search_hooks;

#[path = "command.tests.rs"]
mod command;

#[path = "edit_write.tests.rs"]
mod edit_write;

#[path = "maintenance.tests.rs"]
mod maintenance;

#[path = "cli.tests.rs"]
mod cli;

#[path = "robustness.tests.rs"]
mod robustness;
