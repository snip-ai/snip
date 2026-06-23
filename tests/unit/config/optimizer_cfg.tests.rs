//! Unit tests for [`OptimizerCfg`], in AAA form. Compiled into `snip_lib` via a
//! `#[path]` include in `src/config/optimizer_cfg.rs`.

use assert2::check;

use super::OptimizerCfg;

#[test]
fn default_is_enabled_with_no_overflow_override() {
    // Arrange + Act
    let cfg = OptimizerCfg::default();

    // Assert
    check!(cfg.enabled);
    check!(cfg.overflow.is_none());
}

#[test]
fn deserializes_a_per_optimizer_overflow_override() {
    // Arrange + Act
    let cfg: OptimizerCfg =
        serde_json::from_str(r#"{"enabled":true,"overflow":{"max_tokens":1234}}"#).unwrap();

    // Assert
    assert2::assert!(let Some(overflow) = cfg.overflow);
    check!(overflow.max_tokens == 1234);
}
