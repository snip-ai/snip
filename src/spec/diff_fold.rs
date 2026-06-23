//! Diff-hunk pruning: fold long runs of unchanged context lines in a diff.
//!
//! Unified-diff output is dominated by unchanged context lines (leading space);
//! changed lines (`+`/`-`), file headers (`diff --git`, `---`, `+++`) and hunk
//! headers (`@@`) carry the signal. This collapses each run of consecutive
//! context lines longer than `min_run` into one `… (N unchanged)` line, keeping
//! `context` lines on each side. Classification is by leading sign run only, so
//! combined-diff change lines (`" -"`/`" +"`) are never folded. No regex.

use serde::{Deserialize, Serialize};

/// Options for the `FoldDiff` transform (unified-diff context folding).
#[derive(Clone, Serialize, Deserialize)]
pub struct DiffFoldCfg {
    /// Master toggle for diff-hunk folding.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Minimum unchanged-context run length before any folding happens.
    #[serde(default = "default_min_run")]
    pub min_run: usize,
    /// Context lines kept on each side of an elided run.
    #[serde(default)]
    pub context: usize,
}

impl Default for DiffFoldCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            min_run: default_min_run(),
            context: 0,
        }
    }
}

const fn default_true() -> bool {
    true
}

const fn default_min_run() -> usize {
    4
}

/// Fold unchanged-context runs in `records` per `cfg` (disabled → unchanged).
#[must_use]
pub(crate) fn fold_diff(records: Vec<String>, cfg: &DiffFoldCfg) -> Vec<String> {
    if !cfg.enabled {
        return records;
    }
    let mut out = Vec::with_capacity(records.len());
    let mut run: Vec<String> = Vec::new();
    for line in records {
        if is_context(&line) {
            run.push(line);
        } else {
            flush(&mut out, &mut run, cfg);
            out.push(line);
        }
    }
    flush(&mut out, &mut run, cfg);
    out
}

/// Emit the pending context run: verbatim if short, else head + marker + tail.
fn flush(out: &mut Vec<String>, run: &mut Vec<String>, cfg: &DiffFoldCfg) {
    let total = run.len();
    let elided = total.saturating_sub(2 * cfg.context);
    if total <= cfg.min_run || elided < 1 {
        out.append(run);
        return;
    }
    out.extend(run.drain(..cfg.context));
    out.push(format!("… ({elided} unchanged)"));
    let tail_start = run.len() - cfg.context;
    out.extend(run.drain(tail_start..));
    run.clear();
}

/// Whether `line` is an unchanged context line (blank, or space-led with no
/// `+`/`-` in its leading sign run — guarding combined-diff change lines).
fn is_context(line: &str) -> bool {
    match line.as_bytes().first() {
        None => true,
        Some(b' ') => !leading_change(line),
        Some(_) => false,
    }
}

/// Whether the first non-space byte of `line` is a `+` or `-` (a change line).
fn leading_change(line: &str) -> bool {
    line.bytes()
        .find(|&b| b != b' ')
        .is_some_and(|b| matches!(b, b'+' | b'-'))
}

#[cfg(test)]
#[path = "../../tests/unit/spec/diff_fold.tests.rs"]
mod tests;
