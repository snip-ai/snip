//! Auto-detect structured output of an *unrecognized* command and compact it.
//!
//! The fallback so even an unknown command can't dump big raw output: sniff the
//! captured text and route it through the matching compaction — JSON-shaped output
//! (a `{`/`[` lead) through the JSON encoders, any other repetitive text through
//! log fingerprinting. Conservative + guarded — only fires above `min_lines`, and
//! only when the result is strictly smaller (else verbatim). Worst case is no change.

use crate::config::AutodetectCfg;
use crate::spec::FingerprintCfg;
use crate::spec::formats::json;
use crate::spec::log_fold::fingerprint;
use crate::tokens::estimate_tokens;

/// Compact `output` per `cfg`, or return `None` to leave it verbatim.
///
/// JSON-shaped output (a `{`/`[` lead) is routed through the JSON encoders; any
/// other repetitive text through log fingerprinting. `None` results when
/// auto-detect is disabled, the output is too short, nothing is recognized, or the
/// view would not be strictly smaller.
///
/// With `secret_safe`, secret-bearing lines are masked before encoding; once a
/// secret is masked the masked view is always returned (never the unmasked
/// verbatim output), mirroring [`crate::optimizers::SpecOptimizer::apply_to`].
#[must_use]
pub fn compact(output: &str, cfg: &AutodetectCfg, secret_safe: bool) -> Option<String> {
    if !cfg.enabled || output.lines().count() < cfg.min_lines {
        return None;
    }
    let first = output.trim_start().chars().next()?;
    // JSON-shaped output is JSON territory — folding it as a log would corrupt its
    // structure, so it is only ever compacted by the JSON encoders.
    if is_json_shaped(output) {
        return cfg
            .json
            .then(|| compact_json(output, first, cfg, secret_safe))
            .flatten();
    }
    // Anything else: a repetitive log, when log auto-detect is enabled.
    cfg.log.then(|| compact_log(output, secret_safe)).flatten()
}

/// Whether `output` leads with `{`/`[` (after whitespace) — JSON, not a log.
///
/// JSON is compacted by the faithful encoders; everything else is a log to fold.
/// Exposed so the exec runtime can tell that faithful path from the *lossy*
/// log-fold path, which must stay recoverable.
#[must_use]
pub fn is_json_shaped(output: &str) -> bool {
    matches!(output.trim_start().chars().next(), Some('{' | '['))
}

/// Whether folded `view` dropped distinct lines that re-expansion can't recover.
///
/// The fingerprinter collapses two or more *distinct* masked-equal lines to one
/// sample, so only the first survives. Re-expands each `sample (×N)` line back to N
/// copies of its sample: a fold of byte-identical lines (the count reconstructs them)
/// re-expands to `original` and is lossless, so the caller can skip the recoverable
/// spill; a fold of distinct lines can't, and must be preserved. The caller gates out
/// the faithful JSON encoders and `secret_safe` (where `view` is masked but `original`
/// is not), so here `view` is always a log fold derived directly from `original`.
#[must_use]
pub fn fold_is_lossy(view: &str, original: &str) -> bool {
    reexpand(view) != original.lines().collect::<Vec<_>>()
}

/// Re-expand a fingerprint fold: a `sample (×N)` line becomes N copies of `sample`,
/// every other line passes through. Mirrors [`crate::spec::log_fold`]'s render shape.
fn reexpand(view: &str) -> Vec<&str> {
    let mut out = Vec::new();
    for line in view.lines() {
        match split_fold(line) {
            Some((sample, count)) => out.extend(std::iter::repeat_n(sample, count)),
            None => out.push(line),
        }
    }
    out
}

/// Split a folded `sample (×N)` line into its `(sample, N)`, else `None`.
fn split_fold(line: &str) -> Option<(&str, usize)> {
    let (sample, count) = line.strip_suffix(')')?.rsplit_once(" (×")?;
    Some((sample, count.parse().ok()?))
}

/// Compact JSON-shaped `output`: a uniform array → the columnar TOON table (when
/// `cfg.table`), any other document → minified to one line.
fn compact_json(
    output: &str,
    first: char,
    cfg: &AutodetectCfg,
    secret_safe: bool,
) -> Option<String> {
    let mut records: Vec<String> = output.lines().map(str::to_owned).collect();
    let masked_any = secret_safe && crate::optimizers::redact::mask_records(&mut records);
    // Route to the columnar TOON table when the shape is a uniform set of objects —
    // a JSON array (`[`) or line-delimited NDJSON (`{` per line); otherwise minify
    // (a single, possibly pretty-printed, document). `table_or_minify` keeps the
    // smaller of the table and verbatim.
    let candidate = match first {
        '[' if cfg.table => table_or_minify(json::json_array_table(&records), &records),
        '{' if cfg.table => table_or_minify(json::ndjson_table(&records), &records),
        _ => json::json_minify(&records),
    };
    emit(candidate.join("\n"), output, masked_any)
}

/// The TOON `table` when it actually re-shaped the input (a uniform object set),
/// else a minified single document — whichever the encoder produced.
fn table_or_minify(table: Vec<String>, records: &[String]) -> Vec<String> {
    if table == records {
        json::json_minify(records)
    } else {
        table
    }
}

/// Fold a repetitive `output` (unrecognized, non-JSON) via log fingerprinting.
///
/// "Sniffs as a repetitive log" is defined operationally: fingerprinting actually
/// collapses at least one run. If nothing collapses, the output is left verbatim —
/// so a non-repetitive log is never altered (not even its trailing newline) for no
/// real gain. A masked secret is the one exception: it must never fall back to the
/// unmasked view, mirroring the JSON path.
fn compact_log(output: &str, secret_safe: bool) -> Option<String> {
    let mut records: Vec<String> = output.lines().map(str::to_owned).collect();
    let masked_any = secret_safe && crate::optimizers::redact::mask_records(&mut records);
    let line_count = records.len();
    let folded = fingerprint(records, &FingerprintCfg::default());
    if !masked_any && folded.len() == line_count {
        return None;
    }
    emit(folded.join("\n"), output, masked_any)
}

/// Apply the no-inflation guard: emit `view` only when strictly smaller than
/// `output`. A masked view is always emitted — masking can't lengthen the output,
/// and the unmasked verbatim must never be returned once a secret was found.
fn emit(view: String, output: &str, masked_any: bool) -> Option<String> {
    if masked_any {
        return Some(view);
    }
    (estimate_tokens(&view) < estimate_tokens(output)).then_some(view)
}

#[cfg(test)]
#[path = "../../../tests/unit/optimizers/command/autodetect.tests.rs"]
mod tests;
