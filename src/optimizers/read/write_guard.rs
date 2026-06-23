//! Soft-mode Write overwrite guard: ask before a Write reproduces the stripped view.
//!
//! `Edit` is safe after a compacted read (it patches the real file), but a full
//! `Write` is not: if Claude reproduces the compacted view it saw, the original
//! comments are overwritten and lost. This asks for confirmation **only** when the
//! written content is essentially the compacted view and drops comment lines the
//! file still has; genuine rewrites pass through.

use crate::compaction::{Compactor, code_lines};
use crate::languages::LanguageSpec;

/// Jaccard similarity (over trimmed code lines) at or above which content is
/// treated as a reproduction of the compacted view. High by design.
const COMPACTED_VIEW_THRESHOLD: f64 = 0.90;

/// Return `Some(reason)` if writing `content` over `existing` looks like
/// reproducing the compacted (comment-stripped) view and would drop comments,
/// else `None`.
///
/// Guards, in order: the file must have comments to lose; compaction must
/// materially change it; `content` must be highly similar to the compacted view
/// **and** drop comment lines the original still has.
#[must_use]
pub fn should_ask(spec: &LanguageSpec, existing: &str, content: &str) -> Option<String> {
    let existing_comments = comment_line_count(spec, existing);
    if existing_comments == 0 {
        return None;
    }
    let compacted = Compactor::new(spec).compress(existing)?;
    // Strip a leading snip header line if Claude echoed the whole Read output.
    let content = content
        .strip_prefix(crate::domain::HEADER_PREFIX)
        .map_or(content, |_| {
            content
                .split_once('\n')
                .map_or(content, |(_, rest)| rest.trim_start_matches('\n'))
        });
    if jaccard_lines(&compacted, existing) >= COMPACTED_VIEW_THRESHOLD {
        return None; // compaction removed ~nothing distinguishable; not risky
    }
    let content_comments = comment_line_count(spec, content);
    if content_comments >= existing_comments {
        return None; // content keeps the comments — a genuine, safe write
    }
    if jaccard_lines(content, &compacted) >= COMPACTED_VIEW_THRESHOLD {
        let lost = existing_comments - content_comments;
        return Some(format!(
            "snip: the content being written matches the compacted view of this file \
             (comments stripped). Writing it would delete ~{lost} comment/docstring line(s) \
             from the original. Use Edit for targeted changes, or include the original \
             comments in the new content."
        ));
    }
    None
}

/// Count comment-only lines (a proxy for comment density) via the AST: the
/// non-blank lines that are not code lines.
fn comment_line_count(spec: &LanguageSpec, src: &str) -> usize {
    let non_blank = src.lines().filter(|l| !l.trim().is_empty()).count();
    let code = code_lines(spec, src).len();
    non_blank.saturating_sub(code)
}

/// Jaccard similarity of the two texts' sets of trimmed, non-empty lines.
fn jaccard_lines(a: &str, b: &str) -> f64 {
    use std::collections::HashSet;
    let set_a: HashSet<&str> = a.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let set_b: HashSet<&str> = b.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    if set_a.is_empty() && set_b.is_empty() {
        return 1.0;
    }
    let inter = set_a.intersection(&set_b).count();
    let union = set_a.union(&set_b).count();
    #[allow(clippy::cast_precision_loss)] // line counts are always << 2^53
    if union == 0 {
        0.0
    } else {
        inter as f64 / union as f64
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/optimizers/read/write_guard.tests.rs"]
mod tests;
