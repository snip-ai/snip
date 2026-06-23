//! `snip status` — version, the master switch, per-optimizer state, and the
//! running NET savings.

use crate::config::Config;
use crate::stats::Tracker;

/// The reference optimizers, always shown first in dispatch order. The search
/// surfaces carry one spec each (Grep / Glob), named per-surface so a user
/// override targets exactly one.
const REFERENCE: [&str; 4] = ["read", "search-grep", "search-glob", "command"];

/// Optimizer names to show: the three references plus any user-defined specs
/// (`config.specs[]`) that add a new name — so `status` reflects user specs.
fn optimizer_names(cfg: &Config) -> Vec<String> {
    let mut names: Vec<String> = REFERENCE.iter().map(|s| (*s).to_owned()).collect();
    for spec in &cfg.specs {
        if !names.contains(&spec.name) {
            names.push(spec.name.clone());
        }
    }
    names
}

/// Print snip's status: version, the master switch, each optimizer's state, and
/// the recorded NET token savings.
///
/// # Errors
/// Never; the result type is kept for a uniform command signature.
#[allow(clippy::unnecessary_wraps)] // uniform command signature; printing never errors
pub fn run() -> anyhow::Result<()> {
    let cfg = Config::load();
    println!("snip {}", env!("CARGO_PKG_VERSION"));
    println!(
        "master: {}",
        if cfg.master_enabled {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!("optimizers:");
    for name in optimizer_names(&cfg) {
        println!("  {name}: {}", optimizer_state(&cfg, &name));
    }
    let summary = Tracker::summary();
    println!("net savings: {} tok (estimated, see `gain`)", summary.net);
    Ok(())
}

fn optimizer_state(cfg: &Config, name: &str) -> &'static str {
    if cfg.optimizer_enabled(name) {
        "enabled"
    } else {
        "disabled"
    }
}

#[cfg(test)]
#[path = "../../tests/unit/commands/status.tests.rs"]
mod tests;
