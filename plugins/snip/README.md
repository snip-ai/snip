# snip (Claude Code plugin)

**Token & context optimizer for Claude Code.** A single native binary, wired
entirely through hooks, that rewrites token-heavy tool output **in-flight** —
with **zero markdown injected** into the model's context and **no MCP tools to
learn**.

## What it does

- **Read** — strips comments via tree-sitter across **29 languages**
  (`soft` / `medium` / `high` modes), keeping code lines **byte-identical** so
  your Edits still apply. Typically 30–70% fewer tokens per code read.
- **Bash** — rewrites the output of **60+ commands** (`git`, the base shell
  toolkit, and the major language toolchains); unrecognized output is
  auto-compacted (JSON or repetitive logs).
- **Grep / Glob** — rewrites (not just truncates) results, grouping noisy path
  lists by directory.
- **Recoverable overflow** — huge output is compacted *first*, then spilled to a
  session file with a one-line breadcrumb; nothing is ever discarded.

Savings are reported **net** (input saved − induced re-reads). Every hook
**exits 0 no matter what** — if optimization can't run, you get the original
output unchanged — and snip **never writes to your source files**.

## Install

```
/plugin marketplace add snip-ai/snip
/plugin install snip@snip
```

On first run the plugin downloads the prebuilt binary for your platform from the
matching GitHub release, verifies its checksum, and installs it under your OS data
dir; hooks then run it by absolute path. Install **and** updates both flow through
the plugin — there is no separate installer, and snip never patches your
`settings.json`.

## Slash-commands

| Command | What it does |
|---|---|
| `/snip-gain` | Net token savings (input saved − induced re-read cost) |
| `/snip-status` | Version, master switch, and per-optimizer state |
| `/snip-config` | Get / set / list / reset configuration (dotted paths) |
| `/snip-enable` · `/snip-disable` | Master switch on / off |
| `/snip-update` | Re-check the version and fetch the matching binary |

## Platforms

macOS (arm64 / x64), Linux (x64), Windows (x64). Hooks run through **bash**; on
Windows that means **Git Bash** (already required by Claude Code's Bash tool). If
no POSIX shell or supported prebuilt binary is available, the hooks degrade to
pass-through.

## License

Apache-2.0.
