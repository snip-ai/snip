# Testing

Tests live **entirely outside `src/`**, in a root `tests/` tree: three Rust
tiers (unit/integration/e2e) each **mirroring the `src/` layout**, plus a
Node/Docker tier that drives the **real `claude` CLI**. Rust test files are named
`<name_under_test>.tests.rs`.

```
tests/
├── unit/          # white-box: compiled INTO snip_lib (reaches private items)
│   ├── config/config.tests.rs
│   ├── compaction/compactor.tests.rs
│   ├── languages/registry.tests.rs
│   ├── engine/{dispatcher,tool_response,outcome_serializer}.tests.rs
│   └── optimizers/read/read_optimizer.tests.rs
├── integration/   # black-box: a separate crate vs the PUBLIC API
│   ├── main.rs            # harness: #[path]-includes the mirror files
│   └── read_pipeline.tests.rs
├── e2e/           # black-box: drives the BUILT `snip` binary on stdin
│   ├── main.rs            # harness: #[path]-includes the mirror files
│   ├── support.rs         # `Snip` helper (assert_cmd + a tempfile SNIP_HOME)
│   └── read_hook.tests.rs
└── docker/        # real Claude Code, headless, in a network-isolated container
    ├── Dockerfile · run.sh · run-docker.sh   # build snip + claude; run offline
    ├── harness/          # Node stdlib: fake Anthropic server, runner, fixtures
    └── phase-*.test.mjs  # A: contract drift · B: conformance/langs/bash/edit/
                          #    grep-modes/lifecycle · C: efficacy
```

## How each tier is wired

- **unit** — included into the library with a `#[path]` module at the bottom of
  the source file under test:

  ```rust
  #[cfg(test)]
  #[path = "../../tests/unit/<area>/<name>.tests.rs"]
  mod tests;
  ```

  The path is **relative to the source file** (`../../` from `src/<area>/`,
  `../../../` from `src/optimizers/read/`). The test uses `use super::*` to reach
  the module's private items and `use crate::…` for cross-module types. This is
  what lets unit tests cover private fns (`parse_enabled`, `process`, `SPECS`)
  while still living outside `src/`.

- **integration** — a single `[[test]]` target (`tests/integration/main.rs`) that
  `#[path]`-includes the mirror files. They use only `snip_lib::…` **public**
  items, so they double as a public-API compile gate.

- **e2e** — its own `[[test]]` target (`tests/e2e/main.rs`) that drives the
  **built `snip` binary** exactly as Claude Code does: a hook's JSON on stdin →
  assert the JSON on stdout + the always-exit-0 invariant. The binary is located
  via `assert_cmd` (`Command::cargo_bin("snip")`); the shared `Snip` harness
  (`tests/e2e/support.rs`) roots each run at a throwaway `tempfile::TempDir`
  `SNIP_HOME`, so on-disk state never leaks between tests and they stay
  parallel-safe. This is the one tier covering `main` → `cli::run` → dispatch and
  the `std::process::exit` paths end-to-end. See `tests/e2e/README.md` for the
  per-surface coverage map.

- **docker** — the highest-fidelity tier: a self-contained image builds snip and
  installs a pinned `claude`, then runs it **headless** against a loopback **fake
  Anthropic server** (Node stdlib), fully isolated (`docker run --network=none` —
  offline, secretless, deterministic). It proves the *integration boundary* the
  Rust tiers can't: real plugin/hook dispatch, the real `tool_response` shapes,
  and that the model actually receives snip's rewrites. Phases: **A** contract
  drift (asserts the real shapes snip reads), **B** conformance + all 29 Read
  languages + Bash families + Edit/Write + Grep modes + plugin lifecycle, **C**
  deterministic efficacy (token deltas). Not part of `cargo test`; runs via
  `tests/docker/run-docker.sh` and the `docker-e2e` workflow (nightly/dispatch/
  release). See `tests/docker/README.md`.

## Cargo wiring

