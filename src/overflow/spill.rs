//! The overflow service: cap an over-budget view, spill the full body, leave a
//! breadcrumb. Never discards output — the spill is recoverable with a plain
//! `Read`. The breadcrumb lives in the tool output, never in markdown.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::domain::HEADER_PREFIX;
use crate::overflow::OverflowCfg;
use crate::paths::{SESSION_CACHE_DIRNAME, session_cache_dir};
use crate::tokens::estimate_tokens;

/// Filename prefix of every spill file. The single source of truth shared by the
/// writer, the LRU evictor, and [`Spill::is_spill_path`] (which the engine uses to
/// detect induced re-reads) — so a rename here can never silently break the NET
/// accounting.
const SPILL_PREFIX: &str = "spill-";

/// Max spill files kept per session-cache dir; the oldest are evicted past this so
/// a long session can't grow the cache without bound (the full output is always
/// re-derivable by re-running the command, so eviction is safe).
const MAX_SPILL_FILES: usize = 64;

/// Caps an already-optimized body to its token budget, spilling the full body to
/// a session-scoped file when it overflows.
pub struct Spill;

impl Spill {
    /// Return `body` unchanged if within budget; otherwise spill the full body
    /// and return the elided view with a recovery breadcrumb appended.
    #[must_use]
    pub fn apply(body: String, session_id: Option<&str>, opt: &str, cfg: &OverflowCfg) -> String {
        let est = estimate_tokens(&body);
        if est <= cfg.max_tokens {
            return body;
        }
        let trailing = if body.ends_with('\n') { "\n" } else { "" };
        let records: Vec<String> = body.lines().map(str::to_owned).collect();
        let kept = cfg.strategy.elide(&records, cfg.max_tokens, cfg.head_frac);
        let max = cfg.max_tokens;
        let breadcrumb = write_spill(&body, session_id, opt).map_or_else(
            || format!("{HEADER_PREFIX} output truncated ~{max}/{est} tok — full output unavailable (spill failed).]"),
            |path| {
                format!(
                    "{HEADER_PREFIX} output truncated ~{max}/{est} tok, full: {} (read if needed)]",
                    path.display()
                )
            },
        );
        format!("{}\n{breadcrumb}{trailing}", kept.join("\n"))
    }

    /// Persist `original` to a recoverable spill file and append a breadcrumb to
    /// `view`, so a *lossy* rewrite never discards output: the compacted view is
    /// shown, the full original stays one `Read` away.
    ///
    /// Unlike [`Self::apply`] (a budget-gated overflow cap that spills the
    /// already-optimized body), this is for transforms whose view can lose distinct
    /// content — the autodetect log fingerprinter (which collapses masked-equal
    /// lines) and the `Truncate` transform (which elides the middle records). Falls
    /// back to `original` verbatim if the spill cannot be written: never a lossy view
    /// with no path back to the dropped lines.
    #[must_use]
    pub fn keep_recoverable(
        view: &str,
        original: &str,
        session_id: Option<&str>,
        opt: &str,
    ) -> String {
        write_spill(original, session_id, opt).map_or_else(
            || original.to_owned(),
            |path| {
                format!(
                    "{view}\n{HEADER_PREFIX} {} lines recoverable: {} (read if needed)]",
                    original.lines().count(),
                    path.display()
                )
            },
        )
    }

    /// Whether `path` points at one of snip's own spill files — a Read of it is an
    /// induced recovery cost, not a fresh read. Asks the owning module rather than
    /// having the engine substring-match internals it does not own.
    #[must_use]
    pub fn is_spill_path(path: &str) -> bool {
        path.contains(SESSION_CACHE_DIRNAME)
            && Path::new(path)
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(SPILL_PREFIX))
    }
}

/// Write the full body to `<session cache>/spill-<opt>-<fingerprint>.txt`,
/// returning its path (best-effort: any failure yields `None`).
fn write_spill(body: &str, session_id: Option<&str>, opt: &str) -> Option<PathBuf> {
    let dir = session_cache_dir(session_id)?;
    fs::create_dir_all(&dir).ok()?;
    let path = dir.join(format!("{SPILL_PREFIX}{opt}-{}.txt", fingerprint(body)));
    fs::write(&path, body).ok()?;
    evict_old_spills(&dir, MAX_SPILL_FILES);
    Some(path)
}

/// Best-effort LRU cap: keep at most `max` `spill-*.txt` files in `dir`, deleting
/// the oldest by mtime. Any I/O error is ignored (the cap is hygiene, not safety).
fn evict_old_spills(dir: &Path, max: usize) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut spills: Vec<(SystemTime, PathBuf)> = entries
        .filter_map(Result::ok)
        .filter_map(|e| {
            let path = e.path();
            let name = path.file_name()?.to_str()?;
            let is_spill = name.starts_with(SPILL_PREFIX)
                && path
                    .extension()
                    .is_some_and(|x| x.eq_ignore_ascii_case("txt"));
            if !is_spill {
                return None;
            }
            let mtime = e.metadata().ok()?.modified().ok()?;
            Some((mtime, path))
        })
        .collect();
    if spills.len() <= max {
        return;
    }
    spills.sort_by_key(|(mtime, _)| *mtime);
    for (_, path) in &spills[..spills.len() - max] {
        let _ = fs::remove_file(path);
    }
}

/// A stable content fingerprint (deterministic `SipHash` with fixed keys) so a
/// repeated overflow of identical content reuses the same spill file.
fn fingerprint(body: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    body.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[cfg(test)]
#[path = "../../tests/unit/overflow/spill.tests.rs"]
mod tests;
