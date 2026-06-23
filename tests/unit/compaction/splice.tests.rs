//! Unit tests for byte-range splicing + full-line residue expansion, in AAA form.
//! Compiled into `snip_lib` via a `#[path]` include in `src/compaction/splice.rs`.

use assert2::check;

use super::{full_line_expanded, splice_out};

/// Byte range of `needle` in `hay`; `eat_nl` extends it over the following `\n`
/// to mimic a grammar (e.g. Rust `///`) whose comment node swallows its newline.
fn span(hay: &str, needle: &str, eat_nl: bool) -> (usize, usize) {
    let start = hay.find(needle).expect("needle present");
    let end = start + needle.len();
    let end = if eat_nl && hay.as_bytes().get(end) == Some(&b'\n') {
        end + 1
    } else {
        end
    };
    (start, end)
}

#[test]
fn splice_out_removes_sorted_ranges() {
    // Arrange
    let src = "abcdefgh";

    // Act
    let out = splice_out(src, &[(1, 3), (5, 6)]);

    // Assert: "bc" and "f" gone, the rest verbatim
    check!(out == "adegh");
}

#[test]
fn regular_full_line_comment_takes_its_whole_line() {
    // Arrange: a `//` node that does NOT include the trailing newline
    let src = "fn f() {\n    // note here\n    body;\n}\n";
    let ranges = vec![span(src, "// note here", false)];

    // Act
    let out = splice_out(src, &full_line_expanded(src, &ranges));

    // Assert: the indented comment line vanishes whole — no `    ` residue, code intact
    check!(out == "fn f() {\n    body;\n}\n");
}

#[test]
fn doc_comment_node_that_ate_its_newline_does_not_orphan_indent() {
    // Arrange: simulate Rust `///` — the comment node range includes the `\n`.
    // The pre-fix bug glued the line's 4-space indent onto `fn f`.
    let src = "impl X {\n    /// doc line\n    fn f() {}\n}\n";
    let ranges = vec![span(src, "/// doc line", true)];

    // Act
    let out = splice_out(src, &full_line_expanded(src, &ranges));

    // Assert: `fn f` keeps EXACTLY its 4-space indent (byte-identical code)
    check!(out == "impl X {\n    fn f() {}\n}\n");
}

#[test]
fn consecutive_doc_lines_each_clean_up_their_indent() {
    // Arrange: three stacked `///` lines, each node swallowing its newline
    let src = "impl X {\n    /// a\n    /// b\n    /// c\n    fn f() {}\n}\n";
    let ranges = vec![
        span(src, "/// a", true),
        span(src, "/// b", true),
        span(src, "/// c", true),
    ];

    // Act
    let out = splice_out(src, &full_line_expanded(src, &ranges));

    // Assert: no accumulated indent — `fn f` stays at 4 spaces
    check!(out == "impl X {\n    fn f() {}\n}\n");
}

#[test]
fn trailing_comment_drops_its_leading_gap_but_keeps_the_code() {
    // Arrange: code precedes the comment, separated by a whitespace gap
    let src = "    let x = 1;    // tail\n";
    let ranges = vec![span(src, "// tail", false)];

    // Act
    let out = splice_out(src, &full_line_expanded(src, &ranges));

    // Assert: the comment AND the gap before it go; the code keeps no trailing space
    check!(out == "    let x = 1;\n");
}

#[test]
fn full_line_block_comment_with_code_after_it_is_left_alone() {
    // Arrange: a full-line-start block comment but with code after it on the line —
    // widening to the newline would delete that code, so it must keep the node range
    let src = "    /* note */ let y = 2;\n";
    let ranges = vec![span(src, "/* note */", false)];

    // Act
    let out = splice_out(src, &full_line_expanded(src, &ranges));

    // Assert: only the comment bytes go; `let y = 2;` is preserved
    check!(out == "     let y = 2;\n");
}
