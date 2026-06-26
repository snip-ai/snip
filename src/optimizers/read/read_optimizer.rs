//! The `read` optimizer — soft/medium/high AST compaction plus edit-safety.
//!
//! On `Read` it compacts code for the configured [`CompactMode`] (soft strips
//! comments byte-identically; medium/high collapse code), dedupes identical
//! re-reads, and returns a token-saving [`Outcome::Rewrite`] with a mode-specific
//! recovery-guidance header. On `Edit` it maps a compacted `old_string` back to
//! real bytes ([`Outcome::FixInput`], re-expanding a collapsed `new_string`); on
//! `Write` it asks before reproducing the stripped view ([`Outcome::Ask`]).

use serde_json::Value;

use super::{dedupe, edit_write};
use crate::compaction::Compactor;
use crate::config::CompactMode;
use crate::domain::{HEADER_PREFIX, HookCtx, Optimizer, Outcome, Surface};
use crate::languages;
use crate::tokens::estimate_tokens;

/// Code-reading optimizer attached to the Read, Edit, and Write surfaces.
pub struct ReadOptimizer;

const READ_SURFACES: &[Surface] = &[Surface::Read, Surface::Edit, Surface::Write];

/// Hard byte cap: a cheap pre-filter, not the parse guarantee. Files this large
/// are almost always generated/minified (low compaction upside, high parse cost),
/// so past this we pass through without attempting a parse. Everything under it
/// IS optimized — the size-scaled parse budget ([`crate::compaction::parse`])
/// grants a large file enough time to finish. Identical re-reads still dedupe
/// (that check is cheap and runs first).
const MAX_READ_BYTES: usize = 5_000_000;

// Guidance templates: `{snip}` is substituted at runtime with a runnable
// invocation (the plugin installs snip to `$SNIP_HOME/bin`, which it does NOT add
// to `PATH`, so a bare `snip resolve` would fail in a real install). `guidance`
// wraps the final line in the `[snip: …]` header so `write-guard` strips it.
/// Soft-mode guidance: code-only lines match as-is; a comment-spanning Edit needs
/// a verbatim slice (a windowed re-Read) or `resolve`.
const GUIDANCE_SOFT: &str = "comments removed — code lines without a comment match as-is; \
for an Edit whose old_string spans a removed comment, re-Read those lines with offset/limit \
(returns the verbatim slice) or pipe old_string to `{snip} resolve <file>`.";
/// Medium/high guidance: lines are rewritten, so use a verbatim slice or resolve.
const GUIDANCE_COLLAPSED: &str = "comments stripped AND code collapsed — text copied from \
this view will NOT match the file. To Edit, re-Read the target lines with offset/limit (the \
verbatim slice) or pipe old_string to `{snip} resolve <file>`.";

impl Optimizer for ReadOptimizer {
    // The trait fixes the return as `-> &str`, so the 'static literal trips a lint.
    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "read"
    }

    fn surfaces(&self) -> &[Surface] {
        READ_SURFACES
    }

    fn matches(&self, _ctx: &HookCtx) -> bool {
        true
    }

    fn apply(&self, ctx: &HookCtx) -> anyhow::Result<Outcome> {
        Ok(match ctx.surface {
            Surface::Read => apply_read(ctx),
            Surface::Edit => edit_write::apply_edit(ctx),
            Surface::Write => edit_write::apply_write(ctx),
            _ => Outcome::PassThrough,
        })
    }
}

