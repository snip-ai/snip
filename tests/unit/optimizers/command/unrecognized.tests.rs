//! Unit tests for the unrecognized-output optimizer, in AAA form. Compiled into
//! `snip_lib` via a `#[path]` include in `src/optimizers/command/unrecognized.rs`.

use std::fmt::Write;

use assert2::check;

use super::optimized_view;
use crate::config::Config;

#[test]
fn cat_of_a_recognized_source_file_is_compacted_by_the_read_engine() {
    // Arrange: enough comments that stripping beats the injected banner's cost.
    let cfg = Config::default();
    let mut src = String::new();
    for i in 0..40 {
        let _ = writeln!(src, "// explanatory remark number {i} about the code below");
    }
    src.push_str("fn main() {\n    let answer = 42;\n}\n");

    // Act: the file is dumped via `cat`, and the path's `.rs` selects the grammar.
    let view = optimized_view("cat src/main.rs", &src, &cfg, None);

    // Assert: the read engine ran — comments gone, code kept, banner present.
    check!(view.contains("source-compacted"));
    check!(view.contains("rust"));
    check!(!view.contains("explanatory remark"));
    check!(view.contains("fn main()"));
    check!(view.contains("let answer = 42;"));
}

#[test]
fn a_non_dump_command_skips_the_read_engine() {
    // Arrange: identical-looking source, but `grep` is not a file-dump command, and
    // the short body has nothing for auto-detect to fold.
    let cfg = Config::default();
    let src = "// a comment\nfn main() {}\n";

    // Act
    let view = optimized_view("grep needle bar.rs", src, &cfg, None);

    // Assert: left byte-identical (the caller still applies the overflow cap).
    check!(!view.contains("source-compacted"));
    check!(view == src);
}

#[test]
fn a_comment_free_source_dump_does_not_inflate() {
    // Arrange: soft mode strips only comments, so a comment-free file yields no
    // savings; the banner must not be injected (no-inflation at the view level).
    let cfg = Config::default();
    let src = "fn main() {\n    let x = 1;\n    let y = 2;\n}\n";

    // Act
    let view = optimized_view("cat src/lib.rs", src, &cfg, None);

    // Assert
    check!(!view.contains("source-compacted"));
    check!(view == src);
}
