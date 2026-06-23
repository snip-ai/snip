//! `bash-route` (`PreToolUse`/Bash) + `exec` end-to-end: a wrappable command is
//! rewritten to `snip exec -- <base64>`; the runtime then runs it on its exact
//! bytes and compacts recognized output. Mirrors `src/optimizers/command`.

use assert2::check;
use serde_json::{Value, json};
use snip_lib::optimizers::command::b64;

use crate::support::{Snip, sh_available, stdout_json, stdout_str};

/// A `bash-route` payload for `command` in session `sid`.
fn bash_hook(command: &str, sid: &str) -> String {
    json!({"tool_input": {"command": command}, "session_id": sid}).to_string()
}

/// The rewritten `command` of a `bash-route` `updatedInput`, or `None`.
fn rewritten_command(out: &std::process::Output) -> Option<String> {
    if out.stdout.is_empty() {
        return None;
    }
    stdout_json(out)
        .pointer("/hookSpecificOutput/updatedInput/command")
        .and_then(Value::as_str)
        .map(str::to_owned)
}

#[test]
fn wrappable_command_is_rerouted_through_exec() {
    // Arrange
    let snip = Snip::fresh();
    let payload = bash_hook("git status", "sess-abc");

    // Act
    let out = snip.run(&["bash-route"], &payload);

    // Assert: input replaced by the snip exec wrapper, exit 0
    check!(out.status.success());
    assert2::assert!(let Some(command) = rewritten_command(&out));
    check!(command.contains("exec -- "));
    check!(command.contains("SNIP_SESSION=sess-abc"));
}

#[test]
fn already_wrapped_command_passes_through() {
    // Arrange: an env-marked wrap must never be re-wrapped (anti-recursion)
    let snip = Snip::fresh();
    let payload = bash_hook("git status", "sess-abc");

    // Act
    let out = snip
        .command()
        .arg("bash-route")
        .env("SNIP_WRAPPED", "1")
        .write_stdin(payload)
        .output()
        .expect("snip runs");

    // Assert
    check!(out.status.success());
    check!(stdout_str(&out).trim().is_empty());
}

#[test]
fn disabled_via_env_does_not_reroute() {
    // Arrange
    let snip = Snip::fresh();
    let payload = bash_hook("git status", "sess-abc");

    // Act
    let out = snip
        .command()
        .arg("bash-route")
        .env("SNIP_ENABLED", "0")
        .write_stdin(payload)
        .output()
        .expect("snip runs");

    // Assert
    check!(out.status.success());
    check!(stdout_str(&out).trim().is_empty());
}

#[test]
fn exec_runs_a_plain_command_verbatim() {
    // Arrange: nothing recognized ⇒ exec is byte-transparent, exit code preserved
    if !sh_available() {
        return;
    }
    let snip = Snip::fresh();
    let encoded = b64::encode(b"printf 'hello\\n'");

    // Act
    let out = snip.run(&["exec", "--", &encoded], "");

    // Assert
    check!(out.status.success());
    check!(stdout_str(&out) == "hello\n");
}

#[test]
fn exec_preserves_a_nonzero_exit_code() {
    // Arrange
    if !sh_available() {
        return;
    }
    let snip = Snip::fresh();
    let encoded = b64::encode(b"exit 3");

    // Act
    let out = snip.run(&["exec", "--", &encoded], "");

    // Assert: the wrapped command's exit code flows through unchanged
    check!(out.status.code() == Some(3));
}
