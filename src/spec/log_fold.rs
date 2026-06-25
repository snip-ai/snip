//! Log fingerprinting: collapse near-identical lines (identical *modulo* their
//! variable tokens) to one template with an occurrence count.
//!
//! Variable spans — canonical UUIDs (`<uuid>`), digit runs (`<n>`) and long hex
//! runs (`<x>`) — are masked to placeholders before grouping; the first concrete
//! line is kept as the sample. Protected lines (errors/warnings) are never folded.
//! No regex — a closed char-class scan.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::fp_window::FpWindow;
use super::log_mask::{is_trivial_template, mask};

/// Options for the `Fingerprint` transform.
// A config flag struct: independent on/off toggles, not a state machine.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Serialize, Deserialize)]
pub struct FingerprintCfg {
    /// Master toggle for fingerprinting.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Mask runs of digits before grouping.
    #[serde(default = "default_true")]
    pub mask_numbers: bool,
    /// Mask hex runs (ids, hashes) before grouping.
    #[serde(default = "default_true")]
    pub mask_hex: bool,
    /// Mask canonical `8-4-4-4-12` UUIDs to `<uuid>` before grouping — needed
    /// because their short 4-hex segments would otherwise stay verbatim and two
    /// UUIDs would never collapse. (ISO-8601 timestamps already collapse via
    /// `mask_numbers`, so they need no separate flag.)
    #[serde(default = "default_true")]
    pub mask_uuid: bool,
    /// Grouping window (consecutive vs whole-output).
    #[serde(default)]
    pub window: FpWindow,
    /// Keep one verbatim sample line alongside the collapsed template.
    #[serde(default = "default_true")]
    pub keep_sample: bool,
    /// Substrings that protect a line from collapse (errors/warnings).
    #[serde(default = "default_protect")]
    pub protect: Vec<String>,
}

impl Default for FingerprintCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            mask_numbers: true,
            mask_hex: true,
            mask_uuid: true,
            window: FpWindow::default(),
            keep_sample: true,
            protect: default_protect(),
        }
    }
}

const fn default_true() -> bool {
    true
}

fn default_protect() -> Vec<String> {
    crate::relevance::ERROR_MARKERS
        .iter()
        .map(|s| (*s).to_owned())
        .collect()
}

/// Collapse masked-equal records per `cfg`. Disabled config returns input as-is.
#[must_use]
pub(crate) fn fingerprint(records: Vec<String>, cfg: &FingerprintCfg) -> Vec<String> {
    if !cfg.enabled {
        return records;
    }
    match cfg.window {
        FpWindow::Consecutive => consecutive(records, cfg),
        FpWindow::Whole => whole(records, cfg),
    }
}

/// Collapse only consecutive masked-equal runs (order-preserving).
fn consecutive(records: Vec<String>, cfg: &FingerprintCfg) -> Vec<String> {
    let protect_lower = lower_protect(cfg);
    let mut out = Vec::with_capacity(records.len());
    let mut run: Option<(String, String, usize)> = None; // (mask, sample, count)
    for line in records {
        if is_protected(&line, &protect_lower) {
            flush(&mut out, run.take(), cfg);
            out.push(line);
            continue;
        }
        let masked = mask(&line, cfg);
        if is_trivial_template(&masked) {
            // The masked-away tokens ARE this line's content (a `seq`, an id
            // column); folding distinct values to one row would misrepresent them.
            flush(&mut out, run.take(), cfg);
            out.push(line);
            continue;
        }
        match &mut run {
            Some((m, _, count)) if *m == masked => *count += 1,
            _ => {
                flush(&mut out, run.take(), cfg);
                run = Some((masked, line, 1));
            }
        }
    }
    flush(&mut out, run.take(), cfg);
    out
}

/// Collapse masked-equal lines across the whole output, keeping first position.
fn whole(records: Vec<String>, cfg: &FingerprintCfg) -> Vec<String> {
    let protect_lower = lower_protect(cfg);
    let mut counts: HashMap<String, usize> = HashMap::new();
    for line in &records {
        if !is_protected(line, &protect_lower) {
            let masked = mask(line, cfg);
            if !is_trivial_template(&masked) {
                *counts.entry(masked).or_insert(0) += 1;
            }
        }
    }
    let mut seen: HashSet<String> = HashSet::new();
    let mut out = Vec::with_capacity(records.len());
    for line in records {
        if is_protected(&line, &protect_lower) {
            out.push(line);
            continue;
        }
        let masked = mask(&line, cfg);
        if is_trivial_template(&masked) {
            out.push(line); // pure-placeholder template → never folded (distinct data)
            continue;
        }
        if !seen.insert(masked.clone()) {
            continue; // a later occurrence — already emitted with its count
        }
        let count = counts.get(&masked).copied().unwrap_or(1);
        out.push(render(&masked, &line, count, cfg));
    }
    out
}

/// Emit the accumulated consecutive run.
fn flush(out: &mut Vec<String>, run: Option<(String, String, usize)>, cfg: &FingerprintCfg) {
    if let Some((masked, sample, count)) = run {
        out.push(render(&masked, &sample, count, cfg));
    }
}

/// Render a (template, sample, count) group: the verbatim line for a singleton,
/// else `<sample-or-template> (×N)`.
fn render(masked: &str, sample: &str, count: usize, cfg: &FingerprintCfg) -> String {
    if count <= 1 {
        return sample.to_owned();
    }
    let shown = if cfg.keep_sample { sample } else { masked };
    format!("{shown} (×{count})")
}

/// Lower-case the protect markers once. They are constant across the output, so
/// hoisting this out of the per-line scan avoids re-lower-casing every marker on
/// every line (`O(lines × markers)` allocations → `O(markers)`).
fn lower_protect(cfg: &FingerprintCfg) -> Vec<String> {
    cfg.protect.iter().map(|p| p.to_ascii_lowercase()).collect()
}

/// Whether `line` contains a protect marker (case-insensitive) and is left alone.
/// `protect_lower` is pre-lower-cased once by the caller via [`lower_protect`].
fn is_protected(line: &str, protect_lower: &[String]) -> bool {
    let lower = line.to_ascii_lowercase();
    protect_lower.iter().any(|p| lower.contains(p.as_str()))
}

#[cfg(test)]
#[path = "../../tests/unit/spec/log_fold.tests.rs"]
mod tests;
