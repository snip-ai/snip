# Dependency policy

**Fewer dependencies = smaller binary + faster builds + smaller attack surface.**

Before adding any crate, ask: *can this be done in ≤ 20 lines of std?* If yes, do
it in std. Justify any addition beyond the approved set in the PR.

## Core dependencies

| Crate | Purpose |
|---|---|
| `clap` (derive) | CLI parsing |
| `serde` + `serde_json` | hook JSON protocol + declarative specs |
| `dirs` | cross-platform data-dir paths |
| `anyhow` | error propagation |

## Approved additions

| Crate | Purpose |
|---|---|
| `tree-sitter` + grammar crates | AST compaction (the `read` engine) |
| `rusqlite` (`bundled`) | stats event store, off the hot path |

Grammar crates must be ABI-compatible with the pinned `tree-sitter` (e.g. Kotlin
uses `tree-sitter-kotlin-ng`).

### Stats store: SQLite

An earlier design used a `stats.jsonl` log to avoid bundling SQLite. That was
**revised**: `rusqlite` (with the **`bundled`** feature, so no system SQLite is
needed) is now the stats store — for ACID writes, safe concurrent writers across
sessions (WAL + busy-timeout), and SQL aggregation. The non-negotiable holds:
**no SQLite on the hot path** — and, just as important, **no process spawn on the
hot path**: `record_*` append one line to `<data_dir>/events.log` (a single
`O_APPEND` write, fork-free), and a **drain** folds that log into the DB off the hot
path (from `gain`/`status` and the `SessionStart` update-check). Only the drain and
`gain`/`status` open the DB. Token counts still use the `estimate_tokens` heuristic
(no `tiktoken-rs`, labeled an estimate); timestamps are `SystemTime` epoch seconds
(no `chrono`).

## Dev-dependencies (tests only)

`[dev-dependencies]` ship in **no** released artifact, so the binary-size /
attack-surface rule above does not gate them — pick the best testing tool. They
must still be well-maintained and earn their place.

| Crate | Purpose | Status |
|---|---|---|
| `assert2` | fluent, value-introspecting assertions (`check!`, `let_assert!`) | in use |
| `assert_cmd` | drive the built `snip` binary in the e2e tier (run, stdin, exit/stdout) | in use |
| `predicates` | composable assertions for `assert_cmd` | in use |
| `tempfile` | RAII temp dirs (`TempDir`) for isolated `SNIP_HOME` in e2e/integration | in use |
| `serial_test` | `#[serial]` to serialize env-mutating tests in the **integration** crate (which cannot reach the lib-private `crate::paths::ENV_LOCK`) | in use |
| `temp-env` | set/unset env vars in tests via a **safe**, auto-restoring, lock-serialized closure API (`with_var`/`with_vars`/`with_var_unset`) | in use |
| `insta` | snapshot testing — available for larger compacted-view outputs | in use |
| `mockall` | trait mocking | approved — add only when a test needs to mock a trait |

**`temp-env` and `unsafe_code = "forbid"` (edition 2024):** edition 2024 makes
`std::env::set_var`/`remove_var` **`unsafe`** — which would clash with the crate's
`unsafe_code = "forbid"` invariant (and `forbid` can't be locally `allow`ed). So
tests never call them directly: they use `temp_env::with_var(s)` (a safe API whose
`unsafe` lives inside the crate, outside our `forbid` scope), which also
auto-restores the prior value. This keeps `forbid` intact with **zero `unsafe` in
our code**.

Serialization still layers on top: in-lib **unit** tests hold
`crate::paths::ENV_LOCK` (it also guards `set_current_dir`, which `temp-env`
doesn't manage); the **integration** crate uses `serial_test`'s `#[serial]`. The
e2e tier needs neither — each test gets its own `tempfile::TempDir` `SNIP_HOME`,
so process-level isolation makes them parallel-safe. The Node/Docker tier
(`tests/docker/`) adds **no** Rust dependency. See `testing.md`.

## Explicitly rejected

- `regex` — **forbidden by design**: the spec transform vocabulary is closed;
  matching is tree-sitter or simple string ops. No regex, no scripting/RCE surface
  in specs.
- `diesel` / `sea-orm` / `sqlx` — ORM overhead for one tiny table.
- `lazy_static` — use `std::sync::OnceLock`.
- `tokio` / async — synchronous I/O is correct here.
- Any HTTP client — the plugin's bootstrap script uses system `curl`.

## Toolchain & edition

- **Rust is pinned** via `rust-toolchain.toml` (`channel = "1.96.0"` + `rustfmt`,
  `clippy`) so every local, CI, and Docker build uses the same compiler. Bump it
  in lockstep with `Cargo.toml`'s `rust-version`, the `dtolnay/rust-toolchain@…`
  pins in `.github/workflows/*.yml`, and `tests/docker/Dockerfile`.
- **Edition 2024.** `unsafe_code = "forbid"` stays — see the `temp-env` note above
  for how the env-test fallout is handled with zero `unsafe`.

## Versions

Use the **latest stable** versions. `cargo update` / `cargo search <crate>`.
