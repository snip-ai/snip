//! Unit tests for the config resolution accessors, in AAA form. Compiled into
//! `snip_lib` via a `#[path]` include in `src/config/accessors.rs`.

use assert2::check;

use crate::config::{CompactMode, Config};

#[test]
fn dedupe_is_enabled_by_default() {
    // Arrange: no per-optimizer config at all
    let cfg = Config::default();

    // Act + Assert: default on, even for an unknown name
    check!(cfg.dedupe_enabled("read"));
    check!(cfg.dedupe_enabled("anything-unknown"));
}

#[test]
fn dedupe_can_be_turned_off_per_optimizer() {
    // Arrange: read opts out of dedupe; command does not
    let cfg: Config = serde_json::from_str(r#"{"optimizers":{"read":{"dedupe":false}}}"#).unwrap();

    // Act + Assert
    check!(!cfg.dedupe_enabled("read"));
    check!(cfg.dedupe_enabled("command"));
}

#[test]
fn mode_and_rule_defaults_are_safe() {
    // Arrange
    let cfg = Config::default();

    // Act + Assert
    check!(cfg.mode_for("read") == CompactMode::Soft);
    check!(cfg.rule_enabled("command", "git.diff"));
}

#[test]
fn overflow_for_falls_back_to_the_global_default() {
    // Arrange: no per-optimizer override
    let cfg = Config::default();

    // Act
    let global = &cfg.overflow;
    let resolved = cfg.overflow_for("read");

    // Assert: the resolved budget is the global one
    check!(resolved.max_tokens == global.max_tokens);
}

#[test]
fn overflow_for_command_uses_the_leaner_default() {
    // Arrange: no per-optimizer override, so the command default applies
    let cfg = Config::default();

    // Act
    let command = cfg.overflow_for_command("command");

    // Assert: the Bash surface runs leaner than the read/grep/glob default
    check!(command.max_tokens == 6000);
}

#[test]
fn overflow_overrides_apply_to_both_resolvers() {
    // Arrange: a per-optimizer overflow override
    let cfg: Config =
        serde_json::from_str(r#"{"optimizers":{"command":{"overflow":{"max_tokens":1000}}}}"#)
            .unwrap();

    // Act + Assert: the override wins over both the global and command defaults
    check!(cfg.overflow_for("command").max_tokens == 1000);
    check!(cfg.overflow_for_command("command").max_tokens == 1000);
}

#[test]
fn command_overflow_family_override_applies_to_a_spec_name() {
    // Arrange: the per-unit command budget is keyed by the recognized SPEC name
    // (e.g. `git-diff`), but a user sets the family key `optimizers.command.overflow`
    // — like every other `optimizers.command.*` setting, it must take effect.
    let cfg: Config =
        serde_json::from_str(r#"{"optimizers":{"command":{"overflow":{"max_tokens":150}}}}"#)
            .unwrap();

    // Act: resolve for a concrete command spec name, not the family key
    let resolved = cfg.overflow_for_command("git-diff");

    // Assert: the family override wins over the 6000 command default
    check!(resolved.max_tokens == 150);
}

#[test]
fn per_spec_overflow_override_beats_the_command_family() {
    // Arrange: both a per-spec override and a family-wide override
    let cfg: Config = serde_json::from_str(
        r#"{"optimizers":{"command":{"overflow":{"max_tokens":150}},"git-diff":{"overflow":{"max_tokens":42}}}}"#,
    )
    .unwrap();

    // Act + Assert: the more specific per-spec budget wins
    check!(cfg.overflow_for_command("git-diff").max_tokens == 42);
}
