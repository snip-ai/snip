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

> The `/plugin` commands above are for the Claude Code **CLI**. In the **integrated** Claude Code (IDE extension / desktop app), add the marketplace and install snip from the **Manage plugins** panel instead.

On first run the plugin downloads the prebuilt binary for your platform from the
matching GitHub release, verifies its checksum, and installs it under your OS data
dir; hooks then run it by absolute path. It also adds a clearly-marked, removable
line to your shell rc so you can run `snip …` directly (undo with
`/snip shell-setup remove`). Install **and** updates both flow through the plugin —
there is no separate installer, and snip never patches your `settings.json`.

## Updates

Two layers, two channels. The **binary** self-updates: on `SessionStart` snip
checks the latest GitHub release in the background and fetches it (checksum-verified)
when the running binary is older — independent of the plugin manifest version, which
a third-party marketplace does not auto-refresh. `snip update` forces a check now.
The **plugin wiring** (hooks, the `/snip` command, manifest) refreshes only when
Claude Code re-pulls the marketplace;
enable auto-update for the `snip` marketplace once (`/plugin` menu, or
`"autoUpdate": true` on its `extraKnownMarketplaces` entry), or run
`/plugin marketplace update snip` + `/plugin install snip@snip` on demand.

## Commands — from your shell

snip adds its binary to your **`PATH`** at first install (one clearly-marked,
removable line in your shell rc), so the meta-commands run straight from a git
bash shell, with **no model turn**:

| From a shell | What it does |
|---|---|
| `snip status` | Version, master switch, and per-optimizer state |
| `snip gain` | Net token savings (input saved − induced re-read cost) |
| `snip config …` | Get / set / list / reset configuration (dotted paths) |
| `snip enable` · `snip disable` | Master switch on / off |
| `snip update` | Check for the latest release and fetch the binary if newer |

No terminal handy? The same commands are one slash-command in Claude Code —
`/snip <sub>` (e.g. `/snip status`) — which is also where `/snip shell-setup`
(add or `remove` the `PATH` line) and `/snip uninstall` (full teardown, removing
the binary too) live. This only reaches the **already-installed** binary; install
and updates still flow only through the plugin.

## Platforms

macOS (arm64 / x64), Linux (x64), Windows (x64). Hooks run through **bash**; on
Windows that means **Git Bash** (already required by Claude Code's Bash tool). If
no POSIX shell or supported prebuilt binary is available, the hooks degrade to
pass-through.

## License

Apache-2.0.
