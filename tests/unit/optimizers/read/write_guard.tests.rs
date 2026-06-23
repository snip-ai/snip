//! Unit tests for the soft-mode Write [`should_ask`] guard, in AAA form. Compiled
//! into `snip_lib` via a `#[path]` include in `src/optimizers/read/write_guard.rs`.

use assert2::check;

use super::{jaccard_lines, should_ask};
use crate::languages::detect;

#[test]
fn asks_when_a_write_reproduces_the_compacted_view() {
    // Arrange: a file with a comment, and content that is its stripped view
    let spec = detect("a.rs").expect("a rust spec");
    let existing = "// header\nfn a() {\n    let x = 1;\n}\n";
    let stripped = "\nfn a() {\n    let x = 1;\n}\n";

    // Act
    let reason = should_ask(spec, existing, stripped);

    // Assert
    check!(reason.is_some());
}

#[test]
fn allows_a_genuine_rewrite_that_keeps_comments() {
    // Arrange: content changes code but keeps the comment
    let spec = detect("a.rs").expect("a rust spec");
    let existing = "// header\nfn a() {\n    let x = 1;\n}\n";
    let genuine = "// header\nfn a() {\n    let x = 2;\n}\n";

    // Act
    let reason = should_ask(spec, existing, genuine);

    // Assert
    check!(reason.is_none());
}

#[test]
fn allows_a_write_to_a_file_without_comments() {
    // Arrange: nothing to lose → never asks
    let spec = detect("a.rs").expect("a rust spec");
    let existing = "fn a() {\n    let x = 1;\n}\n";

    // Act
    let reason = should_ask(spec, existing, "fn a() {}\n");

    // Assert
    check!(reason.is_none());
}

#[test]
fn asks_even_when_content_carries_a_leading_snip_header() {
    // Arrange: Claude echoed the full Read output — a leading `[snip:` header line
    // precedes the stripped view; the header is dropped before comparison (34-37).
    let spec = detect("a.rs").expect("a rust spec");
    let existing = "// header\nfn a() {\n    let x = 1;\n}\n";
    let content = "[snip: read | rust | soft | 9->8 tok (-11%)]\n\nfn a() {\n    let x = 1;\n}\n";

    // Act
    let reason = should_ask(spec, existing, content);

    // Assert: still recognized as the compacted view despite the header
    check!(reason.is_some());
}

#[test]
fn allows_when_compaction_changes_almost_nothing() {
    // Arrange: one comment among many code lines, so the compacted view stays
    // >= 0.90 jaccard-similar to the original → not risky, returns None (line 39).
    let spec = detect("a.rs").expect("a rust spec");
    let existing = "// note\nfn f() {\n    let a = 1;\n    let b = 2;\n    let c = 3;\n    \
                    let d = 4;\n    let e = 5;\n    let g = 6;\n    let h = 7;\n    let i = 8;\n}\n";

    // Act: write the same code with the comment dropped
    let content = "fn f() {\n    let a = 1;\n    let b = 2;\n    let c = 3;\n    let d = 4;\n    \
                   let e = 5;\n    let g = 6;\n    let h = 7;\n    let i = 8;\n}\n";
    let reason = should_ask(spec, existing, content);

    // Assert
    check!(reason.is_none());
}

#[test]
fn allows_a_genuine_rewrite_that_drops_a_comment() {
    // Arrange: content drops the comment but is an unrelated rewrite, so it is not
    // similar to the compacted view → falls through to the trailing None (53-54).
    let spec = detect("a.rs").expect("a rust spec");
    let existing = "// old design\nfn legacy() {\n    step_one();\n    step_two();\n}\n";
    let content = "fn rebuilt() {\n    brand_new();\n    totally_other();\n    extra_line();\n}\n";

    // Act
    let reason = should_ask(spec, existing, content);

    // Assert
    check!(reason.is_none());
}

#[test]
fn jaccard_of_two_empty_texts_is_one() {
    // Arrange + Act: both line-sets empty → the early 1.0 return (line 71). The
    // sibling `union == 0` arm (line 77) is unreachable: union is 0 only when both
    // sets are empty, which this branch already handles.
    let got = jaccard_lines("", "   \n  \n");

    // Assert
    check!((got - 1.0).abs() < f64::EPSILON);
}
