// SNIP_SECRET_MARKER: this comment must never reach the model. The read
// optimizer strips it before Claude Code feeds the file to the conversation,
// so a test asserting its absence proves the PostToolUse rewrite was applied.
//
// Several comment lines here guarantee the compacted view is unambiguously
// smaller than the original, so the no-inflation guard does not pass it through.

/// Adds two integers — kept (code survives compaction).
fn add(a: i32, b: i32) -> i32 {
    // SNIP_SECRET_MARKER inline note, also stripped
    a + b
}

fn main() {
    let total = add(2, 3);
    println!("{total}");
}
