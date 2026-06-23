//! Per-optimizer configuration (a section of [`crate::config::Config`]).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::config::{AutodetectCfg, CompactMode};
use crate::overflow::OverflowCfg;

/// Per-optimizer configuration, keyed by optimizer `name`.
#[derive(Clone, Serialize, Deserialize)]
pub struct OptimizerCfg {
    /// Whether this optimizer is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Whether identical re-reads are deduped to a notice (the `read` optimizer).
    #[serde(default = "default_true")]
    pub dedupe: bool,
    /// Read compaction aggressiveness (the `read` optimizer): soft/medium/high.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<CompactMode>,
    /// Per-spec / per-family toggles (e.g. `"git.diff" -> false`); default on.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub rules: HashMap<String, bool>,
    /// Per-optimizer auto-detect override; falls back to the global default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub autodetect: Option<AutodetectCfg>,
    /// Optional per-optimizer overflow budget; falls back to the global default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overflow: Option<OverflowCfg>,
}

impl Default for OptimizerCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            dedupe: true,
            mode: None,
            rules: HashMap::new(),
            autodetect: None,
            overflow: None,
        }
    }
}

const fn default_true() -> bool {
    true
}

#[cfg(test)]
#[path = "../../tests/unit/config/optimizer_cfg.tests.rs"]
mod tests;
