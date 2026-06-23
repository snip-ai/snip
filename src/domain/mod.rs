//! The domain model shared by every hook entry point.
//!
//! The dispatcher normalizes each tool invocation into a [`HookCtx`]; an
//! [`Optimizer`] returns an [`Outcome`]; the engine serializes it back into the
//! correct `hookSpecificOutput` JSON. Optimizers never touch the wire format.
//! This layer is dependency-light (serde + [`crate::config`]) and inward-facing.

pub mod hook_ctx;
pub mod optimizer;
pub mod outcome;
pub mod surface;

pub use hook_ctx::HookCtx;
pub use optimizer::Optimizer;
pub use outcome::{HEADER_PREFIX, Outcome};
pub use surface::Surface;
