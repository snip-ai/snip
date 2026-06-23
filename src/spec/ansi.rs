//! Strip ANSI escapes and carriage-return overwrites from records (lossless).

/// Remove ANSI CSI/OSC escape sequences and collapse `\r` progress overwrites
/// (keep the final segment) from each record — pure noise removal.
#[must_use]
pub(crate) fn strip_ansi(records: Vec<String>) -> Vec<String> {
    records.into_iter().map(|r| clean(&r)).collect()
}

/// Clean one line: drop a trailing CRLF `\r`, keep only the text after the last
/// in-line `\r` (a progress-bar overwrite), and strip escape sequences.
fn clean(line: &str) -> String {
    let line = line.strip_suffix('\r').unwrap_or(line);
    let line = line.rsplit('\r').next().unwrap_or(line);
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        if c != '\u{1b}' {
            out.push(c);
            continue;
        }
        // ESC: skip a CSI (`[ … <letter>`) or OSC (`] … BEL`) sequence.
        match chars.next() {
            Some('[') => {
                for d in chars.by_ref() {
                    if d.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            Some(']') => {
                for d in chars.by_ref() {
                    if d == '\u{7}' {
                        break;
                    }
                }
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
#[path = "../../tests/unit/spec/ansi.tests.rs"]
mod tests;
