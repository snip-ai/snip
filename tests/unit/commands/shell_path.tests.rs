//! Unit tests for `commands::shell_path` — stripping the marked PATH block.

use super::*;
use assert2::check;
use std::fs;

#[test]
fn strip_block_removes_the_marked_region() {
    // Arrange
    let content = "before\n# >>> snip shell setup >>>\nexport PATH=\"x:$PATH\"\n# <<< snip shell setup <<<\nafter\n";

    // Act
    let out = strip_block(content);

    // Assert
    assert2::assert!(let Some(text) = out);
    check!(!text.contains(MARK_BEGIN));
    check!(!text.contains("export PATH"));
    check!(text.contains("before"));
    check!(text.contains("after"));
}

#[test]
fn strip_block_returns_none_when_no_block_present() {
    // Arrange
    let content = "export PATH=\"x:$PATH\"\n";

    // Act
    let out = strip_block(content);

    // Assert
    check!(out.is_none());
}

#[test]
fn strip_path_from_rcs_rewrites_only_matching_files() {
    // Arrange
    let home = tempfile::TempDir::new().expect("temp home");
    let bashrc = home.path().join(".bashrc");
    fs::write(
        &bashrc,
        "keep\n# >>> snip shell setup >>>\nexport PATH=\"p\"\n# <<< snip shell setup <<<\n",
    )
    .expect("write bashrc");
    fs::write(home.path().join(".zshrc"), "no markers here\n").expect("write zshrc");

    // Act
    let changed = strip_path_from_rcs(Some(home.path()));

    // Assert
    check!(changed == vec![bashrc.clone()]);
    let after = fs::read_to_string(&bashrc).expect("read bashrc");
    check!(!after.contains(MARK_BEGIN));
    check!(after.contains("keep"));
}

#[test]
fn strip_path_from_rcs_with_no_home_is_empty() {
    // Arrange — (no home directory available)

    // Act
    let changed = strip_path_from_rcs(None);

    // Assert
    check!(changed.is_empty());
}
