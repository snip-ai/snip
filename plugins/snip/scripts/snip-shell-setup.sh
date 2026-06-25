#!/usr/bin/env bash
# snip shell setup — put the managed binary on your PATH so you can run
# `snip status`, `snip gain`, `snip config …` directly (no model turn), alongside
# the `/snip <sub>` slash-command.
#
# Invoked two ways: automatically by snip-bootstrap.sh on the FIRST install, and
# manually via `/snip shell-setup` (or `/snip shell-setup remove`). It writes one
# clearly-marked, removable block to the rc file(s) your interactive shell sources
# and — on Windows — also adds the dir to your USER PATH env var, so non-interactive
# shells (Claude Code's Bash tool), PowerShell, and cmd see it too. `remove` (and
# `snip uninstall`) take all of that back out. Nothing else is touched.
#
# Usage: snip-shell-setup.sh [setup|remove]   (default: setup)
set -u

OS="$(uname -s)"
ACTION="${1:-setup}"

MARK_BEGIN="# >>> snip shell setup >>>"
MARK_END="# <<< snip shell setup <<<"

# Resolve the data dir the SAME way as snip-run.sh::snip_home() and
# src/paths.rs::data_dir(). KEEP THESE THREE IN SYNC if the mapping ever changes.
snip_home() {
  if [ -n "${SNIP_HOME:-}" ]; then printf '%s' "$SNIP_HOME"; return; fi
  case "$OS" in
    Darwin)               printf '%s' "$HOME/Library/Application Support/snip" ;;
    MINGW*|MSYS*|CYGWIN*) printf '%s' "${APPDATA:-$HOME/AppData/Roaming}/snip" ;;
    *)                    printf '%s' "${XDG_DATA_HOME:-$HOME/.local/share}/snip" ;;
  esac
}

is_windows() { case "$OS" in MINGW*|MSYS*|CYGWIN*) return 0 ;; *) return 1 ;; esac; }

# The PATH line written to the rc. Variable-relative so it survives a profile move.
case "$OS" in
  Darwin)               PATH_LINE='export PATH="$HOME/Library/Application Support/snip/bin:$PATH"' ;;
  MINGW*|MSYS*|CYGWIN*) PATH_LINE='export PATH="$(cygpath -u "${APPDATA:-$HOME/AppData/Roaming}")/snip/bin:$PATH"' ;;
  *)                    PATH_LINE='export PATH="${XDG_DATA_HOME:-$HOME/.local/share}/snip/bin:$PATH"' ;;
esac

# --- marked-block helpers (idempotent) --------------------------------------
has_block()   { grep -Fq -- "$MARK_BEGIN" "$1" 2>/dev/null; }
write_block() { # $1=file $2=content
  has_block "$1" && return 0
  { printf '\n%s\n' "$MARK_BEGIN"; printf '%s\n' "$2"; printf '%s\n' "$MARK_END"; } >> "$1"
}
strip_block() { # $1=file
  has_block "$1" || return 0
  tmp="$(mktemp 2>/dev/null || printf '%s' "$1.snip.tmp.$$")"
  awk -v b="$MARK_BEGIN" -v e="$MARK_END" \
    '$0==b{inblk=1;next} $0==e{inblk=0;next} inblk!=1{print}' "$1" >"$tmp" \
    && mv -f "$tmp" "$1" || rm -f "$tmp" 2>/dev/null
}

# Put the PATH line where the user's interactive shell sources it. bash uses
# .bashrc (sourced by NON-login interactive shells — IDE terminals) plus a
# .bash_profile that sources .bashrc (so LOGIN shells — macOS Terminal, Windows
# Git Bash — get it too, with no git-for-windows "incorrect setup" warning).
setup_rc() {
  sh="$(basename "${SHELL:-bash}")"; sh="${sh%.exe}"
  case "$sh" in
    zsh)  write_block "$HOME/.zshrc" "$PATH_LINE"; printf '%s' "$HOME/.zshrc"; return ;;
    bash) : ;;
    *)    write_block "$HOME/.profile" "$PATH_LINE"; printf '%s' "$HOME/.profile"; return ;;
  esac
  write_block "$HOME/.bashrc" "$PATH_LINE"
  if ! { [ -f "$HOME/.bash_profile" ] && grep -q '\.bashrc' "$HOME/.bash_profile" 2>/dev/null; } \
     && ! { [ -f "$HOME/.bash_login" ] && grep -q '\.bashrc' "$HOME/.bash_login" 2>/dev/null; }; then
    write_block "$HOME/.bash_profile" '[ -r ~/.bashrc ] && . ~/.bashrc'
  fi
  printf '%s' "$HOME/.bashrc"
}

remove_rc() {
  for f in .bashrc .bash_profile .bash_login .zshrc .profile; do strip_block "$HOME/$f"; done
}

# --- Windows USER PATH (covers non-interactive shells, PowerShell, cmd) ------
# The dir is passed via an env var so the PowerShell command needs no interpolation.
win_path_add() {
  SNIP_BIN_WIN="$(cygpath -w "$(snip_home)/bin")" powershell -NoProfile -NonInteractive -Command '
    $d = $env:SNIP_BIN_WIN; $p = [Environment]::GetEnvironmentVariable("PATH","User"); if (-not $p) { $p = "" }
    if (($p -split ";") -notcontains $d) {
      [Environment]::SetEnvironmentVariable("PATH", (($p.TrimEnd(";") + ";" + $d).TrimStart(";")), "User")
    }' >/dev/null 2>&1
}
win_path_remove() {
  SNIP_BIN_WIN="$(cygpath -w "$(snip_home)/bin")" powershell -NoProfile -NonInteractive -Command '
    $d = $env:SNIP_BIN_WIN; $p = [Environment]::GetEnvironmentVariable("PATH","User")
    if ($p) {
      $new = (($p -split ";") | Where-Object { $_ -ne $d -and $_ -ne "" }) -join ";"
      [Environment]::SetEnvironmentVariable("PATH", $new, "User")
    }' >/dev/null 2>&1
}

# --- actions ----------------------------------------------------------------
do_setup() {
  rc="$(setup_rc)"
  printf 'snip: ensured the binary dir is on your shell PATH (via %s + a login chain).\n' "$rc"
  if is_windows && win_path_add; then
    printf 'snip: and on your Windows USER PATH (covers non-interactive shells,\n'
    printf '      PowerShell, and cmd). New shells/sessions pick it up.\n'
  fi
  printf '\nOpen a NEW shell, then `snip` runs directly:  snip status   snip gain   snip config list\n'
  printf 'Undo anytime with:  /snip shell-setup remove  (or `snip uninstall`).\n'
  if [ ! -x "$(snip_home)/bin/snip" ] && [ ! -x "$(snip_home)/bin/snip.exe" ]; then
    printf '\nHeads-up: the binary is not installed yet — it appears after the plugin'\''s\n'
    printf 'first SessionStart on a supported platform; the PATH entries work then.\n'
  fi
}

do_remove() {
  remove_rc
  extra=""
  if is_windows; then win_path_remove; extra=" + Windows USER PATH"; fi
  printf 'snip: removed the snip PATH entries (rc files%s).\n' "$extra"
  printf 'Open a new shell for the change to take effect.\n'
}

case "$ACTION" in
  setup)  do_setup ;;
  remove) do_remove ;;
  *)
    printf 'snip-shell-setup: unknown action "%s" (use: setup | remove).\n' "$ACTION"
    exit 2 ;;
esac
