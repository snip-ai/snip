//! The `tool_response` payload in every wire shape an optimizer must read.

use serde_json::Value;

/// A tool's `tool_response` JSON across its wire shapes.
///
/// The nested Read shape `{ "file": { "content", "numLines" } }`, the top-level
/// `{ "content": … }` (Grep, and the legacy form), the Glob shape
/// `{ "filenames": [ … ] }` (a path array, no content), and a bare string.
/// Owning this knowledge here keeps the wire format out of the optimizers.
pub struct ToolResponse<'a> {
    value: Option<&'a Value>,
}

impl<'a> ToolResponse<'a> {
    /// Wrap a `tool_response` value (or its absence).
    #[must_use]
    pub const fn new(value: Option<&'a Value>) -> Self {
        Self { value }
    }

    /// Extract the textual output, accepting every wire shape.
    ///
    /// Order matters: Grep carries both a top-level `content` (its match lines,
    /// what the model renders) and a `filenames` array, so `content` must win;
    /// Glob has only `filenames` (no content), which the model renders joined by
    /// newlines, so that is the fallback before a bare string.
    #[must_use]
    pub fn extract_text(&self) -> Option<String> {
        let resp = self.value?;
        if let Some(s) = resp.pointer("/file/content").and_then(Value::as_str) {
            return Some(s.to_owned());
        }
        if let Some(s) = resp.get("content").and_then(Value::as_str) {
            return Some(s.to_owned());
        }
        if let Some(list) = resp.get("filenames").and_then(Value::as_array) {
            return Some(join_filenames(list));
        }
        resp.as_str().map(str::to_owned)
    }

    /// Rebuild the response with `header + body` as its content, preserving the
    /// original wire shape so Claude Code's output schema still validates and the
    /// model renders the compacted view.
    #[must_use]
    pub fn rewrite(&self, header: &str, body: &str) -> Value {
        let combined = format!("{header}{body}");
        let num_lines = combined.lines().count();
        match self.value {
            Some(v) if v.pointer("/file/content").is_some() => {
                let mut v = v.clone();
                if let Some(obj) = v.get_mut("file").and_then(Value::as_object_mut) {
                    obj.insert("content".to_owned(), Value::String(combined));
                    obj.insert("numLines".to_owned(), Value::from(num_lines));
                }
                v
            }
            Some(v) if v.get("content").is_some() => {
                let mut v = v.clone();
                if let Some(obj) = v.as_object_mut() {
                    obj.insert("content".to_owned(), Value::String(combined));
                }
                v
            }
            // Glob: the model renders `filenames` joined by newlines, so write the
            // grouped view back there (one array entry per line) — not `content`,
            // which Claude Code ignores for this shape.
            Some(v) if v.get("filenames").and_then(Value::as_array).is_some() => {
                let mut v = v.clone();
                if let Some(obj) = v.as_object_mut() {
                    let lines: Vec<Value> = combined
                        .lines()
                        .map(|l| Value::String(l.to_owned()))
                        .collect();
                    obj.insert("filenames".to_owned(), Value::Array(lines));
                }
                v
            }
            _ => Value::String(combined),
        }
    }
}

/// Join a `filenames` array into the newline-delimited text the model sees.
fn join_filenames(list: &[Value]) -> String {
    list.iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
#[path = "../../tests/unit/engine/tool_response.tests.rs"]
mod tests;
