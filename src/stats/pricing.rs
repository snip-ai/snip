//! Coarse per-model prices for the `gain` dollar estimate.
//!
//! Saved tool-output tokens are not all billed at the same rate: on a cached turn
//! they re-enter context at the **cache-read** tier (~10% of input on Anthropic's
//! pricing), on a fresh turn at the **input** tier. A single flat input price
//! therefore overstates the real dollar saving (the well-documented gap between
//! 50–99% headline figures and ~single-digit-% real spend reduction). So `gain`
//! reports a **range** — cache-read floor to fresh-input ceiling — rather than one
//! misleadingly precise number. Token counts are still an estimate, not tiktoken.

/// Cache-read tokens bill at ~10% of the fresh-input rate on Anthropic's tiers.
const CACHE_READ_RATIO: f64 = 0.10;

/// USD per million input tokens for `model` (substring match; mid-tier default).
#[must_use]
pub fn usd_per_mtok(model: &str) -> f64 {
    let model = model.to_ascii_lowercase();
    if model.contains("opus") {
        15.0
    } else if model.contains("haiku") {
        0.80
    } else {
        3.0 // sonnet / unknown → mid tier
    }
}

/// Convert a (signed) token count to a USD estimate at `model`'s input rate.
#[must_use]
#[allow(clippy::cast_precision_loss)] // a dollar estimate; sub-cent precision is irrelevant
pub fn tokens_to_usd(tokens: i64, model: &str) -> f64 {
    tokens as f64 / 1_000_000.0 * usd_per_mtok(model)
}

/// A NET dollar **range** for `tokens` at `model`, ordered low→high.
///
/// Returns `(cache-read floor, fresh-input ceiling)`: the floor reflects the rate
/// repeated tool output is usually billed at (cached re-reads); the ceiling assumes
/// every saved token was fresh input. Honest about tier uncertainty, not one flat
/// rate.
#[must_use]
#[allow(clippy::cast_precision_loss)] // a dollar estimate; sub-cent precision is irrelevant
pub fn net_usd_range(tokens: i64, model: &str) -> (f64, f64) {
    let input = tokens as f64 / 1_000_000.0 * usd_per_mtok(model);
    let cache_read = input * CACHE_READ_RATIO;
    (cache_read.min(input), cache_read.max(input))
}

#[cfg(test)]
#[path = "../../tests/unit/stats/pricing.tests.rs"]
mod tests;
