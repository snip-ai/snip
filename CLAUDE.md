# snip — unified token/context optimizer for Claude Code

**snip** is a single static Rust binary, shipped as a **Claude Code plugin**, that
optimizes every token-heavy tool surface (Read, Bash, Grep, Glob) via hooks — with
**zero markdown injected into the model's context**. It unifies and supersedes two
predecessor tools — AST code compaction on Read, and command-output compression on
Bash — as one optimizer that absorbs the read engine, covers every surface, injects
structured flags before execution, and never discards output (recoverable overflow).

> **Source of truth:** the code + [`.claude/rules/`](.claude/rules/) for the
> working/as-built layout.

## The core idea

A generic **framework** dispatches each surface to an **optimizer**. Most
optimizers are **declarative data** (`OptimizerSpec` — a closed transform
vocabulary, **no regex, no scripting**), overridable by the user via config
without recompiling (built-in → user → an opt-in repo-local project layer
`.snip/config.json`, off by default and trust-gated, by `name`). The Rust **escape hatch**
(`trait Optimizer`) is reserved for what a spec cannot express — in v1, only
`read` (AST-based) plus the generic `SpecOptimizer` adapter.

**3 reference optimizers:** `read` (Read/Edit/Write), `command` (Bash; 3 families:
base shell / git / per-language-framework), `search` (Grep/Glob).

## Surfaces → hooks

| Surface | Hook | snip subcommand | Optimizer |
|---|---|---|---|
| Read | PostToolUse/Read | `read-hook` | `read` |
| Grep / Glob | PostToolUse | `grep-hook` / `glob-hook` | `search` |
| Bash | PreToolUse/Bash | `bash-route` → `exec` | `command` |
| Edit / Write | PreToolUse | `edit-fix` / `write-guard` | `read` |
| PreCompact | PreCompact | `session-reset` | clears the session cache |
| SessionStart | SessionStart | `update-check` | binary ↔ latest release |

## Non-negotiables

- **Hot path < 15 ms.** Config = a tiny JSON read once; **no SQLite/tiktoken on
  the hot path**; O(1) spec lookup; lazy grammars.
- **Hooks always exit 0** — catch every error **and panic** (`catch_unwind`), log
  to stderr, return `Ok(())`.
- **Never writes to user source files** — snip's own state (config, stats DB,
  session dedupe/spill cache) lives under the data dir.
- **If optimization fails, return the original output unchanged.**
- **Fuzzy-match threshold 0.85** (+ length-ratio ≥ 0.80) — do not lower.
- **No markers in-band; no regex/scripting in specs.** No-inflation guard: never
  emit a view larger than the original.

## Distribution

**Claude Code plugin only** — install **and** updates both flow through the plugin
(marketplace / self-installing). **No** cargo/crates.io/npm, **no** curl /
PowerShell installer. There is therefore no `init` or `self-update` subcommand.

## Commands

```bash
cargo build --release         # optimized binary
cargo test                    # full suite
cargo fmt --check             # formatting
cargo clippy --all-targets -- -D warnings   # lints (all/pedantic/nursery denied)
```

Toolchain is pinned via `rust-toolchain.toml` (Rust **1.96**, edition **2024**);
CI and `tests/docker/` pin the same. `unsafe_code = "forbid"` is kept — tests
mutate env via the `temp-env` crate (never `std::env::set_var`, unsafe in 2024).

snip subcommands (dispatched in `src/cli/command.rs`): `read-hook`, `grep-hook`,
`glob-hook`, `bash-route`, `edit-fix`, `write-guard`, `session-reset`,
`update-check`, `exec`, `resolve`, `gain`, `status`, `stat-record`, `config`,
`enable`, `disable`. Meta-commands are surfaced as plugin slash-commands
(`/snip-gain`, `/snip-status`, `/snip-config`, …).

## Project layout (`src/`, layered — one public type per file)

