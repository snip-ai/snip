//! The `Squeeze` transform: collapse blank-line runs and trim trailing space.

/// Collapse runs of blank records to one and trim trailing whitespace per record.
pub(crate) fn squeeze(records: Vec<String>) -> Vec<String> {
    let mut out = Vec::with_capacity(records.len());
    let mut prev_blank = false;
    for record in records {
        let trimmed = record.trim_end().to_owned();
        let blank = trimmed.is_empty();
        if blank && prev_blank {
            continue;
        }
        prev_blank = blank;
        out.push(trimmed);
    }
    out
}
