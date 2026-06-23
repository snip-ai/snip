//! Unit tests for the `config` command backends, in AAA form. Compiled into
//! `snip_lib` via a `#[path]` include in `src/commands/config_cmd.rs`, so these
//! reach the private `parse_scalar`/`assign`/`navigate` helpers.

use std::env;
use std::fs;

use assert2::check;
use serde_json::json;

use super::{assign, navigate, parse_scalar, run};
use crate::config::Config;

#[test]
fn parse_scalar_classifies_bool_int_string() {
    // Act + Assert (pure lookups)
    check!(parse_scalar("true") == json!(true));
    check!(parse_scalar("false") == json!(false));
    check!(parse_scalar("4000") == json!(4000));
    check!(parse_scalar("0.6") == json!(0.6)); // float path (head_frac etc.)
    check!(parse_scalar("soft") == json!("soft"));
}

#[test]
fn assign_then_navigate_round_trips_a_dotted_path() {
    // Arrange
    let mut root = json!({});

    // Act
    assign(&mut root, "optimizers.read.enabled", json!(false));

    // Assert
    check!(navigate(&root, "optimizers.read.enabled") == Some(&json!(false)));
    check!(navigate(&root, "missing.path").is_none());
}

#[test]
fn set_persists_through_the_config_file() {
    // Arrange: point snip at a throwaway, process-unique config file. The env lock
    // serializes the process-global `SNIP_CONFIG_PATH` mutation against the other
    // env-mutating config tests (otherwise a concurrent test can clobber it).
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let path = env::temp_dir().join(format!("snip-test-config-{}.json", std::process::id()));
    let _ = fs::remove_file(&path);
    temp_env::with_var("SNIP_CONFIG_PATH", Some(&path), || {
        // Act
        run(&["set".into(), "master_enabled".into(), "false".into()]).unwrap();

        // Assert: the on-disk config now reads disabled
        check!(!Config::load_raw().master_enabled);
    });

    // Cleanup
    let _ = fs::remove_file(&path);
}

#[test]
fn parse_scalar_classifies_unsigned_and_float_separately() {
    // Act + Assert: an unsigned literal stays an integer; a fractional one is a float
    check!(parse_scalar("42") == json!(42));
    check!(parse_scalar("1.5") == json!(1.5));
    check!(parse_scalar("hi") == json!("hi"));
}

#[test]
fn get_on_an_unset_path_is_ok() {
    // Arrange: isolate the config file so the read sees only defaults. The env
    // lock serializes the process-global `SNIP_CONFIG_PATH` mutation.
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let path = env::temp_dir().join(format!("snip-cfgtest-get-{}.json", std::process::id()));
    let _ = fs::remove_file(&path);
    temp_env::with_var("SNIP_CONFIG_PATH", Some(&path), || {
        // Act: a path that does not exist prints "(unset)" rather than failing
        let result = run(&["get".into(), "no.such.path".into()]);

        // Assert
        check!(result.is_ok());
    });

    // Cleanup
    let _ = fs::remove_file(&path);
}

#[test]
fn reset_restores_defaults_on_disk() {
    // Arrange: isolate, then persist a non-default config
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let path = env::temp_dir().join(format!("snip-cfgtest-reset-{}.json", std::process::id()));
    let _ = fs::remove_file(&path);
    temp_env::with_var("SNIP_CONFIG_PATH", Some(&path), || {
        run(&["set".into(), "master_enabled".into(), "false".into()]).unwrap();

        // Act
        run(&["reset".into()]).unwrap();

        // Assert: the on-disk config is back to the (enabled) default
        check!(Config::load_raw().master_enabled);
    });

    // Cleanup
    let _ = fs::remove_file(&path);
}

#[test]
fn spec_add_then_rm_round_trips_through_the_config_file() {
    // Arrange: isolate the config file
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let path = env::temp_dir().join(format!("snip-cfgtest-spec-{}.json", std::process::id()));
    let _ = fs::remove_file(&path);
    temp_env::with_var("SNIP_CONFIG_PATH", Some(&path), || {
        let spec_json = r#"{"name":"my-grep","surface":"grep"}"#;

        // Act: add the spec, then remove it
        run(&["spec".into(), "add".into(), spec_json.into()]).unwrap();
        let after_add = Config::load_raw();
        run(&["spec".into(), "rm".into(), "my-grep".into()]).unwrap();
        let after_rm = Config::load_raw();

        // Assert
        check!(after_add.specs.iter().any(|s| s.name == "my-grep"));
        check!(after_rm.specs.iter().all(|s| s.name != "my-grep"));
    });

    // Cleanup
    let _ = fs::remove_file(&path);
}

#[test]
fn spec_rm_absent_name_errors() {
    // Arrange: isolate an empty config (no user specs)
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let path = env::temp_dir().join(format!("snip-cfgtest-specrm-{}.json", std::process::id()));
    let _ = fs::remove_file(&path);
    temp_env::with_var("SNIP_CONFIG_PATH", Some(&path), || {
        // Act
        let result = run(&["spec".into(), "rm".into(), "nope".into()]);

        // Assert
        check!(result.is_err());
    });

    // Cleanup
    let _ = fs::remove_file(&path);
}

#[test]
fn spec_add_invalid_json_errors() {
    // Arrange: isolate so a failed add never reads/writes a shared file
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let path = env::temp_dir().join(format!("snip-cfgtest-specbad-{}.json", std::process::id()));
    let _ = fs::remove_file(&path);
    temp_env::with_var("SNIP_CONFIG_PATH", Some(&path), || {
        // Act
        let result = run(&["spec".into(), "add".into(), "{bad json".into()]);

        // Assert
        check!(result.is_err());
    });

    // Cleanup
    let _ = fs::remove_file(&path);
}

#[test]
fn spec_add_inert_spec_is_rejected_by_validate() {
    // Arrange: a Read-surface spec is valid JSON but can never run (not spec-extensible)
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let path = env::temp_dir().join(format!(
        "snip-cfgtest-specinert-{}.json",
        std::process::id()
    ));
    let _ = fs::remove_file(&path);
    temp_env::with_var("SNIP_CONFIG_PATH", Some(&path), || {
        let inert = r#"{"name":"x","surface":"read"}"#;

        // Act
        let result = run(&["spec".into(), "add".into(), inert.into()]);

        // Assert
        check!(result.is_err());
    });

    // Cleanup
    let _ = fs::remove_file(&path);
}

#[test]
fn unknown_subcommand_errors() {
    // Act
    let result = run(&["bogus".into()]);

    // Assert
    check!(result.is_err());
}
