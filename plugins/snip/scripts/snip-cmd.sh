#!/usr/bin/env bash
# snip meta-command dispatcher backing the per-purpose `/snip:<name>` slash-commands
# (commands/<name>.md each call this with their subcommand). Cold path (never a tool
# hook): routes one subcommand to the right backend. Most go to the binary via
# snip-run.sh; `shell-setup` is a pure rc-file edit handled by its own script so it
# works even before the binary is installed.
set -u
DIR="${0%/*}"

case "${1:-}" in
  ""|help|-h|--help)
    printf 'snip-cmd.sh: backend for the /snip:<name> commands — pass a subcommand (status|gain|config|enable|disable|update|shell-setup|uninstall) [args].\n' ;;
  shell-setup) shift; exec bash "$DIR/snip-shell-setup.sh" "$@" ;;
  uninstall)   exec bash "$DIR/snip-uninstall.sh" ;;
  *) exec bash "$DIR/snip-run.sh" "$@" ;;
esac
