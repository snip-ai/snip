//! `read-hook` (`PostToolUse`/Read) end-to-end: a commented-code Read is replaced
//! by the compacted view in the nested `tool_response` shape; everything else
//! passes through (empty stdout). Mirrors `src/optimizers/read`.

use assert2::check;
use serde_json::{Value, json};

use crate::support::{Snip, stdout_json, stdout_str};

/// The `read-hook` payload for a file at `path` with `content`.
fn read_hook(path: &str, content: &str) -> String {
    json!({
        "tool_input": {"file_path": path},
        "tool_response": {"type": "text", "file": {"filePath": path, "content": content, "numLines": content.lines().count()}}
    })
    .to_string()
}

#[test]
fn commented_rust_is_replaced_by_the_compacted_view() {
    // Arrange: comment-heavy enough that stripping beats the recovery-guidance
    // header cost (the header is counted in the savings gate).
    let snip = Snip::fresh();
    let comments =
        "// a private note explaining this code in enough words to strip real tokens\n".repeat(8);
    let payload = read_hook(
        "/src/main.rs",
        &format!("{comments}fn main() {{\n    let x = 1;\n    let y = 2;\n}}\n"),
    );

    // Act
    let out = snip.run(&["read-hook"], &payload);

    // Assert: same nested shape, file.content swapped for the tagged compaction
    check!(out.status.success());
    let v = stdout_json(&out);
    let content = v
        .pointer("/hookSpecificOutput/updatedToolOutput/file/content")
        .and_then(Value::as_str);
    assert2::assert!(let Some(content) = content);
    check!(content.contains("[snip: read | rust"));
    check!(!content.contains("a private note"));
    // The event name + nested file shape round-trip (schema-preserving rewrite).
    check!(
        v.pointer("/hookSpecificOutput/hookEventName")
            .and_then(Value::as_str)
            == Some("PostToolUse")
    );
    check!(
        v.pointer("/hookSpecificOutput/updatedToolOutput/file/filePath")
            .is_some()
    );
}

#[test]
fn comment_free_code_passes_through() {
    // Arrange: nothing to strip ⇒ no view smaller than the original
    let snip = Snip::fresh();
    let payload = read_hook("/src/lib.rs", "pub fn id(x: i32) -> i32 { x }\n");

    // Act
    let out = snip.run(&["read-hook"], &payload);

    // Assert: empty stdout (Claude Code keeps the original), exit 0
    check!(out.status.success());
    check!(stdout_str(&out).trim().is_empty());
}

#[test]
fn disabled_via_env_passes_through_even_for_commented_code() {
    // Arrange: SNIP_ENABLED=0 forces the master switch off
    let snip = Snip::fresh();
    let payload = read_hook("/src/main.rs", "// strip me\nfn main() {}\n");

    // Act
    let out = snip
        .command()
        .arg("read-hook")
        .env("SNIP_ENABLED", "0")
        .write_stdin(payload)
        .output()
        .expect("snip runs");

    // Assert: the switch wins — no rewrite, exit 0
    check!(out.status.success());
    check!(stdout_str(&out).trim().is_empty());
}
