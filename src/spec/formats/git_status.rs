//! `git status --porcelain=v2 --branch` → the familiar `git status -s` short form.

/// Porcelain v2 → a branch line plus one `XY path` entry per change.
#[must_use]
pub fn git_status_v2(records: &[String]) -> Vec<String> {
    let mut branch = None;
    let mut ahead_behind = None;
    let mut entries = Vec::new();
    for line in records {
        if let Some(head) = line.strip_prefix("# branch.head ") {
            branch = Some(head.to_owned());
        } else if let Some(ab) = line.strip_prefix("# branch.ab ") {
            ahead_behind = Some(ab.to_owned());
        } else if line.starts_with("# ") {
            // other branch headers (oid, upstream) — dropped
        } else if let Some(entry) = changed_entry(line) {
            entries.push(entry);
        } else if let Some(path) = line.strip_prefix("? ") {
            entries.push(format!("?? {path}"));
        } else if let Some(path) = line.strip_prefix("! ") {
            entries.push(format!("!! {path}"));
        } else if !line.trim().is_empty() {
            entries.push(line.clone()); // not porcelain → keep verbatim, never drop
        }
    }
    let mut out = Vec::with_capacity(entries.len() + 1);
    if let Some(head) = branch {
        let suffix = ahead_behind.map(|ab| format!(" {ab}")).unwrap_or_default();
        out.push(format!("on {head}{suffix}"));
    }
    out.extend(entries);
    out
}

/// An ordinary (`1 `) or renamed (`2 `) changed entry → `XY path` (`XY old -> new`).
fn changed_entry(line: &str) -> Option<String> {
    if let Some(rest) = line.strip_prefix("1 ") {
        // <XY> <sub> <mH> <mI> <mW> <hH> <hI> <path>
        let mut fields = rest.splitn(8, ' ');
        let xy = fields.next()?;
        let path = fields.nth(6)?;
        Some(format!("{xy} {path}"))
    } else if let Some(rest) = line.strip_prefix("2 ") {
        // <XY> <sub> <mH> <mI> <mW> <hH> <hI> <Xscore> <path>\t<origPath>
        let mut fields = rest.splitn(9, ' ');
        let xy = fields.next()?;
        let tail = fields.nth(7)?;
        let (path, orig) = tail.split_once('\t').unwrap_or((tail, ""));
        Some(if orig.is_empty() {
            format!("{xy} {path}")
        } else {
            format!("{xy} {orig} -> {path}")
        })
    } else {
        None
    }
}
