//! Unit tests for [`CompactMode`], in AAA form. Compiled into `snip_lib` via a
//! `#[path]` include in `src/config/compact_mode.rs`.

use assert2::check;

use super::CompactMode;

#[test]
fn deserializes_from_its_lowercase_name() {
    // Arrange + Act
    let medium: CompactMode = serde_json::from_str("\"medium\"").unwrap();
    let high: CompactMode = serde_json::from_str("\"high\"").unwrap();

    // Assert
    check!(medium == CompactMode::Medium);
    check!(high == CompactMode::High);
}

#[test]
fn default_is_soft_and_round_trips_its_name() {
    // Arrange + Act
    let default = CompactMode::default();

    // Assert
    check!(default == CompactMode::Soft);
    check!(default.as_str() == "soft");
}

#[test]
fn as_str_names_the_aggressive_modes() {
    // Act + Assert: the medium/high arms
    check!(CompactMode::Medium.as_str() == "medium");
    check!(CompactMode::High.as_str() == "high");
}
