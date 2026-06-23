//! Unit tests for [`Surface`] (post/pre classification), in AAA form. Compiled
//! into `snip_lib` via a `#[path]` include in `src/domain/surface.rs`.

use assert2::check;

use super::Surface;

#[test]
fn post_surfaces_are_read_grep_glob() {
    // Act + Assert: Post surfaces rewrite tool output; Pre surfaces rewrite input.
    check!(Surface::Read.is_post());
    check!(Surface::Grep.is_post());
    check!(Surface::Glob.is_post());
    check!(!Surface::Bash.is_post());
    check!(!Surface::Edit.is_post());
    check!(!Surface::Write.is_post());
}

#[test]
fn name_is_the_lowercase_surface_for_each_variant() {
    // Act + Assert: the stats/report key for every surface
    check!(Surface::Read.name() == "read");
    check!(Surface::Grep.name() == "grep");
    check!(Surface::Glob.name() == "glob");
    check!(Surface::Bash.name() == "bash");
    check!(Surface::Edit.name() == "edit");
    check!(Surface::Write.name() == "write");
}
