//! Token counting. A fast heuristic ([`estimate_tokens`]) is used everywhere —
//! the hot path and the detached stats recorder alike.
//!
//! Exact tiktoken is deliberately **not** a dependency (see
//! `.claude/rules/dependencies.md`); every token figure is labeled an estimate.

/// Estimate the token count of `text` cheaply — no tiktoken anywhere.
///
/// Uses the standard ~4-bytes-per-token heuristic, enough to drive the overflow
/// budget and the no-inflation ratio gate.
#[must_use]
pub const fn estimate_tokens(text: &str) -> usize {
    text.len().div_ceil(4)
}
