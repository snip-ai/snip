//! The search pipeline through the public API, in AAA form: the shipped `search`
//! spec runs its transform chain (dedupe → group-by-file → cap) on grep output
//! and produces a token-saving `Rewrite`. Black-box: only `pub` items.

use assert2::check;
use serde_json::json;
use snip_lib::config::Config;
use snip_lib::domain::{HookCtx, Optimizer, Outcome, Surface};
use snip_lib::optimizers::SpecOptimizer;
use snip_lib::spec::builtin::builtin_specs;

/// The built-in `search` spec bound to a given surface.
fn search_for(surface: Surface) -> SpecOptimizer {
    let spec = builtin_specs()
        .into_iter()
        .find(|spec| spec.surface == surface)
        .expect("a built-in search spec for the surface");
    SpecOptimizer::new(spec)
}

#[test]
fn grep_output_is_grouped_by_file_and_compacted() {
    // Arrange: many matches across two files with realistic (long) paths, so
    // de-repeating the path per match saves well beyond the injected header cost
    // (short names wouldn't beat the header-inclusive no-inflation gate).
    let cfg = Config::default();
    let input = json!({"pattern": "TODO"});
    let output = "src/optimizers/command/segmenter.rs:12:TODO\n\
                  src/optimizers/command/segmenter.rs:34:TODO\n\
                  src/optimizers/command/segmenter.rs:56:TODO\n\
                  src/optimizers/command/segmenter.rs:78:TODO\n\
                  src/spec/builtin/mod.rs:9:TODO\n\
                  src/spec/builtin/mod.rs:21:TODO\n\
                  src/spec/builtin/mod.rs:43:TODO\n";
    let ctx = HookCtx {
        surface: Surface::Grep,
        session_id: None,
        transcript_path: None,
        input: &input,
        output: Some(output),
        cfg: &cfg,
    };

    // Act
    assert2::assert!(let Ok(outcome) = search_for(Surface::Grep).apply(&ctx));

    // Assert: one `path:` header per file, matches indented, fewer tokens
    assert2::assert!(let Outcome::Rewrite { header, body, original_tokens, new_tokens } = outcome);
    check!(header.contains("[snip: search-grep |"));
    check!(body.contains("src/optimizers/command/segmenter.rs:\n  12:TODO"));
    check!(body.contains("src/spec/builtin/mod.rs:\n  9:TODO"));
    check!(new_tokens < original_tokens);
}

#[test]
fn glob_output_is_grouped_by_directory() {
    // Arrange: several files under one deep directory — the repeated prefix is
    // where grouping pays off (a short prefix wouldn't beat the no-inflation gate)
    let cfg = Config::default();
    let input = json!({"pattern": "**/*.rs"});
    let dir = "src/optimizers/read";
    let mut output = String::new();
    for f in ["a", "b", "c", "d", "e"] {
        output.push_str(dir);
        output.push('/');
        output.push_str(f);
        output.push_str(".rs\n");
    }
    let ctx = HookCtx {
        surface: Surface::Glob,
        session_id: None,
        transcript_path: None,
        input: &input,
        output: Some(&output),
        cfg: &cfg,
    };

    // Act
    assert2::assert!(let Ok(outcome) = search_for(Surface::Glob).apply(&ctx));

    // Assert: the shared directory becomes one header, paths indented under it
    assert2::assert!(let Outcome::Rewrite { body, new_tokens, original_tokens, .. } = outcome);
    check!(body.contains("src/optimizers/read:\n  a.rs"));
    check!(new_tokens < original_tokens);
}
