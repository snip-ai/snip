//! Shell rc PATH-block management — the opt-in line `snip-shell-setup.sh` writes.
//!
//! Today only [`strip_path_from_rcs`] is needed (by `snip uninstall`); it removes
//! the clearly-marked block from every candidate rc so a teardown leaves no trace.

use std::fs;
use std::path::{Path, PathBuf};

/// Begin/end markers of the PATH block written by `snip-shell-setup.sh`.
/// KEEP IN SYNC with `plugins/snip/scripts/snip-shell-setup.sh`.
pub(crate) const MARK_BEGIN: &str = "# >>> snip shell setup >>>";
pub(crate) const MARK_END: &str = "# <<< snip shell setup <<<";

/// Strip the marked PATH block from every candidate rc under `home`, returning
/// the files actually changed. A missing home or unreadable file is skipped.
pub(crate) fn strip_path_from_rcs(home: Option<&Path>) -> Vec<PathBuf> {
    let Some(home) = home else {
        return Vec::new();
    };
    let mut changed = Vec::new();
    for rc in rc_candidates(home) {
        let Ok(content) = fs::read_to_string(&rc) else {
            continue;
        };
        if let Some(stripped) = strip_block(&content)
            && fs::write(&rc, stripped).is_ok()
        {
            changed.push(rc);
        }
    }
    changed
}

/// Candidate shell rc files that may hold the snip PATH block.
fn rc_candidates(home: &Path) -> Vec<PathBuf> {
    [".bashrc", ".bash_profile", ".zshrc", ".profile"]
        .iter()
        .map(|f| home.join(f))
        .collect()
}

/// Drop the `MARK_BEGIN..=MARK_END` block (inclusive). Returns `None` when no
/// block is present so the caller can skip an unnecessary rewrite.
fn strip_block(content: &str) -> Option<String> {
    if !content.contains(MARK_BEGIN) {
        return None;
    }
    let mut out = String::with_capacity(content.len());
    let mut in_block = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == MARK_BEGIN {
            in_block = true;
        } else if trimmed == MARK_END {
            in_block = false;
        } else if !in_block {
            out.push_str(line);
            out.push('\n');
        }
    }
    Some(out)
}

#[cfg(test)]
#[path = "../../tests/unit/commands/shell_path.tests.rs"]
mod tests;
