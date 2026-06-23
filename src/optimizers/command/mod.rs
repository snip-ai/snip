//! The `command` optimizer (Bash surface).
//!
//! A POSIX segmenter, unit recognition, and the sentinel-based rewrite/exec plan,
//! plus the `bash-route` `PreToolUse` hook and the `exec` runtime. Recognized units
//! are optimized through the declarative [`crate::optimizers::SpecOptimizer`] (the
//! per-command behavior is data — see `spec/builtin/specs/`); only this
//! segmentation/exec plumbing is Rust.

pub mod assemble;
pub mod autodetect;
pub mod b64;
pub mod bash_route;
pub mod capture;
pub mod command_specs;
pub mod exec;
pub mod op;
pub mod plan;
pub mod recognition;
pub mod segment;
pub mod segmenter;
pub mod unit;

pub use command_specs::CommandSpecs;
pub use op::Op;
pub use plan::Plan;
pub use segment::Segment;
pub use segmenter::Segmenter;
pub use unit::Unit;
