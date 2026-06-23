//! Config resolution accessors: effective per-optimizer settings.
//!
//! Split out of `config/mod.rs` (which keeps the data model + load/save I/O) so
//! the "what is the effective setting for optimizer X" queries live together.

use super::{AutodetectCfg, CompactMode, Config};
use crate::overflow::OverflowCfg;

/// Command-surface overflow default — leaner than the 8000 of read/grep/glob.
const COMMAND_MAX_TOKENS: usize = 6000;

/// The command optimizer family key (`optimizers.command.*`) — the family-wide
/// fallback for an overflow override, since the per-unit budget is keyed by the
/// recognized *spec* name (e.g. `git-diff`), not the family.
const COMMAND_OPTIMIZER: &str = "command";

impl Config {
    /// Whether the named optimizer is active (master switch AND its own switch).
    #[must_use]
    pub fn optimizer_enabled(&self, name: &str) -> bool {
        self.master_enabled && self.optimizers.get(name).is_none_or(|o| o.enabled)
    }

    /// Whether re-read dedupe is active for `name` (its own switch; default on).
    #[must_use]
    pub fn dedupe_enabled(&self, name: &str) -> bool {
        self.optimizers.get(name).is_none_or(|o| o.dedupe)
    }

    /// Read compaction mode for `name` (default `Soft`).
    #[must_use]
    pub fn mode_for(&self, name: &str) -> CompactMode {
        self.optimizers
            .get(name)
            .and_then(|o| o.mode)
            .unwrap_or_default()
    }

    /// Whether per-spec/family rule `key` is enabled for `name` (default true).
    #[must_use]
    pub fn rule_enabled(&self, name: &str, key: &str) -> bool {
        self.optimizers
            .get(name)
            .is_none_or(|o| o.rules.get(key).copied().unwrap_or(true))
    }

    /// Auto-detect options for `name`: its override, else the global default.
    #[must_use]
    pub fn autodetect_for(&self, name: &str) -> &AutodetectCfg {
        self.optimizers
            .get(name)
            .and_then(|o| o.autodetect.as_ref())
            .unwrap_or(&self.autodetect)
    }

    /// The per-optimizer overflow override for `name`, if any (the shared lookup
    /// behind [`Self::overflow_for`] and [`Self::overflow_for_command`]).
    fn optimizer_overflow(&self, name: &str) -> Option<&OverflowCfg> {
        self.optimizers.get(name).and_then(|o| o.overflow.as_ref())
    }

    /// The effective overflow budget for `name`: its per-optimizer override if
    /// set, else the global default.
    #[must_use]
    pub fn overflow_for(&self, name: &str) -> &OverflowCfg {
        self.optimizer_overflow(name).unwrap_or(&self.overflow)
    }

    /// The effective overflow budget for a **command** optimizer `name`: the
    /// per-spec override (`optimizers.<spec>.overflow`) if set, else the
    /// command-family override (`optimizers.command.overflow`), else the command
    /// default ([`COMMAND_MAX_TOKENS`] — the Bash surface runs leaner than the
    /// read/grep/glob default).
    #[must_use]
    pub fn overflow_for_command(&self, name: &str) -> OverflowCfg {
        self.optimizer_overflow(name)
            .or_else(|| self.optimizer_overflow(COMMAND_OPTIMIZER))
            .cloned()
            .unwrap_or_else(|| OverflowCfg {
                max_tokens: COMMAND_MAX_TOKENS,
                ..OverflowCfg::default()
            })
    }
}

#[cfg(test)]
#[path = "../../tests/unit/config/accessors.tests.rs"]
mod tests;
