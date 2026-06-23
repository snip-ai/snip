//! The optional repo-local project config layer (`<cwd>/.snip/config.json`).
//!
//! Off by default — loaded only when the user config sets `allow_project_config`.
//! Trust model: a project layer may **tune** optimization (opt the repo out,
//! adjust per-optimizer settings / overflow / autodetect, add specs) but can never
//! **weaken** safety. `secret_safe` is OR'd (enable-only), `master_enabled` is
//! AND'd (a repo may turn snip off, never force it on against a global off), and
//! `allow_project_config` itself is not project-overridable — so an untrusted repo
//! can't disable redaction, re-enable a globally-disabled snip, or deepen its own
//! loading. Specs stay a closed, regex-free vocabulary, so they carry no RCE.

use serde_json::Value;

/// Overlay `<cwd>/.snip/config.json` onto `user` (a config JSON object), in place,
/// when the user opted in. No-op otherwise.
pub(super) fn apply(user: &mut Value) {
    if !opted_in(user) {
        return;
    }
    if let Some(project) = load() {
        merge(user, &project);
    }
}

/// Whether the user config opted into the project layer.
fn opted_in(user: &Value) -> bool {
    user.get("allow_project_config").and_then(Value::as_bool) == Some(true)
}

/// Read `<cwd>/.snip/config.json` as a JSON value, if present and well-formed.
fn load() -> Option<Value> {
    let path = std::env::current_dir()
        .ok()?
        .join(".snip")
        .join("config.json");
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// Merge the `project` object's present keys into `user`, enforcing the trust
/// floors. Keys absent from `project` leave the user value untouched.
fn merge(user: &mut Value, project: &Value) {
    let (Some(dst), Some(src)) = (user.as_object_mut(), project.as_object()) else {
        return;
    };
    for (key, val) in src {
        match key.as_str() {
            // Not project-overridable: a repo can't deepen its own loading.
            "allow_project_config" => {}
            // Security floor: enable-only — a repo can strengthen redaction, never disable it.
            "secret_safe" => {
                let on = bool_at(dst, "secret_safe") || val.as_bool().unwrap_or(false);
                dst.insert(key.clone(), Value::Bool(on));
            }
            // Quiet-only: a repo may opt out (`false`) but can't force snip on against a global off.
            "master_enabled" => {
                let on = bool_at(dst, "master_enabled") && val.as_bool().unwrap_or(true);
                dst.insert(key.clone(), Value::Bool(on));
            }
            // Per-name merge so a repo tunes/extends specific optimizers.
            "optimizers" => merge_objects(
                dst.entry(key.clone())
                    .or_insert(Value::Object(serde_json::Map::new())),
                val,
            ),
            // Extend coverage: project specs append to the user's (shadow-by-name resolves downstream).
            "specs" => append_array(
                dst.entry(key.clone()).or_insert(Value::Array(Vec::new())),
                val,
            ),
            // Tuning overrides (overflow, autodetect, …).
            _ => {
                dst.insert(key.clone(), val.clone());
            }
        }
    }
}

/// A boolean field's value in `obj`, defaulting to its config default: `true` for
/// `master_enabled`, `false` for `secret_safe` (matches `Config::default`).
fn bool_at(obj: &serde_json::Map<String, Value>, key: &str) -> bool {
    obj.get(key)
        .and_then(Value::as_bool)
        .unwrap_or(key == "master_enabled")
}

/// Shallow-merge `src` object entries into `dst` (project overrides user per key).
fn merge_objects(dst: &mut Value, src: &Value) {
    if let (Some(d), Some(s)) = (dst.as_object_mut(), src.as_object()) {
        for (k, v) in s {
            d.insert(k.clone(), v.clone());
        }
    }
}

/// Append `src` array elements to `dst`.
fn append_array(dst: &mut Value, src: &Value) {
    if let (Some(d), Some(s)) = (dst.as_array_mut(), src.as_array()) {
        d.extend(s.iter().cloned());
    }
}

#[cfg(test)]
#[path = "../../tests/unit/config/project.tests.rs"]
mod tests;
