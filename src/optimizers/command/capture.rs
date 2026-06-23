//! Run a command via the real shell, capturing stdout under a hard timeout.
//!
//! The side-thread stdout drain prevents a full pipe from deadlocking the wait
//! loop, which enforces [`HARD_TIMEOUT`] and kills the child on expiry — a hung
//! command can never hang the hook.

use std::io::Read;
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use anyhow::Context;

/// Hard timeout for a wrapped command — below Claude Code's 120 s default so a
/// hung child can never hang the hook. On expiry the child is killed and the
/// partial output returned with exit code 124 (the `timeout(1)` convention).
const HARD_TIMEOUT: Duration = Duration::from_secs(110);
/// Exit code reported when [`HARD_TIMEOUT`] fires.
const TIMEOUT_CODE: i32 = 124;
/// First wait-loop poll delay. Starts small and backs off to [`MAX_POLL`] so a
/// fast command is detected almost immediately (no flat full-interval tail) while
/// a long one still polls cheaply.
const INITIAL_POLL: Duration = Duration::from_millis(1);
/// Upper bound the backing-off poll delay converges to.
const MAX_POLL: Duration = Duration::from_millis(20);
/// Max stdout bytes retained from a wrapped command. Past this the drain keeps
/// reading to EOF (so the child never blocks on a full pipe) but stops storing,
/// and the capture is flagged truncated — mirroring the Read surface's
/// `MAX_READ_BYTES` guard so a log-dumping command (`cat huge.log`, `journalctl`)
/// can't buffer multi-GB and OOM-kill the exec child. Claude Code still natively
/// spills this bounded prefix; the truncation flag lets the caller say it is a
/// prefix, not the whole output.
pub(super) const MAX_CAPTURE_BYTES: usize = 16 * 1024 * 1024;

/// Run `command` via `sh -c` with `SNIP_WRAPPED=1` (+ extra env), capturing
/// stdout and passing stderr/stdin through. Returns `(stdout bytes, exit code)`.
///
/// # Errors
/// Propagates a shell spawn failure.
pub(super) fn capture(
    command: &str,
    extra_env: &[(&str, &str)],
) -> anyhow::Result<(Vec<u8>, i32, bool)> {
    let mut cmd = Command::new("sh");
    cmd.arg("-c")
        .arg(command)
        .env("SNIP_WRAPPED", "1")
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    for (key, value) in extra_env {
        cmd.env(key, value);
    }
    let mut child = cmd.spawn().context("snip exec: failed to spawn `sh`")?;
    let reader = child
        .stdout
        .take()
        .map(|mut out| std::thread::spawn(move || drain_capped(&mut out, MAX_CAPTURE_BYTES)));
    let deadline = Instant::now() + HARD_TIMEOUT;
    let mut poll = INITIAL_POLL;
    let code = loop {
        match child.try_wait()? {
            Some(status) => break exit_code(status),
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                break TIMEOUT_CODE;
            }
            None => {
                std::thread::sleep(poll);
                poll = (poll * 2).min(MAX_POLL);
            }
        }
    };
    let (buf, truncated) =
        reader.map_or_else(|| (Vec::new(), false), |r| r.join().unwrap_or_default());
    Ok((buf, code, truncated))
}

/// Drain `out` to EOF, retaining at most `cap` bytes. Bytes past the cap are read
/// and discarded — so the child never blocks on a full pipe — and the returned flag
/// marks the buffer as a truncated prefix. Any read error ends the drain with the
/// bytes gathered so far (never empties an in-progress buffer). `cap` is a parameter
/// (not the constant directly) so the truncation behavior is cheaply unit-testable.
fn drain_capped(out: &mut impl Read, cap: usize) -> (Vec<u8>, bool) {
    let mut buf = Vec::new();
    // Heap-allocated (not a stack array) so the drain buffer stays off the thread stack.
    let mut scratch = vec![0u8; 64 * 1024];
    let mut truncated = false;
    loop {
        match out.read(&mut scratch) {
            Ok(0) => break,
            Ok(n) => {
                if buf.len() < cap {
                    let take = (cap - buf.len()).min(n);
                    buf.extend_from_slice(&scratch[..take]);
                    truncated |= take < n;
                } else {
                    truncated = true;
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => break,
        }
    }
    (buf, truncated)
}

/// The process exit code, mapping a signal death to `128 + signal` (Unix).
fn exit_code(status: ExitStatus) -> i32 {
    status.code().unwrap_or_else(|| signal_code(status))
}

#[cfg(unix)]
fn signal_code(status: ExitStatus) -> i32 {
    use std::os::unix::process::ExitStatusExt;
    status.signal().map_or(1, |s| 128 + s)
}

#[cfg(not(unix))]
#[allow(clippy::missing_const_for_fn, clippy::needless_pass_by_value)] // parity with Unix
fn signal_code(_status: ExitStatus) -> i32 {
    1
}

#[cfg(test)]
#[path = "../../../tests/unit/optimizers/command/capture.tests.rs"]
mod tests;
