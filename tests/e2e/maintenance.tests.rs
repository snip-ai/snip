//! `session-reset` (`PreCompact`) + `update-check` (`SessionStart`) end-to-end.
//! Both bypass the master switch, write nothing to stdout, and must exit 0. They
//! act on snip's own data dir, so each asserts the on-disk side effect.

use std::fs;

use assert2::check;
use serde_json::json;

use crate::support::{Snip, stdout_json, stdout_str};

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
fn update_check_when_due_records_throttle_and_flags_a_fetch() {
    // Arrange: a fresh data root (no prior throttle stamp) ⇒ a fetch is due
    let snip = Snip::fresh();

    // Act
    let out = snip.run(&["update-check"], "");

    // Assert: silent, exit 0, the throttle is recorded, and the `.fetch-due`
    // sentinel is dropped for snip-run.sh — which does the actual spawn, because a
    // native binary can't spawn a shell that survives its own exit on Windows
    check!(out.status.success());
    check!(stdout_str(&out).trim().is_empty());
    check!(snip.home().join(".update-check").exists());
    check!(snip.home().join(".fetch-due").exists());
}

#[test]
fn update_check_emits_and_consumes_a_pending_lifecycle_banner() {
    let snip = Snip::fresh();
    fs::write(snip.home().join(".lifecycle"), "installed 9.9.9\n").unwrap();

    let out = snip.run(&["update-check"], "");

    check!(out.status.success());
    let banner = stdout_json(&out);
    check!(banner["hookSpecificOutput"]["hookEventName"] == "SessionStart");
    check!(
        banner["systemMessage"]
            .as_str()
            .unwrap_or_default()
            .contains("9.9.9")
    );
    // The model channel is never touched — only `systemMessage` (user-visible).
    check!(banner.get("additionalContext").is_none());
    check!(
        banner["hookSpecificOutput"]
            .get("additionalContext")
            .is_none()
    );
    // Consumed exactly once.
    check!(!snip.home().join(".lifecycle").exists());
}

#[test]
fn update_check_when_throttled_is_a_silent_noop() {
    // Arrange: a just-now throttle stamp (within the 24h window)
    let snip = Snip::fresh();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock after the epoch")
        .as_secs();
    fs::write(snip.home().join(".update-check"), now.to_string()).unwrap();

    // Act
    let out = snip.run(&["update-check"], "");

    // Assert: silent no-op, exit 0, and no fetch is flagged while throttled
    check!(out.status.success());
    check!(stdout_str(&out).trim().is_empty());
    check!(!snip.home().join(".fetch-due").exists());
}
