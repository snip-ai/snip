//! `eslint --format=json` → one compact line per lint message.

use serde_json::Value;

/// Compact `eslint --format=json` output to one line per lint message.
///
/// `ESLint` emits a single JSON array of per-file results; each message becomes
/// `file:line:col: severity rule: message`. A clean run collapses to a one-line
/// notice. Non-array input (failed flag injection, a crash, plain text) is kept
/// verbatim so output is never dropped. No regex.
#[must_use]
pub fn eslint_json(records: &[String]) -> Vec<String> {
    let joined = records.join("\n");
    let Ok(Value::Array(files)) = serde_json::from_str::<Value>(joined.trim()) else {
        return records.to_vec();
    };
    let mut out = Vec::new();
    for file in &files {
        let path = file.get("filePath").and_then(Value::as_str).unwrap_or("?");
        let Some(messages) = file.get("messages").and_then(Value::as_array) else {
            continue;
        };
        for message in messages {
            out.push(render(path, message));
        }
    }
    if out.is_empty() {
        out.push("eslint: 0 problems".to_owned());
    }
    out
}

/// Format one `ESLint` message as `file:line:col: severity rule: message`.
///
/// The severity word (`error`/`warning`) lets a later `Rank` surface errors first.
fn render(path: &str, message: &Value) -> String {
    let line = message.get("line").and_then(Value::as_u64).unwrap_or(0);
    let col = message.get("column").and_then(Value::as_u64).unwrap_or(0);
    let severity = match message.get("severity").and_then(Value::as_u64) {
        Some(2) => "error",
        Some(1) => "warning",
        _ => "note",
    };
    let rule = message.get("ruleId").and_then(Value::as_str).unwrap_or("-");
    let text = message.get("message").and_then(Value::as_str).unwrap_or("");
    format!("{path}:{line}:{col}: {severity} {rule}: {text}")
}
