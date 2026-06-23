//! `snip bash-route` — the `PreToolUse`/Bash hook.
//!
//! Segments the command line and, when at least one segment is a recognized
//! command (and nothing is unsafe to wrap), rewrites `tool_input.command` to
//! `snip exec -- <base64>` so the runtime can optimize each segment's output.
//! Otherwise it passes through (empty stdout). Always exits 0.

use std::io::{Read, Write};

use serde_json::Value;

use crate::config::Config;
use crate::domain::Outcome;
use crate::engine::OutcomeSerializer;
use crate::optimizers::command::{CommandSpecs, Plan, b64, recognition};

/// Run the bash-route hook, degrading every error/panic to a pass-through.
///
/// In production always succeeds (exit-0 invariant). Strict debug mode
/// ([`crate::panic_guard::strict`]) surfaces the failure as a non-zero exit.
///
/// # Errors
/// Only under `SNIP_DEBUG` (strict mode); otherwise never.
pub fn run() -> anyhow::Result<()> {
    crate::panic_guard::guarded("bash-route", reroute)
}

fn reroute() -> anyhow::Result<()> {
    let cfg = Config::load();
    // The per-optimizer switch (and, transitively, the master switch): when
    // `command` is disabled, never rewrite — the Bash output passes through
    // unchanged, matching what `status` reports and the contract Dispatcher
    // enforces for every other surface.
    if !cfg.optimizer_enabled("command") {
        return Ok(());
    }
    let mut raw = String::new();
    std::io::stdin().read_to_string(&mut raw)?;
    if raw.trim().is_empty() {
        return Ok(());
    }
    let hook: Value = serde_json::from_str(&raw)?;
    let Some(input) = hook.get("tool_input") else {
        return Ok(());
    };
    let Some(command) = input.get("command").and_then(Value::as_str) else {
        return Ok(());
    };
    // Never wrap a backgrounded command: `snip exec` runs it synchronously and
    // captures its output, which would defeat `run_in_background` and risk hanging
    // on a streaming process (hook-protocol.md: never wrap backgrounded commands).
    if is_backgrounded(input) {
        return Ok(());
    }
    // Anti-recursion: never re-wrap our own `snip exec` invocation.
    if is_wrapped(command) {
        return Ok(());
    }
    // Decide whether to wrap. With auto-detect on (the default), any wrappable
    // line is rewritten so `exec` can sniff unrecognized output — so the spec
    // catalog need not be parsed here at all (it is parsed once, in `exec`).
    // Only when auto-detect is off do we need the catalog to gate on a recognized
    // command, avoiding overhead when nothing matches.
    let should_wrap = if cfg.autodetect_for("command").enabled {
        Plan::wrappable(command)
    } else {
        let specs = CommandSpecs::load(&cfg);
        Plan::build(command, &specs).is_some_and(|plan| plan.has_recognized())
    };
    if !should_wrap {
        return Ok(());
    }
    let session = hook.get("session_id").and_then(Value::as_str);
    let Some(rewritten) = rewrite_command(command, session) else {
        return Ok(());
    };
    let mut updated = input.clone();
    if let Some(obj) = updated.as_object_mut() {
        obj.insert("command".to_owned(), Value::String(rewritten));
    }
    if let Some(out) = OutcomeSerializer::serialize(&hook, Outcome::FixInput(updated)) {
        let mut writer = std::io::BufWriter::new(std::io::stdout().lock());
        serde_json::to_writer(&mut writer, &out)?;
        writer.flush()?;
    }
    Ok(())
}

/// Whether `tool_input` marks the command as backgrounded. snip must never wrap a
/// backgrounded/streaming command — `snip exec` would run it synchronously and
/// block on its output — so such a command passes through unchanged.
fn is_backgrounded(input: &Value) -> bool {
    input.get("run_in_background").and_then(Value::as_bool) == Some(true)
}

/// Whether `command` is already a `snip exec` wrapper (or we're inside one).
fn is_wrapped(command: &str) -> bool {
    std::env::var_os("SNIP_WRAPPED").is_some()
        || matches!(recognition::parse(command), Some((argv0, Some(sub))) if argv0 == "snip" && sub == "exec")
}

/// Build the rewrite: `[SNIP_SESSION=<id> ] "<snip>" exec -- <base64(command)>`,
/// using the running binary's absolute path (forward-slashed so Git Bash accepts
/// it on Windows). The session id is threaded through so `exec`'s spills land in
/// this session's cache (cleared at `PreCompact`), not a shared no-session bucket.
fn rewrite_command(command: &str, session: Option<&str>) -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let exe = exe.to_string_lossy().replace('\\', "/");
    let prefix = session
        .filter(|s| is_safe_session(s))
        .map_or_else(String::new, |s| format!("SNIP_SESSION={s} "));
    Some(format!(
        "{prefix}\"{exe}\" exec -- {}",
        b64::encode(command.as_bytes())
    ))
}

/// Whether a session id is safe to pass unquoted in a POSIX env assignment
/// (`[A-Za-z0-9-]+` — the shape Claude Code session ids take). Anything else is
/// dropped so it can never be a shell-injection vector.
fn is_safe_session(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

#[cfg(test)]
#[path = "../../../tests/unit/optimizers/command/bash_route.tests.rs"]
mod tests;
