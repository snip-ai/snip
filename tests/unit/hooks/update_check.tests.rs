//! Unit tests for the `update-check` hook, in AAA form. Compiled into `snip_lib`
//! via a `#[path]` include in `src/hooks/update_check.rs`, so these reach the
//! private `reconcile`/`throttled`/`touch_state` helpers.

use std::env;
use std::fs;

use assert2::check;

use super::{reconcile, run, throttled, touch_state};
use crate::clock::now_secs;

#[test]
fn run_upholds_the_exit_zero_invariant() {
    // Arrange: no plugin root → nothing to reconcile, and never an error
    temp_env::with_var_unset("CLAUDE_PLUGIN_ROOT", || {
        // Act
        let result = run(false);

        // Assert
        check!(result.is_ok());
    });
}

#[test]
fn shipped_plugin_manifest_matches_the_crate_version() {
    // Arrange: the in-repo plugin manifest the release actually ships
    let manifest = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/plugins/snip/.claude-plugin/plugin.json"
    );
    let raw = fs::read_to_string(manifest).expect("the shipped plugin manifest is readable");
    let json: serde_json::Value = serde_json::from_str(&raw).expect("the manifest is valid JSON");

    // Act
    let version = json.get("version").and_then(serde_json::Value::as_str);

    // Assert: plugin.json and Cargo.toml must never drift (release-please bumps both)
    check!(version == Some(env!("CARGO_PKG_VERSION")));
}

#[test]
fn shipped_plugin_manifest_does_not_redeclare_auto_loaded_hooks() {
    // Arrange: the in-repo plugin manifest the release actually ships
    let manifest = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/plugins/snip/.claude-plugin/plugin.json"
    );
    let raw = fs::read_to_string(manifest).expect("the shipped plugin manifest is readable");
    let json: serde_json::Value = serde_json::from_str(&raw).expect("the manifest is valid JSON");

    // Act: does the manifest `hooks` field point at the standard auto-loaded file?
    let redeclares_auto_loaded = json
        .get("hooks")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|p| p.trim_start_matches("./") == "hooks/hooks.json");

    // Assert: Claude Code auto-loads hooks/hooks.json, so re-declaring it trips
    // "Duplicate hooks file detected" -> hook-load-failed, silently unregistering
    // every snip hook (the v0.1.0 production regression).
    assert!(
        !redeclares_auto_loaded,
        "plugin.json must not set `\"hooks\": \"./hooks/hooks.json\"`: Claude Code auto-loads that \
         standard path, so re-declaring it trips \"Duplicate hooks file detected\" -> \
         hook-load-failed and silently unregisters every snip hook. Drop the manifest `hooks` \
         field; the file loads automatically."
    );
}

