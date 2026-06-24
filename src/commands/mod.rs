//! Human- and model-facing CLI subcommand backends (gain/status/config/uninstall).
//!
//! Install and updates flow through the Claude Code plugin; the binary owns the
//! rest of its lifecycle (`update-check`, `uninstall`). [`shell_path`] is a shared
//! helper for the opt-in PATH line; the Bash optimizer lives in
//! [`crate::optimizers::command`].

pub mod config_cmd;
pub mod gain;
pub mod resolve;
pub mod shell_path;
pub mod status;
pub mod uninstall;
