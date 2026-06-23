//! NET token-savings accounting: a `SQLite` event store + the `gain` report.
//!
//! The wedge is **net honesty** — gross input saved minus induced cost (spilled
//! output the model had to re-read). Writes go through a detached recorder (off
//! the hot path — see [`tracking`]); `gain`/`status` read the store.

pub mod db;
pub mod event;
pub mod pricing;
pub mod recorder;
pub mod summary;
pub mod tracking;

pub use event::{Kind, StatEvent};
pub use summary::Summary;
pub use tracking::Tracker;
