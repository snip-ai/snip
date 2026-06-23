//! The decision an optimizer returns for one event.

/// The prefix every snip output header starts with.
///
/// The single source of the `[snip:` marker contract: producers emit it, and
/// `write-guard` detects a re-Write of snip's own output by it. (The two `read`
/// guidance consts inline the literal — a `const` can't interpolate this — and
/// are kept in sync.)
pub const HEADER_PREFIX: &str = "[snip:";

/// What an optimizer decided to do with an event.
pub enum Outcome {
    /// Leave the original untouched (empty stdout ⇒ Claude Code keeps it).
    PassThrough,
    /// Replace a Post surface's output with a compacted view. The dispatcher
    /// applies overflow and records stats afterwards.
    Rewrite {
        /// Guidance/stats header prepended to the body.
        header: String,
        /// The compacted body.
        body: String,
        /// Estimated tokens of the original output.
        original_tokens: usize,
        /// Estimated tokens of the rewritten output.
        new_tokens: usize,
    },
    /// Replace a Pre surface's entire `tool_input`.
    FixInput(serde_json::Value),
    /// Ask the user for permission (Pre/Write).
    Ask {
        /// Human-readable reason shown to the user.
        reason: String,
    },
}
