//! Unit tests for [`guarded`] and [`strict`], in AAA form. Compiled into
//! `snip_lib` via a `#[path]` include in `src/panic_guard.rs`.

use assert2::check;

use super::{guarded, strict};

/// Run `f` with `SNIP_DEBUG` forced off, serialized against other env-mutating
/// tests — production behavior must not depend on the ambient shell.
fn without_debug<T>(f: impl FnOnce() -> T) -> T {
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    temp_env::with_var_unset("SNIP_DEBUG", f)
}

/// Run `f` with `SNIP_DEBUG=value`, serialized the same way.
fn with_debug<T>(value: &str, f: impl FnOnce() -> T) -> T {
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    temp_env::with_var("SNIP_DEBUG", Some(value), f)
}

#[test]
fn production_swallows_a_panic_in_the_body() {
    // Arrange + Act: a panicking body must not unwind, and returns Ok (exit-0)
    let result = without_debug(|| guarded("test", || panic!("boom")));

    // Assert
    check!(result.is_ok());
}

#[test]
fn production_swallows_an_error_in_the_body() {
    // Arrange + Act: an Err body is logged and swallowed, not propagated
    let result = without_debug(|| guarded("test", || anyhow::bail!("nope")));

    // Assert
    check!(result.is_ok());
}

#[test]
fn runs_an_ok_body_to_completion() {
    // Arrange
    let mut ran = false;

    // Act
    let result = without_debug(|| {
        guarded("test", || {
            ran = true;
            Ok(())
        })
    });

    // Assert
    check!(ran);
    check!(result.is_ok());
}

#[test]
fn strict_mode_surfaces_a_panic_as_err() {
    // Arrange + Act: under SNIP_DEBUG a caught panic becomes a non-zero exit
    let result = with_debug("1", || guarded("test", || panic!("boom")));

    // Assert: Err carries the panic's message for the developer
    assert2::assert!(let Err(e) = result);
    check!(format!("{e:#}").contains("boom"));
}

#[test]
fn strict_mode_surfaces_an_error_as_err() {
    // Arrange + Act
    let result = with_debug("true", || guarded("test", || anyhow::bail!("nope")));

    // Assert
    assert2::assert!(let Err(e) = result);
    check!(format!("{e:#}").contains("nope"));
}

#[test]
fn strict_mode_still_returns_ok_for_an_ok_body() {
    // Arrange + Act: strict only changes the failure path, not the happy path
    let result = with_debug("1", || guarded("test", || Ok(())));

    // Assert
    check!(result.is_ok());
}

#[test]
fn strict_reads_truthy_values_only() {
    // Arrange + Act + Assert: the opt-in flag is off unless explicitly truthy
    check!(with_debug("1", strict));
    check!(with_debug("true", strict));
    check!(with_debug("on", strict));
    check!(!with_debug("0", strict));
    check!(!with_debug("false", strict));
    check!(!without_debug(strict));
}
