# Architecture

snip is a single static binary with subcommands, shipped as a Claude Code plugin.
This file is the working module map.

## The shape

A **layered / onion** architecture: dependencies point inward to `domain`. The
engine turns every tool surface into a normalized event, finds the first matching
optimizer, and serializes the result back into the right hook JSON. Optimizers
never touch the wire format.

```
Claude Code hook ──> engine::Dispatcher ──> engine::Registry (by surface)
                          │                       │ first_match
                          │                       ▼
                          │                 domain::Optimizer
                          │                  ├─ optimizers::SpecOptimizer (declarative data) ← default
                          │                  └─ optimizers::read (Rust AST escape hatch)
                          ▼
              engine::OutcomeSerializer (+ overflow + stats)
```

## Layers (inner → outer)

- **`domain/`** — the model, dependency-light (serde + `config`). One file per
  type: `Surface`, `HookCtx`, `Outcome`, and the `Optimizer` trait (the port /
  Rust escape hatch). No I/O, no wire format.
- **`engine/`** — dispatch machinery: `Dispatcher` (the shared hook contract:
  stdin → optimizer → stdout, always exit 0), `Registry` (the current surface's
  optimizers, built per run, `first_match`), `ToolResponse` (the
  `tool_response` wire shapes — Read/Grep/Glob/bare: extract + shape-preserving
  rewrite), `OutcomeSerializer`
  (`Outcome` → hook JSON).
- **Adapters** — `optimizers/` (**every `Optimizer` impl**: `read/` AST escape
  hatch, `command/` Bash runtime, `search/` declarative, `SpecOptimizer` the
  declarative adapter, `redact` the `secret_safe` service), `spec/` (the
  declarative model + vocabulary + per-transform impl modules + `builtin` JSON
  specs), `cli/` (`Cli` + `Command`), `hooks/`, `commands/` (CLI backends).
- **Infrastructure** — `config/` (layered `Config`), `overflow/` (the cap/spill
  service + `OverflowCfg`), `languages/` (`LanguageSpec` + registry), `compaction/`
  (the `read` engine), `stats/` (SQLite).
- **Leaf utils** — `paths`, `tokens` (the token-count heuristic), `relevance`
  (the shared error-marker test), `clock` (the Unix-seconds clock), `panic_guard`
  (the `catch_unwind` exit-0 guard for hooks outside `Dispatcher`); dependency-free,
  beneath every layer.

## Module organisation (`src/`, one public type per file)

**One public type per file**, `file = snake_case(Type).rs`; `mod.rs` holds only
`mod`/`pub use` (re-export facades). Exception: a type whose file would equal its
parent module lives in that `mod.rs` (avoids `clippy::module_inception`) — only
`Config` (`config/mod.rs`) and `Cli` (`cli/mod.rs`). Soft limit 150 lines/file,
hard 200 — the one documented over-limit exception is `languages/registry.rs` (a
single cohesive `SPECS` data table; its `grammar` fn-pointer can't be JSON).

```
src/
├── lib.rs            # module tree (enables integration tests)
├── main.rs           # thin wrapper over cli::run
│
├── domain/           # surface.rs · hook_ctx.rs · outcome.rs · optimizer.rs
├── engine/           # dispatcher.rs · registry.rs · tool_response.rs · outcome_serializer.rs
├── config/           # mod.rs(Config + load/save) · accessors.rs · optimizer_cfg.rs · autodetect_cfg.rs · compact_mode.rs
├── spec/             # optimizer_spec.rs · bind.rs · transform.rs(enum+dispatch) · <one module per transform> · group_key/rank_key/str_pred/fp_window · parse_format.rs · formats/ · builtin/(specs/*.json)
├── optimizers/       # spec_optimizer.rs · redact.rs · read/(AST) · command/(Bash runtime) · search/(declarative)
├── overflow/         # mod.rs · overflow_cfg.rs · elide_strategy.rs · spill.rs
├── languages/        # language_spec.rs · registry.rs (SPECS + detect)
├── compaction/       # compactor.rs · single_line.rs · whitespace.rs · collapse.rs · reexpand.rs · code_lines.rs · parse.rs (wall-clock-bounded parse)
├── hooks/            # session_reset.rs · update_check.rs (bypass the master switch)
├── commands/         # gain · status · config_cmd · resolve (CLI backends)
├── cli/              # mod.rs(Cli + run) · command.rs (Command + dispatch)
├── stats/            # db.rs · recorder.rs · tracking.rs · event.rs · summary.rs · pricing.rs (SQLite)
└── paths.rs · tokens.rs · relevance.rs · clock.rs · panic_guard.rs   # dependency-free leaf utils
```

Tests live **outside `src/`** in `tests/{unit,integration,e2e}/`, mirroring the
`src/` tree — see [`testing.md`](testing.md).

## Where new code goes

- A new optimized command → a JSON spec in `spec/builtin/specs/` **or** user
  config (zero recompile); a new transform → a `Transform` variant + a `spec/<name>.rs`
  impl module + one `apply` arm.
- A new surface → a `Surface` variant (classify it in **both** `is_post` and
  `name` — both exhaustive `match`es the compiler forces) + a `Command` arm
  (route an output/Post surface through `Dispatcher`; an input-rewrite surface
  through its own `bash-route`-style runtime) + a `hooks.json` entry + (if it can
  use specs) a `families_for` arm in `spec/builtin`.
- A new overflow elision strategy → an `ElideStrategy` variant + its `keep_*` helper.
- A genuinely complex optimizer a spec can't express → `optimizers/<name>/`.
- A new domain concept → its own file in the owning module; never a second public
  type in an existing file.
- Settings read on the hot path → `config/`; a setting intrinsic to one subsystem
  lives with that subsystem (e.g. transform params in `spec/`, `OverflowCfg` in
  `overflow/`), not centralized in `config/`.

## Surfaces → optimizers

Read → `read` · Grep/Glob → `search` · Bash → `command` · Edit/Write → `read`
(`edit-fix` / `write-guard`). Surfaces are disjoint by tool, so first-match-wins never
conflicts. `PreCompact` (`session_reset`) and `SessionStart` (`update_check`) run
even when snip is disabled.

## Performance notes

- Grammars load lazily; the registry is built once per run (no global cache —
  each hook process serves one surface) and parses only that surface's spec
  families (`builtin_specs_for`), so a Read hook parses no specs at all.
- Config is a tiny JSON read once per process — never SQLite on the hot path.
- `crate::tokens::estimate_tokens` (heuristic) everywhere — hot path and the
  detached stats recorder alike; exact tiktoken is intentionally **not** a
  dependency (see `dependencies.md`), so token figures are labeled an estimate.
- `Dispatcher` is a zero-cost `Copy`-sized struct; `ToolResponse` borrows, it
  never owns the response.
