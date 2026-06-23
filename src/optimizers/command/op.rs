//! The top-level shell operator that follows a command segment.

/// The operator separating one segment from the next.
///
/// Recognized at the top level (outside quotes, `$(…)`, backticks). `Pipe` keeps
/// segments inside one unit; the rest break units apart. Background `&` and
/// heredocs make the segmenter bail — they are never safe to wrap.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Op {
    /// `;` or a newline — sequential.
    Seq,
    /// `&&` — run next only on success.
    And,
    /// `||` — run next only on failure.
    Or,
    /// `|` — pipe into the next stage (stays within a unit).
    Pipe,
    /// End of input — no following operator.
    End,
}

impl Op {
    /// Whether this operator ends a unit (everything except a pipe).
    #[must_use]
    pub const fn breaks_unit(self) -> bool {
        !matches!(self, Self::Pipe)
    }
}
