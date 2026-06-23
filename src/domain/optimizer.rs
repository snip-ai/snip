//! The [`Optimizer`] port — the Rust escape hatch behind the declarative engine.

use super::{HookCtx, Outcome, Surface};

/// An optimizer attaches to one or more surfaces and transforms tool I/O.
///
/// [`Optimizer::matches`] must be cheap and allocation-free; [`Optimizer::apply`]
/// must not panic. The dispatcher additionally wraps the whole call in
/// `catch_unwind` and treats any `Err` as [`Outcome::PassThrough`].
pub trait Optimizer: Send + Sync {
    /// Stable identifier — config key, stats column, and header tag.
    fn name(&self) -> &str;
    /// The surfaces this optimizer attaches to (used to index the registry).
    fn surfaces(&self) -> &[Surface];
    /// Cheap predicate: does this optimizer apply to `ctx`?
    fn matches(&self, ctx: &HookCtx) -> bool;
    /// Produce an [`Outcome`]; any `Err` is treated as [`Outcome::PassThrough`].
    ///
    /// Edit/Write round-tripping flows through `apply` too: the `read` optimizer
    /// returns [`Outcome::FixInput`] (corrected `old_string`) on Edit and
    /// [`Outcome::Ask`] on a risky Write.
    ///
    /// # Errors
    /// Returns an error if the optimizer cannot produce a view; the dispatcher
    /// then falls back to the original output.
    fn apply(&self, ctx: &HookCtx) -> anyhow::Result<Outcome>;
}
