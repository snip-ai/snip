//! Unit tests for the POSIX [`Segmenter`], in AAA form. Compiled into `snip_lib`
//! via a `#[path]` include in `src/commands/route/segmenter.rs`.

use assert2::check;

use super::Segmenter;
use crate::optimizers::command::Op;

/// The `op_after` sequence of every segment (the segmentation's shape).
fn ops(cmd: &str) -> Option<Vec<Op>> {
    Segmenter::split(cmd).map(|segs| segs.iter().map(|s| s.op_after).collect())
}

#[test]
fn a_simple_command_is_one_segment() {
    // Arrange + Act
    let segs = Segmenter::split("git status").expect("a simple command never bails");

    // Assert
    check!(segs.len() == 1);
    check!(segs[0].op_after == Op::End);
    check!(segs[0].text("git status") == "git status");
}

#[test]
fn sequence_and_conditional_operators_split_at_top_level() {
    // Arrange + Act + Assert
    check!(ops("a && b") == Some(vec![Op::And, Op::End]));
    check!(ops("a || b") == Some(vec![Op::Or, Op::End]));
    check!(ops("a; b; c") == Some(vec![Op::Seq, Op::Seq, Op::End]));
    check!(ops("a\nb") == Some(vec![Op::Seq, Op::End]));
    check!(ops("a | b") == Some(vec![Op::Pipe, Op::End]));
}

#[test]
fn operators_inside_quotes_and_substitutions_do_not_split() {
    // Arrange + Act + Assert: each stays a single segment
    check!(ops(r#"echo "a && b""#) == Some(vec![Op::End]));
    check!(ops("echo 'a; b | c'") == Some(vec![Op::End]));
    check!(ops("echo $(a && b)") == Some(vec![Op::End]));
    check!(ops("echo `a | b`") == Some(vec![Op::End]));
    check!(ops("echo ${HOME}/x") == Some(vec![Op::End]));
}

#[test]
fn redirections_are_not_split_points() {
    // Arrange + Act + Assert: `2>&1` and `>` stay inside the segment
    check!(ops("make 2>&1") == Some(vec![Op::End]));
    check!(ops("cmd > out.txt") == Some(vec![Op::End]));
}

#[test]
fn dangerous_constructs_bail() {
    // Arrange + Act + Assert: background, heredoc, comment, groups, `;;`
    check!(Segmenter::split("server &").is_none());
    check!(Segmenter::split("cat <<EOF").is_none());
    check!(Segmenter::split("ls # a comment").is_none());
    check!(Segmenter::split("(cd x && y)").is_none());
    check!(Segmenter::split("{ a; b; }").is_none());
    check!(Segmenter::split("case x in a) ;; esac").is_none());
}

#[test]
fn unterminated_quoting_bails() {
    // Arrange + Act + Assert
    check!(Segmenter::split(r#"echo "unterminated"#).is_none());
    check!(Segmenter::split("echo $(unbalanced").is_none());
}

#[test]
fn segment_ranges_cover_the_pieces_around_operators() {
    // Arrange
    let cmd = "git log && ls";

    // Act
    let segs = Segmenter::split(cmd).expect("valid line");

    // Assert: two segments, byte ranges map back to the original text
    check!(segs.len() == 2);
    check!(segs[0].text(cmd).trim() == "git log");
    check!(segs[1].text(cmd).trim() == "ls");
}

#[test]
fn escaped_operator_inside_double_quotes_does_not_split() {
    // Arrange + Act + Assert: the `\"` keeps the double-quote state open so the
    // `&&` stays quoted and the line is a single segment
    check!(ops(r#"echo "a \" && b""#) == Some(vec![Op::End]));
}

#[test]
fn trailing_backslash_at_top_level_does_not_split() {
    // Arrange + Act: a dangling backslash is the last byte (the `else { 1 }` arm)
    let segs = Segmenter::split(r"echo hi \").expect("a trailing backslash is safe");

    // Assert: still one segment covering the whole line
    check!(segs.len() == 1);
    check!(segs[0].op_after == Op::End);
}
