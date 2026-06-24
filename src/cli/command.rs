//! The snip subcommand set and its dispatch to handlers.

use clap::Subcommand;

use crate::domain::Surface;
use crate::engine::Dispatcher;
use crate::{commands, hooks};

/// Every snip subcommand. Hook variants map to a [`Dispatcher`] surface; the
/// rest are CLI or maintenance handlers.
#[derive(Subcommand)]
pub enum Command {
    /// `PostToolUse`/Read hook — compact code output.
    ReadHook,
    /// `PostToolUse`/Grep hook.
    GrepHook,
    /// `PostToolUse`/Glob hook.
    GlobHook,
    /// `PreToolUse`/Bash hook — route to `exec` or pass through.
    BashRoute,
    /// `PreToolUse`/Edit hook — restore `old_string`.
    EditFix,
    /// `PreToolUse`/Write hook — overwrite guard.
    WriteGuard,
    /// `PreCompact` hook — clear the session cache.
    SessionReset,
    /// `SessionStart` hook — drain stats and flag a self-update when due.
    UpdateCheck,
    /// Run a wrapped Bash command on its exact bytes.
    Exec {
        /// The wrapped command and its arguments.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Map a compacted-view `old_string` back to real file bytes.
    Resolve {
        /// The file the `old_string` came from.
        file: String,
    },
    /// Internal: insert one stats event directly (utility; the hot path appends to
    /// the events log and a drain folds it into `SQLite` off the hot path).
    #[command(hide = true)]
    StatRecord {
        /// `<kind> <optimizer> <surface> <before> <after>`.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Print NET token-savings analytics.
    Gain,
    /// Print version, enabled state, and savings.
    Status,
    /// Force a re-check against the latest release (manual `/snip update`).
    Update,
    /// Get/set/list/reset configuration.
    Config {
        /// The config arguments (e.g. `set overflow.max_tokens 4000`).
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Enable the master switch.
    Enable,
    /// Disable the master switch.
    Disable,
    /// Remove snip's data, binary, and PATH line (the plugin is removed separately).
    Uninstall,
}

impl Command {
    /// Dispatch this subcommand to its handler.
    ///
    /// # Errors
    /// Propagates the handler's error (tool hooks never error; CLI handlers may).
    pub fn dispatch(self) -> anyhow::Result<()> {
        match self {
            Self::ReadHook => Dispatcher::new(Surface::Read).run(),
            Self::GrepHook => Dispatcher::new(Surface::Grep).run(),
            Self::GlobHook => Dispatcher::new(Surface::Glob).run(),
            Self::BashRoute => crate::optimizers::command::bash_route::run(),
            Self::EditFix => Dispatcher::new(Surface::Edit).run(),
            Self::WriteGuard => Dispatcher::new(Surface::Write).run(),
            Self::SessionReset => hooks::session_reset::run(),
            Self::UpdateCheck => hooks::update_check::run(),
            Self::Exec { args } => crate::optimizers::command::exec::run(&args),
            Self::Resolve { file } => commands::resolve::run(&file),
            Self::StatRecord { args } => crate::stats::recorder::run(&args),
            Self::Gain => commands::gain::run(),
            Self::Status => commands::status::run(),
            Self::Update => commands::update::run(),
            Self::Config { args } => commands::config_cmd::run(&args),
            Self::Enable => commands::config_cmd::set_enabled(true),
            Self::Disable => commands::config_cmd::set_enabled(false),
            Self::Uninstall => commands::uninstall::run(),
        }
    }
}
