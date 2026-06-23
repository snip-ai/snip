//! Variable-token masking for log fingerprinting — a closed char-class scan.
//!
//! Masks canonical UUIDs (`<uuid>`), digit runs (`<n>`) and long hex runs (`<x>`)
//! so log lines differing only in ids/numbers/timestamps collapse to one template.
//! No regex — see [`super::log_fold`], its only consumer.

use std::borrow::Cow;

use super::FingerprintCfg;

/// Mask variable tokens so lines differing only in ids/numbers/timestamps share
/// one template: canonical UUIDs → `<uuid>` (a pre-pass, since their short 4-hex
/// segments would otherwise survive the run scan), then digit runs → `<n>` and
/// long hex runs → `<x>`.
pub(crate) fn mask(line: &str, cfg: &FingerprintCfg) -> String {
    let pre = if cfg.mask_uuid && line.contains('-') {
        Cow::Owned(mask_uuids(line))
    } else {
        Cow::Borrowed(line)
    };
    let mut out = String::with_capacity(pre.len());
    let mut chars = pre.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c.is_ascii_alphanumeric() {
            let mut run = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_ascii_alphanumeric() {
                    run.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            out.push_str(&classify(&run, cfg));
        } else {
            out.push(c);
            chars.next();
        }
    }
    out
}

/// Replace every canonical `8-4-4-4-12` hex UUID in `line` with `<uuid>`.
fn mask_uuids(line: &str) -> String {
    let bytes = line.as_bytes();
    let mut out = String::with_capacity(line.len());
    let mut i = 0;
    while i < bytes.len() {
        if is_uuid_at(bytes, i) {
            out.push_str("<uuid>");
            i += UUID_LEN;
        } else if let Some(c) = line[i..].chars().next() {
            out.push(c);
            i += c.len_utf8();
        } else {
            break;
        }
    }
    out
}

/// Length of a canonical hyphenated UUID.
const UUID_LEN: usize = 36;
/// Hyphen offsets within the canonical UUID form (`8-4-4-4-12`).
const UUID_DASHES: [usize; 4] = [8, 13, 18, 23];

/// Whether `bytes` from `i` are a canonical UUID: hex digits with hyphens at the
/// `8-4-4-4-12` boundaries.
fn is_uuid_at(bytes: &[u8], i: usize) -> bool {
    if i + UUID_LEN > bytes.len() {
        return false;
    }
    bytes[i..i + UUID_LEN].iter().enumerate().all(|(off, &b)| {
        if UUID_DASHES.contains(&off) {
            b == b'-'
        } else {
            b.is_ascii_hexdigit()
        }
    })
}

/// Classify one alphanumeric run into a placeholder, or keep it verbatim.
fn classify(run: &str, cfg: &FingerprintCfg) -> String {
    let all_digits = run.bytes().all(|b| b.is_ascii_digit());
    let all_hex = run.bytes().all(|b| b.is_ascii_hexdigit());
    if cfg.mask_numbers && all_digits {
        "<n>".to_owned()
    } else if cfg.mask_hex && !all_digits && all_hex && run.len() >= 8 {
        "<x>".to_owned()
    } else {
        run.to_owned()
    }
}