`Cargo.toml` sets `autotests = false` and declares the `integration` and `e2e`
`[[test]]` targets explicitly, so the `tests/` subdirs are never auto-discovered
as stray targets. Unit tests run as part of the library (`cargo test` /
`cargo test --lib`).

`[lints]` applies to **all** targets, so test code (unit + integration + e2e)
must pass `clippy` deny-all and `rustfmt` too. `rustfmt` follows the `#[path]`
includes, so `cargo fmt` formats the whole `tests/` tree.

## Format: AAA + assert2

Every test is **Arrange / Act / Assert**, with those three `//` section comments,
in that order. Keep one logical behaviour per test; no assertions in the Arrange
or Act blocks.

Assertions use [`assert2`](https://docs.rs/assert2) (a dev-dependency — see
`dependencies.md`):

- `check!(expr)` — the default; on failure it prints the expression **and its
  operand values** (fluent diagnostics without method chaining). Use `==` for
  equality and `!x` / `x.is_none()` for booleans (never `== true/false` —
  `clippy::bool_comparison`).
- `assert2::assert!(let Pattern = expr)` — destructure-or-fail, and it **binds** the
  captured names (assert2 0.4 deprecated `let_assert!`). Use it for the Act step on
  `Result`/`Option` (`assert2::assert!(let Ok(x) = …)`, `… let Some(x) = …`) and to
  match enum variants (`assert2::assert!(let Outcome::Rewrite { body, .. } = outcome)`),
  replacing verbose `match { _ => panic!() }`.
- For **table-driven** loops, use std `assert!(cond, "…{case}…")` so the message
  names the failing row (assert2's introspection can't show the loop variable).

Mocking/fakes: trait collaborators are faked **in-process** today (a `Config`, a
`HookCtx`, a `serde_json` hook value — no I/O). When a future test needs to mock a
trait (e.g. `Optimizer`, or a filesystem/shell port), add
[`mockall`](https://docs.rs/mockall) as a dev-dependency — do not hand-roll mocks.
Keep to YAGNI: add the dev-dependency only when a test actually needs it.

## Env mutation in tests (edition 2024)

Edition 2024 makes `std::env::set_var`/`remove_var` **`unsafe`**, which `forbid`
rejects. **Never** call them — set env via `temp-env`'s safe closures:

```rust
temp_env::with_var("SNIP_HOME", Some(home.path()), || { /* Act + Assert */ });
temp_env::with_vars([("SNIP_HOME", Some(p)), ("SNIP_ENABLED", None)], || { … });
temp_env::with_var_unset("SNIP_ENABLED", || { … });
```

`temp-env` auto-restores the prior value (drop the old manual `remove_var`
cleanup) and serializes env access via its own lock. Keep filesystem/cwd setup
and cleanup **outside** the closure (only the env-dependent body goes inside).
Still hold `crate::paths::ENV_LOCK` (unit) / `#[serial]` (integration) — they also
serialize `set_current_dir`, which `temp-env` does not manage. Zero `unsafe`.

## What to test

- A pure function → its own unit test mirroring its file.
- A cross-module flow through the public surface → an integration test.
- Every new language → a row in `compaction/compactor.tests.rs`'s table + the
  ABI check in `languages/registry.tests.rs` (`every_grammar_loads`).
- Hooks' safety invariants (exit 0, pass-through, shape preservation) → unit
  tests on `Dispatcher`/`ToolResponse`/`OutcomeSerializer`; full-binary coverage
  lands in `e2e/`.
- A new Read language → add it to `tests/docker/harness/languages.mjs` (the
  docker languages phase is data-driven over that table).
- A behaviour that depends on **real** Claude Code (a new tool surface, a
  `tool_response` shape snip parses, hook dispatch, the model receiving a rewrite)
  → a `tests/docker/` phase. The Rust e2e tier feeds hand-written shapes, so it
  can't catch a Claude Code shape change — Phase A's drift canary is the guard.
