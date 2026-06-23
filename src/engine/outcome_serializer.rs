//! Serialize an [`Outcome`] into the `hookSpecificOutput` JSON for one event.

use serde_json::Value;

use crate::domain::Outcome;
use crate::engine::tool_response::ToolResponse;

/// Maps an [`Outcome`] to its Claude Code hook JSON. The variant — not the
/// surface — picks the wire shape.
pub struct OutcomeSerializer;

impl OutcomeSerializer {
    /// Serialize `outcome` for `hook`, or `None` for pass-through (empty stdout
    /// ⇒ Claude Code keeps the original).
    #[must_use]
    pub fn serialize(hook: &Value, outcome: Outcome) -> Option<Value> {
        let out = match outcome {
            Outcome::PassThrough => return None,
            Outcome::Rewrite { header, body, .. } => serde_json::json!({
                "hookSpecificOutput": {
                    "hookEventName": "PostToolUse",
                    "updatedToolOutput":
                        ToolResponse::new(hook.get("tool_response")).rewrite(&header, &body),
                }
            }),
            Outcome::FixInput(input) => serde_json::json!({
                "hookSpecificOutput": {
                    "hookEventName": "PreToolUse",
                    "updatedInput": input,
                }
            }),
            Outcome::Ask { reason } => serde_json::json!({
                "hookSpecificOutput": {
                    "hookEventName": "PreToolUse",
                    "permissionDecision": "ask",
                    "permissionDecisionReason": reason,
                }
            }),
        };
        Some(out)
    }
}

#[cfg(test)]
#[path = "../../tests/unit/engine/outcome_serializer.tests.rs"]
mod tests;