A layered/onion architecture; dependencies point inward to `domain`. **One public
type per file** (`file = snake_case(Type).rs`); **`mod.rs` = `mod`/`pub use` only**
(exception where a `Type.rs` would trip `clippy::module_inception`: `Config` in
`config/mod.rs`, `Cli` in `cli/mod.rs`). Soft limit **150** lines/file, hard **200**
— the one documented over-limit exception is `languages/registry.rs` (a single
cohesive `SPECS` data table whose `grammar` field is a compiled fn-pointer, so it
can't be externalized to JSON).

- `domain/` — the model (inward, dependency-light): `Surface`, `HookCtx`,
  `Outcome`, the `Optimizer` trait (the Rust escape hatch).
- `engine/` — dispatch machinery: `Dispatcher` (the shared hook contract),
  `Registry` (the current surface's optimizers, built per run), `ToolResponse`
  (the `tool_response` wire shapes: Read `file.content`, Grep top-level `content`,
  Glob `filenames` array, bare string), `OutcomeSerializer` (`Outcome` → hook JSON).
- `config/` — layered `Config` (data + load/save in `mod.rs`; resolution accessors
  in `accessors.rs`) + `OptimizerCfg` + `AutodetectCfg` + `CompactMode`, the opt-in
  repo-local `project` overlay, and `validate` (lenient spec load + `config validate`
  diagnostics, so one bad spec can't silently reset the whole config).
- `spec/` — the declarative model (data + vocabulary): `OptimizerSpec`, `Bind`,
  `Transform` (**one module per transform impl** — `dedupe`/`squeeze`/`diff_fold`/
  `log_fold`/…), the vocabulary types (`GroupKey`/`RankKey`/`StrPred`/`FpWindow`),
  `ParseFormat` + `formats/`, `builtin` (embedded JSON specs + `*_for(surface)`).
- `optimizers/` — **every `Optimizer` impl**: `read/` (Rust AST escape hatch — the
  trait impl + Read surface in `read_optimizer`, Edit/Write in `edit_write`,
  re-read `dedupe` + `diff`-on-change), `command/` (the Bash runtime —
  segmenter/plan/exec/capture/bash_route), `search/` (the declarative Grep/Glob
  optimizer), `SpecOptimizer` (the declarative adapter that runs any
  `OptimizerSpec`), and `redact` (the `secret_safe` redaction service).
- `overflow/` — the shared cap/spill service: `OverflowCfg` (budget data),
  `ElideStrategy` (Head/Tail/Middle/RelevanceFirst), `Spill` (recoverable spill).
- `languages/` — `LanguageSpec` + `registry` (the SPECS table + `detect`).
- `compaction/` — the `read` engine: `Compactor` (soft/medium/high) + collapse/reexpand,
  with `parse` (a wall-clock-bounded tree-sitter parse so a huge file can't blow the
  hot-path budget).
- `hooks/` — maintenance hooks that bypass the master switch (`session_reset`,
  `update_check`); tool hooks go through `engine::Dispatcher`.
- `commands/` — CLI / slash-command backends only (`gain`/`status`/`config`/`resolve`).
- `cli/` — `Cli` (clap root) + `Command` (subcommands + dispatch).
- `stats/` — `SQLite` event store (`db`/`recorder`/`tracking`) + pricing.
- `paths` / `tokens` / `relevance` / `clock` / `panic_guard` — dependency-free leaf
  utils (state dirs, the token-count heuristic, the shared error-marker relevance
  test, the Unix-seconds clock, and the `catch_unwind` exit-0 guard for the three
  hooks that run outside `engine::Dispatcher`).
- `lib.rs` — module tree; `main.rs` — thin wrapper over `cli::run`.

Tests live **outside `src/`** in `tests/{unit,integration,e2e}/` (mirroring
`src/`, files `<name>.tests.rs`), plus a real-Claude-Code, network-isolated
`tests/docker/` tier (Node harness + a fake Anthropic server) — see
`.claude/rules/testing.md` and `tests/docker/README.md`.

## Detailed rules

See [`.claude/rules/`](.claude/rules/): `architecture.md`, `hook-protocol.md`,
`rust-conventions.md`, `dependencies.md`, `testing.md`.

## How it works

A layered, one-type-per-file architecture. Rust pinned to 1.96
(`rust-toolchain.toml`), edition 2024, `unsafe_code = "forbid"`. Four test tiers —
unit, integration, e2e, and a network-isolated `tests/docker/` tier that runs the
real `claude` CLI headless against a fake model server, covering every surface and
all 29 Read languages.

`read` compacts over 29 languages in **3 modes** (`optimizers.read.mode`
soft/medium/high): soft strips comments byte-identically; medium/high collapse code
(single-line-safe langs via an origin-map view, others via whitespace). It's
**Edit-safe**: `edit-fix` and the live `snip resolve` map a compacted `old_string`
back to real bytes — AST-anchored fuzzy (LCS 0.85 + length-ratio 0.80) for soft, the
origin map for medium/high (+ `reexpand` of a collapsed `new_string`); `write-guard`
asks before a Write reproduces the stripped view; identical re-reads dedupe to a
session notice (cleared at `PreCompact`).

`search`/`command` **rewrite** output (not just truncate), then the shared
overflow/spill service (Head/Tail/Middle/**RelevanceFirst**) caps the shown view and
spills the full rewritten output recoverably. `search` reads the real Claude Code
shapes — Grep's top-level `content` and Glob's `filenames` path array (grouped by
directory via the `auto` group key). `command` (`bash-route` + `exec`)
segments POSIX lines, optimizes via per-unit **sentinels** (110 s exec timeout),
injects structured flags + parses machine formats, ships a **base/git/lang catalog**,
gates specs via per-command/family **`rules`**, and **auto-detects** unrecognized
output — JSON (minify/TOON) or a repetitive log (fingerprint-folded). Closed
vocabulary (no regex) — **transforms**
Parse/Dedupe/Group/Keep/Drop (`StrPred`)/Rank/Template/Truncate/Project/StripAnsi/Squeeze/
Fingerprint/FoldFrames/FoldDiff/TestRollup; **ParseFormats**
git-status-v2/cargo-json/eslint-json/ruff-json/go-test-json/jest-json + JSON encoders
json-minify/json-array-table (TOON)/table-collapse. **Everything is configurable**
(`config set` + user `specs[]`).

Stats are a **SQLite** event store (`rusqlite` bundled). The hot path never
opens `SQLite` and never forks: `record_*` append one line to `<data_dir>/events.log`
(a single `O_APPEND` write); a **drain** folds that log into `SQLite` **off the hot
path** — from `gain`/`status` and the `SessionStart` update-check. `gain`/`status`
report NET (gross saved − induced spill re-reads). Run the four checks (fmt, clippy
`--all-targets`, test, build --release) before every commit.
