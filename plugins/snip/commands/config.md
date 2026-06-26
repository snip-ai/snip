---
description: Get, set, list, or reset snip configuration (optimizer modes, toggles, overflow, etc.). Use when the user asks to view or change a snip setting. Pass the subcommand and args, e.g. `set optimizers.read.mode high`, `get <key>`, `list`, `reset`.
argument-hint: "get <key> | set <key> <value> | list | reset"
---

!`bash "${CLAUDE_PLUGIN_ROOT}/scripts/snip-cmd.sh" config $ARGUMENTS`

The block above is the output of `snip config`. Relay the result to the user concisely.
