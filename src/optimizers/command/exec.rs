//! `snip exec -- <base64>` — the command runtime.
//!
//! Decodes the original command, builds its sentinel plan, runs it via the real
//! shell (`sh -c`) on its exact bytes, slices the captured stdout on the markers,
//! optimizes each recognized segment's slice, and prints with the original exit
//! code. `stderr` is passed through verbatim. If anything fails, the command runs
//! verbatim — output is never corrupted.

use std::io::Write;

use anyhow::{Context, anyhow};

use crate::config::Config;
use crate::optimizers::command::assemble::assemble;
use crate::optimizers::command::capture::capture;
use crate::optimizers::command::{CommandSpecs, Plan, b64, unrecognized};
use crate::overflow::Spill;
use crate::stats::Tracker;
use crate::tokens::estimate_tokens;

/// The outcome of running a wrapped command: stdout, exit code, and the tokens
/// saved when an optimization applied (`None` ⇒ verbatim, nothing to record).
///
/// Named `RunOutcome` (not `Outcome`) so it never reads as the central
/// [`crate::domain::Outcome`] enum — this is the command runtime's own result.
struct RunOutcome {
    stdout: Vec<u8>,
    code: i32,
    saved: Option<(usize, usize)>,
}

/// Run a wrapped command; `args` is everything after `exec` (`-- <base64>`).
///
/// # Errors
/// Errors only if the payload is missing/undecodable or the shell can't spawn;
/// output-optimization failures degrade to verbatim output.
pub fn run(args: &[String]) -> anyhow::Result<()> {
    let command = decode(args)?;
    let outcome = execute(&command)?;
    if let Some((before, after)) = outcome.saved {
        Tracker::record_saved("command", "bash", before, after);
    }
    let out = std::io::stdout();
    let mut writer = out.lock();
    writer.write_all(&outcome.stdout)?;
    writer.flush()?;
    std::process::exit(outcome.code);
}

/// Route `command`, returning its (optimized) stdout bytes and exit code without
/// printing, exiting, or recording — the side-effect-free core used by tests.
///
/// # Errors
/// Propagates a shell spawn failure.
pub fn run_capture(command: &str) -> anyhow::Result<(Vec<u8>, i32)> {
    let outcome = execute(command)?;
    Ok((outcome.stdout, outcome.code))
}

