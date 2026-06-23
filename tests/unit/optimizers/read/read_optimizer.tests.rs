//! Unit tests for the `read` optimizer's `apply`, in AAA form. Compiled into
//! `snip_lib` via a `#[path]` include in `src/optimizers/read/read_optimizer.rs`.

use std::env;
use std::fs;

use assert2::check;
use serde_json::{Value, json};

use super::ReadOptimizer;
use crate::config::Config;
use crate::domain::{HookCtx, Optimizer, Outcome, Surface};

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

#[test]
fn compacts_rust_read() {
    // Arrange: a comment-heavy file whose stripped comments beat the recovery
    // guidance header's own token cost (the header is counted in the savings gate).
    let cfg = Config::default();
    let input = json!({"file_path": "/x.rs"});
    let src = format!(
        "{}fn main() {{\n    let x = 1;\n    let y = 2;\n}}\n",
        "// EXPLANATORY: long doc comment line with enough words to strip real tokens.\n".repeat(8)
    );

    // Act
    assert2::assert!(let Ok(outcome) = ReadOptimizer.apply(&ctx(Surface::Read, &input, Some(src.as_str()), &cfg)));

    // Assert
    assert2::assert!(let Outcome::Rewrite { header, body, original_tokens, new_tokens } = outcome);
    check!(header.contains("[snip: read | rust"));
    check!(header.contains("resolve <file>")); // recovery guidance present (with a runnable path)
    check!(!body.contains("EXPLANATORY"));
    check!(body.contains("fn main()"));
    check!(new_tokens < original_tokens);
}