/// `Read`: a windowed read (offset/limit) passes through VERBATIM (exact bytes for
/// an Edit); a full read dedupes an identical re-read, else compacts for the
/// configured mode when it saves ≥5% (so the guidance header stays net-positive).
fn apply_read(ctx: &HookCtx<'_>) -> Outcome {
    let (Some(path), Some(source)) = (file_path(ctx), ctx.output) else {
        return Outcome::PassThrough;
    };
    // A windowed read (offset/limit) is the model asking for a specific slice —
    // typically to copy exact bytes for an Edit. Return it verbatim so `old_string`
    // matches the real file; only full reads are compacted (where the savings are).
    if windowed(ctx) {
        return Outcome::PassThrough;
    }
    // secret_safe (opt-in, off by default): pass a secret-bearing source file
    // through uncompacted BEFORE dedupe — so no compacted view, spill file, or
    // dedupe-cache copy of the credential is ever produced. Masking source bytes
    // would break Edit-safety, so passthrough is the safe choice (the model
    // requested this file anyway). Output surfaces mask instead (see `redact`).
    if ctx.cfg.secret_safe && crate::optimizers::redact::any_secret(source) {
        return Outcome::PassThrough;
    }
    if let Some(notice) = dedupe_notice(ctx, path, source) {
        return notice;
    }
    if source.len() > MAX_READ_BYTES {
        return Outcome::PassThrough;
    }
    let Some(spec) = languages::detect(path) else {
        return Outcome::PassThrough;
    };
    let mode = ctx.cfg.mode_for("read");
    let Some(body) = Compactor::new(spec).compress_mode(source, mode) else {
        return Outcome::PassThrough;
    };
    let original_tokens = estimate_tokens(source);
    // Guard the `pct` divide (and skip a no-savings empty source).
    if original_tokens == 0 {
        return Outcome::PassThrough;
    }
    let body_tokens = estimate_tokens(&body);
    // The header reports the code reduction the AST compaction achieved.
    let pct = 100 - body_tokens.min(original_tokens) * 100 / original_tokens;
    let name = spec.name;
    let header = format!(
        "{HEADER_PREFIX} read | {name} | {} | {original_tokens}→{body_tokens} tok (-{pct}%)]\n{}\n",
        mode.as_str(),
        guidance(mode)
    );
    // Header-inclusive ≥5% gate: the model pays for the recovery-guidance header too,
    // so compact only when header + body still beats the original by ≥5% — otherwise
    // the guidance eats the savings and the rewrite is net-negative.
    let new_tokens = body_tokens + estimate_tokens(&header);
    if new_tokens * 105 > original_tokens * 100 {
        return Outcome::PassThrough;
    }
    Outcome::Rewrite {
        header,
        body,
        original_tokens,
        new_tokens,
        // Read compaction is reversible (origin map / edit-fix), never discarded.
        lossy: false,
    }
}

/// The recovery-guidance line for `mode`, wrapped in the `[snip: …]` header and
/// with `{snip}` resolved to a runnable invocation — a bare `snip resolve` would
/// fail in a real install (snip lives in `$SNIP_HOME/bin`, not on `PATH`).
fn guidance(mode: CompactMode) -> String {
    let template = match mode {
        CompactMode::Soft => GUIDANCE_SOFT,
        CompactMode::Medium | CompactMode::High => GUIDANCE_COLLAPSED,
    };
    format!(
        "{HEADER_PREFIX} {}]",
        template.replace("{snip}", &snip_invocation())
    )
}

/// How to invoke snip from a Bash command line: the running binary's absolute
/// path, forward-slashed and quoted so Git Bash accepts it on Windows. Falls back
/// to bare `snip` if the path is unknown. Mirrors `bash_route::rewrite_command`.
fn snip_invocation() -> String {
    std::env::current_exe().map_or_else(
        |_| "snip".to_owned(),
        |p| format!("\"{}\"", p.to_string_lossy().replace('\\', "/")),
    )
}

/// Replace a re-read with the dedupe notice (unchanged) or a diff-vs-last-read
/// (changed), when dedupe is on, the Read is not windowed, and it actually saves.
/// [`dedupe::notice_or_diff`] remembers this read's fingerprint/content as a
/// side effect; a first read returns `None` so normal compaction proceeds.
fn dedupe_notice(ctx: &HookCtx<'_>, path: &str, source: &str) -> Option<Outcome> {
    let sid = ctx.session_id?;
    if windowed(ctx) || !ctx.cfg.dedupe_enabled("read") {
        return None;
    }
    let body = dedupe::notice_or_diff(sid, path, source)?;
    let original_tokens = estimate_tokens(source);
    let new_tokens = estimate_tokens(&body);
    if new_tokens >= original_tokens {
        return None;
    }
    Some(Outcome::Rewrite {
        header: String::new(),
        body,
        original_tokens,
        new_tokens,
        // A dedupe notice / diff is a complete view of the change, nothing dropped.
        lossy: false,
    })
}

/// The `file_path` field of the tool input, if present and a string. Shared with
/// the Edit/Write handlers in [`super::edit_write`].
pub(super) fn file_path<'a>(ctx: &HookCtx<'a>) -> Option<&'a str> {
    ctx.input.get("file_path").and_then(Value::as_str)
}

/// Whether the Read is windowed (`offset`/`limit`) — windowed reads skip dedupe.
fn windowed(ctx: &HookCtx<'_>) -> bool {
    ctx.input.get("offset").is_some() || ctx.input.get("limit").is_some()
}

#[cfg(test)]
#[path = "../../../tests/unit/optimizers/read/read_optimizer.tests.rs"]
mod tests;
