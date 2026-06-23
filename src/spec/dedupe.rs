//! The `Dedupe` transform: collapse consecutive identical records.

/// Collapse runs of identical consecutive records into `"<record> (×N)"`.
pub(crate) fn dedupe(records: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(records.len());
    let mut iter = records.into_iter();
    let Some(mut prev) = iter.next() else {
        return out;
    };
    let mut count = 1usize;
    for record in iter {
        if record == prev {
            count += 1;
        } else {
            out.push(collapse(prev, count));
            prev = record;
            count = 1;
        }
    }
    out.push(collapse(prev, count));
    out
}

/// Render a record with its run count: bare for a singleton, else `"<record> (×N)"`.
fn collapse(record: String, count: usize) -> String {
    if count > 1 {
        format!("{record} (×{count})")
    } else {
        record
    }
}
