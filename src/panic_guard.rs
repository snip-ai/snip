//! Run a hook body under `catch_unwind`, logging any error or panic to stderr.
//!
//! The exit-0 guard for the maintenance hooks (`session-reset`, `update-check`)
//! and `bash-route`, plus the [`crate::engine::Dispatcher`] tool surfaces, which
//! all funnel through it. Centralizing it keeps the invariant in one place instead
//! of hand-copied `catch_unwind` blocks. In strict (dev) mode — [`strict`] — a
//! caught error or panic is surfaced as a non-zero exit instead of swallowed, so
//! failures are visible while debugging locally.

use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};

/// Whether strict (dev) debug mode is on: `SNIP_DEBUG` set to `1`/`true`/`yes`/`on`.
///
/// Off by default and in every plugin-installed session, so the non-negotiable
/// exit-0 invariant holds for real users. When on, [`guarded`] re-raises a hook's
/// error or panic as a non-zero exit (message on stderr) so a developer sees the
/// failure immediately instead of a silent pass-through.
#[must_use]
pub fn strict() -> bool {
    std::env::var("SNIP_DEBUG").is_ok_and(|v| {
        matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

/// Run `body` under `catch_unwind`, logging any error or panic to stderr.
///
/// Both are swallowed (exit-0) in production; in strict mode ([`strict`]) they are
/// returned as `Err` so the process exits non-zero, with the panic's message.
///
/// # Errors
/// Only in strict mode (`SNIP_DEBUG`): the body's error, or a synthesized error
/// carrying a caught panic's message. In production it always returns `Ok(())`.
pub fn guarded(label: &str, body: impl FnOnce() -> anyhow::Result<()>) -> anyhow::Result<()> {
    let strict = strict();
    match catch_unwind(AssertUnwindSafe(body)) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => {
            eprintln!("[snip {label}] error: {e:#}");
            if strict { Err(e) } else { Ok(()) }
        }
        Err(panic) => {
            if strict {
                let msg = panic_message(panic.as_ref());
                eprintln!("[snip {label}] panic: {msg}");
                Err(anyhow::anyhow!("hook panicked: {msg}"))
            } else {
                eprintln!("[snip {label}] panic — passthrough");
                Ok(())
            }
        }
    }
}

/// Best-effort text of a caught panic payload (`&str` / `String`, else a marker).
fn panic_message(payload: &(dyn Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "non-string panic payload".to_owned())
}

#[cfg(test)]
#[path = "../tests/unit/panic_guard.tests.rs"]
mod tests;
