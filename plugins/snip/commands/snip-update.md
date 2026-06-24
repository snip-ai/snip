---
description: Check for the latest snip release and fetch the binary if newer
model: haiku
effort: low
disable-model-invocation: true
---

!`bash "${CLAUDE_PLUGIN_ROOT}/scripts/snip-run.sh" update-check --force && echo "snip: checked the latest release — if a newer binary exists it is fetched in the background and is active next session."`

Relay the command output above to the user verbatim, inside a fenced code block. Add no preamble, commentary, or summary.
