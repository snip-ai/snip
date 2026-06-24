#!/usr/bin/env bash
# snip uninstall wrapper (git bash). Runs the binary's teardown (purges data,
# strips the PATH line, writes the .uninstalled marker), then removes the binary
# itself. A native .exe can't delete its own running file on Windows, but this
# shell outlives the binary it just ran, so bin/ is already unlocked here — no
# detached helper, no PowerShell. KEEP snip_home() IN SYNC with snip-run.sh.
set -u

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

if [ -x "$BIN" ]; then
  "$BIN" uninstall
else
  printf 'snip: no binary at %s — nothing to remove (just remove the plugin).\n' "$BIN"
fi

# The binary has exited; its .exe is unlocked. Remove bin/, leaving the data dir
# holding only the .uninstalled marker (which blocks auto-reinstall until the
# plugin is removed).
rm -rf "$HOME_DIR/bin" 2>/dev/null || true
