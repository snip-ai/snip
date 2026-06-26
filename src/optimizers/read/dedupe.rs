//! Per-session read-dedupe: replace an identical re-read with a tiny notice.
//!
//! A flat JSON map `file_path → fingerprint` per session (no `SQLite` on the hot
//! path), under the same session cache dir the overflow spills use — so the
//! `PreCompact` `session-reset` clears both at once.

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use super::diff;
use crate::domain::HEADER_PREFIX;
use crate::paths::session_cache_dir;
use crate::tokens::estimate_tokens;

/// Upper bound on a file we cache for later diffing. Above it we keep only the
/// fingerprint (a changed re-read of a huge file just serves the full view again),
/// so diff-on-change never stores multi-megabyte blobs per read.
const MAX_CONTENT_BYTES: usize = 262_144;

/// The dedupe map file for a session: `<session cache>/read-dedupe.json`.
fn cache_path(session_id: &str) -> Option<PathBuf> {
    Some(session_cache_dir(Some(session_id))?.join("read-dedupe.json"))
}

/// The stored previous-content file for one path: `<session cache>/read-<h>.txt`.
fn content_path(session_id: &str, file_path: &str) -> Option<PathBuf> {
    Some(session_cache_dir(Some(session_id))?.join(format!("read-{}.txt", fingerprint(file_path))))
}

/// A stable content fingerprint (deterministic `SipHash` with fixed keys).
#[must_use]
pub fn fingerprint(content: &str) -> String {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Whether `file_path` was already read with this exact `fingerprint` this session.
#[must_use]
pub fn is_duplicate(session_id: &str, file_path: &str, fingerprint: &str) -> bool {
    let Some(path) = cache_path(session_id) else {
        return false;
    };
    let Ok(text) = fs::read_to_string(path) else {
        return false;
    };
    let Ok(map) = serde_json::from_str::<serde_json::Value>(&text) else {
        return false;
    };
    map.get(file_path).and_then(serde_json::Value::as_str) == Some(fingerprint)
}

/// Remember this read's fingerprint (best-effort; failures are ignored).
pub fn remember(session_id: &str, file_path: &str, fingerprint: &str) {
    let Some(path) = cache_path(session_id) else {
        return;
    };
    let mut map: serde_json::Map<String, serde_json::Value> = fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    map.insert(
        file_path.to_owned(),
        serde_json::Value::String(fingerprint.to_owned()),
    );
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(&path, serde_json::Value::Object(map).to_string());
}

/// Store this read's content for a later diff-on-change (best-effort, size-capped).
fn remember_content(session_id: &str, file_path: &str, content: &str) {
    let Some(path) = content_path(session_id, file_path) else {
        return;
    };
    if content.len() > MAX_CONTENT_BYTES {
        let _ = fs::remove_file(&path); // too big to diff; drop any stale copy
        return;
    }
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(&path, content);
}

/// The content of this path's previous read this session, if it was stored.
fn previous_content(session_id: &str, file_path: &str) -> Option<String> {
    fs::read_to_string(content_path(session_id, file_path)?).ok()
}

/// Decide what (if anything) should replace a full re-read of `content`:
/// identical to the last read → the tiny "unchanged" notice; changed, with the
/// prior text stored and a diff that actually saves → a compact diff-vs-last-read
/// view; otherwise `None` (first read, or not worthwhile) so the caller compacts
/// normally. Remembers this read's fingerprint + content (size-capped) either way.
#[must_use]
pub fn notice_or_diff(session_id: &str, file_path: &str, content: &str) -> Option<String> {
    let fp = fingerprint(content);
    if is_duplicate(session_id, file_path, &fp) {
        return Some(notice(file_path, estimate_tokens(content)));
    }
    let previous = previous_content(session_id, file_path);
    remember(session_id, file_path, &fp);
    remember_content(session_id, file_path, content);
    diff::changed_notice(file_path, &previous?, content)
}

/// The tiny "unchanged since your last Read" notice that replaces the full view.
#[must_use]
pub fn notice(file_path: &str, full_tokens: usize) -> String {
    let filename = Path::new(file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(file_path);
    format!(
        "{HEADER_PREFIX} dedupe — {filename} is unchanged since your earlier Read this session; \
         full view (~{full_tokens} tok) omitted. Re-Read with offset/limit for the verbatim \
         slice (exact bytes to Edit).]"
    )
}

#[cfg(test)]
#[path = "../../../tests/unit/optimizers/read/dedupe.tests.rs"]
mod tests;
