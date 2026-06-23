//! `grep-hook` / `glob-hook` (`PostToolUse`) end-to-end against the REAL Claude
//! Code `tool_response` shapes: Grep carries match lines in a top-level `content`
//! (plus a `filenames` array), Glob carries only a `filenames` path array (no
//! content). snip rewrites each in its own shape — `content` for Grep (what the
//! model renders), `filenames` for Glob (what the model renders, joined). Small
//! output passes through. Mirrors `src/optimizers/search`. The shapes here are
//! the ones captured from a live `claude` session by `tests/docker`.

use assert2::check;
use serde_json::{Value, json};

use crate::support::{Snip, stdout_json, stdout_str};

/// A real Grep `PostToolUse` payload: match lines live in top-level `content`.
fn grep_hook(pattern: &str, content: &str) -> String {
    json!({
        "tool_input": {"pattern": pattern, "output_mode": "content"},
        "tool_response": {
            "mode": "content",
            "numFiles": 1,
            "filenames": ["a.rs"],
            "content": content,
            "numLines": content.lines().count()
        }
    })
    .to_string()
}

/// A real Grep `files_with_matches` `PostToolUse` payload: `content` is a bare
/// path list under a `"Found N files"` header (no `path:line:` segments).
fn grep_files_hook(pattern: &str, files: &[&str]) -> String {
    let content = format!("Found {} files\n{}\n", files.len(), files.join("\n"));
    json!({
        "tool_input": {"pattern": pattern, "output_mode": "files_with_matches"},
        "tool_response": {
            "mode": "files_with_matches",
            "numFiles": files.len(),
            "filenames": files,
            "content": content,
            "numLines": content.lines().count()
        }
    })
    .to_string()
}

/// A real Glob `PostToolUse` payload: a path array, no content field.
fn glob_hook(pattern: &str, filenames: &[&str]) -> String {
    json!({
        "tool_input": {"pattern": pattern},
        "tool_response": {"filenames": filenames, "numFiles": filenames.len(), "truncated": false}
    })
    .to_string()
}

/// The compacted top-level `content` of a Grep rewrite, or `None` for pass-through.
fn grep_rewritten(out: &std::process::Output) -> Option<String> {
    if out.stdout.is_empty() {
        return None;
    }
    stdout_json(out)
        .pointer("/hookSpecificOutput/updatedToolOutput/content")
        .and_then(Value::as_str)
        .map(str::to_owned)
}

/// The compacted Glob view — the rewritten `filenames` array joined the way the
/// model renders it — or `None` for pass-through.
fn glob_rewritten(out: &std::process::Output) -> Option<String> {
    if out.stdout.is_empty() {
        return None;
    }
    let names = stdout_json(out)
        .pointer("/hookSpecificOutput/updatedToolOutput/filenames")
        .and_then(Value::as_array)?
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    Some(names.join("\n"))
}

#[test]
fn grep_with_repetitive_matches_is_compacted() {
    // Arrange: 40 identical match lines collapse hard through the search spec
    let snip = Snip::fresh();
    let payload = grep_hook("x", &"a.rs:1:x\n".repeat(40));

    // Act
    let out = snip.run(&["grep-hook"], &payload);

    // Assert
    check!(out.status.success());
    assert2::assert!(let Some(content) = grep_rewritten(&out));
    check!(content.contains("[snip: search-grep |"));
    check!(content.contains("(×40)"));
}

#[test]
fn grep_with_tiny_output_passes_through() {
    // Arrange: a single hit can't beat the no-inflation guard
    let snip = Snip::fresh();
    let payload = grep_hook("x", "a.rs:1:hit\n");

    // Act
    let out = snip.run(&["grep-hook"], &payload);

    // Assert
    check!(out.status.success());
    check!(stdout_str(&out).trim().is_empty());
}

#[test]
fn grep_files_with_matches_groups_by_directory() {
    // Arrange: 50 files under one deep shared prefix — the `files_with_matches`
    // bare path list has no `path:line:` segment, so it must fold by directory
    let snip = Snip::fresh();
    let dir = "/repo/matchdir/deep/nested";
    let files: Vec<String> = (0..50).map(|i| format!("{dir}/m{i:02}.txt")).collect();
    let refs: Vec<&str> = files.iter().map(String::as_str).collect();
    let payload = grep_files_hook("NEEDLE", &refs);

    // Act
    let out = snip.run(&["grep-hook"], &payload);

    // Assert: the shared directory collapses to one header; the count line survives
    check!(out.status.success());
    assert2::assert!(let Some(content) = grep_rewritten(&out));
    check!(content.contains("[snip: search-grep |"));
    check!(content.contains(&format!("{dir}:")));
    check!(content.contains("Found 50 files"));
    check!(content.contains("  m00.txt"));
}

#[test]
fn glob_groups_a_shared_directory() {
    // Arrange: five files under one deep dir — the repeated prefix is the win
    let snip = Snip::fresh();
    let dir = "src/optimizers/read";
    let files = [
        format!("{dir}/a.rs"),
        format!("{dir}/b.rs"),
        format!("{dir}/c.rs"),
        format!("{dir}/d.rs"),
        format!("{dir}/e.rs"),
    ];
    let refs: Vec<&str> = files.iter().map(String::as_str).collect();
    let payload = glob_hook("**/*.rs", &refs);

    // Act
    let out = snip.run(&["glob-hook"], &payload);

    // Assert: the shared directory becomes one header, written back to filenames
    check!(out.status.success());
    assert2::assert!(let Some(content) = glob_rewritten(&out));
    check!(content.contains("src/optimizers/read:"));
}
