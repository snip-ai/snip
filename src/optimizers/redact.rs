//! Redaction service: heuristic secret detection + masking for `secret_safe`.
//!
//! When `secret_safe` is on, each output line is scanned before any lossy
//! transform; a line carrying a secret-shaped token is masked so the credential
//! is redacted (and never reaches a spill file). Detection is regex-free — known
//! token prefixes, sensitive assignment keys, and a Shannon-entropy test on long
//! opaque tokens — and is defense-in-depth, not a guarantee. Masking never
//! lengthens a token, so it cannot inflate the output.
//!
//! Two entry points cover the surfaces differently:
//! - [`mask_records`] masks secret lines in place on the **output** surfaces
//!   (Grep/Glob via [`super::SpecOptimizer`], and the command auto-detect fallback),
//!   before the body is ever spilled.
//! - [`any_secret`] gates the **read** surface: a secret-bearing source file is
//!   passed through uncompacted (no compacted view, spill, or dedupe-cache write is
//!   produced for it), since masking source bytes would break Edit-safety.
//!
//! So with `secret_safe` on, a credential never reaches a spill file on any surface.

/// Token prefixes that mark a whitespace token as a credential.
const PREFIXES: &[&str] = &[
    "AKIA",
    "ASIA",
    "ghp_",
    "gho_",
    "ghu_",
    "ghs_",
    "github_pat_",
    "sk-ant-",
    "sk-",
    "xoxb-",
    "xoxp-",
    "xoxo-",
    "xoxa-",
    "AIza",
    "ya29.",
    "glpat-",
    "eyJ",
];

/// Substrings (case-insensitive) that mark an assignment key as sensitive.
const KEY_NEEDLES: &[&str] = &[
    "secret",
    "token",
    "password",
    "passwd",
    "apikey",
    "api_key",
    "access_key",
    "private_key",
];

/// Minimum length for a token to be weighed by the entropy test.
const ENTROPY_MIN_LEN: usize = 24;

/// Whether any line of `text` carries a secret — the read-surface gate.
///
/// The read optimizer passes a secret-bearing source file through uncompacted
/// (rather than masking, which would break Edit-safety on the masked bytes), so a
/// credential in source never reaches a compacted view, a spill, or the dedupe cache.
pub(crate) fn any_secret(text: &str) -> bool {
    text.lines().any(contains_secret)
}

/// Whether `line` carries a secret: a PEM marker, a sensitive assignment, or a
/// prefix/high-entropy token.
fn contains_secret(line: &str) -> bool {
    line.contains("PRIVATE KEY-----")
        || assignment_value(line).is_some()
        || line.split_whitespace().any(is_secret_token)
}

/// Mask a sensitive assignment value AND every secret token in `line`.
fn mask_line(line: &str) -> String {
    let mut out = assignment_value(line).map_or_else(
        || line.to_owned(),
        |idx| {
            let (head, value) = line.split_at(idx);
            format!("{head}{}", mask_first_token(value))
        },
    );
    // Scan the (possibly assignment-masked) line so a prefix/entropy token
    // elsewhere on the line is not left exposed. Masks contain no secret shape,
    // so they are never re-masked.
    let snapshot = out.clone();
    for token in snapshot.split_whitespace() {
        if is_secret_token(token) {
            out = out.replace(token, &mask_token(token));
        }
    }
    out
}

/// Mask every secret-bearing record in place; returns whether any was masked.
///
/// The shared entry both [`crate::optimizers::SpecOptimizer`] and the command
/// auto-detect fallback use, so `secret_safe` masking is applied identically on
/// every surface (masking never lengthens a record).
pub(crate) fn mask_records(records: &mut [String]) -> bool {
    let mut masked = false;
    for record in &mut *records {
        if contains_secret(record) {
            *record = mask_line(record);
            masked = true;
        }
    }
    masked
}

/// Byte index where a sensitive assignment's value starts, if the key matches.
fn assignment_value(line: &str) -> Option<usize> {
    let sep = line.find(['=', ':'])?;
    let key = line[..sep].to_ascii_lowercase();
    KEY_NEEDLES
        .iter()
        .any(|needle| key.contains(*needle))
        .then_some(sep + 1)
}

/// Mask the first whitespace token of `value`, preserving leading/trailing text.
fn mask_first_token(value: &str) -> String {
    let trimmed = value.trim_start();
    let lead = &value[..value.len() - trimmed.len()];
    let end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
    format!("{lead}{}{}", mask_token(&trimmed[..end]), &trimmed[end..])
}

/// Replace a token with a redacted form that is never longer than the original.
fn mask_token(token: &str) -> String {
    let len = token.chars().count();
    if len <= 6 {
        "*".repeat(len)
    } else {
        let head: String = token.chars().take(3).collect();
        format!("{head}***")
    }
}

/// Whether `token` is a credential by prefix or by entropy.
fn is_secret_token(token: &str) -> bool {
    PREFIXES.iter().any(|p| token.starts_with(*p)) || entropy_secretish(token)
}

/// Whether `token` is a long opaque high-entropy string (but not a hex/UUID id).
fn entropy_secretish(token: &str) -> bool {
    token.len() >= ENTROPY_MIN_LEN
        && token.bytes().all(is_secret_charset)
        && !is_id_like(token)
        && shannon_entropy(token) >= 3.5
}

/// Whether `byte` belongs to the base64/url/hex token charset.
const fn is_secret_charset(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'_' | b'=' | b'-')
}

/// Whether `token` is an all-hex/dash identifier — a git SHA, MD5, UUID, or
/// container id — which are pervasive in tool output and not credentials.
fn is_id_like(token: &str) -> bool {
    token.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-')
}

/// Shannon entropy of `token` in bits per character.
#[allow(clippy::cast_precision_loss)] // token lengths are tiny; f64 is exact here
fn shannon_entropy(token: &str) -> f64 {
    let mut counts = [0u32; 256];
    for byte in token.bytes() {
        counts[byte as usize] += 1;
    }
    let len = token.len() as f64;
    counts.iter().filter(|&&c| c > 0).fold(0.0, |acc, &c| {
        let p = f64::from(c) / len;
        p.mul_add(-p.log2(), acc)
    })
}

#[cfg(test)]
#[path = "../../tests/unit/optimizers/redact.tests.rs"]
mod tests;
