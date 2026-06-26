//! Optimized view for an unrecognized command's captured output.
//!
//! Tried in order: a pure source-file dump (`cat`/`head`/`bat` over a file whose
//! extension a language grammar recognizes) compacted by the same AST read engine
//! the Read surface uses; then structured-output auto-detect (JSON/TOON minify or
//! a repetitive-log fold); else the raw text. The caller caps + spills whatever
//! this returns, so nothing is ever discarded.

use crate::compaction::Compactor;
use crate::config::Config;
use crate::domain::HEADER_PREFIX;
use crate::languages;
use crate::optimizers::command::autodetect;
use crate::overflow::Spill;
use crate::tokens::estimate_tokens;

use super::recognition;

/// File-dump commands whose stdout is (a slice of) a single file's bytes.
const DUMP_CMDS: &[&str] = &["cat", "head", "bat", "batcat", "tac", "nl"];

/// The best optimized view of `text` (an unrecognized command's stdout). Pure: the
/// caller wraps this in the panic guard and applies the overflow cap.
pub(super) fn optimized_view(
    command: &str,
    text: &str,
    cfg: &Config,
    session: Option<&str>,
) -> String {
    source_dump(command, text, cfg).unwrap_or_else(|| {
        autodetect::compact(text, cfg.autodetect_for("command"), cfg.secret_safe).map_or_else(
            || text.to_owned(),
            |folded| {
                // A lossy log fold drops distinct lines; keep the full original
                // recoverable. JSON encoders and byte-identical folds lose nothing.
                let lossy_fold = !cfg.secret_safe
                    && !autodetect::is_json_shaped(text)
                    && autodetect::fold_is_lossy(&folded, text);
                if lossy_fold {
                    Spill::keep_recoverable(&folded, text, session, "command-autodetect")
                } else {
                    folded
                }
            },
        )
    })
}

/// Compact a pure source-file dump through the read engine, or `None` when the
/// command isn't a clean single-file dump of a recognized language (or compaction
/// wouldn't save tokens). A partial slice that won't parse falls back to `None`.
fn source_dump(command: &str, text: &str, cfg: &Config) -> Option<String> {
    if command.contains(['|', '&', ';', '<', '>']) {
        return None; // a pipe/redirect/sequence — not a single clean file dump
    }
    let (argv0, _) = recognition::parse(command)?;
    if !DUMP_CMDS.contains(&argv0.as_str()) {
        return None;
    }
    // The dumped file is the last command word an extension maps to a language.
    let spec = command
        .split_whitespace()
        .rev()
        .find_map(|word| languages::detect(word))?;
    let mode = cfg.mode_for("read");
    let body = Compactor::new(spec).compress_mode(text, mode)?;
    let before = estimate_tokens(text);
    let after = estimate_tokens(&body);
    let pct = 100 - after.min(before) * 100 / before.max(1);
    let view = format!(
        "{HEADER_PREFIX} command source-compacted | {} | {} | {before}→{after} tok (-{pct}%) — \
         comments stripped from the dumped file; re-run the command for verbatim bytes.]\n{body}",
        spec.name,
        mode.as_str(),
    );
    // No-inflation at the VIEW level (header included): a comment-light file whose
    // savings don't cover the banner falls through to auto-detect / raw.
    (estimate_tokens(&view) < before).then_some(view)
}

#[cfg(test)]
#[path = "../../../tests/unit/optimizers/command/unrecognized.tests.rs"]
mod tests;
