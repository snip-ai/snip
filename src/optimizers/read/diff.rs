//! Line-level "what changed since your last Read" diff for the dedupe path.
//!
//! On the frequent edit→re-read loop, showing the *change* beats re-serving the
//! whole file. This is a cheap `O(n)` prefix/suffix diff (no regex): it trims the
//! unchanged head and tail and prints the changed middle as `-old` / `+new`. A
//! scattered rewrite collapses to a near-full middle, which the size gate rejects
//! so the caller falls back to the normal compacted view. The diff is NOT
//! edit-safe — the header tells the model to re-read before editing.

use std::fmt::Write as _;
use std::path::Path;

use crate::domain::HEADER_PREFIX;
use crate::tokens::estimate_tokens;

/// Show the diff only when its tokens are under `1/MAX_DIFF_DENOM` of the full new
/// view — otherwise the change is too large to be worth a non-edit-safe view.
const MAX_DIFF_DENOM: usize = 2;

/// A compact diff of `old` → `new` for `file_path`, or `None` when not worthwhile
/// (identical, or the diff isn't meaningfully smaller than the full content).
pub(super) fn changed_notice(file_path: &str, old: &str, new: &str) -> Option<String> {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    let mut prefix = 0;
    while prefix < old_lines.len()
        && prefix < new_lines.len()
        && old_lines[prefix] == new_lines[prefix]
    {
        prefix += 1;
    }
    let mut suffix = 0;
    while suffix < old_lines.len() - prefix
        && suffix < new_lines.len() - prefix
        && old_lines[old_lines.len() - 1 - suffix] == new_lines[new_lines.len() - 1 - suffix]
    {
        suffix += 1;
    }
    let old_mid = &old_lines[prefix..old_lines.len() - suffix];
    let new_mid = &new_lines[prefix..new_lines.len() - suffix];
    if old_mid.is_empty() && new_mid.is_empty() {
        return None;
    }

    let filename = Path::new(file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(file_path);
    let mut body = format!(
        "{HEADER_PREFIX} dedupe diff — {filename} changed since your earlier Read this session; \
         showing only the change. This is NOT the file's full text — Re-Read (offset/limit) \
         before editing.]\n"
    );
    if prefix > 0 {
        let _ = writeln!(body, "  @@ {prefix} unchanged line(s) above @@");
    }
    for line in old_mid {
        let _ = writeln!(body, "- {line}");
    }
    for line in new_mid {
        let _ = writeln!(body, "+ {line}");
    }
    if suffix > 0 {
        let _ = writeln!(body, "  @@ {suffix} unchanged line(s) below @@");
    }

    (estimate_tokens(&body) * MAX_DIFF_DENOM < estimate_tokens(new)).then_some(body)
}

#[cfg(test)]
#[path = "../../../tests/unit/optimizers/read/diff.tests.rs"]
mod tests;
