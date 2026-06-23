//! Unit tests for [`OutcomeSerializer`] (Outcome → `hookSpecificOutput` JSON), in
//! AAA form. Compiled into `snip_lib` via a `#[path]` include in
//! `src/engine/outcome_serializer.rs`.

use assert2::check;
use serde_json::{Value, json};

use super::OutcomeSerializer;
use crate::domain::Outcome;

#[test]
fn passthrough_serializes_to_none() {
    // Arrange
    let hook = Value::Null;

    // Act
    let out = OutcomeSerializer::serialize(&hook, Outcome::PassThrough);

    // Assert
    check!(out.is_none());
}

#[test]
fn rewrite_outcome_targets_post_tool_output() {
    // Arrange
    let hook = json!({"tool_response": {"file": {"content": "x"}}});
    let outcome = Outcome::Rewrite {
        header: String::new(),
        body: "compacted".to_owned(),
        original_tokens: 10,
        new_tokens: 2,
    };

    // Act
    assert2::assert!(let Some(out) = OutcomeSerializer::serialize(&hook, outcome));

    // Assert
    let event = out
        .pointer("/hookSpecificOutput/hookEventName")
        .and_then(Value::as_str);
    let content = out
        .pointer("/hookSpecificOutput/updatedToolOutput/file/content")
        .and_then(Value::as_str);
    check!(event == Some("PostToolUse"));
    check!(content == Some("compacted"));
}

#[test]
fn fix_input_targets_pre_tool_input() {
    // Arrange
    let outcome = Outcome::FixInput(json!({"command":"ls"}));

    // Act
    assert2::assert!(let Some(out) = OutcomeSerializer::serialize(&Value::Null, outcome));

    // Assert
    let event = out
        .pointer("/hookSpecificOutput/hookEventName")
        .and_then(Value::as_str);
    let command = out
        .pointer("/hookSpecificOutput/updatedInput/command")
        .and_then(Value::as_str);
    check!(event == Some("PreToolUse"));
    check!(command == Some("ls"));
}

#[test]
fn ask_outcome_is_permission_decision() {
    // Arrange
    let outcome = Outcome::Ask {
        reason: "why".to_owned(),
    };

    // Act
    assert2::assert!(let Some(out) = OutcomeSerializer::serialize(&Value::Null, outcome));

    // Assert
    let decision = out
        .pointer("/hookSpecificOutput/permissionDecision")
        .and_then(Value::as_str);
    check!(decision == Some("ask"));
}
