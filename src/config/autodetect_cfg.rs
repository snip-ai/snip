//! Auto-detection options for compacting unrecognized command output.

use serde::{Deserialize, Serialize};

/// Whether/how to sniff and compact output that no spec recognized.
// A config flag struct: independent on/off toggles, not a state machine.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Serialize, Deserialize)]
pub struct AutodetectCfg {
    /// Master toggle for auto-detection.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Compact output that sniffs as JSON.
    #[serde(default = "default_true")]
    pub json: bool,
    /// Compact output that sniffs as a delimited/columnar table.
    #[serde(default = "default_true")]
    pub table: bool,
    /// Compact output that sniffs as a repetitive log.
    #[serde(default = "default_true")]
    pub log: bool,
    /// Only fire above this line count (small output is left untouched).
    #[serde(default = "default_min_lines")]
    pub min_lines: usize,
}

impl Default for AutodetectCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            json: true,
            table: true,
            log: true,
            min_lines: default_min_lines(),
        }
    }
}

const fn default_true() -> bool {
    true
}

const fn default_min_lines() -> usize {
    20
}
