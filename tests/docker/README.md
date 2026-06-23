# `tests/docker` — real-Claude-Code e2e tier (isolated, offline)

This is the fourth, highest-fidelity test tier. The other three
(`tests/{unit,integration,e2e}`) prove snip's **hook-contract implementation**:
"given this hook JSON, snip emits that JSON." None of them prove that **real
Claude Code** loads the plugin, fires the hooks on the real tool surfaces, hands
snip the JSON shapes it actually expects, and feeds snip's rewritten output back
to the model. That integration boundary is exactly where a Claude Code release
can silently break snip while every other test stays green.

This tier closes that gap. It drives the **real `claude` CLI**, headless, with
snip installed the way the plugin installs it, against a **fake model server** —
so it is deterministic, free, and offline.

> **It already earned its keep:** it caught that snip silently no-op'd on real
> `Glob` output. Claude Code returns Glob as `{ filenames: [...] }` (a path
> array, no `content`), but the Rust e2e fixture fed `{ file: { content } }`, so
> the unit/e2e suites were green while production did nothing. The fix is in
> `ToolResponse` (handle the `filenames` shape); the e2e fixtures were corrected
> to the real shapes captured here.

## Isolation model

Everything runs **inside a Docker container**. The host only runs `docker build`
+ `docker run --network=none`. Consequences:

- Nothing touches your machine — no host `claude` auth/config, no host file
  pollution, no host network.
- `--network=none` leaves only loopback, so the suite **physically cannot reach
  the real Anthropic API**. The runner also hard-refuses to start without a
  fake-server base URL.
- No secrets. No API key. No cost.

### Why not `testcontainers`?

`testcontainers-rs` exists, but it is the wrong shape here, twice over: (1) it is
`tokio`/async, which [`dependencies.md`](../../.claude/rules/dependencies.md)
explicitly rejects ("no tokio/async"); and (2) it is built to spin up *service
dependencies* (Postgres, Kafka…) that a **host** test talks to — the opposite of
what we want, which is to run the **entire** test inside an isolated container.
The right fit is a self-contained image whose entrypoint runs the suite, launched
by `docker run`. Orchestration stays dependency-free (`docker` CLI + a shell
script), matching the project's "≤20 lines of std" ethos.

## How it works

```
node test process ──spawn──> claude -p (headless)
        │                        │  PreToolUse/PostToolUse hooks
        │                        ▼
        │                   snip-run.sh ──> snip binary (rewrites tool output)
        │                        │
        ▼                        ▼
  MockAnthropic  <──HTTP (loopback /v1/messages)── claude's agent loop
  (records every request)
```

The mock is also the **observation point**: every request body carries the full
message history, so the `tool_result` Claude Code sends back after a hook ran **is
exactly what the model received**. We assert on that — no breadcrumbs, no guessing
— which is why it proves the whole chain (CC dispatch → plugin wrapper → snip →
`updatedToolOutput` → CC consumption → model context), not just snip in isolation.

Turn selection is driven by conversation state (count of `tool_result`s seen +
whether the request advertises `tools`), never a blind request counter, so an
auxiliary request can't desync a scenario.

## The phases

