//! Unit tests for the [`Dispatcher`] event-processing path, in AAA form.
//! Compiled into `snip_lib` via a `#[path]` include in `src/engine/dispatcher.rs`,
//! so these can reach the private `process` method.

use std::env;
use std::fs;

use assert2::check;
use serde_json::{Value, json};

use super::Dispatcher;
use crate::config::Config;
use crate::domain::Surface;

#[test]
fn read_with_no_comments_passes_through() {
    // Arrange: "code" parses as Rust with no comments → nothing to strip
    let hook = json!({
        "tool_input": {"file_path": "/x.rs"},
        "tool_response": {"file": {"content": "code"}}
    });

    // Act
    let out = Dispatcher::new(Surface::Read).process(&hook, &Config::default());

    // Assert
    check!(out.is_none());
}

#[test]
fn read_with_comments_is_rewritten() {
    // Arrange: a Rewrite records a savings event — isolate the stats log
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = env::temp_dir().join(format!("snip-dispatch-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    temp_env::with_var("SNIP_HOME", Some(&home), || {
        // Comment-heavy enough that stripping beats the recovery-guidance header cost.
        let src = format!(
            "{}fn main() {{\n    let x = 1;\n    let y = 2;\n}}\n",
            "// EXPLANATORY: long doc comment line with enough words to strip real tokens.\n"
                .repeat(8)
        );
        let hook = json!({
            "tool_input": {"file_path": "/x.rs"},
            "tool_response": {"file": {"content": src}}
        });

        // Act
        assert2::assert!(let Some(out) = Dispatcher::new(Surface::Read).process(&hook, &Config::default()));

        // Assert
        let content = out
            .pointer("/hookSpecificOutput/updatedToolOutput/file/content")
            .and_then(Value::as_str);
        assert2::assert!(let Some(content) = content);
        check!(content.contains("[snip: read | rust"));
        check!(!content.contains("EXPLANATORY"));
        // numLines flows through the rewrite → the nested shape is preserved end-to-end.
        let num_lines = out.pointer("/hookSpecificOutput/updatedToolOutput/file/numLines");
        check!(num_lines.and_then(Value::as_u64).is_some());
    });

    // Cleanup
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn process_raw_degrades_empty_malformed_and_inputless_payloads_to_passthrough() {
    // Arrange
    let cfg = Config::default();
    let dispatcher = Dispatcher::new(Surface::Read);

    // Act + Assert: none of these may produce output or panic (exit-0 robustness)
    check!(dispatcher.process_raw("", &cfg).is_none()); // empty stdin
    check!(dispatcher.process_raw("   \n ", &cfg).is_none()); // whitespace only
    check!(dispatcher.process_raw("{ not json", &cfg).is_none()); // malformed JSON
    check!(dispatcher.process_raw("{}", &cfg).is_none()); // no tool_input
    check!(dispatcher.process_raw("42", &cfg).is_none()); // valid JSON, wrong shape
}

#[test]
fn unregistered_surface_passes_through() {
    // Arrange
    let hook = json!({"tool_input": {"command": "ls"}});

    // Act
    let out = Dispatcher::new(Surface::Bash).process(&hook, &Config::default());

    // Assert
    check!(out.is_none());
}

#[test]
fn grep_glob_and_write_surfaces_pass_through() {
    // Arrange: Grep/Glob now carry the `search` optimizer, but this tiny output
    // can't be reduced (no-inflation guard), and Write has no rewrite path — so
    // all three pass through.
    let cfg = Config::default();
    let post =
        json!({"tool_input": {"pattern": "x"}, "tool_response": {"file": {"content": "hit"}}});
    let write = json!({"tool_input": {"file_path": "/x.rs", "content": "fn a(){}"}});

    // Act
    let grep = Dispatcher::new(Surface::Grep).process(&post, &cfg);
    let glob = Dispatcher::new(Surface::Glob).process(&post, &cfg);
    let write_out = Dispatcher::new(Surface::Write).process(&write, &cfg);

    // Assert
    check!(grep.is_none());
    check!(glob.is_none());
    check!(write_out.is_none());
}

#[test]
fn grep_with_reducible_output_is_rewritten() {
    // Arrange: many duplicate match lines compress well through `search-grep`
    let body = "a.rs:1:x\n".repeat(40);
    let hook = json!({
        "tool_input": {"pattern": "x"},
        "tool_response": {"file": {"content": body}}
    });

    // Act
    assert2::assert!(let Some(out) = Dispatcher::new(Surface::Grep).process(&hook, &Config::default()));

    // Assert: the search optimizer's view replaces file.content (shape preserved)
    let content = out
        .pointer("/hookSpecificOutput/updatedToolOutput/file/content")
        .and_then(Value::as_str);
    assert2::assert!(let Some(content) = content);
    check!(content.contains("[snip: search-grep |"));
    check!(content.contains("(×40)"));
}
