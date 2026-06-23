//! Reproducible hot-path latency benchmark for snip's optimizers.
//!
//! Times the in-process work each tool hook performs — AST compaction (Read) and
//! the declarative transform pipeline (Grep/Bash) — over the same fixed corpus as
//! `examples/bench.rs`, and prints a Markdown table of min/median/p95 per case.
//! Run with `cargo run --release --example latency`.
//!
//! Scope: this measures the *in-process* hot path (parse + transform), the work
//! the `< 15 ms` budget governs. It deliberately excludes OS process spawn and the
//! one-time `Config::load` JSON read (a few hundred µs, constant and
//! platform-dependent). Build `--release`: the debug profile is not representative.

use std::fmt::Write as _;
use std::hint::black_box;
use std::time::Instant;

use snip_lib::compaction::Compactor;
use snip_lib::config::CompactMode;
use snip_lib::domain::Surface;
use snip_lib::languages::detect;
use snip_lib::optimizers::SpecOptimizer;
use snip_lib::spec::OptimizerSpec;
use snip_lib::spec::builtin::builtin_specs;

/// Timed iterations per case.
const ITERS: usize = 200;
/// Untimed warmup iterations (page-in, branch prediction, grammar init).
const WARMUP: usize = 20;

/// Real, comment-rich source files used as the Read corpus (shared with `bench`).
const READ_SAMPLES: &[(&str, &str)] = &[
    ("registry.rs", include_str!("../src/languages/registry.rs")),
    (
        "compactor.rs",
        include_str!("../src/compaction/compactor.rs"),
    ),
    ("dispatcher.rs", include_str!("../src/engine/dispatcher.rs")),
];

fn main() {
    println!("# snip hot-path latency benchmark\n");
    println!("| surface | case | min | median | p95 |");
    println!("|---|---|---:|---:|---:|");
    bench_read();
    bench_read_large();
    bench_search();
    bench_command();
    println!(
        "\n_In-process work only (parse + transform), {ITERS} timed iterations after {WARMUP} \
         warmups; excludes OS process spawn and the one-time config read. Build `--release`. \
         Harness: `examples/latency.rs`._"
    );
}

/// Time `f` over [`ITERS`] runs (after [`WARMUP`]), print its min/median/p95 row,
/// and return the median in microseconds (for the latency-regression guard).
fn bench<F: FnMut()>(surface: &str, case: &str, mut f: F) -> u128 {
    for _ in 0..WARMUP {
        f();
    }
    let mut samples = Vec::with_capacity(ITERS);
    for _ in 0..ITERS {
        let t = Instant::now();
        f();
        samples.push(t.elapsed().as_micros());
    }
    samples.sort_unstable();
    let p95 = samples[ITERS * 95 / 100];
    let median = samples[ITERS / 2];
    println!(
        "| {surface} | {case} | {} | {} | {} |",
        dur(samples[0]),
        dur(median),
        dur(p95)
    );
    median
}

/// Format a microsecond count as `N µs` or `N.NN ms` (integer math, no float cast).
fn dur(v: u128) -> String {
    if v >= 1000 {
        format!("{}.{:02} ms", v / 1000, (v % 1000) / 10)
    } else {
        format!("{v} µs")
    }
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
            bench("Read", &format!("{name} [{label}]"), || {
                black_box(Compactor::new(spec).compress_mode(black_box(src), mode));
            });
        }
    }
}

/// Read: a large (~just under the 1 MB cap) source — the worst case the hot path
/// actually parses (bigger files pass through uncompacted via `MAX_READ_BYTES`).
///
/// The wall-clock parse guard ([`compaction::parse`]) must keep even this case
/// within the hot-path budget by abandoning an over-long parse; the assertion is
/// the regression backstop the budget knob can't silently break (margin over the
/// 15 ms budget tolerates the parse-callback's cancellation granularity).
fn bench_read_large() {
    let spec = detect("x.rs").expect("rust language spec");
    let unit = READ_SAMPLES[2].1;
    let big = unit.repeat((900_000 / unit.len().max(1)).max(1));
    let case = format!("large ~{}KB [high]", big.len() / 1024);
    let median = bench("Read", &case, || {
        black_box(Compactor::new(spec).compress_mode(black_box(&big), CompactMode::High));
    });
    assert!(
        median < 20_000,
        "Read compaction of a {}KB source took {median} µs — the < 15 ms hot-path budget \
         regressed; the wall-clock parse guard in compaction/parse.rs is not bounding it.",
        big.len() / 1024
    );
}

/// Grep/Glob: the declarative search optimizer over a synthetic match set.
fn bench_search() {
    let output = grep_corpus();
    if let Some(spec) = surface_specs(Surface::Grep).into_iter().next() {
        bench_spec("Grep", &spec, &output);
    }
}

/// Bash: a representative command spec over synthetic command output.
fn bench_command() {
    let specs = surface_specs(Surface::Bash);
    if let Some(find) = specs.iter().find(|s| s.name == "find") {
        bench_spec("Bash", find, &file_tree());
    }
}

/// Time one spec's transform pipeline and print its row.
fn bench_spec(surface: &str, spec: &OptimizerSpec, output: &str) {
    bench(surface, &spec.name, || {
        black_box(SpecOptimizer::new(spec.clone()).apply_to(black_box(output), false));
    });
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
