//! Config layering through the public API, in AAA form: a user `config.json` under
//! an isolated `SNIP_HOME` is loaded, and a `SNIP_ENABLED=0` env override beats an
//! on-disk `master_enabled = true` (env is the outermost layer). Serialized on the
//! process-global `SNIP_HOME`/`SNIP_ENABLED` vars via `serial_test`.

use std::fs;

use assert2::check;
use serial_test::serial;
use snip_lib::config::Config;

/// Write a `config.json` with the given body into an isolated `SNIP_HOME` and
/// return the temp dir (kept alive for the test's duration). The caller points
/// the env at it via `temp_env`.
fn home_with_config(body: &str) -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    fs::write(home.path().join("config.json"), body).unwrap();
    home
}

#[test]
#[serial]
fn loads_user_settings_from_disk() {
    // Arrange: a user config that disables the `read` optimizer and turns on
    // secret-safe mode.
    let body = r#"{"secret_safe": true, "optimizers": {"read": {"enabled": false}}}"#;
    let home = home_with_config(body);

    temp_env::with_vars(
        [("SNIP_HOME", Some(home.path())), ("SNIP_ENABLED", None)],
        || {
            // Act
            let cfg = Config::load();

            // Assert: the on-disk settings took effect (and master is still on by default)
            check!(cfg.secret_safe);
            check!(!cfg.optimizer_enabled("read"));
            check!(cfg.master_enabled);
        },
    );
}

#[test]
#[serial]
fn env_disable_beats_on_disk_master_enabled() {
    // Arrange: the file insists snip is on; the env says off.
    let home = home_with_config(r#"{"master_enabled": true}"#);

    temp_env::with_vars(
        [
            ("SNIP_HOME", Some(home.path())),
            ("SNIP_ENABLED", Some("0".as_ref())),
        ],
        || {
            // Act
            let cfg = Config::load();

            // Assert: the env override wins — the master switch is off
            check!(!cfg.master_enabled);
            check!(!cfg.optimizer_enabled("read"));
        },
    );
}
