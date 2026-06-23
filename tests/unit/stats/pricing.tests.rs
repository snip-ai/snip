//! Unit tests for `stats::pricing`, in AAA form. Compiled into `snip_lib` via a
//! `#[path]` include in `src/stats/pricing.rs`.

use assert2::check;

use super::{tokens_to_usd, usd_per_mtok};

#[test]
fn rate_is_tiered_by_model_with_a_mid_default() {
    // Act + Assert: substring match; unknown → mid tier
    check!((usd_per_mtok("claude-opus-4-8") - 15.0).abs() < f64::EPSILON);
    check!((usd_per_mtok("claude-haiku-4-5") - 0.80).abs() < f64::EPSILON);
    check!((usd_per_mtok("claude-sonnet-4-6") - 3.0).abs() < f64::EPSILON);
    check!((usd_per_mtok("something-unknown") - 3.0).abs() < f64::EPSILON);
}

#[test]
fn tokens_convert_to_dollars_at_the_model_rate() {
    // Arrange: 2M tokens at the opus rate ($15/Mtok) = $30
    let usd = tokens_to_usd(2_000_000, "opus");

    // Assert
    check!((usd - 30.0).abs() < 1e-9);
}

#[test]
fn negative_tokens_yield_a_negative_cost() {
    // Arrange + Act + Assert: a net loss prices negatively
    check!(tokens_to_usd(-1_000_000, "sonnet") < 0.0);
}
