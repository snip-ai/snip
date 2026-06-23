//! Unit tests for the [`Registry`] (per-surface first-match dispatch), in AAA
//! form. Compiled into `snip_lib` via a `#[path]` include in
//! `src/engine/registry.rs`. Each registry serves one surface, so every test
//! builds a fresh per-surface registry.

use assert2::check;
use serde_json::{Value, json};

use super::Registry;
use crate::config::Config;
use crate::domain::{HookCtx, Optimizer, Surface};

fn ctx<'a>(surface: Surface, input: &'a Value, cfg: &'a Config) -> HookCtx<'a> {
    HookCtx {
        surface,
        session_id: None,
        transcript_path: None,
        input,
        output: None,
        cfg,
    }
}

#[test]
fn read_optimizer_registered_on_read_edit_write() {
    // Arrange
    let cfg = Config::default();
    let input = json!({});

    // Act: a per-surface registry for each input surface
    let read = Registry::build(&cfg, Surface::Read);
    let edit = Registry::build(&cfg, Surface::Edit);
    let write = Registry::build(&cfg, Surface::Write);

    // Assert
    check!(
        read.first_match(&ctx(Surface::Read, &input, &cfg))
            .map(Optimizer::name)
            == Some("read")
    );
    check!(
        edit.first_match(&ctx(Surface::Edit, &input, &cfg))
            .map(Optimizer::name)
            == Some("read")
    );
    check!(
        write
            .first_match(&ctx(Surface::Write, &input, &cfg))
            .map(Optimizer::name)
            == Some("read")
    );
}

#[test]
fn search_registered_on_grep_glob_but_not_bash() {
    // Arrange
    let cfg = Config::default();
    let input = json!({});

    // Act
    let grep = Registry::build(&cfg, Surface::Grep);
    let glob = Registry::build(&cfg, Surface::Glob);
    let bash = Registry::build(&cfg, Surface::Bash);

    // Assert: the per-surface search specs wire on Grep/Glob; Bash specs are
    // handled by the command runtime, never the registry, so a Bash registry
    // matches nothing.
    check!(
        grep.first_match(&ctx(Surface::Grep, &input, &cfg))
            .map(Optimizer::name)
            == Some("search-grep")
    );
    check!(
        glob.first_match(&ctx(Surface::Glob, &input, &cfg))
            .map(Optimizer::name)
            == Some("search-glob")
    );
    check!(
        bash.first_match(&ctx(Surface::Bash, &input, &cfg))
            .is_none()
    );
}

#[test]
fn search_family_switch_disables_both_grep_and_glob() {
    // Arrange: the `search` family switch off (the per-spec names are search-grep/
    // search-glob, so the family key must gate the whole surface).
    let cfg: Config =
        serde_json::from_str(r#"{"optimizers":{"search":{"enabled":false}}}"#).unwrap();
    let input = json!({});

    // Act
    let grep = Registry::build(&cfg, Surface::Grep);
    let glob = Registry::build(&cfg, Surface::Glob);

    // Assert: neither surface has any optimizer to match → passthrough
    check!(
        grep.first_match(&ctx(Surface::Grep, &input, &cfg))
            .is_none()
    );
    check!(
        glob.first_match(&ctx(Surface::Glob, &input, &cfg))
            .is_none()
    );
}

#[test]
fn disabled_optimizer_is_not_matched() {
    // Arrange
    let cfg: Config = serde_json::from_str(r#"{"optimizers":{"read":{"enabled":false}}}"#).unwrap();
    let input = json!({});
    let registry = Registry::build(&cfg, Surface::Read);

    // Act
    let read = registry.first_match(&ctx(Surface::Read, &input, &cfg));

    // Assert
    check!(read.is_none());
}

#[test]
fn user_grep_spec_is_dispatched_when_builtin_search_is_off() {
    // Arrange: disable the built-in grep search, add a user grep spec.
    let cfg: Config = serde_json::from_str(
        r#"{"optimizers":{"search-grep":{"enabled":false}},
            "specs":[{"name":"mygrep","surface":"grep","transforms":[{"op":"dedupe"}]}]}"#,
    )
    .unwrap();
    let input = json!({});
    let registry = Registry::build(&cfg, Surface::Grep);

    // Act
    let grep = registry.first_match(&ctx(Surface::Grep, &input, &cfg));

    // Assert: the user spec wins now that the shadowed built-in is disabled
    check!(grep.map(Optimizer::name) == Some("mygrep"));
}
