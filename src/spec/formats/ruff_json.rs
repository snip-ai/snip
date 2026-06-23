//! `ruff check --output-format=json` → one compact line per violation.

use serde_json::Value;

/// Compact `ruff check --output-format=json` to one line per violation.
///
/// Ruff emits a flat JSON array of diagnostics; each becomes
/// `file:row:col: code message`. A clean run collapses to `ruff: 0 problems`.
/// Non-array input (failed flag injection, a crash, plain text) is kept verbatim
/// so output is never dropped. No regex.
#[must_use]
pub fn ruff_json(records: &[String]) -> Vec<String> {
    let joined = records.join("\n");
    let Ok(Value::Array(items)) = serde_json::from_str::<Value>(joined.trim()) else {
        return records.to_vec();
    };
    let mut out: Vec<String> = items.iter().map(render).collect();
    if out.is_empty() {
        out.push("ruff: 0 problems".to_owned());
    }
    out
}

/// Format one ruff diagnostic as `file:row:col: code message`.
fn render(item: &Value) -> String {
    let file = item.get("filename").and_then(Value::as_str).unwrap_or("?");
    let location = item.get("location");
    let row = location
        .and_then(|l| l.get("row"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let col = location
        .and_then(|l| l.get("column"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let code = item.get("code").and_then(Value::as_str).unwrap_or("-");
    let text = item.get("message").and_then(Value::as_str).unwrap_or("");
    format!("{file}:{row}:{col}: {code} {text}")
}
