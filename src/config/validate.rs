//! Config-load resilience: never let one bad field silently wipe the whole file.
//!
//! A single malformed `specs[]` entry used to fail the entire document parse, so
//! [`Config::load`](super::Config::load) reverted to defaults — silently dropping
//! the master switch, `secret_safe`, overflow tuning, and every valid spec. Here
//! the spec list deserializes leniently (a bad spec is skipped, not fatal) and a
//! genuinely unparseable document is reported to stderr instead of vanishing.

use serde::{Deserialize, Deserializer};
use serde_json::Value;

use super::Config;
use crate::spec::OptimizerSpec;

/// Deserialize `specs[]` leniently: a malformed entry is skipped (with a stderr
/// note) rather than failing the whole config — so a typo in one spec can never
/// silently discard the user's master switch, `secret_safe`, or other specs.
pub(super) fn de_specs_lenient<'de, D>(deserializer: D) -> Result<Vec<OptimizerSpec>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = Vec::<Value>::deserialize(deserializer)?;
    let mut specs = Vec::with_capacity(raw.len());
    for value in raw {
        match serde_json::from_value::<OptimizerSpec>(value) {
            Ok(spec) => specs.push(spec),
            Err(e) => eprintln!("[snip] skipping invalid config spec: {e}"),
        }
    }
    Ok(specs)
}

/// Parse the user config text, or default — but **report** a fatal parse error to
/// stderr instead of silently reverting (the lenient spec list above means this
/// now fires only on a genuinely broken document, e.g. a bad `overflow` field).
pub(super) fn parse_user_config(raw: Option<&str>) -> Config {
    let Some(text) = raw else {
        return Config::default();
    };
    match serde_json::from_str(text) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("[snip] config is invalid and was ignored (using defaults): {e}");
            Config::default()
        }
    }
}

/// Human-readable problems with the on-disk config `raw` (empty ⇒ valid). Backs
/// `snip config validate`: surfaces a bad non-spec field and any spec that would
/// never fire, which `load` now tolerates silently.
#[must_use]
pub(super) fn diagnostics(raw: Option<&str>) -> Vec<String> {
    let mut problems = Vec::new();
    let Some(text) = raw else {
        return problems;
    };
    let value: Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(e) => {
            problems.push(format!("config is not valid JSON: {e}"));
            return problems;
        }
    };
    if let Some(specs) = value.get("specs").and_then(Value::as_array) {
        for (i, sv) in specs.iter().enumerate() {
            match serde_json::from_value::<OptimizerSpec>(sv.clone()) {
                Ok(spec) => {
                    if let Err(reason) = spec.validate() {
                        problems.push(format!(
                            "spec[{i}] `{}` would never run: {reason}",
                            spec.name
                        ));
                    }
                }
                Err(e) => problems.push(format!("spec[{i}] is invalid: {e}")),
            }
        }
    }
    // Typed whole-document parse (lenient on specs) — surfaces a bad non-spec
    // field that `load` would otherwise reset to its default.
    if let Err(e) = serde_json::from_str::<Config>(text) {
        problems.push(format!("config has an invalid field: {e}"));
    }
    problems
}
