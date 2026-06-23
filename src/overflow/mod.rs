//! Shared overflow service: caps any optimized view to a token budget, spilling
//! the full body to a recoverable session file with an in-output breadcrumb.
//!
//! Applied by the dispatcher to **every** `Rewrite` (read, search, future
//! command) — the antidote to lossy over-compression: output is never discarded.

mod elide_strategy;
mod overflow_cfg;
mod spill;

pub use elide_strategy::ElideStrategy;
pub use overflow_cfg::OverflowCfg;
pub use spill::Spill;
