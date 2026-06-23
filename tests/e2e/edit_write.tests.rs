//! `edit-fix` + `write-guard` (`PreToolUse`) end-to-end. `edit-fix` maps a
//! compacted `old_string` back to file bytes (verbatim matches pass through);
//! `write-guard` asks before a Write reproduces the stripped view. These read the
//! real file from disk, so each writes a throwaway source file first.

use std::fs;

use assert2::check;
use serde_json::{Value, json};
use tempfile::tempdir;

use crate::support::{Snip, stdout_json, stdout_str};

/// A file with two functions split by a comment, and its soft-compacted view
/// (comments dropped, code byte-identical).
const COMMENTED: &str = "// header note\nfn main() {\n    let x = 1;\n    let y = 2;\n}\n";
const STRIPPED_VIEW: &str = "fn main() {\n    let x = 1;\n    let y = 2;\n}\n";

#[test]
fn edit_with_a_verbatim_old_string_passes_through() {
    // Arrange: old_string already present in the file ⇒ nothing to correct
    let dir = tempdir().unwrap();
    let file = dir.path().join("v.rs");
    fs::write(&file, COMMENTED).unwrap();
    let snip = Snip::fresh();
    let payload = json!({
        "tool_input": {
            "file_path": file.to_string_lossy(),
            "old_string": "let x = 1;",
            "new_string": "let x = 9;"
        }
    })
    .to_string();

    // Act
    let out = snip.run(&["edit-fix"], &payload);

    // Assert
    check!(out.status.success());
    check!(stdout_str(&out).trim().is_empty());
}

#[test]
fn edit_on_a_missing_file_passes_through() {
    // Arrange: no file on disk ⇒ nothing to map back
    let snip = Snip::fresh();
    let payload = json!({
        "tool_input": {
            "file_path": "/no/such/file.rs",
            "old_string": "x",
            "new_string": "y"
        }
    })
    .to_string();

    // Act
    let out = snip.run(&["edit-fix"], &payload);

    // Assert
    check!(out.status.success());
    check!(stdout_str(&out).trim().is_empty());
}

#[test]
fn write_reproducing_the_stripped_view_is_guarded() {
    // Arrange: a Write whose content is the comment-stripped view of the file
    let dir = tempdir().unwrap();
    let file = dir.path().join("w.rs");
    fs::write(&file, COMMENTED).unwrap();
    let snip = Snip::fresh();
    let payload = json!({
        "tool_input": {"file_path": file.to_string_lossy(), "content": STRIPPED_VIEW}
    })
    .to_string();

    // Act
    let out = snip.run(&["write-guard"], &payload);

    // Assert: an `ask` permission decision, exit 0
    check!(out.status.success());
    let v = stdout_json(&out);
    check!(
        v.pointer("/hookSpecificOutput/permissionDecision")
            .and_then(Value::as_str)
            == Some("ask")
    );
    let reason = v
        .pointer("/hookSpecificOutput/permissionDecisionReason")
        .and_then(Value::as_str)
        .unwrap_or_default();
    check!(reason.contains("compacted view"));
}

#[test]
fn write_to_a_new_file_passes_through() {
    // Arrange: a brand-new file has no comments to lose
    let dir = tempdir().unwrap();
    let file = dir.path().join("new.rs");
    let snip = Snip::fresh();
    let payload = json!({
        "tool_input": {"file_path": file.to_string_lossy(), "content": STRIPPED_VIEW}
    })
    .to_string();

    // Act
    let out = snip.run(&["write-guard"], &payload);

    // Assert
    check!(out.status.success());
    check!(stdout_str(&out).trim().is_empty());
}
