//! Normalized per-event context handed to an optimizer.

use super::Surface;
use crate::config::Config;

/// Normalized context for one tool invocation, handed to an optimizer.
pub struct HookCtx<'a> {
    /// The surface this event came from.
    pub surface: Surface,
    /// Session id, when Claude Code provides one.
    pub session_id: Option<&'a str>,
    /// Transcript path, when provided.
    pub transcript_path: Option<&'a str>,
    /// The extracted `tool_input` object (`file_path` / `command` / …).
    pub input: &'a serde_json::Value,
    /// Tool output text for Post surfaces; `None` for Pre surfaces.
    pub output: Option<&'a str>,
    /// The loaded configuration.
    pub cfg: &'a Config,
}
