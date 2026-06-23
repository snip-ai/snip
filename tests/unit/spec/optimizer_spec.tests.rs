//! Unit tests for [`OptimizerSpec::validate`], in AAA form. Compiled into
//! `snip_lib` via a `#[path]` include in `src/spec/optimizer_spec.rs`.

use assert2::check;

use crate::domain::Surface;
use crate::spec::{Bind, OptimizerSpec};

/// Build a spec for `surface` with the given `bind`, no transforms/flags.
fn spec(surface: Surface, bind: Bind) -> OptimizerSpec {
    OptimizerSpec {
        name: "t".to_owned(),
        surface,
        bind,
        transforms: Vec::new(),
        inject_flags: Vec::new(),
    }
}

#[test]
fn bash_spec_with_cmd_is_valid() {
    // Arrange
    let bind = Bind {
        cmd: Some("git".to_owned()),
        ..Bind::default()
    };

    // Act
    let result = spec(Surface::Bash, bind).validate();

    // Assert
    check!(result.is_ok());
}

#[test]
fn bash_spec_without_cmd_is_rejected() {
    // Arrange
    let s = spec(Surface::Bash, Bind::default());

    // Act
    let result = s.validate();

    // Assert: a Bash spec with no command binding can never match
    check!(result.is_err());
}

#[test]
fn bash_spec_with_path_globs_is_rejected() {
    // Arrange: `path_globs` scopes Grep/Glob specs, not Bash — a Bash spec that
    // sets cmd (so it passes the cmd check) but carries path_globs is inert
    let bind = Bind {
        cmd: Some("git".to_owned()),
        path_globs: vec!["src/*".to_owned()],
        ..Bind::default()
    };

    // Act
    let result = spec(Surface::Bash, bind).validate();

    // Assert
    check!(result.is_err());
}

#[test]
fn grep_spec_unscoped_is_valid() {
    // Arrange
    let s = spec(Surface::Grep, Bind::default());

    // Act
    let result = s.validate();

    // Assert
    check!(result.is_ok());
}

#[test]
fn grep_spec_carrying_cmd_is_rejected() {
    // Arrange: `bind.cmd` is ignored off the Bash surface, so it's a footgun
    let bind = Bind {
        cmd: Some("git".to_owned()),
        ..Bind::default()
    };

    // Act
    let result = spec(Surface::Grep, bind).validate();

    // Assert
    check!(result.is_err());
}

#[test]
fn read_edit_write_specs_are_rejected() {
    // Act + Assert: these surfaces are served by the Rust `read` optimizer
    check!(spec(Surface::Read, Bind::default()).validate().is_err());
    check!(spec(Surface::Edit, Bind::default()).validate().is_err());
    check!(spec(Surface::Write, Bind::default()).validate().is_err());
}
