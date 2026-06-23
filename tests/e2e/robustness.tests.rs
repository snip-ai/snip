//! The always-exit-0 invariant end-to-end: every hook subcommand, fed empty,
//! whitespace-only, malformed, or wrong-shape stdin, must exit 0 with empty
//! stdout (a no-op ⇒ Claude Code keeps the original). The single most important
//! property of the whole system — a hook must never fail.

use assert2::check;

use crate::support::{Snip, stdout_str};

/// The stdin-reading hook subcommands (the maintenance hooks ignore most of it
/// but must also never fail).
const HOOKS: &[&str] = &[
    "read-hook",
    "grep-hook",
    "glob-hook",
    "bash-route",
    "edit-fix",
    "write-guard",
    "session-reset",
    "update-check",
];

/// Inputs that must all degrade to a silent no-op.
const JUNK: &[&str] = &[
    "",                       // empty
    "   \n\t ",               // whitespace only
    "{ not json",             // malformed JSON
    "42",                     // valid JSON, wrong shape
    "{}",                     // object with no tool_input
    "{\"tool_input\": null}", // tool_input present but null
    "[]",                     // array
];

#[test]
fn every_hook_survives_every_junk_input() {
    // Arrange
    let snip = Snip::fresh();

    // Act + Assert: the cartesian product all exits 0 with empty stdout
    for &hook in HOOKS {
        for &junk in JUNK {
            let out = snip.run(&[hook], junk);
            assert!(
                out.status.success(),
                "{hook} did not exit 0 on stdin {junk:?}"
            );
            assert!(
                stdout_str(&out).trim().is_empty(),
                "{hook} produced output on stdin {junk:?}"
            );
        }
    }
}

#[test]
fn read_hook_passes_through_a_non_code_file() {
    // Arrange: a file with no language spec (no rewrite path)
    let snip = Snip::fresh();
    let payload = serde_json::json!({
        "tool_input": {"file_path": "/notes/todo.txt"},
        "tool_response": {"file": {"content": "buy milk\ncall dentist\n"}}
    })
    .to_string();

    // Act
    let out = snip.run(&["read-hook"], &payload);

    // Assert
    check!(out.status.success());
    check!(stdout_str(&out).trim().is_empty());
}
