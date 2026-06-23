//! Unit tests for [`strip_ansi`], in AAA form. Compiled into `snip_lib` via a
//! `#[path]` include in `src/spec/ansi.rs`.

use assert2::check;

use super::strip_ansi;

fn lines(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn strips_sgr_color_codes() {
    // Arrange
    let records = lines(&["\u{1b}[1;32mPASS\u{1b}[0m tests/x"]);

    // Act
    let out = strip_ansi(records);

    // Assert
    check!(out == lines(&["PASS tests/x"]));
}

#[test]
fn collapses_cr_progress_to_the_final_segment() {
    // Arrange: a progress bar overwriting one line with `\r`
    let records = lines(&["10%\r50%\r100% done"]);

    // Act
    let out = strip_ansi(records);

    // Assert
    check!(out == lines(&["100% done"]));
}

#[test]
fn keeps_a_trailing_cr_line_intact() {
    // Arrange: a CRLF artifact (lines() keeps the `\r`)
    let records = lines(&["plain text\r"]);

    // Act
    let out = strip_ansi(records);

    // Assert
    check!(out == lines(&["plain text"]));
}

#[test]
fn strips_an_osc_title_sequence_ending_in_bel() {
    // Arrange: ESC ] 0 ; <title> BEL (an OSC set-title sequence)
    let records = lines(&["\u{1b}]0;my title\u{7}prompt$ "]);

    // Act
    let out = strip_ansi(records);

    // Assert: the whole OSC sequence (through BEL) is removed
    check!(out == lines(&["prompt$ "]));
}

#[test]
fn drops_a_lone_escape_with_a_non_csi_non_osc_byte() {
    // Arrange: ESC followed by 'c' (a reset, not CSI `[` or OSC `]`) → the byte
    // after ESC is consumed by the `_ => {}` arm and nothing else is dropped
    let records = lines(&["\u{1b}cafter"]);

    // Act
    let out = strip_ansi(records);

    // Assert: both ESC and the 'c' are skipped, the rest stays
    check!(out == lines(&["after"]));
}
