//! Declarative optimizer specs — data, not code.
//!
//! A spec is deserialized from JSON whether it ships compiled-in ([`builtin`])
//! or comes from user config — one code path. The transform vocabulary is
//! intentionally closed (no regex, no scripting); the Rust escape hatch
//! ([`crate::domain::Optimizer`]) handles anything a spec cannot express.

pub mod ansi;
pub mod bind;
pub mod builtin;
pub mod dedupe;
pub mod diff_fold;
pub mod formats;
pub mod fp_window;
pub mod group_key;
pub mod log_fold;
pub mod log_mask;
pub mod optimizer_spec;
pub mod parse_format;
pub mod project;
pub mod rank_key;
pub mod squeeze;
pub mod stacktrace;
pub mod str_pred;
pub mod template;
pub mod test_rollup;
pub mod transform;
pub mod truncate;

pub use bind::Bind;
pub use diff_fold::DiffFoldCfg;
pub use fp_window::FpWindow;
pub use group_key::GroupKey;
pub use log_fold::FingerprintCfg;
pub use optimizer_spec::OptimizerSpec;
pub use parse_format::ParseFormat;
pub use rank_key::RankKey;
pub use stacktrace::StacktraceCfg;
pub use str_pred::StrPred;
pub use transform::Transform;
