//! A hand-written POSIX tokenizer — no regex.
//!
//! Splits a command line into top-level segments, tracking quotes, escapes,
//! `$(…)`, and backticks so it only splits at genuine top-level operators. It
//! **bails** (`None`) on anything it can't prove safe to wrap: background `&`,
//! heredocs, subshell/brace groups, top-level comments, unterminated quoting.

use super::{Op, Segment};

/// Splits command lines into [`Segment`]s. Stateless.
pub struct Segmenter;

impl Segmenter {
    /// Split `cmd` into top-level segments, or `None` to bail (run verbatim).
    #[must_use]
    pub fn split(cmd: &str) -> Option<Vec<Segment>> {
        let b = cmd.as_bytes();
        let mut segs = Vec::new();
        let mut start = 0;
        let mut i = 0;
        let mut single = false;
        let mut double = false;
        let mut backtick = false;
        let mut paren = 0usize; // `$( … )` / `$(( … ))` depth
        let mut brace = 0usize; // `${ … }` parameter-expansion depth
        while i < b.len() {
            let c = b[i];
            if single {
                single = c != b'\'';
                i += 1;
            } else if double {
                if c == b'\\' && i + 1 < b.len() {
                    i += 2;
                } else {
                    double = c != b'"';
                    i += 1;
                }
            } else if c == b'\\' {
                i += if i + 1 < b.len() { 2 } else { 1 };
            } else if c == b'\'' {
                single = true;
                i += 1;
            } else if c == b'"' {
                double = true;
                i += 1;
            } else if c == b'`' {
                backtick = !backtick;
                i += 1;
            } else if backtick {
                i += 1;
            } else if c == b'$' && b.get(i + 1) == Some(&b'(') {
                paren += 1;
                i += 2;
            } else if c == b'$' && b.get(i + 1) == Some(&b'{') {
                brace += 1;
                i += 2;
            } else if paren > 0 {
                paren += usize::from(c == b'(');
                paren -= usize::from(c == b')');
                i += 1;
            } else if brace > 0 {
                brace += usize::from(c == b'{');
                brace -= usize::from(c == b'}');
                i += 1;
            } else if is_comment(b, i) || is_heredoc(b, i) || is_group(c) {
                return None;
            } else {
                match operator_at(b, i) {
                    Tok::Bail => return None,
                    Tok::Op(op) => {
                        segs.push(Segment {
                            start,
                            end: i,
                            op_after: op,
                        });
                        i += op.width();
                        start = i;
                    }
                    Tok::Skip => i += 1,
                }
            }
        }
        if single || double || backtick || paren > 0 || brace > 0 {
            return None; // unterminated quoting/substitution
        }
        segs.push(Segment {
            start,
            end: b.len(),
            op_after: Op::End,
        });
        Some(segs)
    }
}

/// A `#` that begins a word starts a comment (rest of line) — unsafe to wrap.
fn is_comment(b: &[u8], i: usize) -> bool {
    b[i] == b'#' && (i == 0 || matches!(b[i - 1], b' ' | b'\t' | b'\n' | b';' | b'&' | b'|' | b'('))
}

/// `<<` (or `<<<`) introduces a heredoc/herestring — bail.
fn is_heredoc(b: &[u8], i: usize) -> bool {
    b[i] == b'<' && b.get(i + 1) == Some(&b'<')
}

/// Subshell `(`/`)` and brace groups `{`/`}` clash with our wrapping — bail.
const fn is_group(c: u8) -> bool {
    matches!(c, b'(' | b')' | b'{' | b'}')
}

/// What the byte at `i` means at the top level.
enum Tok {
    /// Unsafe to wrap — the whole line bails.
    Bail,
    /// Not an operator — keep scanning.
    Skip,
    /// A top-level operator that splits a segment.
    Op(Op),
}

/// Classify the byte at `i` (background `&`, `;;`, `|&` → bail).
fn operator_at(b: &[u8], i: usize) -> Tok {
    let c = b[i];
    let next = b.get(i + 1).copied();
    match c {
        b';' if next == Some(b';') => Tok::Bail, // `;;` (case)
        b'\n' | b';' => Tok::Op(Op::Seq),
        b'&' if next == Some(b'&') => Tok::Op(Op::And),
        b'&' if i > 0 && b[i - 1] == b'>' => Tok::Skip, // `>&` redirect, not bg
        b'&' if next == Some(b'>') => Tok::Skip,        // `&>` redirect, not bg
        b'&' => Tok::Bail,                              // standalone background
        b'|' if next == Some(b'|') => Tok::Op(Op::Or),
        b'|' if next == Some(b'&') => Tok::Bail, // `|&`
        b'|' => Tok::Op(Op::Pipe),
        _ => Tok::Skip,
    }
}

impl Op {
    /// The byte width of this operator's token (1 for `;`/`|`/`\n`, 2 for `&&`/`||`).
    const fn width(self) -> usize {
        match self {
            Self::And | Self::Or => 2,
            _ => 1,
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/optimizers/command/segmenter.tests.rs"]
mod tests;
