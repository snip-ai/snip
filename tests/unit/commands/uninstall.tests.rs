//! Unit tests for `commands::uninstall` — purging snip's on-disk state.

use super::*;
use assert2::check;
use std::fs;

#[test]
fn purge_state_keeps_bin_and_marker() {
    // Arrange
    let data = tempfile::TempDir::new().expect("temp data");
    let root = data.path();
    fs::create_dir_all(root.join("bin")).expect("bin dir");
    fs::write(root.join("bin").join("snip"), b"binary").expect("fake binary");
    fs::write(root.join("config.json"), b"{}").expect("config");
    fs::create_dir_all(root.join("session-cache").join("no-session")).expect("cache");
    fs::write(root.join(UNINSTALL_MARKER), b"").expect("marker");

    // Act
    purge_state(root);

    // Assert
    check!(!root.join("config.json").exists());
    check!(!root.join("session-cache").exists());
    check!(root.join("bin").join("snip").exists());
    check!(root.join(UNINSTALL_MARKER).exists());
}
