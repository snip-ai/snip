//! Unit tests for the read-dedupe session cache, in AAA form. Compiled into
//! `snip_lib` via a `#[path]` include in `src/optimizers/read/dedupe.rs`.

use std::env;
use std::fs;

use assert2::check;

use super::{fingerprint, is_duplicate, notice, remember};

#[test]
fn remembers_then_detects_a_duplicate() {
    // Arrange: isolate the cache under a temp SNIP_HOME (env is process-global)
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = env::temp_dir().join(format!("snip-dedupe-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    temp_env::with_var("SNIP_HOME", Some(&home), || {
        let fp = fingerprint("the file contents");

        // Act
        let before = is_duplicate("sess1", "/f.rs", &fp);
        remember("sess1", "/f.rs", &fp);
        let after = is_duplicate("sess1", "/f.rs", &fp);
        let other_content = is_duplicate("sess1", "/f.rs", "00000000deadbeef");
        let other_session = is_duplicate("sess2", "/f.rs", &fp);

        // Assert: a hit needs the same session, file, and fingerprint
        check!(!before);
        check!(after);
        check!(!other_content);
        check!(!other_session);
    });

    // Cleanup
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn notice_names_the_file_and_size() {
    // Arrange + Act
    let n = notice("/some/path/main.rs", 1234);

    // Assert
    check!(n.contains("main.rs"));
    check!(n.contains("1234"));
}

#[test]
fn corrupt_cache_file_is_not_a_duplicate() {
    // Arrange: isolate the cache, then write garbage to the dedupe map so the
    // JSON parse fails (line 38, the `serde_json::from_str` error path → false).
    // (The `cache_path` None early-returns on lines 32/46 are not reachable here:
    // they fire only when `data_dir()` is None, but SNIP_HOME is always set.)
    let _guard = crate::paths::ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = env::temp_dir().join(format!("snip-dedupe-corrupt-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    temp_env::with_var("SNIP_HOME", Some(&home), || {
        let cache = crate::paths::session_cache_dir(Some("sess-corrupt")).expect("a cache dir");
        fs::create_dir_all(&cache).expect("mk cache dir");
        fs::write(cache.join("read-dedupe.json"), "{ not valid json").expect("write garbage");
        let fp = fingerprint("anything");

        // Act
        let got = is_duplicate("sess-corrupt", "/f.rs", &fp);

        // Assert: a corrupt map degrades to "not a duplicate" rather than erroring
        check!(!got);
    });

    // Cleanup
    let _ = fs::remove_dir_all(&home);
}
