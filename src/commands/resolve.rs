//! `snip resolve <file>` — map a compacted-view `old_string` back to file bytes.
//!
//! The live recovery path for Claude Code versions that validate `old_string`
//! against the real file *before* `PreToolUse` hooks run (a mismatch aborts the
//! Edit before `edit-fix` runs). The model pipes the failing `old_string` to
//! stdin and retries the Edit with this command's stdout. The compacted-read
//! guidance header points here.

use std::io::{Read, Write};

use anyhow::{Context, bail};

use crate::optimizers::read::correct::correct_old_string;

/// Read an `old_string` from stdin and print the matching verbatim text of the
/// real `file` (already-matching text is echoed unchanged).
///
/// # Errors
/// Returns an error when stdin or the file can't be read, or when no confident
/// match is found (so the model knows to re-Read and copy more context).
pub fn run(file: &str) -> anyhow::Result<()> {
    let mut needle = String::new();
    std::io::stdin()
        .read_to_string(&mut needle)
        .context("reading old_string from stdin")?;
    // A heredoc/pipe appends a trailing newline that was never part of old_string.
    let needle = needle
        .strip_suffix('\n')
        .map_or(needle.as_str(), |s| s.strip_suffix('\r').unwrap_or(s));

    let raw = std::fs::read_to_string(file).with_context(|| format!("reading {file}"))?;
    // A UTF-8 BOM is invisible in the view and never part of an old_string.
    let content = raw.strip_prefix('\u{FEFF}').unwrap_or(&raw);
    let mode = crate::config::Config::load().mode_for("read");

    let output = if content.contains(needle) {
        needle.to_owned()
    } else if let Some(corrected) = correct_old_string(file, content, needle, mode) {
        corrected
    } else {
        bail!(
            "no confident match for this old_string in {file} — re-Read the file and \
             copy more surrounding lines into old_string"
        );
    };

    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    out.write_all(output.as_bytes())
        .context("writing resolved text")?;
    out.flush().context("flushing stdout")
}
