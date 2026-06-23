//! Edit-safety correction through the public API, in AAA form: `correct_old_string`
//! maps an `old_string` copied from the comment-stripped soft view back to the real
//! file bytes (comments intact), and refuses to guess when nothing matches. Pure —
//! the content is passed in directly, so no filesystem or shell is involved.

use assert2::check;
use snip_lib::config::CompactMode;
use snip_lib::optimizers::read::correct::correct_old_string;

#[test]
fn soft_maps_a_comment_stripped_old_string_back_to_real_bytes() {
    // Arrange: the file keeps a trailing `// go` comment the soft view dropped, so
    // the model's `old_string` (comment-free) must map back to the commented span.
    let file = "fn main() {\n    do_it(); // go\n    done();\n}\n";
    let old = "fn main() {\n    do_it();\n    done();\n}\n";

    // Act
    let got = correct_old_string("a.rs", file, old, CompactMode::Soft);

    // Assert: the recovered text is the real, comment-bearing region of the file
    assert2::assert!(let Some(text) = got);
    check!(text.contains("do_it(); // go"));
    check!(file.contains(&text));
}

#[test]
fn soft_returns_none_when_no_confident_match_exists() {
    // Arrange: an old_string whose lines appear nowhere in the file
    let file = "fn a() {\n    one();\n    two();\n}\n";
    let old = "fn b() {\n    nope();\n    never();\n}\n";

    // Act
    let got = correct_old_string("a.rs", file, old, CompactMode::Soft);

    // Assert
    check!(got.is_none());
}
