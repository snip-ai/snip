#!/usr/bin/env bash
# snip shell setup — add the managed binary's dir to your shell PATH so you can
# run `snip status`, `snip gain`, `snip config …` directly from a shell (no model
# turn), alongside the `/snip <sub>` slash-command.
#
# Invoked two ways: automatically by snip-bootstrap.sh on the FIRST install (so
# the binary is on PATH out of the box), and manually via `/snip shell-setup`
# (or `/snip shell-setup remove`). It writes exactly one small, clearly-marked,
# removable block to the rc file your interactive shell actually sources — nothing
# else. Install and updates still flow only through the plugin; this only reaches
# the binary already placed under the OS data dir.
#
# Usage: snip-shell-setup.sh [setup|remove]   (default: setup)
set -u

OS="$(uname -s)"
ACTION="${1:-setup}"

MARK_BEGIN="# >>> snip shell setup >>>"
MARK_END="# <<< snip shell setup <<<"

# Resolve the data dir the SAME way as snip-run.sh::snip_home() and
# src/paths.rs::data_dir(). Kept as a local copy (not a sourced helper) on
# purpose: snip-run.sh is the per-tool-call hot path and must avoid an extra
# file read. KEEP THESE THREE IN SYNC if the data-dir mapping ever changes.
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
BIN_DIR="$(snip_home)/bin"
BIN="$BIN_DIR/$BIN_NAME"

# The PATH line written to the rc. Variable-relative (re-resolved every shell) so
# it survives a home/profile move and always targets the OS-default store. On
# Git Bash the data dir is a Windows path whose drive colon would break PATH
# splitting, so route it through cygpath to a POSIX path.
case "$OS" in
  Darwin)               PATH_LINE='export PATH="$HOME/Library/Application Support/snip/bin:$PATH"' ;;
  MINGW*|MSYS*|CYGWIN*) PATH_LINE='export PATH="$(cygpath -u "${APPDATA:-$HOME/AppData/Roaming}")/snip/bin:$PATH"' ;;
  *)                    PATH_LINE='export PATH="${XDG_DATA_HOME:-$HOME/.local/share}/snip/bin:$PATH"' ;;
esac

# Pick the rc file the user's interactive shell actually sources. Scope is
# bash/zsh; other shells (fish, PowerShell) must be set up by hand. The `$SHELL`
# basename carries a `.exe` on Windows git bash, so strip it before matching.
detect_rc() {
  sh="$(basename "${SHELL:-bash}")"; sh="${sh%.exe}"
  case "$sh" in
    zsh)  printf '%s' "$HOME/.zshrc"; return ;;
    bash) : ;;  # handled by OS below
    *)    printf '%s' "$HOME/.profile"; return ;;
  esac
  # bash: Linux terminals open NON-login shells (→ .bashrc). macOS Terminal and
  # Windows git bash open LOGIN shells, which source the FIRST existing of
  # .bash_profile / .bash_login / .profile — append to that one (so we never
  # shadow it), creating .bash_profile when none exists. Writing .bashrc on git
  # bash would only work via a warning-printing auto-created .bash_profile.
  case "$OS" in
    Linux) printf '%s' "$HOME/.bashrc" ;;
    *)
      if   [ -f "$HOME/.bash_profile" ]; then printf '%s' "$HOME/.bash_profile"
      elif [ -f "$HOME/.bash_login" ];   then printf '%s' "$HOME/.bash_login"
      elif [ -f "$HOME/.profile" ];      then printf '%s' "$HOME/.profile"
      else                                    printf '%s' "$HOME/.bash_profile"
      fi ;;
  esac
}

RC="$(detect_rc)"

do_setup() {
  if grep -Fq -- "$MARK_BEGIN" "$RC" 2>/dev/null; then
    printf 'snip: your PATH is already configured in %s (no change).\n' "$RC"
    printf 'Run `snip status` in a new shell, or `/snip shell-setup remove` to undo.\n'
    return 0
  fi
  {
    printf '\n%s\n' "$MARK_BEGIN"
    printf '%s\n' "$PATH_LINE"
    printf '%s\n' "$MARK_END"
  } >> "$RC" || {
    printf 'snip: could not write %s — add this line to your shell rc by hand:\n\n  %s\n' "$RC" "$PATH_LINE"
    return 1
  }

  printf 'snip: added the binary dir to your PATH via %s\n\n  %s\n\n' "$RC" "$PATH_LINE"
  printf 'Open a new shell (or run: source "%s") and `snip` works directly:\n' "$RC"
  printf '  snip status    snip gain    snip config list\n\n'
  printf 'Undo anytime with:  /snip shell-setup remove\n'
  if [ ! -x "$BIN" ]; then
    printf '\nHeads-up: the binary is not installed yet at %s — it appears after the\n' "$BIN"
    printf 'plugin'\''s first SessionStart on a supported platform; the PATH line works then.\n'
  fi
  if [ -n "${SNIP_HOME:-}" ]; then
    printf '\nNote: SNIP_HOME is set in this shell, but the PATH line targets the OS-default\n'
    printf 'store, not $SNIP_HOME. Leave SNIP_HOME unset for the line to match your data.\n'
  fi
  printf '\nInstall & updates still flow only through the plugin; this just reaches the\n'
  printf 'already-installed binary. snip wrote only the rc line above, nothing else.\n'
}

do_remove() {
  if ! grep -Fq -- "$MARK_BEGIN" "$RC" 2>/dev/null; then
    printf 'snip: no snip PATH line found in %s (nothing to remove).\n' "$RC"
    return 0
  fi
  tmp="$(mktemp 2>/dev/null || printf '%s' "$RC.snip.tmp.$$")"
  awk -v b="$MARK_BEGIN" -v e="$MARK_END" '
    $0==b { inblk=1; next }
    $0==e { inblk=0; next }
    inblk!=1 { print }
  ' "$RC" > "$tmp" && mv -f "$tmp" "$RC" || {
    rm -f "$tmp" 2>/dev/null
    printf 'snip: could not rewrite %s — remove the block between the snip markers by hand.\n' "$RC"
    return 1
  }
  printf 'snip: removed the snip PATH line from %s.\n' "$RC"
  printf 'Open a new shell for the change to take effect.\n'
}

case "$ACTION" in
  setup)  do_setup ;;
  remove) do_remove ;;
  *)
    printf 'snip-shell-setup: unknown action "%s" (use: setup | remove).\n' "$ACTION"
    exit 2 ;;
esac
