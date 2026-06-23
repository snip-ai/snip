//! CLI/meta subcommands end-to-end: `config`, `enable`/`disable`, `status`,
//! `gain`, `resolve`, version, and an unknown subcommand. These back the
//! `/snip-*` slash-commands and the live `snip resolve` recovery path.

use std::fs;

use assert2::check;
use tempfile::tempdir;

use crate::support::{Snip, stdout_str};

#[test]
fn config_list_emits_the_default_config_as_json() {
    // Arrange
    let snip = Snip::fresh();

    // Act
    let out = snip.run(&["config", "list"], "");

    // Assert: valid JSON carrying the master switch
    check!(out.status.success());
    let text = stdout_str(&out);
    check!(serde_json::from_str::<serde_json::Value>(&text).is_ok());
    check!(text.contains("master_enabled"));
}

#[test]
fn config_set_then_get_round_trips_a_nested_path() {
    // Arrange
    let snip = Snip::fresh();

    // Act
    let set = snip.run(&["config", "set", "overflow.max_tokens", "1234"], "");
    let get = snip.run(&["config", "get", "overflow.max_tokens"], "");

    // Assert
    check!(set.status.success());
    check!(stdout_str(&set).contains("set overflow.max_tokens = 1234"));
    check!(stdout_str(&get).trim() == "1234");
}

#[test]
fn config_set_unknown_path_is_rejected() {
    // Arrange
    let snip = Snip::fresh();

    // Act
    let out = snip.run(&["config", "set", "no.such.path", "1"], "");

    // Assert: a non-zero exit and a helpful message (never a misleading success)
    check!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    check!(stderr.contains("unknown config path"));
}

#[test]
fn disable_then_enable_round_trips_the_master_switch() {
    // Arrange
    let snip = Snip::fresh();

    // Act + Assert: disable persists, status reflects it, enable restores
    let off = snip.run(&["disable"], "");
    check!(stdout_str(&off).contains("snip disabled"));
    let status = snip.run(&["status"], "");
    check!(stdout_str(&status).contains("master: disabled"));
    let on = snip.run(&["enable"], "");
    check!(stdout_str(&on).contains("snip enabled"));
    check!(
        snip.run(&["config", "get", "master_enabled"], "")
            .stdout
            .starts_with(b"true")
    );
}

#[test]
fn status_reports_version_and_net_savings() {
    // Arrange
    let snip = Snip::fresh();

    // Act
    let out = snip.run(&["status"], "");

    // Assert
    check!(out.status.success());
    let text = stdout_str(&out);
    check!(text.contains(&format!("snip {}", env!("CARGO_PKG_VERSION"))));
    check!(text.contains("net savings:"));
}

#[test]
fn gain_is_ok_on_an_empty_stats_store() {
    // Arrange
    let snip = Snip::fresh();

    // Act
    let out = snip.run(&["gain"], "");

    // Assert
    check!(out.status.success());
}

#[test]
fn resolve_echoes_a_verbatim_match() {
    // Arrange: a file containing the needle the model pipes in
    let dir = tempdir().unwrap();
    let file = dir.path().join("r.rs");
    fs::write(&file, "fn a() {}\n// note\nfn b() {}\n").unwrap();
    let snip = Snip::fresh();

    // Act: pipe the old_string (with a trailing newline, as a heredoc would)
    let out = snip.run(&["resolve", &file.to_string_lossy()], "fn a() {}\n");

    // Assert: the verbatim text is echoed back, exit 0
    check!(out.status.success());
    check!(stdout_str(&out) == "fn a() {}");
}

#[test]
fn resolve_fails_loudly_with_no_confident_match() {
    // Arrange
    let dir = tempdir().unwrap();
    let file = dir.path().join("r.rs");
    fs::write(&file, "fn a() {}\n").unwrap();
    let snip = Snip::fresh();

    // Act
    let out = snip.run(
        &["resolve", &file.to_string_lossy()],
        "totally_absent_symbol_zzz",
    );

    // Assert: non-zero exit so the model knows to re-Read and copy more context
    check!(!out.status.success());
    check!(String::from_utf8_lossy(&out.stderr).contains("no confident match"));
}

#[test]
fn version_flag_prints_the_package_version() {
    // Arrange
    let snip = Snip::fresh();

    // Act
    let out = snip.run(&["--version"], "");

    // Assert
    check!(out.status.success());
    check!(stdout_str(&out).contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn an_unknown_subcommand_is_a_clap_usage_error() {
    // Arrange
    let snip = Snip::fresh();

    // Act
    let out = snip.run(&["frobnicate"], "");

    // Assert: clap exits 2 on a usage error
    check!(out.status.code() == Some(2));
}
