//! What the `Group` transform collapses consecutive records by.

use serde::{Deserialize, Serialize};

/// What the `Group` transform collapses by.
///
/// Replaces a repeated prefix with one header (`path:` / `dir:`) followed by
/// indented members. Grouping is **consecutive only** — it never reorders, so
/// it's safe on mtime-sorted glob output.
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupKey {
    /// Grep `path:line:content` lines — group by the file path.
    File,
    /// Glob paths — group by the parent directory.
    Dir,
    /// Grep output of unknown mode — group by file when a record carries a
    /// `:line:` segment (`content` mode), else by its parent directory (the bare
    /// path list of `files_with_matches`). Lets the one `grep`-surface spec serve
    /// both modes, which never mix within a single response.
    Auto,
}

impl GroupKey {
    /// Group `records` by this key. Singleton runs stay verbatim (grouping a lone
    /// line would inflate it); a record that doesn't parse passes through.
    #[must_use]
    pub fn group(self, records: Vec<String>) -> Vec<String> {
        match self {
            Self::File => group(records, split_file),
            Self::Dir => group(records, split_dir),
            Self::Auto => group(records, split_auto),
        }
    }
}

/// Split a grep line into `(key_end, _)`: the byte index of the `:` separating
/// the path from `line:content`. Robust to a Windows `C:` drive letter — the
/// path ends at the `:` that precedes `<digits>:`.
fn split_file(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b != b':' {
            continue;
        }
        let mut j = i + 1;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
        }
        if j > i + 1 && bytes.get(j) == Some(&b':') {
            return Some(i);
        }
    }
    None
}

/// Split a path into `(key_end, _)`: the byte index of the last `/` or `\`.
fn split_dir(line: &str) -> Option<usize> {
    line.rfind(['/', '\\'])
}

/// Split by file when a record looks like a grep `path:line:content` match, else
/// by directory. A `files_with_matches` path list has no `:line:` segment, so it
/// folds by directory; the `"Found N files"` header has neither separator and
/// passes through verbatim.
fn split_auto(line: &str) -> Option<usize> {
    split_file(line).or_else(|| split_dir(line))
}

/// Collapse consecutive records sharing `split`'s key into a header + members.
fn group<F: Fn(&str) -> Option<usize>>(records: Vec<String>, split: F) -> Vec<String> {
    let mut out = Vec::with_capacity(records.len());
    let mut run: Vec<String> = Vec::new();
    let mut idxs: Vec<usize> = Vec::new();
    let mut key: Option<String> = None;
    for rec in records {
        if let Some(i) = split(&rec) {
            if key.as_deref() != Some(&rec[..i]) {
                flush(&mut out, &mut run, &mut idxs, key.as_deref());
                key = Some(rec[..i].to_owned());
            }
            idxs.push(i);
            run.push(rec);
        } else {
            flush(&mut out, &mut run, &mut idxs, key.as_deref());
            key = None;
            out.push(rec);
        }
    }
    flush(&mut out, &mut run, &mut idxs, key.as_deref());
    out
}

/// Emit the accumulated run: one line verbatim, or a `key:` header + members.
fn flush(out: &mut Vec<String>, run: &mut Vec<String>, idxs: &mut Vec<usize>, key: Option<&str>) {
    if run.len() == 1 {
        out.push(run.remove(0));
    } else if run.len() > 1 {
        out.push(format!("{}:", key.unwrap_or("")));
        for (line, i) in run.drain(..).zip(idxs.drain(..)) {
            out.push(format!("  {}", &line[i + 1..]));
        }
    }
    run.clear();
    idxs.clear();
}

#[cfg(test)]
#[path = "../../tests/unit/spec/group_key.tests.rs"]
mod tests;
