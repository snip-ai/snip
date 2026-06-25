//! The read pipeline through the public API, in AAA form: `detect` → `Compactor`,
//! and the `ReadOptimizer` producing a token-saving `Rewrite`. Black-box: only
//! `pub` items are touched, exercising several modules together.

use assert2::check;
use serde_json::json;
use snip_lib::compaction::Compactor;
use snip_lib::config::Config;
use snip_lib::domain::{HookCtx, Optimizer, Outcome, Surface};
use snip_lib::languages::detect;
use snip_lib::optimizers::read::ReadOptimizer;

#[test]
fn detect_then_compress_strips_comments() {
    // Arrange
    let spec = detect("lib.rs").expect("a rust spec for the .rs extension");

    // Act
    let compacted = Compactor::new(spec).compress("// gone\npub fn id(x: i32) -> i32 { x }\n");

    // Assert
    assert2::assert!(let Some(out) = compacted);
    check!(!out.contains("gone"));
    check!(out.contains("pub fn id"));
}

#[test]
fn read_optimizer_rewrites_commented_code() {
    // Arrange: comment-heavy enough that stripping beats the recovery-guidance
    // header cost (the header is counted in the savings gate).
    let cfg = Config::default();
    let input = json!({"file_path": "/src/main.rs"});
    let source = format!(
        "{}fn main() {{\n    let x = 1;\n    let y = 2;\n}}\n",
        "// a comment line with enough words that stripping it saves real tokens\n".repeat(8)
    );
    let ctx = HookCtx {
        surface: Surface::Read,
        session_id: None,
        transcript_path: None,
        input: &input,
        output: Some(source.as_str()),
        cfg: &cfg,
    };

    // Act
    assert2::assert!(let Ok(outcome) = ReadOptimizer.apply(&ctx));

    // Assert
    assert2::assert!(let Outcome::Rewrite { header, body, original_tokens, new_tokens, .. } = outcome);
    check!(header.contains("[snip: read | rust"));
    check!(!body.contains("comment"));
    check!(new_tokens < original_tokens);
}
