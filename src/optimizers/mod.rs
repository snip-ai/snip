//! The optimizer layer — every [`crate::domain::Optimizer`] implementation.
//!
//! Two kinds: the **declarative** adapter [`SpecOptimizer`] (runs any
//! [`crate::spec::OptimizerSpec`] — this is `search` and each recognized command
//! slice), and **Rust escape hatches** for what a spec can't express ([`read`],
//! AST code compaction). The Bash command runtime lives in [`command`]; the
//! cross-cutting `secret_safe` redaction service in [`redact`].

pub mod command;
pub mod read;
pub mod redact;
pub mod search;
pub mod spec_optimizer;

pub use spec_optimizer::SpecOptimizer;
