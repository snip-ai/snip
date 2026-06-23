//! `cargo --message-format=json` → compact compiler diagnostics (with help/notes).

use serde_json::Value;

/// Diagnostic stream → compact compiler diagnostics, one per line.
///
/// `level[code] file:line:col: message`, followed by one indented line per
/// actionable `help`/`note` child (the fix the model needs). Progress/artifact
/// objects are dropped; any non-JSON line is kept verbatim. rustc's full
/// `rendered` field (source snippets + carets) is intentionally not emitted —
/// too large for the token budget; the children carry the actionable signal.
#[must_use]
pub fn cargo_json(records: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for line in records {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(trimmed) {
            Ok(v) if v.get("reason").and_then(Value::as_str) == Some("compiler-message") => {
                out.extend(compiler_message(&v));
            }
            Ok(_) => {}                       // artifact / build-finished noise → drop
            Err(_) => out.push(line.clone()), // not JSON → keep verbatim
        }
    }
    out
}

/// Format one `compiler-message`: a `level[code] file:line:col: message` header
/// line, then one indented line per actionable `help`/`note` child. Empty when the
/// value carries no `message`.
fn compiler_message(v: &Value) -> Vec<String> {
    let Some(msg) = v.get("message") else {
        return Vec::new();
    };
    let level = msg.get("level").and_then(Value::as_str).unwrap_or("note");
    let text = msg.get("message").and_then(Value::as_str).unwrap_or("");
    let code = msg
        .get("code")
        .and_then(|c| c.get("code"))
        .and_then(Value::as_str)
        .map_or_else(String::new, |c| format!("[{c}]"));
    let loc = primary_span(msg).map_or_else(String::new, |s| {
        let file = s.get("file_name").and_then(Value::as_str).unwrap_or("?");
        let line = s.get("line_start").and_then(Value::as_u64).unwrap_or(0);
        let col = s.get("column_start").and_then(Value::as_u64).unwrap_or(0);
        format!("{file}:{line}:{col}: ")
    });
    let mut lines = vec![format!("{level}{code} {loc}{text}")];
    lines.extend(
        msg.get("children")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(child_line),
    );
    lines
}

/// One actionable child as an indented `  level: message` line. `None` for a child
/// that is neither `help` nor `note`, or that carries no message (rustc emits
/// `children` for both fixes and pure decoration).
fn child_line(child: &Value) -> Option<String> {
    let level = child.get("level").and_then(Value::as_str).unwrap_or("");
    if !matches!(level, "help" | "note") {
        return None;
    }
    let text = child.get("message").and_then(Value::as_str).unwrap_or("");
    (!text.is_empty()).then(|| format!("  {level}: {text}"))
}

/// The primary span (or the first) of a compiler message, if any.
fn primary_span(msg: &Value) -> Option<&Value> {
    let spans = msg.get("spans").and_then(Value::as_array)?;
    spans
        .iter()
        .find(|s| s.get("is_primary").and_then(Value::as_bool) == Some(true))
        .or_else(|| spans.first())
}
