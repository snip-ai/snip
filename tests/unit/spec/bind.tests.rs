//! Unit tests for [`Bind`] path-scoping, in AAA form. Compiled into `snip_lib`
//! via a `#[path]` include in `src/spec/bind.rs`.

use assert2::check;

use super::{Bind, glob_match};

fn scoped(globs: &[&str]) -> Bind {
    Bind {
        path_globs: globs.iter().map(|s| (*s).to_owned()).collect(),
        ..Bind::default()
    }
}

#[test]
fn unscoped_bind_matches_any_path_or_none() {
    // Arrange: no path_globs
    let bind = Bind::default();

    // Act + Assert: an unscoped spec applies everywhere, even with no path
    check!(bind.path_matches(Some("anywhere/at/all.rs")));
    check!(bind.path_matches(None));
}

#[test]
fn scoped_bind_requires_a_matching_path() {
    // Arrange
    let bind = scoped(&["src/*", "tests/*.rs"]);

    // Act + Assert
    check!(bind.path_matches(Some("src/spec/bind.rs"))); // `*` spans separators
    check!(bind.path_matches(Some("tests/unit/spec/x.rs")));
    check!(!bind.path_matches(Some("docs/ARCHITECTURE.md")));
    check!(!bind.path_matches(None)); // scoped but no path → cannot confirm → no match
}

#[test]
fn glob_match_handles_star_question_and_anchoring() {
    // Act + Assert: `*` any sequence, `?` exactly one, full-string anchored
    check!(glob_match("*.rs", "lib.rs"));
    check!(glob_match("src/**/mod.rs", "src/a/b/mod.rs"));
    check!(glob_match("a?c", "abc"));
    check!(!glob_match("a?c", "ac"));
    check!(!glob_match("*.rs", "lib.rs.bak"));
    check!(glob_match("", ""));
    check!(glob_match("*", ""));
}
