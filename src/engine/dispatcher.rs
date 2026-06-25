//! The shared hook contract: stdin → optimizer → stdout, always exiting 0.
//!
//! Every tool hook funnels through [`Dispatcher::run`], which loads config,
//! honors the master switch, wraps the body in `catch_unwind`, dispatches to the
//! registry, and serializes the outcome.

use std::io::{Read, Write};

use serde_json::Value;

use crate::config::Config;
use crate::domain::{HookCtx, Outcome, Surface};
use crate::engine::{OutcomeSerializer, Registry, ToolResponse};
use crate::overflow::Spill;
use crate::stats::Tracker;
use crate::tokens::estimate_tokens;

/// Runs one tool hook end-to-end for a given [`Surface`].
///
/// Maintenance hooks (`session-reset`, `update-check`) have their own entry
/// points and bypass the master switch; every surface that reaches here obeys it.
pub struct Dispatcher {
    surface: Surface,
}

impl Dispatcher {
    /// Create a dispatcher for `surface`.
    #[must_use]
    pub const fn new(surface: Surface) -> Self {
        Self { surface }
    }

    /// Run the hook, guarding every error and panic (including `Config::load`).
    ///
    /// In production always returns `Ok(())` (the exit-0 invariant); failures are
    /// caught and logged to stderr. In strict debug mode
    /// ([`crate::panic_guard::strict`]) the failure is surfaced as `Err` (non-zero
    /// exit) so a developer sees it immediately.
    ///
    /// # Errors
    /// Only under `SNIP_DEBUG`; otherwise never.
    pub fn run(&self) -> anyhow::Result<()> {
        // Guard EVERYTHING — including `Config::load` — so a hook can never exit
        // non-zero in production, even on a panic before dispatch.
        crate::panic_guard::guarded(self.surface.name(), || {
            let cfg = Config::load();
            if !cfg.master_enabled {
                return Ok(());
            }
            self.run_inner(&cfg)
        })
    }

    fn run_inner(&self, cfg: &Config) -> anyhow::Result<()> {
        let raw = read_stdin()?;
        if let Some(output) = self.process_raw(&raw, cfg) {
            let mut out = std::io::BufWriter::new(std::io::stdout().lock());
            serde_json::to_writer(&mut out, &output)?;
            out.flush()?;
        }
        Ok(())
    }

    /// Parse one raw hook payload and produce the JSON to print, or `None` for
    /// pass-through. The string-level seam behind the stdin-driven [`Self::run`]:
    /// empty, malformed, and missing-field inputs all degrade to `None` (never an
    /// error or panic), which the unit tests exercise directly.
    pub(crate) fn process_raw(&self, raw: &str, cfg: &Config) -> Option<Value> {
        if raw.trim().is_empty() {
            return None;
        }
        let hook: Value = serde_json::from_str(raw).ok()?;
        self.process(&hook, cfg)
    }

    /// Dispatch one parsed hook event and return the JSON to print, or `None` for
    /// pass-through (empty stdout ⇒ Claude Code keeps the original).
    fn process(&self, hook: &Value, cfg: &Config) -> Option<Value> {
        let input = hook.get("tool_input")?;
        let output_owned = if self.surface.is_post() {
            ToolResponse::new(hook.get("tool_response")).extract_text()
        } else {
            None
        };
        let ctx = HookCtx {
            surface: self.surface,
            session_id: hook.get("session_id").and_then(Value::as_str),
            transcript_path: hook.get("transcript_path").and_then(Value::as_str),
            input,
            output: output_owned.as_deref(),
            cfg,
        };
        // Re-reading a spilled output is an induced cost (the NET subtrahend).
        record_spill_reread(&ctx);
        // No match ⇒ pass-through (empty stdout); the full original is kept.
        let registry = Registry::build(cfg, self.surface);
        let opt = registry.first_match(&ctx)?;
        let outcome = opt.apply(&ctx).unwrap_or(Outcome::PassThrough);
        let outcome = Self::with_overflow(outcome, &ctx, opt.name());
        if let Outcome::Rewrite {
            original_tokens,
            new_tokens,
            ..
        } = &outcome
        {
            Tracker::record_saved(
                opt.name(),
                self.surface.name(),
                *original_tokens,
                *new_tokens,
            );
        }
        OutcomeSerializer::serialize(hook, outcome)
    }

    /// Apply the shared overflow budget to any `Rewrite` body — read and search
    /// here; the command runtime applies the same `Spill` service in `assemble` —
    /// recomputing the post-cap token count.
    fn with_overflow(outcome: Outcome, ctx: &HookCtx, name: &str) -> Outcome {
        let Outcome::Rewrite {
            header,
            body,
            original_tokens,
            lossy,
            ..
        } = outcome
        else {
            return outcome;
        };
        // A lossy rewrite (a `Truncate` elided the middle) would otherwise discard
        // those records: spill the full original and breadcrumb the view so they
        // stay recoverable, then apply the budget cap on top.
        let body = if lossy {
            Spill::keep_recoverable(&body, ctx.output.unwrap_or(&body), ctx.session_id, name)
        } else {
            body
        };
        let body = Spill::apply(body, ctx.session_id, name, ctx.cfg.overflow_for(name));
        // The model pays for the injected header (guidance/breadcrumb) plus the body,
        // so the recorded NET must count both — body-only would overstate the gain by
        // the per-rewrite header tax.
        let new_tokens = estimate_tokens(&header) + estimate_tokens(&body);
        // The breadcrumb a lossy spill appends costs a line; for a near-threshold short
        // output that can erase the saving, so re-apply the no-inflation guard and keep
        // the original verbatim (a `PassThrough` — nothing dropped) when it does.
        if lossy && new_tokens >= original_tokens {
            return Outcome::PassThrough;
        }
        Outcome::Rewrite {
            header,
            body,
            original_tokens,
            new_tokens,
            // The spill above already made any dropped content recoverable.
            lossy: false,
        }
    }
}

fn read_stdin() -> anyhow::Result<String> {
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf)?;
    Ok(buf)
}

/// Record the induced cost when a `Read` targets one of our own spill files: the
/// model is paying to recover output an earlier optimization truncated.
fn record_spill_reread(ctx: &HookCtx) {
    if ctx.surface != Surface::Read {
        return;
    }
    let Some(path) = ctx.input.get("file_path").and_then(Value::as_str) else {
        return;
    };
    if Spill::is_spill_path(path)
        && let Some(output) = ctx.output
    {
        Tracker::record_induced("overflow", "read", estimate_tokens(output));
    }
}

#[cfg(test)]
#[path = "../../tests/unit/engine/dispatcher.tests.rs"]
mod tests;
