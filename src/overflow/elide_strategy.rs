//! Which part of an over-budget output to keep, and the record-level elision.

use serde::{Deserialize, Serialize};

use crate::relevance::contains_error_marker;
use crate::tokens::estimate_tokens;

/// How to elide when output exceeds the budget. Always elides **whole records**
/// (lines), never mid-line, and inserts a single marker where the gap is.
#[derive(Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ElideStrategy {
    /// Keep the head, drop the tail.
    Head,
    /// Keep the tail, drop the head.
    Tail,
    /// Keep both ends, elide the middle (head-biased). The default.
    #[default]
    Middle,
    /// Keep the records that look like errors/failures (the rest are elided),
    /// surfacing the most useful lines first. Falls back to [`Self::Middle`] when
    /// nothing looks relevant.
    RelevanceFirst,
}

impl ElideStrategy {
    /// Keep records fitting `max_tokens`, eliding the rest per this strategy.
    /// `head_frac` (clamped) splits the budget for [`Self::Middle`].
    #[must_use]
    pub fn elide(self, records: &[String], max_tokens: usize, head_frac: f32) -> Vec<String> {
        match self {
            Self::Head => keep_head(records, max_tokens),
            Self::Tail => keep_tail(records, max_tokens),
            Self::Middle => keep_middle(records, max_tokens, head_frac),
            Self::RelevanceFirst => keep_relevant(records, max_tokens, head_frac),
        }
    }
}

/// Token cost of one record, including its newline separator.
const fn line_cost(record: &str) -> usize {
    estimate_tokens(record) + 1
}

/// The elision marker standing in for `n` dropped records.
fn elision(n: usize) -> String {
    format!("… ({n} lines elided)")
}

/// How many records from the start fit in `budget` (always at least one).
fn head_take(records: &[String], budget: usize) -> usize {
    let mut used = 0;
    let mut count = 0;
    for record in records {
        let cost = line_cost(record);
        if used + cost > budget && count > 0 {
            break;
        }
        used += cost;
        count += 1;
    }
    count
}

/// How many records from the end fit in `budget` (always at least one).
fn tail_take(records: &[String], budget: usize) -> usize {
    let mut used = 0;
    let mut count = 0;
    for record in records.iter().rev() {
        let cost = line_cost(record);
        if used + cost > budget && count > 0 {
            break;
        }
        used += cost;
        count += 1;
    }
    count
}

fn keep_head(records: &[String], budget: usize) -> Vec<String> {
    let take = head_take(records, budget);
    if take >= records.len() {
        return records.to_vec();
    }
    let mut out = records[..take].to_vec();
    out.push(elision(records.len() - take));
    out
}

fn keep_tail(records: &[String], budget: usize) -> Vec<String> {
    let take = tail_take(records, budget);
    if take >= records.len() {
        return records.to_vec();
    }
    let mut out = vec![elision(records.len() - take)];
    out.extend_from_slice(&records[records.len() - take..]);
    out
}

fn keep_middle(records: &[String], budget: usize, head_frac: f32) -> Vec<String> {
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    let head_budget = (budget as f32 * head_frac.clamp(0.0, 1.0)) as usize;
    let head_end = head_take(records, head_budget);
    let tail_budget = budget.saturating_sub(head_budget);
    // Tail records, but never overlapping the head we already kept.
    let tail = tail_take(records, tail_budget).min(records.len() - head_end);
    let tail_start = records.len() - tail;
    if tail_start <= head_end {
        return records.to_vec();
    }
    let mut out = records[..head_end].to_vec();
    out.push(elision(tail_start - head_end));
    out.extend_from_slice(&records[tail_start..]);
    out
}

/// Keep the relevant (error/failure) records that fit `budget`, in original
/// order, with one elision marker for everything dropped. With no relevant
/// record, defers to [`keep_middle`] so the output is still capped sensibly.
fn keep_relevant(records: &[String], budget: usize, head_frac: f32) -> Vec<String> {
    if !records.iter().any(|r| contains_error_marker(r)) {
        return keep_middle(records, budget, head_frac);
    }
    let mut out = Vec::new();
    let mut used = 0;
    for record in records.iter().filter(|r| contains_error_marker(r)) {
        let cost = line_cost(record);
        if used + cost > budget && !out.is_empty() {
            break;
        }
        used += cost;
        out.push(record.clone());
    }
    let dropped = records.len() - out.len();
    if dropped > 0 {
        out.push(elision(dropped));
    }
    out
}

#[cfg(test)]
#[path = "../../tests/unit/overflow/elide_strategy.tests.rs"]
mod tests;
