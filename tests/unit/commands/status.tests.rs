//! Unit tests for `snip status`, in AAA form. Compiled into `snip_lib` via a
//! `#[path]` include in `src/commands/status.rs`, so these reach the private
//! `optimizer_state` helper.

use assert2::check;

use super::{optimizer_names, optimizer_state, run};
use crate::config::Config;

#[test]
fn run_is_ok() {
    // Act
    let result = run();

    // Assert
    check!(result.is_ok());
}

#[test]
fn optimizer_names_lists_references_plus_user_specs() {
    // Arrange: a user spec adds a new optimizer name; a same-named one doesn't dup
    let cfg: Config = serde_json::from_str(
        r#"{"specs":[
            {"name":"mygrep","surface":"grep","transforms":[{"op":"dedupe"}]},
            {"name":"read","surface":"read","transforms":[{"op":"dedupe"}]}
        ]}"#,
    )
    .unwrap();

    // Act
    let names = optimizer_names(&cfg);

    // Assert: the references, then the new user name, no duplicate "read"
    check!(names == vec!["read", "search-grep", "search-glob", "command", "mygrep"]);
}

#[test]
fn optimizer_state_reflects_the_master_switch() {
    // Arrange
    let on = Config::default();
    let off: Config = serde_json::from_str(r#"{"master_enabled":false}"#).unwrap();

    // Act + Assert
    check!(optimizer_state(&on, "read") == "enabled");
    check!(optimizer_state(&off, "read") == "disabled");
}
