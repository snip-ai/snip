//! Unit tests for `config` (defaults, the master switch, env parsing), in AAA
//! form. Compiled into `snip_lib` via a `#[path]` include in `src/config/mod.rs`,
//! so these can reach the module's private items (`parse_enabled`).

use assert2::check;

use super::{Config, parse_enabled};

#[test]
fn defaults_are_enabled() {
    // Arrange
    let cfg = Config::default();

    // Act
    let read_enabled = cfg.optimizer_enabled("read");
    let unknown_enabled = cfg.optimizer_enabled("anything-unknown");

    // Assert
    check!(cfg.master_enabled);
    check!(read_enabled);
    check!(unknown_enabled);
}

#[test]
fn disabled_optimizer_is_off_but_others_on() {
    // Arrange
    let cfg: Config = serde_json::from_str(r#"{"optimizers":{"read":{"enabled":false}}}"#).unwrap();

    // Act
    let read_enabled = cfg.optimizer_enabled("read");
    let command_enabled = cfg.optimizer_enabled("command");

    // Assert
    check!(!read_enabled);
    check!(command_enabled);
}

#[test]
fn master_switch_gates_everything() {
    // Arrange
    let cfg: Config = serde_json::from_str(r#"{"master_enabled":false}"#).unwrap();

    // Act
    let read_enabled = cfg.optimizer_enabled("read");

    // Assert
    check!(!read_enabled);
}

#[test]
fn new_accessors_have_safe_defaults() {
    // Arrange
    let cfg = Config::default();

    // Act + Assert
    check!(cfg.mode_for("read") == crate::config::CompactMode::Soft);
    check!(cfg.rule_enabled("command", "git.diff"));
    check!(cfg.autodetect_for("command").json);
    check!(cfg.autodetect_for("command").log);
    check!(!cfg.secret_safe);
}

#[test]
fn per_optimizer_overrides_apply() {
    // Arrange
    let cfg: Config = serde_json::from_str(
        r#"{"optimizers":{"read":{"mode":"high"},
            "command":{"rules":{"git.diff":false},"autodetect":{"json":false,"log":false}}}}"#,
    )
    .unwrap();

    // Act + Assert
    check!(cfg.mode_for("read") == crate::config::CompactMode::High);
    check!(!cfg.rule_enabled("command", "git.diff"));
    check!(cfg.rule_enabled("command", "git.status")); // unset key → default true
    check!(!cfg.autodetect_for("command").json);
    check!(!cfg.autodetect_for("command").log);
    check!(cfg.autodetect_for("read").json); // no override → global default (true)
    check!(cfg.autodetect_for("read").log); // no override → global default (true)
}

#[test]
fn env_value_parsing() {
    // Arrange
    let disabling = ["0", "false", "no", "off", "OFF", " false "];
    let enabling = ["1", "true", "yes", "on", "anything"];

    // Act + Assert (table-driven; std assert! names the failing case)
    for value in disabling {
        assert!(!parse_enabled(value), "{value:?} should disable");
    }
    for value in enabling {
        assert!(parse_enabled(value), "{value:?} should enable");
    }
}

#[test]
fn save_creates_the_data_dir_and_round_trips() {
    // Arrange: point SNIP_HOME at a not-yet-existing nested dir so `save` must
    // create it. Serialize on the env lock (process-global SNIP_HOME).
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = std::env::temp_dir().join(format!("snip-cfgtest-save-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    temp_env::with_var("SNIP_HOME", Some(&home), || {
        let cfg = Config {
            master_enabled: false,
            ..Config::default()
        };

        // Act
        cfg.save().unwrap();

        // Assert: the dir was created and the value round-trips on load
        check!(home.join("config.json").exists());
        check!(!Config::load_raw().master_enabled);
    });

    // Cleanup
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn load_overlays_the_project_layer_when_opted_in() {
    // Arrange: a user config that opts in, plus a cwd `.snip/config.json`. Both
    // SNIP_HOME and cwd are process-global, so serialize on the env lock.
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = std::env::temp_dir().join(format!("snip-cfgtest-load-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(
        home.join("config.json"),
        r#"{"allow_project_config": true, "secret_safe": false}"#,
    )
    .unwrap();
    let original = std::env::current_dir().unwrap();
    let cwd = std::env::temp_dir().join(format!("snip-cfgtest-loadcwd-{}", std::process::id()));
    std::fs::create_dir_all(cwd.join(".snip")).unwrap();
    std::fs::write(
        cwd.join(".snip").join("config.json"),
        r#"{"secret_safe": true}"#,
    )
    .unwrap();
    std::env::set_current_dir(&cwd).unwrap();
    temp_env::with_var("SNIP_HOME", Some(&home), || {
        // Act
        let cfg = Config::load();

        // Assert: the project layer turned redaction on (enable-only OR)
        check!(cfg.secret_safe);
    });

    // Cleanup
    std::env::set_current_dir(&original).unwrap();
    let _ = std::fs::remove_dir_all(&home);
    let _ = std::fs::remove_dir_all(&cwd);
}
