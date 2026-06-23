//! Unit tests for [`ToolResponse`] (every-wire-shape extraction + shape-
//! preserving rewrite), in AAA form. Compiled into `snip_lib` via a `#[path]`
//! include in `src/engine/tool_response.rs`.

use assert2::check;
use serde_json::{Value, json};

use super::ToolResponse;

#[test]
fn extract_handles_three_shapes() {
    // Arrange
    let nested = json!({"type":"text","file":{"content":"hello","numLines":1}});
    let flat = json!({"content":"hi"});
    let bare = Value::String("bare".to_owned());

    // Act
    let from_nested = ToolResponse::new(Some(&nested)).extract_text();
    let from_flat = ToolResponse::new(Some(&flat)).extract_text();
    let from_bare = ToolResponse::new(Some(&bare)).extract_text();
    let from_none = ToolResponse::new(None).extract_text();

    // Assert
    check!(from_nested.as_deref() == Some("hello"));
    check!(from_flat.as_deref() == Some("hi"));
    check!(from_bare.as_deref() == Some("bare"));
    check!(from_none.is_none());
}

#[test]
fn extract_handles_the_glob_filenames_shape() {
    // Arrange: real Claude Code Glob output — a path array, no content field
    let glob = json!({"filenames":["a/b.rs","a/c.rs"],"numFiles":2,"truncated":false});

    // Act
    let text = ToolResponse::new(Some(&glob)).extract_text();

    // Assert: joined exactly as the model renders it (newline-delimited)
    check!(text.as_deref() == Some("a/b.rs\na/c.rs"));
}

#[test]
fn extract_grep_prefers_content_over_filenames() {
    // Arrange: real Grep carries BOTH — `content` (match lines) must win
    let grep = json!({"mode":"content","filenames":["a.rs"],"content":"a.rs:1:hit","numLines":1});

    // Act
    let text = ToolResponse::new(Some(&grep)).extract_text();

    // Assert
    check!(text.as_deref() == Some("a.rs:1:hit"));
}

#[test]
fn rewrite_preserves_the_glob_filenames_shape() {
    // Arrange: the Glob shape — the rewrite must land back in `filenames`, since
    // Claude Code renders that (one array entry per line), not `content`.
    let glob = json!({"filenames":["a/b.rs","a/c.rs"],"numFiles":2});

    // Act
    let out = ToolResponse::new(Some(&glob)).rewrite("[snip]\n", "a:\n  b.rs\n  c.rs");

    // Assert: filenames replaced line-by-line; no stray `content` field added
    let names = out.get("filenames").and_then(Value::as_array);
    assert2::assert!(let Some(names) = names);
    let joined: Vec<&str> = names.iter().filter_map(Value::as_str).collect();
    check!(joined == vec!["[snip]", "a:", "  b.rs", "  c.rs"]);
    check!(out.get("content").is_none());
}

#[test]
fn rewrite_preserves_nested_shape() {
    // Arrange
    let resp = json!({"type":"text","file":{"filePath":"/x.rs","content":"old","numLines":1}});

    // Act
    let out = ToolResponse::new(Some(&resp)).rewrite("[hdr]\n", "new");

    // Assert: content replaced, sibling fields untouched
    let content = out.pointer("/file/content").and_then(Value::as_str);
    let file_path = out.pointer("/file/filePath").and_then(Value::as_str);
    check!(content == Some("[hdr]\nnew"));
    check!(file_path == Some("/x.rs"));
}

#[test]
fn rewrite_preserves_legacy_flat_content_shape() {
    // Arrange: the legacy `{ "content": … }` shape (no nested `file`)
    let resp = json!({"content":"old","extra":42});

    // Act
    let out = ToolResponse::new(Some(&resp)).rewrite("[h]\n", "new");

    // Assert: stays a flat object — content replaced, siblings round-trip
    let content = out.get("content").and_then(Value::as_str);
    let extra = out.get("extra").and_then(Value::as_u64);
    check!(content == Some("[h]\nnew"));
    check!(extra == Some(42));
}

#[test]
fn rewrite_falls_back_to_a_bare_string() {
    // Arrange: a bare-string tool_response carries no object to mutate
    let resp = Value::String("old".to_owned());

    // Act
    let out = ToolResponse::new(Some(&resp)).rewrite("[h]\n", "new");

    // Assert: the rewrite is a bare string of header + body
    check!(out == Value::String("[h]\nnew".to_owned()));
}

#[test]
fn rewrite_of_an_absent_response_is_a_bare_string() {
    // Arrange: no tool_response at all
    let tr = ToolResponse::new(None);

    // Act
    let out = tr.rewrite("[h]\n", "body");

    // Assert: falls through to the bare-string arm
    check!(out == Value::String("[h]\nbody".to_owned()));
}