#[test]
fn secret_bearing_source_passes_through_under_secret_safe() {
    // Arrange: a comment-heavy file that WOULD compact, but carries a credential.
    // With secret_safe on it must pass through — no compacted view, spill, or
    // dedupe-cache copy of the secret is produced.
    let cfg: Config = serde_json::from_str(r#"{"secret_safe":true}"#).unwrap();
    let input = json!({"file_path": "/x.rs"});
    let src = format!(
        "// AKIAIOSFODNN7EXAMPLE is an access key id\n{}fn main() {{\n    let x = 1;\n}}\n",
        "// padding comment line with enough words to otherwise trigger compaction\n".repeat(8)
    );

    // Act
    assert2::assert!(let Ok(outcome) = ReadOptimizer.apply(&ctx(Surface::Read, &input, Some(src.as_str()), &cfg)));

    // Assert
    assert2::assert!(let Outcome::PassThrough = outcome);
}

#[test]
fn secret_bearing_source_still_compacts_without_secret_safe() {
    // Arrange: the gate is opt-in — with secret_safe off (the default), the same
    // comment-heavy file compacts as usual.
    let cfg = Config::default();
    let input = json!({"file_path": "/x.rs"});
    let src = format!(
        "// AKIAIOSFODNN7EXAMPLE is an access key id\n{}fn main() {{\n    let x = 1;\n}}\n",
        "// padding comment line with enough words to otherwise trigger compaction\n".repeat(8)
    );

    // Act
    assert2::assert!(let Ok(outcome) = ReadOptimizer.apply(&ctx(Surface::Read, &input, Some(src.as_str()), &cfg)));

    // Assert
    assert2::assert!(let Outcome::Rewrite { .. } = outcome);
}

#[test]
fn non_code_read_passes_through() {
    // Arrange
    let cfg = Config::default();
    let input = json!({"file_path": "/readme.md"});

    // Act
    assert2::assert!(let Ok(outcome) = ReadOptimizer.apply(&ctx(Surface::Read, &input, Some("# hi\n"), &cfg)));

    // Assert
    assert2::assert!(let Outcome::PassThrough = outcome);
}

#[test]
fn edit_passes_through_when_file_unreadable() {
    // Arrange: a non-existent file → edit-fix can't read it → let Edit run
    let cfg = Config::default();
    let input = json!({"file_path": "/no/such/x.rs", "old_string": "a", "new_string": "b"});

    // Act
    assert2::assert!(let Ok(outcome) = ReadOptimizer.apply(&ctx(Surface::Edit, &input, None, &cfg)));

    // Assert
    assert2::assert!(let Outcome::PassThrough = outcome);
}

#[test]
fn no_inflation_guard_passes_through_when_not_smaller() {
    // Arrange: stripping the 3-byte comment leaves both source and view in the
    // same ~4-bytes/token bucket, so the view is not actually smaller in tokens.
    let cfg = Config::default();
    let input = json!({"file_path": "/x.rs"});
    let src = "fn a(){let b=1;}//x\n";

    // Act
    assert2::assert!(let Ok(outcome) = ReadOptimizer.apply(&ctx(Surface::Read, &input, Some(src), &cfg)));

    // Assert: compaction ran but did not save enough → gate fires
    assert2::assert!(let Outcome::PassThrough = outcome);
}

#[test]
fn edit_corrects_a_stripped_old_string() {
    // Arrange: a real file whose inline comment was stripped in the view
    let path = env::temp_dir().join(format!("snip-edit-{}.rs", std::process::id()));
    fs::write(&path, "fn main() {\n    run(); // go\n    stop();\n}\n").expect("write temp");
    let path_str = path.to_string_lossy().into_owned();
    let cfg = Config::default();
    let input = json!({
        "file_path": path_str,
        "old_string": "fn main() {\n    run();\n    stop();\n}",
        "new_string": "fn main() {\n    run2();\n    stop();\n}",
        "replace_all": false,
    });

    // Act
    assert2::assert!(let Ok(outcome) = ReadOptimizer.apply(&ctx(Surface::Edit, &input, None, &cfg)));

    // Assert: old_string corrected to the real bytes; other fields preserved
    assert2::assert!(let Outcome::FixInput(updated) = outcome);
    check!(
        updated.get("old_string").and_then(Value::as_str)
            == Some("fn main() {\n    run(); // go\n    stop();\n}")
    );
    check!(
        updated.get("new_string").and_then(Value::as_str)
            == Some("fn main() {\n    run2();\n    stop();\n}")
    );
    check!(updated.get("replace_all").and_then(Value::as_bool) == Some(false));

    // Cleanup
    let _ = fs::remove_file(&path);
}

#[test]
fn oversize_source_passes_through_unparsed() {
    // Arrange: a source above MAX_READ_BYTES (1 MB) and no session → no dedupe,
    // so the oversize guard (line 77) is the path taken.
    let cfg = Config::default();
    let input = json!({"file_path": "/big.rs"});
    let big = "fn a() {}\n".repeat(120_000); // ~1.08 MB > 1_000_000 bytes

    // Act
    assert2::assert!(let Ok(outcome) = ReadOptimizer.apply(&ctx(Surface::Read, &input, Some(&big), &cfg)));

    // Assert
    assert2::assert!(let Outcome::PassThrough = outcome);
}

#[test]
fn windowed_read_never_returns_a_dedupe_notice() {
    // Arrange: isolate the cache, pre-remember this exact content as a duplicate,
    // then re-read it WITH an offset — windowed reads skip dedupe (lines 198-200).
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = env::temp_dir().join(format!("snip-readtest-win-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    temp_env::with_var("SNIP_HOME", Some(&home), || {
        let cfg = Config::default();
        let src = "// a comment\nfn main() {\n    let x = 1;\n    let y = 2;\n}\n";
        let plain = json!({"file_path": "/win.rs", "session_id": "s1"});
        let windowed = json!({"file_path": "/win.rs", "offset": 1, "session_id": "s1"});
        let plain_ctx = HookCtx {
            surface: Surface::Read,
            session_id: Some("s1"),
            transcript_path: None,
            input: &plain,
            output: Some(src),
            cfg: &cfg,
        };
        // First read remembers the fingerprint.
        let _ = ReadOptimizer.apply(&plain_ctx);
        let win_ctx = HookCtx {
            surface: Surface::Read,
            session_id: Some("s1"),
            transcript_path: None,
            input: &windowed,
            output: Some(src),
            cfg: &cfg,
        };

        // Act: the duplicate content, but windowed
        assert2::assert!(let Ok(outcome) = ReadOptimizer.apply(&win_ctx));

        // Assert: not the dedupe notice (which would carry "unchanged since")
        if let Outcome::Rewrite { body, .. } = &outcome {
            check!(!body.contains("unchanged since"));
        }
    });

    // Cleanup
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn identical_reread_returns_a_dedupe_notice() {
    // Arrange: isolate the cache so the per-session dedupe map has a home.
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = env::temp_dir().join(format!("snip-readtest-dup-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    temp_env::with_var("SNIP_HOME", Some(&home), || {
        let cfg = Config::default();
        // Large enough that the tiny dedupe notice is clearly fewer tokens than the
        // full view (the no-inflation guard in `dedupe_notice` would otherwise skip it).
        let body = "    let value = compute_something(a, b, c);\n".repeat(40);
        let src = format!("// a leading comment\nfn main() {{\n{body}}}\n");
        let input = json!({"file_path": "/dup/main.rs", "session_id": "sess-dedupe"});
        let make_ctx = || HookCtx {
            surface: Surface::Read,
            session_id: Some("sess-dedupe"),
            transcript_path: None,
            input: &input,
            output: Some(src.as_str()),
            cfg: &cfg,
        };

        // Act: read the same (file_path, content) twice
        assert2::assert!(let Ok(first) = ReadOptimizer.apply(&make_ctx()));
        assert2::assert!(let Ok(second) = ReadOptimizer.apply(&make_ctx()));

        // Assert: the second read collapses to the dedupe notice
        assert2::assert!(let Outcome::Rewrite { body, original_tokens, new_tokens, .. } = second);
        check!(body.contains("dedupe"));
        check!(body.contains("unchanged since"));
        check!(body.contains("main.rs"));
        check!(new_tokens < original_tokens);
        // The notice differs from whatever the first read produced.
        if let Outcome::Rewrite {
            body: first_body, ..
        } = first
        {
            check!(body != first_body);
        }
    });

    // Cleanup
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn edit_corrects_a_comment_only_stripped_line() {
    // Arrange: the soft view drops the "// note" line; the old_string is that
    // stripped view, so the real file does not contain it verbatim (apply_edit
    // lines 155, 161-168 with the soft-mode default).
    let path = env::temp_dir().join(format!("snip-readtest-editnote-{}.rs", std::process::id()));
    fs::write(&path, "fn a() {}\n// note\nfn b() {}\n").expect("write temp");
    let path_str = path.to_string_lossy().into_owned();
    let cfg = Config::default();
    let input = json!({
        "file_path": path_str,
        "old_string": "fn a() {}\nfn b() {}",
        "new_string": "fn a() {}\nfn c() {}",
    });

    // Act
    assert2::assert!(let Ok(outcome) = ReadOptimizer.apply(&ctx(Surface::Edit, &input, None, &cfg)));

    // Assert: the comment line is restored into the corrected old_string
    assert2::assert!(let Outcome::FixInput(updated) = outcome);
    assert2::assert!(let Some(corrected) = updated.get("old_string").and_then(Value::as_str));
    check!(corrected.contains("// note"));
    check!(corrected.contains("fn a() {}"));
    check!(corrected.contains("fn b() {}"));

    // Cleanup
    let _ = fs::remove_file(&path);
}

#[test]
fn write_guard_asks_when_reproducing_the_view() {
    // Arrange: an existing file with a comment; content is its stripped view
    let path = env::temp_dir().join(format!("snip-write-{}.rs", std::process::id()));
    fs::write(&path, "// header\nfn a() {\n    let x = 1;\n}\n").expect("write temp");
    let path_str = path.to_string_lossy().into_owned();
    let cfg = Config::default();
    let input = json!({"file_path": path_str, "content": "\nfn a() {\n    let x = 1;\n}\n"});

    // Act
    assert2::assert!(let Ok(outcome) = ReadOptimizer.apply(&ctx(Surface::Write, &input, None, &cfg)));

    // Assert
    assert2::assert!(let Outcome::Ask { reason } = outcome);
    check!(reason.contains("compacted view"));

    // Cleanup
    let _ = fs::remove_file(&path);
}
