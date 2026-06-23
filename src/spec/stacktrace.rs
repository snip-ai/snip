//! Stacktrace pruning: fold framework/runtime frames, keep the app frames.
//!
//! Errors and tracebacks are dominated by deep framework/runtime frames the model
//! rarely needs. This collapses runs of consecutive *framework* frames — lines
//! containing a configured marker (`node_modules`, `site-packages`, `std::`, …) —
//! into one `… (N framework frames)` line, keeping app frames and non-frame lines
//! verbatim. Cross-language by construction (it matches markers, not a frame
//! grammar); no regex.

use serde::{Deserialize, Serialize};

/// Options for the `FoldFrames` transform (stacktrace pruning).
#[derive(Clone, Serialize, Deserialize)]
pub struct StacktraceCfg {
    /// Master toggle for stacktrace pruning.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Path/module markers that classify a frame as framework/runtime (folded).
    #[serde(default = "default_framework_prefixes")]
    pub framework_prefixes: Vec<String>,
    /// Frames kept at the top of the trace regardless of classification.
    #[serde(default = "default_keep_top")]
    pub keep_top: usize,
}

impl Default for StacktraceCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            framework_prefixes: default_framework_prefixes(),
            keep_top: default_keep_top(),
        }
    }
}

const fn default_true() -> bool {
    true
}

const fn default_keep_top() -> usize {
    1
}

fn default_framework_prefixes() -> Vec<String> {
    [
        "node_modules",
        "site-packages",
        "/rustc",
        "std::",
        "core::",
        "alloc::",
        "java.base",
        "jdk.",
        "runtime/",
        "golang",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect()
}

/// Fold framework-frame runs in `records` per `cfg` (disabled → unchanged).
///
/// `keep_top` frames of each run are kept for context before the rest fold.
#[must_use]
pub(crate) fn fold_frames(records: Vec<String>, cfg: &StacktraceCfg) -> Vec<String> {
    if !cfg.enabled {
        return records;
    }
    let mut out = Vec::with_capacity(records.len());
    let mut folded = 0usize; // framework frames pending in the current run
    let mut kept = 0usize; // framework frames already kept in the current run
    for line in records {
        if is_framework(&line, cfg) {
            if kept < cfg.keep_top {
                out.push(line);
                kept += 1;
            } else {
                folded += 1;
            }
        } else {
            flush(&mut out, &mut folded);
            kept = 0;
            out.push(line);
        }
    }
    flush(&mut out, &mut folded);
    out
}

/// Emit the pending fold marker (if any) and reset the run counter.
fn flush(out: &mut Vec<String>, folded: &mut usize) {
    let n = *folded;
    if n > 0 {
        out.push(format!("… ({n} framework frames)"));
        *folded = 0;
    }
}

/// Whether `line` belongs to a framework/runtime frame (contains a marker).
fn is_framework(line: &str, cfg: &StacktraceCfg) -> bool {
    cfg.framework_prefixes
        .iter()
        .any(|marker| line.contains(marker.as_str()))
}

#[cfg(test)]
#[path = "../../tests/unit/spec/stacktrace.tests.rs"]
mod tests;
