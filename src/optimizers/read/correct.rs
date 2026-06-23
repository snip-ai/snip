//! Map a compacted-view `old_string` back to the real file's bytes (mode-aware).

use super::fuzzy::fuzzy_match;
use super::single_line_correct::single_line_correct;
use crate::config::CompactMode;
use crate::languages::{self, LanguageSpec};

/// Resolve `old_string` (copied from the compacted view) to the verbatim text of
/// the real file, or `None` when no confident unique match exists.
///
/// Soft keeps code byte-identical → an AST-anchored line fuzzy match recovers the
/// original region. Medium/high collapse code on single-line-safe languages →
/// map the substring back through the view's origin map (retrying both line-ending
/// forms, since the model may type LF while the file keeps `\r\n`, or paste CRLF
/// against an LF file); other languages fall back to the line fuzzy match. Shared
/// by `edit-fix` and `resolve`.
#[must_use]
pub fn correct_old_string(
    file_path: &str,
    file_content: &str,
    old_string: &str,
    mode: CompactMode,
) -> Option<String> {
    let spec = languages::detect(file_path);
    if mode != CompactMode::Soft && spec.is_some_and(|s| s.is_single_line_safe) {
        let spec = spec?;
        single_line_correct(file_content, old_string, spec, mode)
            .or_else(|| line_ending_retry(file_content, old_string, spec, mode))
    } else {
        // Medium/high on an indent-based language (Python) merges single-statement
        // blocks in the view; `fuzzy_match` mirrors that merge so the merged line
        // maps back to its two source lines. Soft / non-indent → one unit per line.
        let collapse_blocks = mode != CompactMode::Soft && spec.is_some_and(|s| s.indent_based);
        fuzzy_match(spec, file_content, old_string, collapse_blocks)
    }
}

/// Retry the origin-map match with `old_string`'s line endings normalized to LF
/// and to CRLF. The view keeps the file's own endings outside collapse ranges, so
/// a mismatch (model typed LF vs a `\r\n` file, or a pasted CRLF vs an LF file) is
/// recovered by whichever form matches. Each variant is tried only when it differs
/// from the original.
fn line_ending_retry(
    file_content: &str,
    old_string: &str,
    spec: &LanguageSpec,
    mode: CompactMode,
) -> Option<String> {
    let lf = old_string.replace("\r\n", "\n");
    let crlf = lf.replace('\n', "\r\n");
    [lf.as_str(), crlf.as_str()]
        .into_iter()
        .filter(|v| *v != old_string)
        .find_map(|v| single_line_correct(file_content, v, spec, mode))
}

#[cfg(test)]
#[path = "../../../tests/unit/optimizers/read/correct.tests.rs"]
mod tests;