#[test]
fn throttled_is_true_for_a_fresh_timestamp() {
    // Arrange: an isolated data root with a just-now check timestamp
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = env::temp_dir().join(format!("snip-throttle-fresh-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    fs::create_dir_all(&home).unwrap();
    fs::write(home.join(".update-check"), now_secs().to_string()).unwrap();
    temp_env::with_var("SNIP_HOME", Some(&home), || {
        // Act
        let is_throttled = throttled();

        // Assert: a check within the 24h window throttles
        check!(is_throttled);
    });

    // Cleanup
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn throttled_is_false_for_an_old_timestamp() {
    // Arrange: a check timestamp at the epoch — far outside the window
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = env::temp_dir().join(format!("snip-throttle-old-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    fs::create_dir_all(&home).unwrap();
    fs::write(home.join(".update-check"), "0").unwrap();
    temp_env::with_var("SNIP_HOME", Some(&home), || {
        // Act
        let is_throttled = throttled();

        // Assert
        check!(!is_throttled);
    });

    // Cleanup
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn throttled_is_false_without_a_state_file() {
    // Arrange: an isolated data root with no throttle file
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = env::temp_dir().join(format!("snip-throttle-none-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    temp_env::with_var("SNIP_HOME", Some(&home), || {
        // Act
        let is_throttled = throttled();

        // Assert: a missing file means "never checked" → not throttled
        check!(!is_throttled);
    });

    // Cleanup
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn touch_state_writes_a_parseable_timestamp() {
    // Arrange: an isolated data root that doesn't yet exist
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = env::temp_dir().join(format!("snip-touch-state-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    temp_env::with_var("SNIP_HOME", Some(&home), || {
        // Act: touch_state creates the dir and writes the current time
        touch_state();

        // Assert: the file exists and parses as a u64 timestamp
        let path = home.join(".update-check");
        check!(path.exists());
        let text = fs::read_to_string(&path).unwrap();
        check!(text.trim().parse::<u64>().is_ok());
    });

    // Cleanup
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn reconcile_without_a_bootstrap_script_records_throttle() {
    // Arrange: a plugin root with no scripts dir → nothing to spawn
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = env::temp_dir().join(format!("snip-reconcile-noscript-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    let root = env::temp_dir().join(format!(
        "snip-reconcile-noscript-plugin-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    temp_env::with_vars(
        [
            ("SNIP_HOME", Some(home.as_path())),
            ("CLAUDE_PLUGIN_ROOT", Some(root.as_path())),
        ],
        || {
            // Act
            let result = reconcile(false);

            // Assert: reconciled, and the throttle file was recorded
            check!(result == Some(()));
            check!(home.join(".update-check").exists());
        },
    );

    // Cleanup
    let _ = fs::remove_dir_all(&home);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn reconcile_with_a_bootstrap_script_spawns_and_returns_some() {
    // Arrange: a plugin root with a no-op bootstrap script present
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = env::temp_dir().join(format!("snip-reconcile-spawn-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    let root = env::temp_dir().join(format!(
        "snip-reconcile-spawn-plugin-{}",
        std::process::id()
    ));
    let scripts_dir = root.join("scripts");
    fs::create_dir_all(&scripts_dir).unwrap();
    fs::write(
        scripts_dir.join("snip-bootstrap.sh"),
        "#!/usr/bin/env bash\nexit 0\n",
    )
    .unwrap();
    temp_env::with_vars(
        [
            ("SNIP_HOME", Some(home.as_path())),
            ("CLAUDE_PLUGIN_ROOT", Some(root.as_path())),
        ],
        || {
            // Act: it tries to spawn the bootstrap detached (best-effort)
            let result = reconcile(false);

            // Assert: never panics, always returns Some (spawn failure is swallowed)
            check!(result == Some(()));
        },
    );

    // Cleanup
    let _ = fs::remove_dir_all(&home);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn reconcile_throttled_without_force_keeps_the_existing_timestamp() {
    // Arrange: a fresh throttle stamp (within the 24h window) → would throttle
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = env::temp_dir().join(format!("snip-reconcile-throttled-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    fs::create_dir_all(&home).unwrap();
    let root = env::temp_dir().join(format!(
        "snip-reconcile-throttled-plugin-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    let stamp = now_secs() - 100;
    fs::write(home.join(".update-check"), stamp.to_string()).unwrap();
    temp_env::with_vars(
        [
            ("SNIP_HOME", Some(home.as_path())),
            ("CLAUDE_PLUGIN_ROOT", Some(root.as_path())),
        ],
        || {
            // Act: not forced → the throttle short-circuits before touch_state
            let result = reconcile(false);

            // Assert: returned Some, and the stamp is untouched (no re-check happened)
            check!(result == Some(()));
            let text = fs::read_to_string(home.join(".update-check")).unwrap();
            check!(text.trim().parse::<u64>() == Ok(stamp));
        },
    );

    // Cleanup
    let _ = fs::remove_dir_all(&home);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn reconcile_forced_bypasses_throttle_and_updates_the_timestamp() {
    // Arrange: the same fresh throttle stamp that would normally short-circuit
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = env::temp_dir().join(format!("snip-reconcile-forced-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    fs::create_dir_all(&home).unwrap();
    let root = env::temp_dir().join(format!(
        "snip-reconcile-forced-plugin-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    let stamp = now_secs() - 100;
    fs::write(home.join(".update-check"), stamp.to_string()).unwrap();
    temp_env::with_vars(
        [
            ("SNIP_HOME", Some(home.as_path())),
            ("CLAUDE_PLUGIN_ROOT", Some(root.as_path())),
        ],
        || {
            // Act: forced → skip the throttle and re-check now
            let result = reconcile(true);

            // Assert: returned Some, and the stamp advanced past the old one
            check!(result == Some(()));
            let text = fs::read_to_string(home.join(".update-check")).unwrap();
            let written = text.trim().parse::<u64>().unwrap();
            check!(written > stamp);
        },
    );

    // Cleanup
    let _ = fs::remove_dir_all(&home);
    let _ = fs::remove_dir_all(&root);
}