/// Run `command` (optimized when recognized, else verbatim) and report savings.
fn execute(command: &str) -> anyhow::Result<RunOutcome> {
    let cfg = Config::load();
    // Honor the per-optimizer switch (and master switch): when `command` is
    // disabled, run the command verbatim and optimize nothing. `bash-route`
    // already gates the rewrite, so this only fires on a direct `snip exec` or a
    // config change between routing and execution — never corrupt, never silent.
    if !cfg.optimizer_enabled("command") {
        let (buf, code, truncated) = capture(command, &[])?;
        return Ok(RunOutcome {
            stdout: if truncated {
                truncated_verbatim(buf)
            } else {
                buf
            },
            code,
            saved: None,
        });
    }
    // Set by `bash-route` so command spills are scoped to this session (and thus
    // cleared at `PreCompact`); absent on a direct `snip exec`, falling back to the
    // shared no-session bucket.
    let session = std::env::var("SNIP_SESSION").ok();
    let specs = CommandSpecs::load(&cfg);
    match Plan::build(command, &specs) {
        Some(plan) if plan.has_recognized() => {
            let (buf, code, truncated) = capture(&plan.wrapped, &[("SNIP_M", &plan.token)])?;
            // Capture hit the byte cap: the marker stream is incomplete, so skip
            // optimization and return the verbatim prefix (markers stripped) + notice.
            if truncated {
                return Ok(RunOutcome {
                    stdout: truncated_verbatim(strip_token(&buf, &plan.token)),
                    code,
                    saved: None,
                });
            }
            let before = estimate_tokens(&String::from_utf8_lossy(&strip_token(&buf, &plan.token)));
            let stdout = match std::str::from_utf8(&buf) {
                // The command has already run; a panic in the pure assemble/optimize
                // step must degrade to the verbatim output (markers stripped), never
                // unwind to `main` — exec is the one hook-adjacent path not wrapped by
                // Dispatcher or panic_guard.
                Ok(text) => guarded_opt(
                    || assemble(text, &plan, &specs, &cfg, session.as_deref()).into_bytes(),
                    || strip_token(&buf, &plan.token),
                ),
                Err(_) => strip_token(&buf, &plan.token), // binary → verbatim minus markers
            };
            let after = estimate_tokens(&String::from_utf8_lossy(&stdout));
            let saved = (after < before).then_some((before, after));
            Ok(RunOutcome {
                stdout,
                code,
                saved,
            })
        }
        _ => {
            // Nothing recognized: run verbatim, then auto-detect structured output.
            let (buf, code, truncated) = capture(command, &[])?;
            if truncated {
                return Ok(RunOutcome {
                    stdout: truncated_verbatim(buf),
                    code,
                    saved: None,
                });
            }
            // Non-UTF8 (binary) output: nothing safe to rewrite — pass through.
            let Ok(text) = std::str::from_utf8(&buf) else {
                return Ok(RunOutcome {
                    stdout: buf,
                    code,
                    saved: None,
                });
            };
            let before = estimate_tokens(text);
            // Source dump → AST read-engine compaction; else structured auto-detect
            // (JSON/TOON, repetitive-log fold); else raw. Guarded so a panic degrades
            // to verbatim output.
            let view = guarded_opt(
                || unrecognized::optimized_view(command, text, &cfg, session.as_deref()),
                || text.to_owned(),
            );
            // Universal floor: cap the shown view + recoverable spill so even
            // unrecognized, non-foldable output never floods context. A no-op when
            // the view is already under budget — small output stays byte-identical.
            let ov = cfg.overflow_for_command("command");
            let capped = Spill::apply(view, session.as_deref(), "command", &ov);
            let after = estimate_tokens(&capped);
            // No-inflation guard: keep the cap only when it actually saved tokens.
            if after < before {
                Ok(RunOutcome {
                    stdout: capped.into_bytes(),
                    code,
                    saved: Some((before, after)),
                })
            } else {
                Ok(RunOutcome {
                    stdout: buf,
                    code,
                    saved: None,
                })
            }
        }
    }
}

/// Decode the base64 payload (the last non-`--` argument) into the command.
fn decode(args: &[String]) -> anyhow::Result<String> {
    let encoded = args
        .iter()
        .rev()
        .find(|a| a.as_str() != "--")
        .ok_or_else(|| anyhow!("snip exec: missing command payload"))?;
    let bytes = b64::decode(encoded).ok_or_else(|| anyhow!("snip exec: invalid base64 payload"))?;
    String::from_utf8(bytes).context("snip exec: payload is not valid UTF-8")
}

/// Append a one-line notice (the standard `[snip: …]` marker, never a sentinel)
/// when capture hit the byte cap, so the bounded prefix isn't mistaken for the
/// whole output. Claude Code still natively spills this prefix; a re-run recovers
/// the full output.
fn truncated_verbatim(mut bytes: Vec<u8>) -> Vec<u8> {
    let mib = crate::optimizers::command::capture::MAX_CAPTURE_BYTES / (1024 * 1024);
    bytes.extend_from_slice(
        format!(
            "\n{} command output exceeded snip's {mib} MiB capture cap — showing the \
             prefix; re-run the command to see the full output.]\n",
            crate::domain::HEADER_PREFIX,
        )
        .as_bytes(),
    );
    bytes
}

/// Run a pure post-capture optimization under `catch_unwind`: on panic, log to
/// stderr and return `fallback()` so an already-executed command's output is
/// preserved verbatim. The fallback is lazy, so the happy path pays nothing.
fn guarded_opt<T>(f: impl FnOnce() -> T, fallback: impl FnOnce() -> T) -> T {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).unwrap_or_else(|_| {
        eprintln!("[snip exec] panic during optimization — passthrough");
        fallback()
    })
}

/// Remove every occurrence of the marker token from raw (non-UTF-8) output.
fn strip_token(buf: &[u8], token: &str) -> Vec<u8> {
    let needle = token.as_bytes();
    let mut out = Vec::with_capacity(buf.len());
    let mut i = 0;
    while i < buf.len() {
        if buf[i..].starts_with(needle) {
            i += needle.len();
        } else {
            out.push(buf[i]);
            i += 1;
        }
    }
    out
}

#[cfg(test)]
#[path = "../../../tests/unit/optimizers/command/exec.tests.rs"]
mod tests;
