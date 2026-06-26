//! Unit tests for the `lifecycle` banner module.

use assert2::check;
use tempfile::tempdir;

use super::*;

#[test]
fn parse_reads_installed() {
    // Arrange
    let line = "installed 0.7.0";

    // Act
    let event = Lifecycle::parse(line);

    // Assert
    check!(event == Some(Lifecycle::Installed("0.7.0".to_owned())));
}

#[test]
fn parse_reads_updated_from_to() {
    // Arrange
    let line = "updated 0.6.0 0.7.0";

    // Act
    let event = Lifecycle::parse(line);

    // Assert
    check!(
        event
            == Some(Lifecycle::Updated {
                from: "0.6.0".to_owned(),
                to: "0.7.0".to_owned(),
            })
    );
}

#[test]
fn parse_reads_the_failure_states() {
    // Arrange + Act + Assert
    check!(Lifecycle::parse("download-failed") == Some(Lifecycle::DownloadFailed));
    check!(Lifecycle::parse("unsupported-platform") == Some(Lifecycle::UnsupportedPlatform));
}

#[test]
fn parse_rejects_malformed_lines() {
    // Arrange
    let cases = [
        "",
        "bogus",
        "installed",                 // missing version
        "installed 1.0.0 extra",     // trailing junk
        "updated 1.0.0",             // missing `to`
        "download-failed something", // unexpected arg
    ];

    // Act + Assert
    for line in cases {
        assert!(Lifecycle::parse(line).is_none(), "should reject: {line:?}");
    }
}

#[test]
fn message_names_the_installed_version() {
    // Arrange
    let event = Lifecycle::Installed("0.7.0".to_owned());

    // Act
    let message = event.message();

    // Assert
    check!(message.contains("0.7.0"));
    check!(message.starts_with("snip:"));
}

#[test]
fn message_shows_the_update_transition() {
    // Arrange
    let event = Lifecycle::Updated {
        from: "0.6.0".to_owned(),
        to: "0.7.0".to_owned(),
    };

    // Act
    let message = event.message();

    // Assert
    check!(message.contains("0.6.0"));
    check!(message.contains("0.7.0"));
}

#[test]
fn banner_json_is_a_user_only_session_start_message() {
    // Arrange
    let event = Lifecycle::Installed("1.2.3".to_owned());

    // Act
    let json = event.banner_json();
    assert2::assert!(let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json));

    // Assert
    check!(parsed["systemMessage"].as_str() == Some(event.message().as_str()));
    check!(parsed["hookSpecificOutput"]["hookEventName"] == "SessionStart");
    // `additionalContext` would reach the model — it must never appear.
    check!(!json.contains("additionalContext"));
}

#[test]
fn consume_reads_and_deletes_the_sentinel() {
    // Arrange
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join(".lifecycle"), "updated 0.6.0 0.7.0\n").expect("write sentinel");

    // Act
    let event = Lifecycle::consume(dir.path());

    // Assert
    check!(
        event
            == Some(Lifecycle::Updated {
                from: "0.6.0".to_owned(),
                to: "0.7.0".to_owned(),
            })
    );
    check!(!dir.path().join(".lifecycle").exists());
}

#[test]
fn consume_returns_none_when_absent() {
    // Arrange
    let dir = tempdir().expect("tempdir");

    // Act
    let event = Lifecycle::consume(dir.path());

    // Assert
    check!(event.is_none());
}

#[test]
fn consume_discards_a_malformed_sentinel() {
    // Arrange
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join(".lifecycle"), "garbage line\n").expect("write sentinel");

    // Act
    let event = Lifecycle::consume(dir.path());

    // Assert
    check!(event.is_none());
    check!(!dir.path().join(".lifecycle").exists());
}
