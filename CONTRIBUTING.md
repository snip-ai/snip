# Contributing to snip

Thanks for your interest in **snip** — the unified token/context optimizer for
Claude Code (one static Rust binary, shipped as a Claude Code plugin, that compacts
every token-heavy tool surface via hooks with zero markdown injected into the
model's context).

This guide covers everything you need to land a change: getting set up, the
architecture and conventions we hold the line on, how we test, how commits and PRs
flow, and how releases happen. For deeper detail, the source of truth is the code
plus [`.claude/rules/`](.claude/rules/) — pointers are throughout and collected at
the [end](#where-to-go-deeper).

## Code of conduct

Be respectful, assume good faith, and keep discussion technical. Maintainer:
Aymeric Pasco.

## Getting started

snip is Rust (edition 2024), with a **minimum supported Rust version (MSRV) of
1.96** (pinned in `rust-toolchain.toml`). Install a stable toolchain via
[rustup](https://rustup.rs/); CI runs on stable.

```bash
git clone https://github.com/snip-ai/snip
cd snip
cargo build
```

### The four checks

Every change must pass these four commands locally before you push — they are
exactly what CI enforces, so running them first saves a round trip:

```bash
cargo fmt --check                            # formatting (rustfmt.toml)
cargo clippy --all-targets -- -D warnings    # lints: all / pedantic / nursery, denied
cargo test                                   # full suite (unit + integration)
cargo build --release                        # optimized binary
```

Run `cargo fmt` (no `--check`) to auto-format. The clippy gate is strict on
purpose: `clippy::all`, `clippy::pedantic`, and `clippy::nursery` are all denied,
and `unsafe_code` is forbidden. Use `#[allow(clippy::…)]` only for a genuine false
positive or a documented stub, always with a one-line comment explaining why.

## Architecture in brief

snip is a **layered / onion** architecture: dependencies point inward to
`domain/`. The engine turns each tool surface into a normalized event, finds the
first matching optimizer, and serializes the result back into the right hook JSON.
Optimizers never touch the wire format.

Layers, inner to outer:

- **`domain/`** — the model: `Surface`, `HookCtx`, `Outcome`, and the `Optimizer`
  trait (the Rust escape hatch). No I/O, no wire format.
- **`engine/`** — dispatch machinery: `Dispatcher`, `Registry`, `ToolResponse`,
  `OutcomeSerializer`.
- **Adapters** — `optimizers/` (Rust optimizers), `spec/` (the declarative model +
  `SpecOptimizer` + built-in specs), `cli/`, `hooks/`, `commands/`.
- **Infrastructure** — `config/`, `languages/`, `compaction/`, `stats/`.

Most optimizers are **declarative data** (`OptimizerSpec` — a closed transform
vocabulary, **no regex, no scripting**), overridable by users via config without
recompiling. The Rust escape hatch (`trait Optimizer`) is reserved for what a spec
cannot express — in v1, only `read` (AST-based).

### Where new code goes

- A new optimized command → a JSON spec in `spec/builtin/specs/` **or** user
  config (zero recompile); a new transform → a `Transform` variant.
- A new surface → a `Surface` variant + a `Command` arm + a `hooks.json` entry.
- A genuinely complex optimizer a spec can't express → `optimizers/<name>/`.
- A new domain concept → its own file in the owning module.
- Settings read on the hot path → `config/`.

Prefer declarative specs over Rust: a new command optimizer should be data, not a
module.

The working module map is
[`.claude/rules/architecture.md`](.claude/rules/architecture.md).

## Code conventions

### One public type per file

We keep **one public type per file**, with the file named `snake_case(Type).rs`
(e.g. `Dispatcher` → `dispatcher.rs`). Private helpers serving that type may share
its file. `mod.rs` holds **only** `mod` + `pub use` (a re-export facade) — no type
or free-function definitions.

The single exception: a type whose natural file name would equal its parent module
(which would trip `clippy::module_inception`) lives in that `mod.rs`. Today that is
only `Config` (`config/mod.rs`) and `Cli` (`cli/mod.rs`).

### Line limits

**Soft limit 150 lines per file, hard limit 200.** Split by responsibility before
you exceed it.

### Style essentials

- `anyhow::Result<T>` and `?` propagation in application code; never `.unwrap()` /
  `.expect()` on production paths.
- `&str` over `&String`, `&[T]` over `&Vec<T>` in signatures.
- No `unsafe` (it is forbidden). No `lazy_static` — use `std::sync::OnceLock`.
- Behaviour lives on types with methods, not loose free functions (except
  genuinely stateless one-liners).
- Every public item gets a `///` doc comment; every module starts with a `//!`
  summary whose first paragraph is one short line.
- Before adding a dependency, ask: *can this be done in ≤ 20 lines of std?* If yes,
  do it in std. See [`.claude/rules/dependencies.md`](.claude/rules/dependencies.md)
  — `regex` and async are rejected by design.

### Safety invariants (non-negotiable)

These protect users and must never regress:

- **Every hook exits 0 under all circumstances** — errors *and* panics are caught
  at the top level (`engine/dispatcher.rs`).
- **snip never writes to user source files** — its state lives under the OS data
  dir.
- **If optimization fails, return the original output unchanged.**
- **No regex, no scripting in specs.** No view larger than the original.

Deeper detail: [`.claude/rules/rust-conventions.md`](.claude/rules/rust-conventions.md)
and [`.claude/rules/hook-protocol.md`](.claude/rules/hook-protocol.md).

## Testing

Tests live **entirely outside `src/`**, in a root `tests/` tree mirroring the
`src/` layout, split into three tiers. Test files are named
`<name_under_test>.tests.rs`.

```
tests/
├── unit/          # white-box: compiled INTO snip_lib, reaches private items
├── integration/   # black-box: a separate crate vs the PUBLIC API
├── e2e/           # black-box: drives the built binary on stdin
└── docker/        # real Claude Code, headless, network-isolated (not in cargo test)
```

### White-box convention

Unit tests are **white-box**: each is included into the library via a `#[cfg(test)]`
`#[path]` module at the bottom of the source file under test, so it can reach that
module's private items while still living outside `src/`. The `#[cfg(test)]` gate
keeps all test code out of the released binary.

```rust
#[cfg(test)]
#[path = "../../tests/unit/<area>/<name>.tests.rs"]
mod tests;
```

Never inline a `#[cfg(test)] mod tests { … }` block in a source file — attach a
`#[path]` include instead. Integration tests use only `snip_lib::…` public items
and double as a public-API compile gate. Because `[lints]` applies to all targets,
test code must also pass clippy deny-all and rustfmt.

### Format: AAA + assert2

Every test follows **Arrange / Act / Assert**, with those three `//` section
comments in order, one logical behaviour per test. Assertions use
[`assert2`](https://docs.rs/assert2):

- `check!(expr)` — the default; on failure it prints the expression and its
  operand values. Use `==` and `!x` / `x.is_none()` (never `== true/false`).
- `assert2::assert!(let Pattern = expr)` — destructure-or-fail for the Act step on
  `Result`/`Option` and to match enum variants.
- For table-driven loops, use std `assert!(cond, "…{case}…")` so the message names
  the failing row.

Don't hand-roll mocks; add [`mockall`](https://docs.rs/mockall) as a dev-dependency
only when a test actually needs to mock a trait.

### What to test

- A pure function → a unit test mirroring its file.
- A cross-module flow through the public surface → an integration test.
- Every new language → a row in the compactor test table plus the ABI check in
  `languages/registry.tests.rs`.
- Hook safety invariants (exit 0, pass-through, shape preservation) → unit tests.

### Coverage targets

Measured with [`cargo-llvm-cov`](https://github.com/tarpaulin/cargo-llvm-cov):

- **Unit tests: ≥ 90% line coverage.**
- **Integration tests: ≥ 80% line coverage.**
- The e2e and docker tiers drive the built binary / real Claude Code; they are not
  line-coverage-gated.

```bash
cargo llvm-cov --lib          # unit (library) coverage
cargo llvm-cov --test integration
```

Full testing rules: [`.claude/rules/testing.md`](.claude/rules/testing.md).

## Commits

We use [Conventional Commits](https://www.conventionalcommits.org/). The type
drives both the changelog section and the automated version bump:

| Type        | Meaning                                  | Version bump |
|-------------|------------------------------------------|--------------|
| `feat`      | a new feature                            | minor        |
| `fix`       | a bug fix                                | patch        |
| `perf`      | a performance improvement                | patch        |
| `docs`      | documentation only                       | none\*       |
| `refactor`  | code change, no behaviour change         | none\*       |
| `test`      | tests only                               | none         |
| `build`     | build system / dependencies              | none         |
| `ci`        | CI configuration                         | none         |
| `chore`     | maintenance, no production code change    | none         |
| `feat!` / `BREAKING CHANGE:` | a breaking change       | major        |

\* While the project is pre-1.0, exact bump behaviour is governed by
`release-please-config.json` (`bump-minor-pre-major`); `feat` bumps minor and `fix`
bumps patch, and a `!` / `BREAKING CHANGE` footer marks a major.

Format: `type(scope): summary`, e.g. `feat(read): extend soft compaction to 16
languages`. Keep the summary imperative and lower-case; explain the *why* in the
body when it isn't obvious.

## Pull request workflow

All changes land via **squash-merged PRs**; don't commit directly to `main`.

1. **Branch** off `main` — never commit directly to it. Use a short descriptive
   name (e.g. `feat/search-optimizer`).
2. Make your change, keeping commits focused. Run the [four checks](#the-four-checks)
   locally.
3. Add or update tests so the [coverage targets](#coverage-targets) hold.
4. Open a PR with a clear description of the *what* and *why*. Link any related
   issue.
5. **CI must pass.** The CI workflow runs `cargo fmt --all --check` + `cargo clippy
   --all-targets -- -D warnings` on Linux, and `cargo test` + `cargo build
   --release` on Linux, macOS, and Windows.
6. Address review feedback. When approved, a maintainer **squash-merges** the PR —
   so the PR title should itself be a valid Conventional Commit, since it becomes
   the squashed commit message that release automation reads.

Keep PRs reasonably small and single-purpose; it makes review and bisection easier.

## How releases work

Releases are **fully automated by
[release-please](https://github.com/googleapis/release-please)** from the
Conventional Commits merged into `main`:

- release-please opens and maintains a **release PR** that bumps the version in
  `Cargo.toml` and the plugin manifest
  (`plugins/snip/.claude-plugin/plugin.json`) and updates `CHANGELOG.md`.
- Merging that release PR tags the release and triggers the release workflow, which
  builds and ships the prebuilt binary. Distribution and updates both flow through
  the **Claude Code plugin** — there is no cargo/crates.io/npm path and no curl /
  PowerShell installer.

**Never hand-edit `CHANGELOG.md` or bump versions manually** — release-please owns
both, derived entirely from commit messages. This is the single biggest reason
commit hygiene matters here.

## Where to go deeper

- [`.claude/rules/architecture.md`](.claude/rules/architecture.md) — working module map.
- [`.claude/rules/rust-conventions.md`](.claude/rules/rust-conventions.md) — style + lints.
- [`.claude/rules/hook-protocol.md`](.claude/rules/hook-protocol.md) — the Claude Code hook contract.
- [`.claude/rules/dependencies.md`](.claude/rules/dependencies.md) — dependency policy.
- [`.claude/rules/testing.md`](.claude/rules/testing.md) — the full testing convention.

## License

By contributing, you agree that your contributions are licensed under the project's
[Apache-2.0](LICENSE) license.
