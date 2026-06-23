//! Reproducible token-economy benchmark for snip's optimizers.
//!
//! Runs each optimizer over a fixed, in-repo corpus and prints a Markdown table of
//! tokens before→after per surface. Run with `cargo run --example bench`.
//!
//! Figures use snip's `estimate_tokens` heuristic (see `docs/BENCHMARKS.md`), not
//! an exact tokenizer — directionally honest, not billing-exact.

use std::fmt::Write as _;

use snip_lib::compaction::Compactor;
use snip_lib::config::CompactMode;
use snip_lib::domain::{Outcome, Surface};
use snip_lib::languages::detect;
use snip_lib::optimizers::SpecOptimizer;
use snip_lib::spec::OptimizerSpec;
use snip_lib::spec::builtin::builtin_specs;
use snip_lib::tokens::estimate_tokens;

/// Real, comment-rich source files used as the Read corpus.
const READ_SAMPLES: &[(&str, &str)] = &[
    ("registry.rs", include_str!("../src/languages/registry.rs")),
    (
        "compactor.rs",
        include_str!("../src/compaction/compactor.rs"),
    ),
    ("dispatcher.rs", include_str!("../src/engine/dispatcher.rs")),
];

fn main() {
    println!("# snip token-economy benchmark\n");
    println!("| surface | case | before (tok) | after (tok) | saved | reduction |");
    println!("|---|---|---:|---:|---:|---:|");
    bench_read();
    bench_search();
    bench_command();
    println!(
        "\n_Token counts are snip's `estimate_tokens` heuristic, not exact tiktoken \
         (see docs/BENCHMARKS.md). Corpus + harness: `examples/bench.rs`._"
    );
}

/// Print one result row.
fn row(surface: &str, case: &str, before: usize, after: usize) {
    let saved = before.saturating_sub(after);
    let pct = (saved * 100).checked_div(before).unwrap_or(0);
    println!("| {surface} | {case} | {before} | {after} | {saved} | {pct}% |");
}

/// Read: AST compaction in all three modes over real source files.
fn bench_read() {
    let spec = detect("x.rs").expect("rust language spec");
    for &(name, src) in READ_SAMPLES {
        for (label, mode) in [
            ("soft", CompactMode::Soft),
            ("medium", CompactMode::Medium),
            ("high", CompactMode::High),
        ] {
            let before = estimate_tokens(src);
            let after = Compactor::new(spec)
                .compress_mode(src, mode)
                .map_or(before, |view| estimate_tokens(&view));
            row("Read", &format!("{name} [{label}]"), before, after);
        }
    }
}

/// Grep/Glob: the declarative search optimizer over a synthetic match set.
fn bench_search() {
    let output = grep_corpus();
    for spec in surface_specs(Surface::Grep) {
        bench_spec("Grep", &spec, &output);
    }
}

/// Bash: representative command specs over synthetic command output.
fn bench_command() {
    let specs = surface_specs(Surface::Bash);
    if let Some(ls) = specs.iter().find(|s| s.name == "ls") {
        bench_spec("Bash", ls, &file_list(200));
    }
    if let Some(find) = specs.iter().find(|s| s.name == "find") {
        bench_spec("Bash", find, &file_tree());
    }
}

/// Run one spec's pipeline and print its row.
fn bench_spec(surface: &str, spec: &OptimizerSpec, output: &str) {
    let before = estimate_tokens(output);
    let after = match SpecOptimizer::new(spec.clone()).apply_to(output, false) {
        Outcome::Rewrite { body, .. } => estimate_tokens(&body),
        _ => before,
    };
    row(surface, &spec.name, before, after);
}

/// Built-in specs bound to `surface`.
fn surface_specs(surface: Surface) -> Vec<OptimizerSpec> {
    builtin_specs()
        .into_iter()
        .filter(|s| s.surface == surface)
        .collect()
}

/// 15 files × 20 matches each — repetitive, same-file-grouped grep output.
fn grep_corpus() -> String {
    let mut s = String::new();
    for f in 0..15 {
        for l in 0..20 {
            let _ = writeln!(
                s,
                "src/module{f}/file{f}.rs:{}:    let value = compute(input);",
                l + 1
            );
        }
    }
    s
}

/// A flat list of `n` unique file names (the `ls` corpus).
fn file_list(n: usize) -> String {
    let mut s = String::new();
    for i in 0..n {
        let _ = writeln!(s, "file_{i:04}.rs");
    }
    s
}

/// A deep file tree sharing directory prefixes (the `find` corpus).
fn file_tree() -> String {
    let mut s = String::new();
    for d in 0..20 {
        for f in 0..15 {
            let _ = writeln!(s, "./src/area{d:02}/sub/file_{f:02}.rs");
        }
    }
    s
}
