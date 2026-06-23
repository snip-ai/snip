//! Aggregate recorded events into the NET savings report.

use std::collections::BTreeMap;

use super::event::{Kind, StatEvent};

/// One SQL-aggregated group: `(optimizer, surface, kind, Σbefore, Σafter, count)`.
pub(crate) type Aggregate = (String, String, Kind, i64, i64, usize);

/// Aggregated NET token accounting over a set of events.
pub struct Summary {
    /// Total tokens saved by optimizations (gross).
    pub gross_saved: i64,
    /// Total induced cost (spilled-output re-reads).
    pub induced: i64,
    /// NET = `gross_saved − induced`.
    pub net: i64,
    /// Number of events aggregated.
    pub events: usize,
    /// NET token delta per optimizer, sorted by name.
    pub per_optimizer: Vec<(String, i64)>,
    /// NET token delta per surface, sorted by name.
    pub per_surface: Vec<(String, i64)>,
}

impl Summary {
    /// Aggregate `events` into gross/induced/NET totals and per-key breakdowns.
    #[must_use]
    pub fn from_events(events: &[StatEvent]) -> Self {
        let mut fold = Fold::default();
        for event in events {
            fold.add(event.kind, &event.optimizer, &event.surface, event.net(), 1);
        }
        fold.finish()
    }

    /// Build the same totals from SQL-aggregated groups (one row per
    /// optimizer/surface/kind), so `gain`/`status` never materialize every event.
    /// Equivalent to [`Self::from_events`] because `net` is linear in
    /// `before`/`after` for a fixed kind — both feed the one [`Fold`] below, so
    /// the accounting can never drift between the two entry points.
    #[must_use]
    pub(crate) fn from_aggregates(groups: &[Aggregate]) -> Self {
        let mut fold = Fold::default();
        for (optimizer, surface, kind, sum_before, sum_after, count) in groups {
            let net = match kind {
                Kind::Saved => sum_before - sum_after,
                Kind::Induced => -sum_before,
            };
            fold.add(*kind, optimizer, surface, net, *count);
        }
        fold.finish()
    }
}

/// The single NET-accounting fold shared by [`Summary::from_events`] and
/// [`Summary::from_aggregates`]. Centralizing it means a change to how a `Kind`
/// contributes to gross/induced/per-key totals is made in exactly one place.
#[derive(Default)]
struct Fold {
    gross_saved: i64,
    induced: i64,
    events: usize,
    by_optimizer: BTreeMap<String, i64>,
    by_surface: BTreeMap<String, i64>,
}

impl Fold {
    /// Accumulate one event (or `count`-collapsed group) contributing `net` tokens.
    fn add(&mut self, kind: Kind, optimizer: &str, surface: &str, net: i64, count: usize) {
        match kind {
            Kind::Saved => self.gross_saved += net,
            Kind::Induced => self.induced += -net,
        }
        self.events += count;
        *self.by_optimizer.entry(optimizer.to_owned()).or_default() += net;
        *self.by_surface.entry(surface.to_owned()).or_default() += net;
    }

    /// Collapse the accumulators into the final [`Summary`].
    fn finish(self) -> Summary {
        Summary {
            gross_saved: self.gross_saved,
            induced: self.induced,
            net: self.gross_saved - self.induced,
            events: self.events,
            per_optimizer: self.by_optimizer.into_iter().collect(),
            per_surface: self.by_surface.into_iter().collect(),
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/stats/summary.tests.rs"]
mod tests;