| Phase | File | What it proves |
|---|---|---|
| **A — contract drift** | `phase-a-contract.test.mjs` | A pass-through spy hook captures the verbatim payloads real Claude Code sends, and asserts the exact fields snip's optimizers read are still present (Read `file.content`, Grep top-level `content`, Glob `filenames` array, Bash `tool_input.command`). Fails loudly on shape drift. |
| **B — conformance** | `phase-b-conformance.test.mjs` | The model receives snip's **compacted** view for Read/Grep/Glob; Bash is wrapped transparently; source files are byte-identical after the run (snip never writes user sources); a missing binary degrades to raw passthrough with no hook error. |
| **B — all languages** | `phase-b-languages.test.mjs` | One commented fixture **per supported language (29)** is read through the real Read tool; each must arrive as `[snip: read \| <lang>` with the buried marker comment stripped — proving correct detection + compaction across every language. |
| **B — Bash families** | `phase-b-bash.test.mjs` | The command optimizer through the real Bash tool: base-shell families that compact on overflow (`ls`/`find`/`grep`), the git family (`git status` → git_status_v2, `git log` → one-line-per-commit), and transparency (small output + shell operators/pipes reach the model byte-for-byte). Framework families (cargo/eslint/ruff/go-test/jest) are unit-tested in Rust against captured output. |
| **B — Edit/Write** | `phase-b-edit-write.test.mjs` | An Edit applies end-to-end (edit-fix passthrough), a Write creates a new file (write-guard passthrough), and the documented live recovery `snip resolve` maps a comment-stripped block back to real bytes through the real Bash tool. |
| **B — Grep modes** | `phase-b-grep-modes.test.mjs` | Grep `output_mode` variants beyond `content`: `files_with_matches` and `count` reach the model intact. (Grouping `files_with_matches` is a tracked enhancement — see caveats.) |
| **B — lifecycle** | `phase-b-lifecycle.test.mjs` | The plugin self-install path: `snip-bootstrap.sh` (and `snip-run.sh update-check`) download + verify + install the binary from a loopback fake release server. |
| **C — efficacy** | `phase-c-efficacy.test.mjs` | Replays the **same** tool calls with snip off then on and diffs the tokens of the tool_results the model received — a precise, deterministic savings figure (e.g. Read −39%, Grep −80%, Glob −31%). Asserts snip never inflates and nets a reduction. |

## Running

```bash
# Canonical, fully isolated (build the image + run all phases, no network):
tests/docker/run-docker.sh

# A subset:
tests/docker/run-docker.sh tests/docker/phase-a-contract.test.mjs

# Pin a different Claude Code version:
CLAUDE_VERSION=2.1.181 tests/docker/run-docker.sh
```

### Local dev (not isolated)

The harness is plain Node (stdlib only) and also runs on the host if you have a
`claude` CLI on `PATH` and a built binary (`cargo build --release`, or
`SNIP_TEST_BINARY=/path/to/snip`). This is a convenience for fast iteration and
is **not** isolated — the canonical, shippable path is Docker.

```bash
node tests/docker/harness/mock.selftest.mjs          # mock protocol, no claude
node --test tests/docker/phase-a-contract.test.mjs   # one phase
```

## Harness layout (`harness/`, Node stdlib only)

- `mock-anthropic.mjs` — the fake Messages API server + observation helpers.
- `sse.mjs` — streaming/non-streaming response encoders.
- `scenario.mjs` — scripted assistant turns (deterministic tool_use ids).
- `claude-runner.mjs` — spawn `claude` headless; pinned to the mock, never the
  real API.
- `install.mjs` — install the binary into an isolated `$SNIP_HOME`; generate the
  settings.json `hooks` block (production `snip-run.sh`, or the spy).
- `run.mjs` — per-test glue (`makeWorkspace`, `runScenario`, temp cleanup).
- `fixtures.mjs` / `binary.mjs` / `lib.mjs` — fixtures, binary resolution, leaf
  utils (checksums, the chars/4 token estimate, `claude` detection).
- `spy.mjs` — the pass-through capture hook (Phase A).

## Notes & caveats

- **Claude Code flags discovered empirically** (validated against `claude`
  2.1.175): hooks load via `--settings` only **without** `--bare` (which strips
  auto-discovery); non-core tools (Grep/Glob) require `--allowedTools`. If a
  future release changes these, the runner is the one place to adjust.
- **Arch:** snip's release matrix ships Linux as `x86_64-unknown-linux-musl`. The
  lifecycle phase asserts the bootstrap install only on amd64 Linux; on other
  arches it verifies the documented graceful no-op. CI runs amd64.
- **Pinned version:** the image pins `@anthropic-ai/claude-code`. Bump it via
  `CLAUDE_VERSION` and let Phase A tell you whether the tool_response shapes
  drifted.
- **Bugs found & fixed here:** snip silently no-op'd on real Grep
  `files_with_matches` output (a bare path list on the grep surface that previously
  grouped *by file*, not by dir) — now grouped by directory via the search spec's
  `auto` group key, asserted by the Grep-modes phase. The earlier Glob `filenames`
  no-op was fixed the same way (in `ToolResponse`).
- **Toolchain:** Rust is pinned to 1.96 (`rust-toolchain.toml`, edition 2024); the
  build image's `rust:1.96-bookworm` matches it — bump both together.
