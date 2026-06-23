//! A unit: a maximal run of pipe-connected segments.
//!
//! Only the **last** stage's stdout is visible, so that stage decides
//! recognition; the whole unit's bytes are what gets wrapped (pipes stay inside,
//! never marked mid-pipe).

use super::{Op, Segment};

/// One wrappable unit of a command line.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Unit {
    /// Start byte of the unit (its first pipe stage).
    pub start: usize,
    /// End byte of the unit (its last pipe stage).
    pub end: usize,
    /// The unit-breaking operator that followed it (`Seq`/`And`/`Or`/`End`).
    pub op_after: Op,
    /// Byte range of the last pipe stage — the visible-stdout command.
    pub last: (usize, usize),
}

impl Unit {
    /// Group `segments` into units, breaking at every non-pipe operator.
    #[must_use]
    pub fn build(segments: &[Segment]) -> Vec<Self> {
        let mut units = Vec::new();
        let mut start: Option<usize> = None;
        for seg in segments {
            let unit_start = *start.get_or_insert(seg.start);
            if seg.op_after.breaks_unit() {
                units.push(Self {
                    start: unit_start,
                    end: seg.end,
                    op_after: seg.op_after,
                    last: (seg.start, seg.end),
                });
                start = None;
            }
        }
        units
    }

    /// The unit's exact bytes (wrapped verbatim — quote-safe, never re-tokenized).
    #[must_use]
    pub fn text<'a>(&self, cmd: &'a str) -> &'a str {
        &cmd[self.start..self.end]
    }

    /// The last pipe stage's bytes — used to identify the command.
    #[must_use]
    pub fn last_text<'a>(&self, cmd: &'a str) -> &'a str {
        &cmd[self.last.0..self.last.1]
    }

    /// Whether the unit is blank (only whitespace — e.g. from a trailing `;`).
    #[must_use]
    pub fn is_blank(&self, cmd: &str) -> bool {
        self.text(cmd).trim().is_empty()
    }

    /// Whether the last stage redirects its stdout (`>`/`>>`) — then there's no
    /// visible output to optimize. Quote-aware; ignores `2>`/`>&` stderr forms
    /// only loosely (a redirected unit is simply left verbatim, which is safe).
    #[must_use]
    pub fn redirects_stdout(&self, cmd: &str) -> bool {
        let text = self.last_text(cmd);
        let bytes = text.as_bytes();
        let (mut single, mut double) = (false, false);
        for (i, &c) in bytes.iter().enumerate() {
            match c {
                b'\'' if !double => single = !single,
                b'"' if !single => double = !double,
                b'>' if !single && !double => {
                    // `2>` / `1>` are stderr/explicit-fd forms; a bare `>` or
                    // `>>` targets stdout.
                    let prev = i.checked_sub(1).map(|p| bytes[p]);
                    if !matches!(prev, Some(b'2' | b'&')) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/optimizers/command/unit.tests.rs"]
mod tests;
