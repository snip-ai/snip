//! Layered configuration (built-in defaults → user file → optional project file →
//! env overrides), read once per hook. The hot path never touches `SQLite` — only
//! this tiny JSON.
//!
//! [`Config`] lives here (not in a `config/config.rs`, which would trip
//! `clippy::module_inception`); the per-section types are split out by concern.

mod accessors;
pub mod autodetect_cfg;
pub mod compact_mode;
pub mod optimizer_cfg;
mod project;
mod validate;

pub use autodetect_cfg::AutodetectCfg;
pub use compact_mode::CompactMode;
pub use optimizer_cfg::OptimizerCfg;

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::overflow::OverflowCfg;
use crate::spec::OptimizerSpec;

/// Top-level snip configuration.
#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    /// Master on/off switch. When `false`, tool hooks pass through unchanged.
    #[serde(default = "default_true")]
    pub master_enabled: bool,
    /// Per-optimizer settings, keyed by optimizer `name`.
    #[serde(default)]
    pub optimizers: HashMap<String, OptimizerCfg>,
    /// Global overflow/truncation defaults.
    #[serde(default)]
    pub overflow: OverflowCfg,
    /// User-defined specs, layered over the built-ins and shadowed by `name`.
    /// Deserialized leniently: one malformed spec is dropped (with a stderr note),
    /// never failing the whole config — see [`validate::de_specs_lenient`].
    #[serde(
        default,
        deserialize_with = "validate::de_specs_lenient",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub specs: Vec<OptimizerSpec>,
    /// Global opt-in secret-safe mode: protect secret-shaped lines from lossy
    /// transforms (and, with redaction enabled, mask them).
    #[serde(default)]
    pub secret_safe: bool,
    /// Global auto-detect defaults for unrecognized command output.
    #[serde(default)]
    pub autodetect: AutodetectCfg,
    /// Opt in to a repo-local project config layer (`<cwd>/.snip/config.json`),
    /// **off by default**. See [`project`] for the trust model (project config may
    /// tune optimization but can never weaken `secret_safe` or force snip on).
    #[serde(default)]
    pub allow_project_config: bool,
    /// The Claude model `gain` prices against (substring-matched: `opus`/`haiku`,
    /// else the mid tier). `None` ⇒ mid tier. Only affects the dollar estimate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            master_enabled: true,
            optimizers: HashMap::new(),
            overflow: OverflowCfg::default(),
            specs: Vec::new(),
            secret_safe: false,
            autodetect: AutodetectCfg::default(),
            allow_project_config: false,
            model: None,
        }
    }
}

impl Config {
    /// Load configuration: built-in defaults, overlaid by the user file (if any),
    /// the opt-in project file, then environment overrides. Never fails — a missing
    /// or malformed file falls back to the layer below.
    ///
    /// The user layer is parsed directly (the fast path); the project layer is
    /// reached only when the user opted in, and is adopted only if the merged
    /// config still deserializes — so a malformed `.snip` can never downgrade the
    /// user's settings.
    #[must_use]
    pub fn load() -> Self {
        let raw = config_path().and_then(|p| std::fs::read_to_string(p).ok());
        let mut cfg = validate::parse_user_config(raw.as_deref());
        if cfg.allow_project_config
            && let Some(text) = raw.as_deref()
            && let Ok(mut value) = serde_json::from_str::<Value>(text)
        {
            project::apply(&mut value);
            if let Ok(merged) = serde_json::from_value::<Self>(value) {
                cfg = merged;
            }
        }
        cfg.apply_env_overrides();
        cfg
    }

    /// Human-readable problems with the on-disk config (empty ⇒ valid). Backs
    /// `snip config validate`, so a silently-tolerated bad field or inert spec is
    /// discoverable rather than just dropped at load.
    #[must_use]
    pub fn diagnostics() -> Vec<String> {
        let raw = config_path().and_then(|p| std::fs::read_to_string(p).ok());
        validate::diagnostics(raw.as_deref())
    }

    fn apply_env_overrides(&mut self) {
        if let Ok(v) = std::env::var("SNIP_ENABLED") {
            self.master_enabled = parse_enabled(&v);
        }
    }

    /// Load only the on-disk config (no env overrides), defaulting on error.
    /// Used by the `config`/`enable`/`disable` backends, which round-trip the
    /// persisted file rather than the env-overlaid view.
    #[must_use]
    pub fn load_raw() -> Self {
        config_path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    /// Persist this config as pretty JSON at the config path, creating the dir.
    ///
    /// # Errors
    /// Returns an error if the path can't be resolved or the file can't be written.
    pub fn save(&self) -> anyhow::Result<()> {
        let path = config_path().context("could not resolve the snip config path")?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        // Atomic write: a torn write would make `load` silently reset to defaults.
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(self)?)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }
}

const fn default_true() -> bool {
    true
}

/// The config file path: `$SNIP_CONFIG_PATH`, else `<data_dir>/config.json`
/// (the shared root in [`crate::paths`], which honors `SNIP_HOME`).
fn config_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("SNIP_CONFIG_PATH") {
        return Some(PathBuf::from(p));
    }
    Some(crate::paths::data_dir()?.join("config.json"))
}

/// Parse a boolean-ish env value; anything but `0`/`false`/`no`/`off` is enabled.
fn parse_enabled(v: &str) -> bool {
    !matches!(
        v.trim().to_ascii_lowercase().as_str(),
        "0" | "false" | "no" | "off"
    )
}

#[cfg(test)]
#[path = "../../tests/unit/config/config.tests.rs"]
mod tests;
