//! Human- and model-facing CLI subcommand backends (gain/status/config/resolve).
//!
//! Install and updates flow through the Claude Code plugin (no `init`/`self-update`);
//! the Bash command optimizer lives in [`crate::optimizers::command`].

pub mod config_cmd;
pub mod gain;
pub mod resolve;
pub mod status;
