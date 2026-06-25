//! Unit tests for `bash-route` helpers, in AAA form. Compiled into `snip_lib`
//! via a `#[path]` include in `src/optimizers/command/bash_route.rs`.

use assert2::check;
use serde_json::json;

use super::{is_backgrounded, is_wrapped, rewrite_command};
use crate::optimizers::command::b64;

#[test]
fn backgrounded_commands_are_not_wrapped() {
    // Arrange: tool_input shapes with/without the run_in_background flag
    let bg = json!({"command": "npm run dev", "run_in_background": true});
    let fg = json!({"command": "npm run dev", "run_in_background": false});
    let none = json!({"command": "npm run dev"});

    // Act + Assert: only an explicitly-backgrounded command is skipped (passes through)
    check!(is_backgrounded(&bg));
    check!(!is_backgrounded(&fg));
    check!(!is_backgrounded(&none));
}

#[test]
fn recognizes_our_own_exec_wrapper() {
    // Arrange: force `SNIP_WRAPPED` unset so the assertions exercise the
    // command-parse branch, not the ambient env-var branch. Otherwise running the
    // suite UNDER snip's own `snip exec` wrapper (dogfooding) — which sets
    // `SNIP_WRAPPED=1` for the wrapped child — makes `is_wrapped()` short-circuit to
    // true for every input. The lock serializes the env mutation against sibling tests.
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    temp_env::with_var_unset("SNIP_WRAPPED", || {
        // Act + Assert: the anti-recursion guard
        check!(is_wrapped("snip exec -- Zm9v"));
        check!(is_wrapped(r#""/usr/local/bin/snip" exec -- Zm9v"#));
        check!(!is_wrapped("git status"));
        check!(!is_wrapped("snippet show")); // argv0 is `snippet`, not `snip`
    });
}

#[test]
fn rewrite_round_trips_the_original_command() {
    // Arrange: a command with shell metacharacters
    let cmd = "git status && ls -la | sort";

    // Act
    assert2::assert!(let Some(rewritten) = rewrite_command(cmd, None));

    // Assert: `<snip> exec -- <base64>` whose payload decodes back exactly
    check!(rewritten.contains("exec -- "));
    let payload = rewritten.rsplit(" -- ").next().expect("a payload");
    let decoded = b64::decode(payload).and_then(|b| String::from_utf8(b).ok());
    check!(decoded.as_deref() == Some(cmd));
}

#[test]
fn safe_session_is_injected_and_unsafe_is_dropped() {
    // Arrange + Act: a UUID-shaped id vs an injection-shaped one
    let safe = rewrite_command("ls", Some("abc-123")).expect("rewrite");
    let unsafe_id = rewrite_command("ls", Some("a; rm -rf /")).expect("rewrite");

    // Assert: the safe id is exported as an env prefix; the unsafe id is dropped
    check!(safe.starts_with("SNIP_SESSION=abc-123 "));
    check!(!unsafe_id.contains("SNIP_SESSION="));
}
