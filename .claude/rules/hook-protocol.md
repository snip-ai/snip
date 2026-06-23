# Claude Code hook protocol

stdin/stdout are **exclusively** for the hook JSON protocol. All logging, debug
output, and errors go to **stderr only**. Empty stdout + exit 0 means "no change"
(Claude Code keeps the original).

## Authoritative tool field names

Getting these wrong makes a hook silently no-op (deserialization fails → exit 0).

- **`Read`** `tool_input`: `file_path` (+ optional `offset`/`limit`).
- **`Edit`** `tool_input`: `file_path`, `old_string`, `new_string`, optional `replace_all`.
- **`Write`** `tool_input`: `file_path`, `content`.
- **`Bash`** `tool_input`: `command`, optional `description`, `timeout`, `run_in_background`.
- **`Grep`/`Glob`** `tool_input`: `pattern` (+ `path`, `glob`, …).

Do **not** use `path` / `old_str` / `new_str`.

`Read`'s `tool_response` (current Claude Code) is a nested object — raw content,
no `cat -n` prefixes: `{ "type": "text", "file": { "filePath", "content",
"numLines", … } }`. Accept the legacy string and `{"content": …}` shapes too.

## Hook outputs

- **PostToolUse** (`read-hook`, `grep-hook`, `glob-hook`) → replace what the model
  sees: `{ "hookSpecificOutput": { "hookEventName": "PostToolUse",
  "updatedToolOutput": … } }`. **Schema-validated**: it must be the **same nested
  object shape** as the incoming `tool_response`, with only `file.content` (and
  `file.numLines`) replaced — never a bare string. Mutate the original JSON in
  place so every other field round-trips.
- **PreToolUse** (`bash-route`, `edit-fix`) → `updatedInput` replaces the **entire**
  `tool_input`, so re-emit every field you aren't changing.
- **PreToolUse** (`write-guard`) → `permissionDecision: "ask"` with a reason.
- **PreCompact** (`session-reset`) → no output; deletes the session cache.
- **SessionStart** (`update-check`) → no output; detached, never blocks startup.

## Pre-hook validation limit (Claude Code ≥ 2.1.x)

The `Edit` tool validates `old_string` against the real file **before** PreToolUse
hooks run. A non-matching `old_string` aborts before `edit-fix` ever runs. The
live recovery path is **`snip resolve <file>`** (the model pipes the failing
`old_string` to stdin and retries with the stdout); the compact guidance header
documents this. The `edit-fix` hook stays for versions that honor `updatedInput`.

## Command surface (Bash) specifics

`bash-route` rewrites the command to `snip exec -- <base64>` and **snip runs it**
on the exact bytes via the real shell (semantics preserved). Anti-recursion:
commands already starting with `snip exec` / carrying `SNIP_WRAPPED=1` pass
through untouched. Never wrap interactive/streaming/backgrounded/non-POSIX
commands — a hanging hook is the worst failure.

## Safety invariants (non-negotiable)

- Every hook **exits 0 under all circumstances** — catch all errors *and panics*
  (`catch_unwind`) at the top level (centralized in `panic_guard::guarded`, used by
  `engine/dispatcher.rs` and the maintenance hooks); the release profile keeps
  `panic = "unwind"`. **Dev escape hatch:** `SNIP_DEBUG=1` (strict mode,
  `panic_guard::strict`) re-raises a caught error/panic as a **non-zero exit** with
  the message on stderr, so a developer sees failures instead of a silent
  pass-through. Off by default and in every plugin-installed session — the exit-0
  invariant is unconditional for real users.
- snip **never writes to user source files**; its state lives under the data dir.
- If optimization fails, return the **original unchanged**.
- Fuzzy-match threshold **0.85** (+ length-ratio ≥ 0.80) — do not lower.

## Install (plugin only)

Hooks are registered **declaratively by the Claude Code plugin** (`plugins/snip/`)
— there is no `settings.json` patching, no `init`, and no curl/PowerShell
installer. Install and updates both flow through the plugin.
