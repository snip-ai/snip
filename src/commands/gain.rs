//! `snip gain` — NET token-savings analytics (input saved − induced cost),
//! per optimizer and per surface. Backed by the `SQLite` event store (P6).

use crate::config::Config;
use crate::stats::{Tracker, pricing};

/// Print the NET gain report.
///
/// # Errors
/// Never; reading the stats store degrades to an empty report.
#[allow(clippy::unnecessary_wraps)] // uniform command signature; printing never errors
pub fn run() -> anyhow::Result<()> {
    let summary = Tracker::summary();
    if summary.events == 0 {
        println!("snip gain — no optimizations recorded yet");
        return Ok(());
    }
    let model = Config::load().model.unwrap_or_default();
    println!("snip gain — NET token savings (estimated)\n");
    println!("  gross saved : {:>10} tok", summary.gross_saved);
    println!(
        "  induced cost: {:>10} tok  (spilled-output re-reads)",
        summary.induced
    );
    let (low, high) = pricing::net_usd_range(summary.net, &model);
    println!(
        "  NET         : {:>10} tok  ≈ ${low:.2}–${high:.2}",
        summary.net
    );
    println!("                  (cache-read floor → fresh-input ceiling; tokens are an estimate)");
    print_breakdown("by optimizer", &summary.per_optimizer);
    print_breakdown("by surface", &summary.per_surface);
    Ok(())
}

/// Print one `(name, net tokens)` breakdown block.
fn print_breakdown(title: &str, rows: &[(String, i64)]) {
    println!("\n  {title}:");
    for (name, net) in rows {
        println!("    {name:<12} {net:>10} tok");
    }
}

#[cfg(test)]
#[path = "../../tests/unit/commands/gain.tests.rs"]
mod tests;
