//! The `TestRollup` transform: collapse runs of passing-test lines.

use crate::spec::StrPred;

/// Collapse each consecutive run of `pred`-matching passing-test lines to
/// `… (N passed)`.
///
/// A lone match (run of 1) stays verbatim so it never inflates; failures and the
/// summary line (which don't match `pred`) pass through in place. Regex-free.
pub(crate) fn test_rollup(records: Vec<String>, pred: &StrPred) -> Vec<String> {
    let mut out = Vec::with_capacity(records.len());
    let mut run: Vec<String> = Vec::new();
    for record in records {
        if pred.matches(&record) {
            run.push(record);
        } else {
            flush_pass_run(&mut out, &mut run);
            out.push(record);
        }
    }
    flush_pass_run(&mut out, &mut run);
    out
}

/// Emit the pending passing-run: nothing for 0, the line verbatim for 1, else a
/// single `… (N passed)` marker.
fn flush_pass_run(out: &mut Vec<String>, run: &mut Vec<String>) {
    match run.len() {
        0 => {}
        1 => out.append(run),
        n => {
            out.push(format!("… ({n} passed)"));
            run.clear();
        }
    }
}
