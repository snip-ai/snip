//! The `Truncate` transform: keep the first `head` and last `tail` records.

/// Keep the first `head` + last `tail` records, eliding the middle with a marker.
pub(crate) fn truncate(records: Vec<String>, head: usize, tail: usize) -> Vec<String> {
    if records.len() <= head + tail + 1 {
        return records;
    }
    let elided = records.len() - head - tail;
    let mut out = Vec::with_capacity(head + tail + 1);
    out.extend_from_slice(&records[..head]);
    out.push(format!("… ({elided} lines elided)"));
    out.extend_from_slice(&records[records.len() - tail..]);
    out
}
