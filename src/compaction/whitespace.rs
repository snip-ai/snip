//! Whitespace normalization for the non-single-line compaction path.
//!
//! Strips trailing whitespace, collapses runs of blank lines to one, and — for
//! indent-based languages (Python) — collapses single-statement blocks onto their
//! header line (`if x:\n    y` → `if x: y`). Single-lining for brace languages
//! lives in [`crate::compaction::single_line`].

/// Normalize whitespace in `src` (medium/high on a non-single-line-safe language).
#[must_use]
pub fn compact_whitespace(src: &[u8], indent_based: bool) -> Vec<u8> {
    let text = std::str::from_utf8(src).unwrap_or("");
    let lines: Vec<&str> = text.lines().collect();
    if indent_based {
        let compacted = compact_python_lines(&lines);
        build_normalized(compacted.iter().map(String::as_str))
    } else {
        build_normalized(lines.into_iter().map(str::trim_end))
    }
}

/// Build the normalized byte output: trailing-ws-trimmed lines, blank runs
/// collapsed to one, leading/trailing blanks dropped, with a final newline.
fn build_normalized<'a>(lines: impl Iterator<Item = &'a str>) -> Vec<u8> {
    let mut result: Vec<&str> = Vec::new();
    let mut was_blank = false;
    for line in lines {
        let stripped = line.trim_end();
        if stripped.is_empty() {
            if !was_blank {
                result.push("");
                was_blank = true;
            }
        } else {
            result.push(stripped);
            was_blank = false;
        }
    }
    while result.first().is_some_and(|l| l.is_empty()) {
        result.remove(0);
    }
    while result.last().is_some_and(|l| l.is_empty()) {
        result.pop();
    }
    let mut out = result.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    out.into_bytes()
}

/// Collapse single-statement Python blocks onto their header line. Only a header
/// ending in `:` followed by exactly one non-empty indented statement merges.
fn compact_python_lines(lines: &[&str]) -> Vec<String> {
    let mut result: Vec<String> = Vec::with_capacity(lines.len());
    let mut i = 0usize;
    while i < lines.len() {
        let line = lines[i];
        let rstripped = line.trim_end();
        if rstripped.trim().is_empty() {
            result.push(String::new());
            i += 1;
            continue;
        }
        if rstripped.ends_with(':') && i + 1 < lines.len() {
            let curr_indent = line.len() - line.trim_start().len();
            let next_line = lines[i + 1];
            if !next_line.trim().is_empty() {
                let next_indent = next_line.len() - next_line.trim_start().len();
                if next_indent > curr_indent {
                    let mut block_end = i + 1;
                    while block_end < lines.len() {
                        let bl = lines[block_end];
                        if !bl.trim().is_empty() && bl.len() - bl.trim_start().len() <= curr_indent
                        {
                            break;
                        }
                        block_end += 1;
                    }
                    let statements: Vec<&str> = lines[i + 1..block_end]
                        .iter()
                        .map(|l| l.trim())
                        .filter(|l| !l.is_empty())
                        .collect();
                    if statements.len() == 1 {
                        result.push(format!("{rstripped} {}", statements[0]));
                        i = block_end;
                        continue;
                    }
                }
            }
        }
        result.push(rstripped.to_owned());
        i += 1;
    }
    result
}

#[cfg(test)]
#[path = "../../tests/unit/compaction/whitespace.tests.rs"]
mod tests;
