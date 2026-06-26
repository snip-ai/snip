---
description: "snip: add or remove a PATH line so `snip` runs directly from a shell"
argument-hint: "setup | remove"
model: haiku
effort: low
disable-model-invocation: true
---

!`bash "${CLAUDE_PLUGIN_ROOT}/scripts/snip-cmd.sh" shell-setup $ARGUMENTS`

Relay the command output above to the user verbatim, inside a fenced code block. Add no preamble, commentary, or summary.
