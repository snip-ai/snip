//! `jest --json` → failed assertions + a pass/fail tally.

use serde_json::{Map, Value};

/// Compact `jest --json` to failed assertions plus a pass/fail/total tally.
///
/// jest emits a single JSON object; each `testResults[].assertionResults[]`
/// failure becomes `file: title — first failure line`, and a suite that fails to
/// load (suite `status` "failed" with no assertions) is surfaced via its
/// `message`. A clean run collapses to just the tally. Non-object/non-JSON input
/// is kept verbatim so output is never dropped. No regex.
#[must_use]
pub fn jest_json(records: &[String]) -> Vec<String> {
    let joined = records.join("\n");
    let Ok(Value::Object(root)) = serde_json::from_str::<Value>(joined.trim()) else {
        return records.to_vec();
    };
    let mut out = vec![tally(&root)];
    for suite in root
        .get("testResults")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let file = suite.get("name").and_then(Value::as_str).unwrap_or("?");
        let failures: Vec<&Value> = suite
            .get("assertionResults")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|a| a.get("status").and_then(Value::as_str) == Some("failed"))
            .collect();
        if failures.is_empty() && suite.get("status").and_then(Value::as_str) == Some("failed") {
            let message = first_line(suite.get("message").and_then(Value::as_str).unwrap_or(""));
            out.push(join_msg(file, "<suite failed>", message));
        }
        for assertion in failures {
            out.push(render_failure(file, assertion));
        }
    }
    out
}

/// Build the `jest: F failed, P passed, T total[, N pending]` tally line.
fn tally(root: &Map<String, Value>) -> String {
    let count = |key: &str| root.get(key).and_then(Value::as_u64).unwrap_or(0);
    let pending = count("numPendingTests");
    let suffix = if pending > 0 {
        format!(", {pending} pending")
    } else {
        String::new()
    };
    format!(
        "jest: {} failed, {} passed, {} total{suffix}",
        count("numFailedTests"),
        count("numPassedTests"),
        count("numTotalTests"),
    )
}

/// Render one failed assertion as `file: title — first failure line`.
fn render_failure(file: &str, assertion: &Value) -> String {
    let title = assertion
        .get("fullName")
        .or_else(|| assertion.get("title"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let message = assertion
        .get("failureMessages")
        .and_then(Value::as_array)
        .and_then(|m| m.first())
        .and_then(Value::as_str)
        .unwrap_or("");
    join_msg(file, title, first_line(message))
}

/// `file: title`, or `file: title — message` when a message is present.
fn join_msg(file: &str, title: &str, message: &str) -> String {
    if message.is_empty() {
        format!("{file}: {title}")
    } else {
        format!("{file}: {title} — {message}")
    }
}

/// The first non-empty trimmed line of `text` (jest pads messages with blanks).
fn first_line(text: &str) -> &str {
    text.lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("")
}
