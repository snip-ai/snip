//! Unit tests for the [`Spill`] overflow service, in AAA form. Compiled into
//! `snip_lib` via a `#[path]` include in `src/overflow/spill.rs`.

use std::env;
use std::fs;

use assert2::check;

use super::{MAX_SPILL_FILES, Spill, evict_old_spills};
use crate::overflow::OverflowCfg;

#[test]
fn under_budget_returns_the_body_unchanged() {
    // Arrange: well within budget ⇒ no spill, no I/O
    let cfg = OverflowCfg::default();
    let body = "a small body\n".to_owned();

    // Act
    let out = Spill::apply(body.clone(), Some("s1"), "read", &cfg);

    // Assert
    check!(out == body);
}

#[test]
fn over_budget_elides_spills_and_leaves_a_breadcrumb() {
    // Arrange: a unique data root so the spill never touches the real one
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = env::temp_dir().join(format!("snip-spill-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    temp_env::with_var("SNIP_HOME", Some(&home), || {
        let cfg = OverflowCfg {
            max_tokens: 4,
            ..OverflowCfg::default()
        };
        let body = "a match line\n".repeat(200);

        // Act
        let out = Spill::apply(body.clone(), Some("sess-1"), "search", &cfg);

        // Assert: view shrank, carries the breadcrumb, and the full body is on disk
        check!(out.len() < body.len());
        check!(out.contains("output truncated"));
        let spill = fs::read_dir(home.join("session-cache").join("sess-1"))
            .unwrap()
            .filter_map(Result::ok)
            .find(|e| e.file_name().to_string_lossy().starts_with("spill-search-"));
        assert2::assert!(let Some(entry) = spill);
        check!(fs::read_to_string(entry.path()).unwrap() == body);
    });
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn spill_failure_yields_an_unavailable_breadcrumb() {
    // Arrange: point SNIP_HOME at a regular *file* so create_dir_all under it
    // fails — write_spill returns None and the breadcrumb reports the failure
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = env::temp_dir().join(format!("snip-spill-fail-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    let _ = fs::remove_file(&home);
    fs::write(&home, b"not a dir").unwrap();
    temp_env::with_var("SNIP_HOME", Some(&home), || {
        let cfg = OverflowCfg {
            max_tokens: 4,
            ..OverflowCfg::default()
        };
        let body = "a match line\n".repeat(200);

        // Act
        let out = Spill::apply(body, Some("sess-x"), "search", &cfg);

        // Assert: still capped, but the breadcrumb flags the spill as unavailable
        check!(out.contains("output truncated"));
        check!(out.contains("spill failed"));
    });
    let _ = fs::remove_file(&home);
}

#[test]
fn keep_recoverable_spills_the_original_and_breadcrumbs_the_view() {
    // Arrange: a lossy folded view + the full original it dropped lines from
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = env::temp_dir().join(format!("snip-keep-rec-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    temp_env::with_var("SNIP_HOME", Some(&home), || {
        let original = (0..25)
            .map(|i| format!("/src/module_{i}/file_{i}.rs"))
            .collect::<Vec<_>>()
            .join("\n");
        let view = "/src/module_0/file_0.rs (×25)".to_owned();

        // Act
        let out = Spill::keep_recoverable(&view, &original, Some("sess-r"), "command-autodetect");

        // Assert: the folded view is kept, a breadcrumb points at the spill, and the
        // spill file holds the FULL original (every dropped line is recoverable)
        check!(out.starts_with(&view));
        check!(out.contains("recoverable"));
        let spill = fs::read_dir(home.join("session-cache").join("sess-r"))
            .unwrap()
            .filter_map(Result::ok)
            .find(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("spill-command-autodetect-")
            });
        assert2::assert!(let Some(entry) = spill);
        let recovered = fs::read_to_string(entry.path()).unwrap();
        check!(recovered == original);
        check!(recovered.contains("/src/module_24/file_24.rs"));
    });
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn keep_recoverable_falls_back_to_verbatim_when_the_spill_fails() {
    // Arrange: SNIP_HOME is a regular file ⇒ the spill write fails
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = env::temp_dir().join(format!("snip-keep-rec-fail-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    let _ = fs::remove_file(&home);
    fs::write(&home, b"not a dir").unwrap();
    temp_env::with_var("SNIP_HOME", Some(&home), || {
        let original = "line one\nline two\nline three".to_owned();

        // Act
        let out =
            Spill::keep_recoverable("folded (×3)", &original, Some("s"), "command-autodetect");

        // Assert: no recoverable view possible ⇒ the full original verbatim, nothing lost
        check!(out == original);
    });
    let _ = fs::remove_file(&home);
}

#[test]
fn evict_ignores_a_missing_directory() {
    // Arrange: a path that does not exist → read_dir errors
    let dir = env::temp_dir().join(format!("snip-evict-missing-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);

    // Act + Assert: must return without panicking on the read_dir error
    evict_old_spills(&dir, MAX_SPILL_FILES);
    check!(!dir.exists());
}

#[test]
fn evict_caps_spill_files_and_spares_other_files() {
    // Arrange: more than MAX_SPILL_FILES spill files plus an unrelated file
    let dir = env::temp_dir().join(format!("snip-evict-cap-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let total = MAX_SPILL_FILES + 10;
    for i in 0..total {
        // distinct fingerprints; mtime ordering follows creation order well enough
        fs::write(dir.join(format!("spill-search-{i:04}.txt")), b"x").unwrap();
    }
    fs::write(dir.join("keep-me.log"), b"y").unwrap();

    // Act
    evict_old_spills(&dir, MAX_SPILL_FILES);

    // Assert: spill files capped to MAX; the non-spill file is untouched
    let entries: Vec<String> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    let spills = entries.iter().filter(|n| n.starts_with("spill-")).count();
    check!(spills == MAX_SPILL_FILES);
    check!(entries.iter().any(|n| n.as_str() == "keep-me.log"));

    // Cleanup
    let _ = fs::remove_dir_all(&dir);
}
