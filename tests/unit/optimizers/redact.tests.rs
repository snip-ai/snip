//! Unit tests for the redaction service, in AAA form. Compiled into `snip_lib`
//! via a `#[path]` include in `src/optimizers/redact.rs`.

use assert2::check;

use super::{contains_secret, mask_line};

#[test]
fn detects_known_token_prefixes() {
    // Act + Assert: one representative per prefix family
    check!(contains_secret("key AKIAIOSFODNN7EXAMPLE here"));
    check!(contains_secret("ghp_16C7e42F292c6912E7710c838347Ae178B4a"));
    check!(contains_secret("token sk-ant-api03-abcdefghijklmnop"));
    check!(contains_secret("slack xoxb-123456789012-abcdEFGH"));
    check!(contains_secret("g AIzaSyD-aBcDeFgHiJkLmNoPqRsTuVwXyZ012"));
    check!(contains_secret("gl glpat-ABCDEFGHIJKLMNOPQRST"));
}

#[test]
fn detects_pem_private_key_marker() {
    // Act + Assert: PEM headers contain spaces, so match on the whole line
    check!(contains_secret("-----BEGIN RSA PRIVATE KEY-----"));
}

#[test]
fn detects_a_high_entropy_token() {
    // Arrange: a 32-char mixed-charset opaque token
    let line = "value Xy9aQ2bV7zR4tW6mC1nB8kD0pL5sF3hJ done";

    // Act + Assert
    check!(contains_secret(line));
}

#[test]
fn detects_a_sensitive_assignment_value() {
    // Act + Assert: low-entropy value caught by the key, not the token shape
    check!(contains_secret("password: hunter2"));
    check!(contains_secret("API_KEY=abc123def"));
}

#[test]
fn ignores_non_secrets() {
    // Act + Assert: prose, a path, a short word, and a git SHA stay clean
    check!(!contains_secret(
        "The quick brown fox jumps over the lazy dog"
    ));
    check!(!contains_secret(
        "edit src/main.rs and node_modules/foo/bar.js"
    ));
    check!(!contains_secret("just a secret"));
    // hex/UUID identifiers (git SHA, MD5, container UUID) are not credentials
    check!(!contains_secret(
        "commit e7f3a9c2b1d4f6a8c0e2b4d6f8a0c2e4b6d8f0a2"
    ));
    check!(!contains_secret("md5 5d41402abc4b2a76b9719d911017c592"));
    check!(!contains_secret("id 550e8400-e29b-41d4-a716-446655440000"));
}

#[test]
fn masks_an_assignment_value_and_a_trailing_token() {
    // Arrange: a sensitive assignment AND a prefix token later on the same line —
    // both must be redacted (no early return)
    let line = "PASSWORD=secretval note ghp_16C7e42F292c6912E7710c838347Ae178";

    // Act
    let masked = mask_line(line);

    // Assert
    check!(!masked.contains("secretval"));
    check!(!masked.contains("ghp_16C7"));
    check!(masked.contains("ghp***"));
}

#[test]
fn masks_a_token_shorter_than_the_original() {
    // Arrange
    let line = "export TOKEN ghp_16C7e42F292c6912E7710c838347Ae178B4a";

    // Act
    let masked = mask_line(line);

    // Assert: the credential is redacted and the result never inflates
    check!(masked.contains("ghp***"));
    check!(!masked.contains("ghp_16C7"));
    check!(masked.len() <= line.len());
}

#[test]
fn masks_an_assignment_value_keeping_the_key() {
    // Arrange
    let line = "password: hunter2";

    // Act
    let masked = mask_line(line);

    // Assert: key preserved, value redacted, no inflation
    check!(masked.starts_with("password: "));
    check!(!masked.contains("hunter2"));
    check!(masked.len() <= line.len());
}

#[test]
fn masks_a_short_value_to_equal_length() {
    // Arrange: a 2-char value must mask to 2 chars (never inflate)
    let line = "password=hi";

    // Act
    let masked = mask_line(line);

    // Assert
    check!(masked == "password=**");
}
