//! `session-reset` (`PreCompact`) + `update-check` (`SessionStart`) end-to-end.
//! Both bypass the master switch, write nothing to stdout, and must exit 0. They
//! act on snip's own data dir, so each asserts the on-disk side effect.

use std::fs;

use assert2::check;
use serde_json::json;
use tempfile::tempdir;

use crate::support::{Snip, stdout_str};

#[test]
fn session_reset_drops_only_the_named_session_cache() {
    // Arrange: two session caches under an isolated data root
    let snip = Snip::fresh();
    let cache = snip.home().join("session-cache");
    let target = cache.join("sess-A");
    let keep = cache.join("sess-B");
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(&keep).unwrap();
    let payload = json!({"session_id": "sess-A"}).to_string();

    // Act
    let out = snip.run(&["session-reset"], &payload);

    // Assert: the named session is gone, the other (fresh) one survives
    check!(out.status.success());
    check!(stdout_str(&out).trim().is_empty());
    check!(!target.exists());
    check!(keep.exists());
}

#[test]
fn update_check_without_a_plugin_root_is_a_noop() {
    // Arrange: command() already clears CLAUDE_PLUGIN_ROOT
    let snip = Snip::fresh();

    // Act
    let out = snip.run(&["update-check"], "");

    // Assert: silent no-op, exit 0, no throttle file written
    check!(out.status.success());
    check!(stdout_str(&out).trim().is_empty());
    check!(!snip.home().join(".update-check").exists());
}

#[test]
fn update_check_at_matching_version_records_the_throttle_and_does_not_respawn() {
    // Arrange: a plugin root declaring the binary's own version ⇒ no drift
    let plugin = tempdir().unwrap();
    fs::create_dir_all(plugin.path().join(".claude-plugin")).unwrap();
    fs::write(
        plugin.path().join(".claude-plugin").join("plugin.json"),
        json!({"version": env!("CARGO_PKG_VERSION")}).to_string(),
    )
    .unwrap();
    let snip = Snip::fresh();

    // Act
    let out = snip
        .command()
        .arg("update-check")
        .env("CLAUDE_PLUGIN_ROOT", plugin.path())
        .write_stdin(String::new())
        .output()
        .expect("snip runs");

    // Assert: silent, exit 0, and the once-a-day throttle stamp is on disk
    check!(out.status.success());
    check!(stdout_str(&out).trim().is_empty());
    check!(snip.home().join(".update-check").exists());
}
