//! Overflow/truncation budget for tool output.
//!
//! When an already-optimized output still exceeds [`OverflowCfg::max_tokens`],
//! the dispatcher truncates the shown view and spills the full optimized output
//! to a session-scoped temp file, leaving a breadcrumb in the tool output (never
//! in markdown). This budget data lives beside the elision algorithm it drives
//! ([`super::ElideStrategy`] / [`super::Spill`]); [`crate::config`] only references it.

use serde::{Deserialize, Serialize};

use super::ElideStrategy;

/// How much to keep, and how to elide, when output overflows the budget.
#[derive(Clone, Serialize, Deserialize)]
pub struct OverflowCfg {
    /// Token budget above which the view is truncated and the rest spilled.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
    /// Which part of the output to keep when eliding.
    #[serde(default)]
    pub strategy: ElideStrategy,
    /// Fraction of the kept budget biased toward the head (the rest goes to the
    /// tail) for the `Middle` strategy. Clamped to `0.0..=1.0` at use.
    #[serde(default = "default_head_frac")]
    pub head_frac: f32,
}

impl Default for OverflowCfg {
    fn default() -> Self {
        Self {
            max_tokens: default_max_tokens(),
            strategy: ElideStrategy::default(),
            head_frac: default_head_frac(),
        }
    }
}

/// 8000 suits read/grep/glob; the command optimizer overrides to 6000.
const fn default_max_tokens() -> usize {
    8000
}

/// Bias the kept budget toward the head — errors/signatures usually lead.
const fn default_head_frac() -> f32 {
    0.6
}
