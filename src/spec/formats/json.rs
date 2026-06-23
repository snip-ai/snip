//! JSON encoders: minify a document, or re-encode a uniform array of objects as
//! a compact header + value-rows table (TOON).

use serde_json::Value;

/// Minify a pretty-printed JSON document to its whitespace-free form.
///
/// Non-JSON input (or already-minified) is returned verbatim — the no-inflation
/// guard drops the result if it isn't smaller.
#[must_use]
pub fn json_minify(records: &[String]) -> Vec<String> {
    let joined = records.join("\n");
    serde_json::from_str::<Value>(&joined)
        .map_or_else(|_| records.to_vec(), |v| vec![v.to_string()])
}

/// A uniform JSON array of objects → a `key1,key2,…` header + one value row per
/// object (TOON). Ragged records, non-arrays, or arrays under two elements are
/// returned verbatim.
#[must_use]
pub fn json_array_table(records: &[String]) -> Vec<String> {
    let joined = records.join("\n");
    match serde_json::from_str::<Value>(&joined) {
        Ok(Value::Array(arr)) => array_to_table(&arr).unwrap_or_else(|| records.to_vec()),
        _ => records.to_vec(),
    }
}

/// Line-delimited JSON objects (NDJSON) → the same TOON table as a JSON array.
///
/// Many tools stream one JSON object per line; joined, that is not a single JSON
/// document, so `json_minify` can't help. Parse each non-empty line and, if every
/// one is an object sharing a uniform key set (≥2 rows), emit the columnar table;
/// otherwise return verbatim (a single multi-line object falls back to minify).
#[must_use]
pub fn ndjson_table(records: &[String]) -> Vec<String> {
    let mut arr = Vec::new();
    for line in records {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(trimmed) {
            Ok(v) if v.is_object() => arr.push(v),
            _ => return records.to_vec(), // not line-delimited objects → verbatim
        }
    }
    array_to_table(&arr).unwrap_or_else(|| records.to_vec())
}

/// Core: a uniform array of objects → header + value rows. `None` when not a
/// uniform ≥2-object array (the caller then keeps the input verbatim).
fn array_to_table(arr: &[Value]) -> Option<Vec<String>> {
    let keys = uniform_keys(arr)?;
    let mut out = Vec::with_capacity(arr.len() + 1);
    out.push(keys.join(","));
    for item in arr {
        if let Value::Object(obj) = item {
            let row: Vec<String> = keys.iter().map(|k| cell(obj.get(k))).collect();
            out.push(row.join(","));
        }
    }
    Some(out)
}

/// The shared key set if `arr` is ≥2 objects with identical keys, else `None`.
fn uniform_keys(arr: &[Value]) -> Option<Vec<String>> {
    if arr.len() < 2 {
        return None;
    }
    let Some(Value::Object(first)) = arr.first() else {
        return None;
    };
    let keys: Vec<String> = first.keys().cloned().collect();
    for item in arr {
        let Value::Object(obj) = item else {
            return None;
        };
        if obj.len() != keys.len() || !keys.iter().all(|k| obj.contains_key(k)) {
            return None;
        }
    }
    Some(keys)
}

/// Render one cell value, quoting if it contains the delimiter/newline/quote.
fn cell(v: Option<&Value>) -> String {
    match v {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => quote_if_needed(s),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Number(n)) => n.to_string(),
        Some(other) => quote_if_needed(&other.to_string()), // nested obj/arr → JSON
    }
}

/// CSV-style quoting: wrap in quotes (doubling inner quotes) when ambiguous.
fn quote_if_needed(s: &str) -> String {
    if s.contains([',', '\n', '"']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_owned()
    }
}
