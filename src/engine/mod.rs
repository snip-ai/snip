//! The dispatch engine: surface event → optimizer → hook JSON.
//!
//! [`Dispatcher`] is the shared entry for every tool hook; [`Registry`] indexes
//! optimizers by surface; [`ToolResponse`] and [`OutcomeSerializer`] own the
//! Claude Code wire format so optimizers never touch it.

pub mod dispatcher;
pub mod outcome_serializer;
pub mod registry;
pub mod tool_response;

pub use dispatcher::Dispatcher;
pub use outcome_serializer::OutcomeSerializer;
pub use registry::Registry;
pub use tool_response::ToolResponse;
