//! Unit tests for the [`SpecOptimizer`] execution engine, in AAA form. Compiled
//! into `snip_lib` via a `#[path]` include in `src/spec/spec_optimizer.rs`.

use assert2::check;
use serde_json::{Value, json};

use super::SpecOptimizer;
use crate::config::Config;
use crate::domain::{HookCtx, Optimizer, Outcome, Surface};
use crate::spec::OptimizerSpec;

fn ctx<'a>(
    surface: Surface,
    input: &'a Value,
    output: Option<&'a str>,
    cfg: &'a Config,
) -> HookCtx<'a> {
    HookCtx {
        surface,
        session_id: None,
        transcript_path: None,
        input,
        output,
        cfg,
    }
}

fn grep_spec() -> OptimizerSpec {
    serde_json::from_str(
        r#"{"name":"search","surface":"grep","transforms":[{"op":"dedupe"},{"op":"truncate","head":2,"tail":1}]}"#,
    )
    .unwrap()
}

#[test]
fn matches_only_its_surface() {
    // Arrange
    let opt = SpecOptimizer::new(grep_spec());
    let input = json!({});
    let cfg = Config::default();

    // Act + Assert
    check!(opt.matches(&ctx(Surface::Grep, &input, None, &cfg)));
    check!(!opt.matches(&ctx(Surface::Glob, &input, None, &cfg)));
}

#[test]
fn surfaces_reports_the_specs_single_surface() {
    // Arrange: the optimizer advertises exactly the spec's surface
    let opt = SpecOptimizer::new(grep_spec());

    // Act
    let surfaces = opt.surfaces();

    // Assert
    check!(surfaces == [Surface::Grep].as_slice());
}

#[test]
fn name_reports_the_spec_name() {
    // Arrange
    let opt = SpecOptimizer::new(grep_spec());

    // Act + Assert: the optimizer name is the spec's name
    check!(opt.name() == "search");
}

#[test]
fn path_globs_scope_the_match_to_the_search_path() {
    // Arrange: a grep spec scoped to src/
    let spec: OptimizerSpec = serde_json::from_str(
        r#"{"name":"src-only","surface":"grep","bind":{"path_globs":["src/*"]},
            "transforms":[{"op":"dedupe"}]}"#,
    )
    .unwrap();
    let opt = SpecOptimizer::new(spec);
    let cfg = Config::default();
    let in_scope = json!({"pattern": "x", "path": "src/spec"});
    let out_scope = json!({"pattern": "x", "path": "docs"});
    let no_path = json!({"pattern": "x"});

    // Act + Assert: matches under src/, not elsewhere, and not without a path
    check!(opt.matches(&ctx(Surface::Grep, &in_scope, None, &cfg)));
    check!(!opt.matches(&ctx(Surface::Grep, &out_scope, None, &cfg)));
    check!(!opt.matches(&ctx(Surface::Grep, &no_path, None, &cfg)));
}

#[test]
fn apply_dedupes_and_caps_then_rewrites() {
    // Arrange: many duplicate match lines that compress far below the header cost
    let opt = SpecOptimizer::new(grep_spec());
    let input = json!({"pattern": "x"});
    let cfg = Config::default();
    let output = format!("{}b.rs:2:x\nc.rs:3:x\nd.rs:4:x\n", "a.rs:1:x\n".repeat(30));

    // Act
    assert2::assert!(
        let Ok(outcome) = opt.apply(&ctx(Surface::Grep, &input, Some(output.as_str()), &cfg))
    );

    // Assert
    assert2::assert!(let Outcome::Rewrite { header, original_tokens, new_tokens, .. } = outcome);
    check!(header.contains("[snip: search |"));
    check!(new_tokens < original_tokens);
}

#[test]
fn no_inflation_guard_counts_the_header() {
    // Arrange: a body that shrinks by only a few tokens — fewer than the injected
    // header's own cost — must pass through, since the model pays for header + body
    // (3 dupes → 1 line saves ~3 tok; the `[snip: …]` header costs ~10).
    let opt = SpecOptimizer::new(grep_spec());
    let input = json!({"pattern": "x"});
    let cfg = Config::default();
    let output = "a.rs:1:x\na.rs:1:x\na.rs:1:x\nb.rs:2:x\nc.rs:3:x\nd.rs:4:x\n";

    // Act
    assert2::assert!(let Ok(outcome) = opt.apply(&ctx(Surface::Grep, &input, Some(output), &cfg)));

    // Assert: the header-inclusive no-inflation guard rejects the marginal gain
    assert2::assert!(let Outcome::PassThrough = outcome);
}

#[test]
fn no_inflation_guard_passes_through_small_output() {
    // Arrange: a single short line cannot be reduced
    let opt = SpecOptimizer::new(grep_spec());
    let input = json!({"pattern": "x"});
    let cfg = Config::default();

    // Act
    assert2::assert!(let Ok(outcome) = opt.apply(&ctx(Surface::Grep, &input, Some("hit\n"), &cfg)));

    // Assert
    assert2::assert!(let Outcome::PassThrough = outcome);
}

#[test]
fn secret_safe_masks_a_credential_line() {
    // Arrange: a credential on its own line within a compressible output
    let opt = SpecOptimizer::new(grep_spec());
    let output = "AKIAIOSFODNN7EXAMPLE\nb.rs:2:x\nc.rs:3:x\nd.rs:4:x\n";

    // Act
    let masked = opt.apply_to(output, true);
    let raw = opt.apply_to(output, false);

    // Assert: secret_safe redacts the credential in the rewrite; without it the
    // optimizer makes no claim and passes the raw output through unchanged
    assert2::assert!(let Outcome::Rewrite { body, .. } = masked);
    check!(body.contains("AKI***"));
    check!(!body.contains("AKIAIOSFODNN7EXAMPLE"));
    assert2::assert!(let Outcome::PassThrough = raw);
}

#[test]
fn truncate_that_elides_flags_the_rewrite_lossy() {
    // Arrange: more distinct match lines than the truncate cap (head 2 + tail 1)
    // can keep, so the middle is elided and must be flagged for recovery.
    let opt = SpecOptimizer::new(grep_spec());
    let output = (0..40)
        .map(|i| format!("src/file_{i:02}.rs:{i}:hit"))
        .collect::<Vec<_>>()
        .join("\n");

    // Act
    let outcome = opt.apply_to(&output, false);

    // Assert: a lossy rewrite — the caller is responsible for spilling the original
    assert2::assert!(let Outcome::Rewrite { lossy, .. } = outcome);
    check!(lossy);
}

#[test]
fn rewrite_without_an_elision_is_not_lossy() {
    // Arrange: many duplicates collapse to a single `(×N)` record, so the truncate
    // cap never fires — nothing is dropped, so the rewrite is not lossy.
    let opt = SpecOptimizer::new(grep_spec());
    let output = "src/optimizers/command/segmenter.rs:12:hit\n".repeat(30);

    // Act
    let outcome = opt.apply_to(&output, false);

    // Assert
    assert2::assert!(let Outcome::Rewrite { lossy, .. } = outcome);
    check!(!lossy);
}

#[test]
fn missing_output_passes_through() {
    // Arrange
    let opt = SpecOptimizer::new(grep_spec());
    let input = json!({"pattern": "x"});
    let cfg = Config::default();

    // Act
    assert2::assert!(let Ok(outcome) = opt.apply(&ctx(Surface::Grep, &input, None, &cfg)));

    // Assert
    assert2::assert!(let Outcome::PassThrough = outcome);
}
