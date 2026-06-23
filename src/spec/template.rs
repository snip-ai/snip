//! The `Template` transform: re-emit each record through a compact template.

/// Re-emit each record through `each`, replacing the first `{}` with the record.
pub(crate) fn template(records: Vec<String>, each: &str) -> Vec<String> {
    records
        .into_iter()
        .map(|r| each.replacen("{}", &r, 1))
        .collect()
}
