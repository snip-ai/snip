//! Whitespace-delimited table → drop constant columns + a `[const …]` note.
//!
//! A space-aligned table (e.g. `kubectl get`, `docker ps`) often repeats the
//! same value down a column. This drops every column whose value is identical
//! across all data rows, emitting a `[const COL=val …]` note (in ascending
//! column order) then the reduced header + rows. Ragged rows (a token count that
//! differs from the header), a blank data row, or nothing droppable → the input
//! is kept verbatim, so a non-table or space-bearing table is never mangled. No
//! regex.

/// Collapse constant columns of a whitespace-delimited table; see the module doc.
#[must_use]
pub fn table_collapse(records: &[String]) -> Vec<String> {
    if records.len() < 2 {
        return records.to_vec();
    }
    let header: Vec<&str> = records[0].split_whitespace().collect();
    let ncol = header.len();
    if ncol == 0 {
        return records.to_vec();
    }
    let mut rows: Vec<Vec<&str>> = Vec::with_capacity(records.len() - 1);
    for row in &records[1..] {
        let cells: Vec<&str> = row.split_whitespace().collect();
        if cells.len() != ncol {
            return records.to_vec();
        }
        rows.push(cells);
    }
    let keep: Vec<bool> = (0..ncol)
        .map(|c| !rows.iter().all(|r| r[c] == rows[0][c]))
        .collect();
    let kept: Vec<usize> = (0..ncol).filter(|&c| keep[c]).collect();
    let dropped: Vec<usize> = (0..ncol).filter(|&c| !keep[c]).collect();
    if dropped.is_empty() || kept.is_empty() {
        return records.to_vec();
    }
    let mut out = Vec::with_capacity(records.len());
    out.push(build_note(&header, &rows[0], &dropped));
    out.push(join_cols(&header, &kept));
    for row in &rows {
        out.push(join_cols(row, &kept));
    }
    out
}

/// `[const COL=val …]` for each dropped column, in ascending column order.
fn build_note(header: &[&str], first_row: &[&str], dropped: &[usize]) -> String {
    let mut note = String::from("[const");
    for &c in dropped {
        note.push(' ');
        note.push_str(header[c]);
        note.push('=');
        note.push_str(first_row[c]);
    }
    note.push(']');
    note
}

/// Join the cells at the `kept` column indices with single spaces.
fn join_cols(cells: &[&str], kept: &[usize]) -> String {
    kept.iter().map(|&c| cells[c]).collect::<Vec<_>>().join(" ")
}
