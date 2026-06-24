<div align="center">

<img src="https://raw.githubusercontent.com/snip-ai/snip-ai.github.io/refs/heads/main/assets/img/logo.svg" alt="snip logo" width="120" />

# snip 🦀✂️

### The token & context optimizer for Claude Code

**snip is a tiny, blazing-fast Rust binary that plugs into Claude Code and quietly trims the fat from everything Claude reads — your files, your command output, your searches — so you burn fewer tokens and your context window lasts a lot longer.**

No config to learn. No clutter in the chat. Nothing the model has to think about. It just works.

[![Version](https://img.shields.io/github/v/release/snip-ai/snip?label=version&sort=semver)](https://github.com/snip-ai/snip/releases/latest)
[![CI](https://github.com/snip-ai/snip/actions/workflows/ci.yml/badge.svg)](https://github.com/snip-ai/snip/actions/workflows/ci.yml)

</div>

---

## 🤔 Why should I care?

Claude reads **a lot**. Every file you open, every `git diff`, every `grep`, every test run — it all gets stuffed into the context window, and every token costs you money and eats into the space Claude has left to actually think.

The thing is, most of that text is noise. Doc comments Claude already understood. The 400 lines of a stack trace that repeat the same frame. The `node_modules` paths in your search results. snip strips the noise and keeps the signal — **automatically, on every tool call, before it ever reaches the model.**

> 💡 **The result:** typically **30–70% fewer tokens** on code reads, a context window that lasts way longer, and a smaller bill — with zero change to how you work.

---

## ✨ What makes snip different

Plenty of tools try to save tokens for Claude Code. Most stop at the Bash tool, use fragile regex to *guess* where your comments end (and quietly break Claude's edits), and brag about "savings" that ignore the re-reads they cause. snip was built to fix exactly those three things.

| | **snip** | Typical token optimizers |
|---|:---:|:---:|
| **Surfaces optimized** | Read · Bash · Grep · Glob | Bash only |
| **Code understanding** | tree-sitter **AST** (29 languages) | regex guesswork |
| **Keeps Claude's edits working** | ✅ byte-identical code | ❌ can corrupt edits |
| **Savings reported** | **NET** (saved − re-reads) | gross (inflated) |
| **Footprint in the model's context** | **zero** — no markdown, no MCP tools | varies |
| **When output is huge** | rewrites, then spills it **recoverably** | truncates / drops it |

In plain English: snip understands your code instead of pattern-matching it, covers every place tokens leak (not just the terminal), and is **honest** about what it saves — it counts the cost of any re-read against the gain, so the number you see is the number you actually keep.

---

## 🚀 Install

snip ships as a **Claude Code plugin** — install *and* updates both flow through it. No curl-pipe-bash, no package manager, no editing `settings.json`.

```text
/plugin marketplace add snip-ai/snip
/plugin install snip@snip
```

> The `/plugin` commands above are for the Claude Code **CLI**. In the **integrated** Claude Code (IDE extension / desktop app), add the marketplace and install snip from the **Manage plugins** panel instead.

On first run, the plugin grabs the prebuilt binary for your platform, verifies its checksum, and drops it in your OS data dir. Hooks call it by absolute path from there. That's it — you're optimizing.

**Supported:** macOS (arm64 / x64), Linux (x64), Windows (x64). On Windows the hooks run through Git Bash (which Claude Code already needs). If anything's missing, snip safely does nothing and you get your original output back.

---

## 🧠 How it works

snip hooks into Claude Code's **native tools** and rewrites their output in flight — so the model just receives a leaner version, with no idea snip was ever there.

- **📖 Read** — strips comments via tree-sitter across **29 languages**, with `soft` / `medium` / `high` modes. Code lines stay **byte-identical**, so Claude's follow-up Edits still land perfectly. (And if an Edit ever drifts, `snip resolve` maps it back to the real bytes.)
- **💻 Bash** — recognizes **60+ commands** out of the box and rewrites their output: `git`, the base shell toolkit (`ls`, `grep`, `find`, `docker`, `kubectl`…), and the major language toolchains (`cargo`, `npm`, `pytest`, `go test`, `gradle`, `dotnet`, `eslint`…). Don't recognize it? snip auto-compacts JSON and folds repetitive logs anyway.
- **🔎 Grep / Glob** — rewrites (not just truncates) results, grouping noisy path lists by directory so you see the shape, not the spam.
- **🛟 Never loses your data** — when output is genuinely huge, snip compacts it *first*, then spills the full version to a session file with a one-line breadcrumb. Recoverable, always.

Everything runs on a strict **sub-15ms budget** (most reads finish in 1–2ms), every hook **exits cleanly no matter what**, and snip **never writes to your source files**.

---

## 🎛️ Usage

snip works the moment it's installed — there's nothing to run. When you want to peek under the hood, use the slash-commands:

| Command | What it does |
|---|---|
| `/snip-gain` | See your token savings (the honest **net** number) |
| `/snip-status` | Version, master switch, and per-optimizer state |
| `/snip-config` | Get / set / list / reset settings |
| `/snip-enable` · `/snip-disable` | Flip the master switch |
| `/snip-update` | Sync the binary with the installed plugin |

---

## ⚙️ Configuration

Sane defaults, fully tunable. Settings layer cleanly: **built-in → your user config → an opt-in, trust-gated project layer** (`.snip/config.json`). Tweak a mode, toggle an optimizer, or add your own command rules — all as plain JSON, **no recompile**:

```text
/snip-config set optimizers.read.mode high
```

The transform vocabulary is **closed and declarative** — no regex, no scripting, no remote-code-execution surface hiding in a config file. Safe by design.

---

## 🌍 Supported languages

29 and counting:

> Rust · Python · JavaScript · TypeScript · TSX · Go · C · C++ · Java · Ruby · Bash · C# · PHP · CSS/SCSS · Lua · Elixir · Kotlin · Scala · YAML · TOML · SQL · HTML · Swift · Dart · R · Zig · Julia · Haskell · Objective-C

---

## 🛠️ Building from source

You don't need this to use snip — but if you want to hack on it:

```bash
cargo build --release                        # optimized binary
cargo test                                   # full suite (unit + integration + e2e)
cargo fmt --check                            # formatting
cargo clippy --all-targets -- -D warnings    # lints (all / pedantic / nursery denied)
```

Rust is pinned to **1.96** (edition 2024) via `rust-toolchain.toml`. See [`CONTRIBUTING.md`](CONTRIBUTING.md) and [`.claude/rules/`](.claude/rules/) for the architecture and conventions — PRs welcome. 🙌

---

<div align="center">

Made with ❤️ by **[Aymeric Pasco](https://buymeacoffee.com/aymericp)** & the snip contributors.

If snip saved you some tokens (and a few bucks), maybe [buy me a coffee ☕](https://buymeacoffee.com/aymericp)

</div>
