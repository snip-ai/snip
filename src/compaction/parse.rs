//! Wall-clock-bounded tree-sitter parse for the model-blocking Read path.
//!
//! A multi-hundred-KB source can take tens to ~180 ms to parse — well over the
//! `< 15 ms` hot-path budget. [`parse_bounded`] caps the parse with a deadline
//! progress callback: past [`PARSE_BUDGET`] the parse is abandoned and `None` is
//! returned, so the caller passes the file through unchanged instead of stalling
//! the tool call. Latency is then bounded by time, independent of file size or
//! grammar — the byte cap upstream is only a cheap pre-filter, not the guarantee.

use std::ops::ControlFlow;
use std::time::{Duration, Instant};

use tree_sitter::{ParseOptions, ParseState, Parser, Tree};

/// Hard wall-clock budget for a single parse on the Read hot path. Below the
/// `< 15 ms` whole-hook budget, leaving headroom for config load + the DFS scan.
const PARSE_BUDGET: Duration = Duration::from_millis(10);

/// Parse `source` with `parser`, abandoning past [`PARSE_BUDGET`].
///
/// Returns `None` on a parse failure **or** a budget timeout — both mean "leave
/// the file uncompacted", which is always safe.
#[must_use]
pub fn parse_bounded(parser: &mut Parser, source: &str) -> Option<Tree> {
    let deadline = Instant::now() + PARSE_BUDGET;
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
