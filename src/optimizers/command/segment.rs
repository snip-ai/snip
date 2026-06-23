//! One top-level command segment: a byte range plus the operator after it.

use super::Op;

/// A top-level segment of a command line.
///
/// A half-open byte range `[start, end)` into the original command plus the
/// [`Op`] that followed it. Bytes are kept verbatim, never re-tokenized
/// (lossless, quote-safe).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Segment {
    /// Start byte offset (inclusive) into the original command.
    pub start: usize,
    /// End byte offset (exclusive) into the original command.
    pub end: usize,
    /// The operator separating this segment from the next.
    pub op_after: Op,
}

impl Segment {
    /// The segment's exact bytes within `cmd`.
    #[must_use]
    pub fn text<'a>(&self, cmd: &'a str) -> &'a str {
        &cmd[self.start..self.end]
    }

    /// Whether the segment is empty once surrounding whitespace is ignored.
    #[must_use]
    pub fn is_blank(&self, cmd: &str) -> bool {
        self.text(cmd).trim().is_empty()
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/optimizers/command/segment.tests.rs"]
mod tests;
