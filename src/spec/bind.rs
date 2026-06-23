//! How a tool event is matched to a spec.

use serde::{Deserialize, Serialize};

/// How a tool event is matched to a spec.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Bind {
    /// Bash argv0 — e.g. `"git"`, `"cargo"`, `"ls"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cmd: Option<String>,
    /// Sub-commands — e.g. `"status"`, `"diff"`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subcommands: Vec<String>,
    /// Glob(s) the Grep/Glob search `path` must match for the spec to apply.
    /// Empty = unscoped (matches every path). `*` spans separators, `?` is one char.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub path_globs: Vec<String>,
}

impl Bind {
    /// Whether `path` is in scope for this spec.
    ///
    /// `true` when no `path_globs` are set (unscoped); otherwise `path` must match
    /// at least one glob. A scoped spec with no path available does **not** match —
    /// the scope can't be confirmed, so the spec stays out of the way.
    #[must_use]
    pub fn path_matches(&self, path: Option<&str>) -> bool {
        self.path_globs.is_empty()
            || path.is_some_and(|p| self.path_globs.iter().any(|g| glob_match(g, p)))
    }
}

/// Whether `text` matches `pattern`, where `*` is any (possibly empty) sequence
/// and `?` is exactly one character. Regex-free — the classic linear wildcard
/// match with backtracking; `*` deliberately spans `/` (so `src/*` is `**`-like).
fn glob_match(pattern: &str, text: &str) -> bool {
    let (p, t) = (pattern.as_bytes(), text.as_bytes());
    let (mut pi, mut ti) = (0, 0);
    // The last `*` seen and the text position to resume from on a backtrack.
    let (mut star, mut resume) = (None, 0);
    while ti < t.len() {
        if pi < p.len() && (p[pi] == b'?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == b'*' {
            star = Some(pi);
            resume = ti;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            resume += 1;
            ti = resume;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == b'*' {
        pi += 1;
    }
    pi == p.len()
}

#[cfg(test)]
#[path = "../../tests/unit/spec/bind.tests.rs"]
mod tests;
