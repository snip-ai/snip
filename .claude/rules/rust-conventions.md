# Rust conventions

Rust is pinned to **1.96** (`rust-toolchain.toml`) on **edition 2024**, with
`unsafe_code = "forbid"` kept — see [`dependencies.md`](dependencies.md) for the
toolchain/edition policy.

## Error handling

- `anyhow::Result<T>` for application code; `?` propagation everywhere.
- Never `.unwrap()` / `.expect()` on production paths.
- Every hook **must never exit non-zero**: catch all errors at the top level, log
  to stderr, return `Ok(())` — and wrap the body in `std::panic::catch_unwind` so
  a panic can't break the invariant (the release profile stays on
  `panic = "unwind"`). This is centralized in `engine/dispatcher.rs`
  (`Dispatcher::run`).

## Performance

- `OnceLock` for one-time inits (the registry; later the tiktoken BPE).
- `BufWriter<Stdout>` for hook stdout writes.
- SQLite in WAL mode, off the hot path only.
- Pre-allocate and reuse buffers in tight loops; no per-iteration heap allocation
  in the transform pipeline or fuzzy-match.

## Style

- `&str` over `&String`, `&[T]` over `&Vec<T>` in signatures.
- No `unsafe` — `unsafe_code = "forbid"`. Edition 2024 makes `std::env::set_var`
  unsafe, so tests set env via `temp-env`'s safe closures, never directly (see
  `testing.md`); zero `unsafe` anywhere.
- No `pub` on items that don't need it; use `pub(crate)` if a helper must be
  reachable from tests, never `pub` just for tests.
- Short, single-purpose functions (KISS); extract shared helpers (DRY).
- Prefer declarative specs over Rust: a new command optimizer should be data, not
  a module. The Rust escape hatch is for what a spec genuinely cannot express.

## File organisation

- **One public type per file** (`struct`/`enum`/`trait`), the file named
  `snake_case(Type).rs`. Private helpers serving that type may share its file.
- **`mod.rs` holds only `mod` + `pub use`** (a re-export facade) — no type or
  free-function definitions. The single exception: a type whose natural file name
  would equal its parent module (e.g. `config/config.rs`, `cli/cli.rs`) lives in
  that `mod.rs` to avoid `clippy::module_inception` — today only `Config` and `Cli`.
- **File size:** soft limit **150** lines, hard **200**. Split by responsibility
  before exceeding it. The one documented exception is `languages/registry.rs`: a
  single cohesive `SPECS` data table whose `grammar` field is a compiled
  fn-pointer (so it can't be externalized to JSON) — splitting the table by an
  arbitrary line count would hurt, not help, cohesion.
- **Prefer OOP / encapsulation:** behaviour lives on types with methods
  (`Dispatcher`, `Compactor`, `ToolResponse`), not loose free functions, except
  for genuinely stateless one-line utilities (`estimate_tokens`).
- **SOLID / DRY / KISS / YAGNI:** one responsibility per type; depend inward on
  `domain`; don't add a type, layer, or abstraction until a second caller needs it.

## Tests

All tests live outside `src/` in `tests/{unit,integration,e2e}/` (mirroring
`src/`), plus a real-Claude-Code `tests/docker/` tier — see
[`testing.md`](testing.md). Never inline `#[cfg(test)] mod tests { … }` blocks in
a source file; attach a `#[path]` include instead.

## Lints (deny-all, via `[lints]` in Cargo.toml)

`rust`: `unsafe_code = "forbid"`, `missing_docs = "deny"`.
`clippy`: `all` / `pedantic` / `nursery` all denied.

Use `#[allow(clippy::...)]` only for a genuine false positive or a documented stub,
always with a one-line comment explaining why (e.g. a trait fixes a return type, a
stub body lands in a later phase, or a pedantic lint suggests an **unstable** API
on stable — `duration_suboptimal_units` proposing `Duration::from_days`). Run
`cargo clippy --all-targets -- -D warnings` before commit.

## Formatting (`rustfmt.toml`)

`max_width = 100`, 4-space tabs, `use_small_heuristics = "Default"`. Stable-only
options (CI runs `cargo fmt --check` on stable). Run `cargo fmt` before commit.

## Documentation

Every public item (`pub fn/struct/enum/const`) has a `///` doc comment; every
module starts with a `//!` summary whose **first paragraph is one short line**
(clippy `too_long_first_doc_paragraph`). Back-tick identifiers and product names
in docs (`SQLite`, `PreCompact`, `SessionStart` …). Comments explain **why**, not
what. Never restate the code or pad with filler.
