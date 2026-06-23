//! Unit tests for the [`Unit`] builder, in AAA form. Compiled into `snip_lib`
//! via a `#[path]` include in `src/commands/route/unit.rs`.

use assert2::check;

use super::Unit;
use crate::optimizers::command::{Op, Segmenter};

fn units(cmd: &str) -> Vec<Unit> {
    Unit::build(&Segmenter::split(cmd).expect("valid line"))
}

#[test]
fn pipes_stay_inside_one_unit_sequences_break_units() {
    // Arrange + Act: `a | b` is one unit; `&& c` starts another
    let cmd = "a | b && c";
    let units = units(cmd);

    // Assert
    check!(units.len() == 2);
    check!(units[0].op_after == Op::And);
    check!(units[0].text(cmd).trim() == "a | b");
    check!(units[0].last_text(cmd).trim() == "b"); // last stage = visible stdout
    check!(units[1].last_text(cmd).trim() == "c");
}

#[test]
fn blank_trailing_segment_is_a_blank_unit() {
    // Arrange + Act: a trailing `;` yields an empty unit
    let cmd = "ls;";
    let units = units(cmd);

    // Assert: the blank one is detectable so the planner can skip it
    check!(units.iter().any(|u| u.is_blank(cmd)));
    check!(units.iter().any(|u| !u.is_blank(cmd)));
}

#[test]
fn stdout_redirection_is_detected_but_stderr_forms_are_not() {
    // Arrange
    let redirected = "echo hi > out.txt";
    let stderr_only = "make 2> errs.txt";
    let plain = "echo hi";

    // Act
    let r = units(redirected)[0].redirects_stdout(redirected);
    let s = units(stderr_only)[0].redirects_stdout(stderr_only);
    let p = units(plain)[0].redirects_stdout(plain);

    // Assert
    check!(r);
    check!(!s);
    check!(!p);
}

#[test]
fn redirection_char_inside_single_quotes_is_not_a_redirect() {
    // Arrange: the `>` is quoted, so it's an argument, not a stdout redirect
    let cmd = "echo '>' there";

    // Act
    let redirected = units(cmd)[0].redirects_stdout(cmd);

    // Assert
    check!(!redirected);
}
