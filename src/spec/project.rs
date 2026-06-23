//! The `Project` transform: keep a chosen subset of each record's fields.
//!
//! Whitespace-splits each record and re-emits the fields at the requested 0-based
//! indices, joined by a separator — the regex-free way to reshape columnar output
//! (e.g. `ps`/`df` columns) without a bespoke Rust `ParseFormat`. A record with
//! none of the requested columns is kept verbatim, so it never destroys a line.

/// Reproject each record to the whitespace-split fields at `cols` (0-based, in
/// output order), joined by `sep`.
#[must_use]
pub(crate) fn project(records: Vec<String>, cols: &[usize], sep: &str) -> Vec<String> {
    records
        .into_iter()
        .map(|record| {
            let joined = {
                let fields: Vec<&str> = record.split_whitespace().collect();
                let kept: Vec<&str> = cols
                    .iter()
                    .filter_map(|&i| fields.get(i).copied())
                    .collect();
                if kept.is_empty() {
                    None
                } else {
                    Some(kept.join(sep))
                }
            };
            joined.unwrap_or(record)
        })
        .collect()
}

#[cfg(test)]
#[path = "../../tests/unit/spec/project.tests.rs"]
mod tests;
