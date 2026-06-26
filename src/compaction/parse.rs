//! Wall-clock-bounded tree-sitter parse for the model-blocking Read path.
//!
//! snip's promise is to optimize *every* file, including large ones — so the
//! parse deadline **scales with input size** rather than capping at a flat
//! `< 15 ms`. The budget is a *deadline*, not a fixed cost: a file that parses
//! quickly returns immediately (small/medium files keep their sub-millisecond
//! speed), while a large file is granted proportionally more time so it still
//! gets compacted instead of being passed through unoptimized. A generous
//! [`PARSE_BUDGET_CEILING`] remains as an anti-hang backstop so a pathological
//! input can never freeze the tool call; only then does [`parse_bounded`] abandon
//! the parse and return `None` (the caller passes the file through unchanged —
//! always safe).

use std::ops::ControlFlow;
use std::time::{Duration, Instant};

use tree_sitter::{ParseOptions, ParseState, Parser, Tree};

/// Minimum parse deadline. Small files parse far faster than this; the floor only
/// keeps tiny inputs from being starved by the size-scaled budget.
const PARSE_BUDGET_FLOOR: Duration = Duration::from_millis(10);
/// Parse time granted per MiB of source. Sized with ample headroom over the
/// slowest observed real-world parse throughput, so realistic files always finish
/// within budget and get optimized regardless of size.
const PARSE_BUDGET_PER_MIB: u64 = 400;
/// Absolute ceiling — the anti-hang backstop for degenerate input. A single parse
/// can never block longer than this, independent of file size or grammar.
const PARSE_BUDGET_CEILING: Duration = Duration::from_secs(2);

/// Size-scaled wall-clock deadline for parsing `len` bytes of source.
fn parse_budget(len: usize) -> Duration {
    const BYTES_PER_MIB: u64 = 1024 * 1024;
    let scaled_ms = (len as u64).saturating_mul(PARSE_BUDGET_PER_MIB) / BYTES_PER_MIB;
    Duration::from_millis(scaled_ms).clamp(PARSE_BUDGET_FLOOR, PARSE_BUDGET_CEILING)
}

/// Parse `source` with `parser`, abandoning past the size-scaled [`parse_budget`].
///
/// Returns `None` on a parse failure **or** a budget timeout — both mean "leave
/// the file uncompacted", which is always safe.
#[must_use]
pub fn parse_bounded(parser: &mut Parser, source: &str) -> Option<Tree> {
    let deadline = Instant::now() + parse_budget(source.len());
    let mut over_budget = |_: &ParseState| {
        if Instant::now() >= deadline {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    };
    let options = ParseOptions::new().progress_callback(&mut over_budget);
    let mut read = |byte: usize, _| source.as_bytes().get(byte..).unwrap_or(&[]);
    parser.parse_with_options(&mut read, None, Some(options))
}
