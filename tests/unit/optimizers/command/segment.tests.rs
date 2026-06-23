//! Unit tests for the [`Segment`] accessors, in AAA form. Compiled into
//! `snip_lib` via a `#[path]` include in `src/optimizers/command/segment.rs`.

use assert2::check;

use super::Segment;
use crate::optimizers::command::Op;

#[test]
fn text_returns_the_exact_byte_range() {
    // Arrange: a segment spanning "log" within the wider command
    let cmd = "git log && ls";
    let seg = Segment {
        start: 4,
        end: 7,
        op_after: Op::And,
    };

    // Act
    let text = seg.text(cmd);

    // Assert
    check!(text == "log");
}

#[test]
fn is_blank_is_true_for_a_whitespace_only_range() {
    // Arrange: the range covers only spaces between two operators
    let cmd = "a ;   ; b";
    let seg = Segment {
        start: 3,
        end: 6,
        op_after: Op::Seq,
    };

    // Act + Assert
    check!(seg.is_blank(cmd));
}

#[test]
fn is_blank_is_false_when_the_range_has_content() {
    // Arrange: the range covers a real word (surrounding spaces ignored)
    let cmd = "  ls  ";
    let seg = Segment {
        start: 0,
        end: cmd.len(),
        op_after: Op::End,
    };

    // Act + Assert
    check!(!seg.is_blank(cmd));
}
