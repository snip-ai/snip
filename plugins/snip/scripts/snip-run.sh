#!/usr/bin/env bash
# snip plugin front door — invoked by every hook (see hooks/hooks.json).
#
# Resolve the managed binary under the OS data dir and exec it, forwarding all
# args + stdin untouched. The exception is update-check/update: the binary decides
# (drain + throttle) but the BOOTSTRAP is spawned from HERE, because a native .exe
# can't spawn a shell that survives its own exit on Windows — bash can (nohup). On
# the very first run (binary absent) this is also where the install bootstrap fires,
# without ever blocking a hook. This script ALWAYS exits 0: a hook must never fail
# (empty stdout + exit 0 = "no change", Claude Code keeps the original).
set -u

# This front door runs on every tool call, so keep its own overhead minimal:
# probe the OS once (not per use) and derive the script dir by parameter
# expansion instead of a `dirname` + `cd` subshell.
SUB="${1:-}"
SCRIPT_DIR="${0%/*}"
OS="$(uname -s)"

snip_home() {
  if [ -n "${SNIP_HOME:-}" ]; then printf '%s' "$SNIP_HOME"; return; fi
  case "$OS" in
    Darwin)               printf '%s' "$HOME/Library/Application Support/snip" ;;
    MINGW*|MSYS*|CYGWIN*) printf '%s' "${APPDATA:-$HOME/AppData/Roaming}/snip" ;;
    *)                    printf '%s' "${XDG_DATA_HOME:-$HOME/.local/share}/snip" ;;
  esac
}

case "$OS" in
  MINGW*|MSYS*|CYGWIN*) BIN_NAME="snip.exe" ;;
  *)                    BIN_NAME="snip" ;;
esac

HOME_DIR="$(snip_home)"
BIN="$HOME_DIR/bin/$BIN_NAME"

plugin_version() {
  f="${CLAUDE_PLUGIN_ROOT:-}/.claude-plugin/plugin.json"
  [ -f "$f" ] || return 0
  sed -n 's/.*"version"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$f" | head -n1
}

if [ "$SUB" = "update-check" ] || [ "$SUB" = "update" ]; then
  if [ ! -x "$BIN" ]; then
    # First install (binary absent): trigger the detached bootstrap from the
    # designated install moment (SessionStart, the `update-check` hook). The
    # `.uninstalled` marker (written by `snip uninstall`; KEEP IN SYNC with
    # src/commands/uninstall.rs) suppresses it so a teardown is not silently undone
    # before the plugin is removed.
    if [ "$SUB" = "update-check" ] && [ -f "$SCRIPT_DIR/snip-bootstrap.sh" ] && [ ! -f "$HOME_DIR/.uninstalled" ]; then
      nohup bash "$SCRIPT_DIR/snip-bootstrap.sh" "$(plugin_version)" "$HOME_DIR" >/dev/null 2>&1 &
    fi
    exit 0
  fi
  # Binary present: it drains stats (and prints the message for `update`) and drops
  # a `.fetch-due` sentinel (KEEP IN SYNC with src/hooks/update_check.rs) when the
  # 24h throttle is up. `update` always fetches; otherwise fetch only when flagged.
  "$BIN" "$@"
  if [ -f "$SCRIPT_DIR/snip-bootstrap.sh" ] && [ ! -f "$HOME_DIR/.uninstalled" ] \
     && { [ "$SUB" = "update" ] || [ -f "$HOME_DIR/.fetch-due" ]; }; then
    rm -f "$HOME_DIR/.fetch-due" 2>/dev/null || true
    CURRENT="$("$BIN" --version 2>/dev/null | awk '{print $NF}')"
    nohup bash "$SCRIPT_DIR/snip-bootstrap.sh" "" "$HOME_DIR" "$CURRENT" >/dev/null 2>&1 &
  fi
  exit 0
fi

if [ ! -x "$BIN" ]; then
  # Binary not installed and not an update check; pass through until it appears.
  exit 0
fi

exec "$BIN" "$@"
