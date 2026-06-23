//! The set of Claude Code tool surfaces snip can optimize.

use serde::{Deserialize, Serialize};

/// A Claude Code tool surface that snip can optimize.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Surface {
    /// `Read` tool output (code compaction).
    Read,
    /// `Grep` tool output.
    Grep,
    /// `Glob` tool output.
    Glob,
    /// `Bash` tool input (command routing/optimization).
    Bash,
    /// `Edit` tool input (`old_string` restoration).
    Edit,
    /// `Write` tool input (overwrite guard).
    Write,
}

impl Surface {
    /// Post surfaces rewrite tool **output** (Read/Grep/Glob); Pre surfaces
    /// rewrite tool **input** (Bash/Edit/Write).
    ///
    /// Exhaustive (not a wildcard) on purpose: a new surface variant then fails to
    /// compile until it is classified here, instead of silently defaulting to Pre
    /// and never extracting tool output.
    #[must_use]
    pub const fn is_post(self) -> bool {
        match self {
            Self::Read | Self::Grep | Self::Glob => true,
            Self::Bash | Self::Edit | Self::Write => false,
        }
    }

    /// The lowercase surface name (stats column / report key).
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Grep => "grep",
            Self::Glob => "glob",
            Self::Bash => "bash",
            Self::Edit => "edit",
            Self::Write => "write",
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/domain/surface.tests.rs"]
mod tests;
