//! The command-line interface: argument parsing and dispatch.
//!
//! [`Cli`] is the clap entry point (kept here, not in a `cli/cli.rs`, which would
//! trip `clippy::module_inception`); [`run`] parses argv and dispatches.

pub mod command;

pub use command::Command;

use clap::Parser;

/// The snip command-line interface.
#[derive(Parser)]
#[command(name = "snip", version, about)]
pub struct Cli {
    /// The subcommand to run.
    #[command(subcommand)]
    pub command: Command,
}

/// Parse argv and dispatch the selected subcommand.
///
/// # Errors
/// Propagates the subcommand handler's error.
pub fn run() -> anyhow::Result<()> {
    Cli::parse().command.dispatch()
}
