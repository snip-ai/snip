//! Maintenance hook entry points that bypass the master switch.
//!
//! Tool hooks funnel through [`crate::engine::Dispatcher`]; these two
//! ([`session_reset`], [`update_check`]) run regardless of the switch.

pub mod session_reset;
pub mod update_check;
