//! `snip config` (get/set/list/reset) and the enable/disable master-switch backends.
//!
//! These back the `/snip config`, `/snip enable`, and `/snip disable` slash-commands.
//! Settings round-trip the on-disk config file (not the env-overlaid view) so a
//! `set` never accidentally bakes in a `SNIP_*` override.

use anyhow::{Context, bail};

use crate::config::Config;
use crate::spec::OptimizerSpec;

/// Handle `snip config <args>`: `list` (default), `get <path>`, `set <path> <value>`, `reset`.
///
/// # Errors
/// Returns an error on an unknown subcommand, a missing argument, an invalid
/// setting for the given path, or a write failure.
pub fn run(args: &[String]) -> anyhow::Result<()> {
    match args.first().map(String::as_str) {
        None | Some("list") => list(),
        Some("get") => get(args.get(1).context("usage: config get <path>")?),
        Some("set") => set(
            args.get(1).context("usage: config set <path> <value>")?,
            args.get(2).context("usage: config set <path> <value>")?,
        ),
        Some("reset") => reset(),
        Some("spec") => spec_cmd(&args[1..]),
        Some("validate") => validate(),
        Some(other) => bail!("unknown config command: {other}"),
    }
}

/// Handle `snip config spec <add <json>|rm <name>>` — manage user specs.
fn spec_cmd(args: &[String]) -> anyhow::Result<()> {
    match args.first().map(String::as_str) {
        Some("add") => spec_add(args.get(1).context("usage: config spec add <json>")?),
        Some("rm") => spec_rm(args.get(1).context("usage: config spec rm <name>")?),
        _ => bail!("usage: config spec <add <json>|rm <name>>"),
    }
}

/// Add (or replace, by `name`) a user spec from its JSON.
fn spec_add(json: &str) -> anyhow::Result<()> {
    let spec: OptimizerSpec = serde_json::from_str(json).context("invalid spec JSON")?;
    if let Err(reason) = spec.validate() {
        bail!("spec `{}` would never run: {reason}", spec.name);
    }
    let name = spec.name.clone();
    let mut cfg = Config::load_raw();
    cfg.specs.retain(|s| s.name != name);
    cfg.specs.push(spec);
    cfg.save()?;
    println!("added spec {name}");
    Ok(())
}

/// Remove the user spec named `name`.
fn spec_rm(name: &str) -> anyhow::Result<()> {
    let mut cfg = Config::load_raw();
    let before = cfg.specs.len();
    cfg.specs.retain(|s| s.name != name);
    if cfg.specs.len() == before {
        bail!("no user spec named {name}");
    }
    cfg.save()?;
    println!("removed spec {name}");
    Ok(())
}

/// Set the master switch (`snip enable` / `snip disable`).
///
/// # Errors
/// Returns an error if the config file cannot be written.
pub fn set_enabled(enabled: bool) -> anyhow::Result<()> {
    let mut cfg = Config::load_raw();
    cfg.master_enabled = enabled;
    cfg.save()?;
    println!("snip {}", if enabled { "enabled" } else { "disabled" });
    Ok(())
}

fn list() -> anyhow::Result<()> {
    let value = serde_json::to_value(Config::load_raw())?;
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

fn get(path: &str) -> anyhow::Result<()> {
    let value = serde_json::to_value(Config::load_raw())?;
    match navigate(&value, path) {
        Some(found) => println!("{found}"),
        None => println!("(unset)"),
    }
    Ok(())
}

fn set(path: &str, raw: &str) -> anyhow::Result<()> {
    let mut value = serde_json::to_value(Config::load_raw())?;
    assign(&mut value, path, parse_scalar(raw));
    let cfg: Config =
        serde_json::from_value(value).context("invalid value for this config path")?;
    // A typo'd top-level path is silently dropped by `from_value`; reject it
    // rather than report a misleading success for a setting that never applied.
    if navigate(&serde_json::to_value(&cfg)?, path).is_none() {
        bail!("unknown config path: {path}");
    }
    cfg.save()?;
    println!("set {path} = {raw}");
    Ok(())
}

fn reset() -> anyhow::Result<()> {
    Config::default().save()?;
    println!("config reset to defaults");
    Ok(())
}

/// Report any problems with the on-disk config (`snip config validate`): a bad
/// field `load` would silently default, or a spec that could never fire.
fn validate() -> anyhow::Result<()> {
    let problems = Config::diagnostics();
    if problems.is_empty() {
        println!("config OK");
        return Ok(());
    }
    for problem in &problems {
        println!("• {problem}");
    }
    bail!("{} config problem(s) found", problems.len());
}

/// Walk a dotted path through a JSON object, returning the value if present.
fn navigate<'a>(root: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let mut cur = root;
    for key in path.split('.') {
        cur = cur.get(key)?;
    }
    Some(cur)
}

/// Set a dotted path to `value`, creating intermediate objects as needed.
fn assign(root: &mut serde_json::Value, path: &str, value: serde_json::Value) {
    let keys: Vec<&str> = path.split('.').collect();
    let Some((last, parents)) = keys.split_last() else {
        return;
    };
    let mut cur = root;
    for key in parents {
        if !cur.get(*key).is_some_and(serde_json::Value::is_object) {
            cur[*key] = serde_json::json!({});
        }
        let Some(next) = cur.get_mut(*key) else {
            return;
        };
        cur = next;
    }
    cur[*last] = value;
}

/// Parse a CLI scalar into JSON: `bool`, then unsigned int, then float, else string.
fn parse_scalar(s: &str) -> serde_json::Value {
    if let Ok(b) = s.parse::<bool>() {
        return serde_json::Value::Bool(b);
    }
    if let Ok(n) = s.parse::<u64>() {
        return serde_json::Value::Number(n.into());
    }
    if let Ok(f) = s.parse::<f64>()
        && let Some(num) = serde_json::Number::from_f64(f)
    {
        return serde_json::Value::Number(num);
    }
    serde_json::Value::String(s.to_owned())
}

#[cfg(test)]
#[path = "../../tests/unit/commands/config_cmd.tests.rs"]
mod tests;
