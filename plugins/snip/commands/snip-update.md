---
description: Re-check the snip version and fetch the matching binary automatically
model: haiku
effort: low
disable-model-invocation: true
---

!`bash "${CLAUDE_PLUGIN_ROOT}/scripts/snip-run.sh" update-check && echo "snip: version re-check triggered — if the plugin version changed, the matching binary is fetched in the background and is active next session."`

Relay the command output above to the user verbatim, inside a fenced code block. Add no preamble, commentary, or summary.
