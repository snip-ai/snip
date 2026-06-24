#!/usr/bin/env bash
# snip plugin bootstrap — download the prebuilt binary for this platform and
# install it under the OS data dir. Invoked detached by snip-run.sh on first run
# and by the binary's `update-check` to fetch a newer release. Idempotent and best-effort:
# any failure (offline, unsupported platform) installs nothing and exits 0 — the
# hooks degrade to pass-through until a later attempt succeeds. Integrity, however,
# is fail-CLOSED: a missing/empty checksum sidecar, an unavailable hasher, or any
# mismatch installs NOTHING. Availability may degrade; an UNVERIFIED native binary
# must never be installed and run.
#
# Usage: snip-bootstrap.sh <version> <data-dir> [current]
#   <version>  release version WITHOUT the leading 'v' (e.g. 0.1.0). Empty => latest.
#   <data-dir> snip data dir; the binary lands at <data-dir>/bin/snip[.exe].
#   [current]  the running binary's version; with an empty <version>, skip the
#              download when the resolved latest release already equals it.
set -u

VERSION="${1:-}"
HOME_DIR="${2:-}"
CURRENT="${3:-}"
REPO="snip-ai/snip"
RELEASES_API="${SNIP_RELEASES_API:-https://api.github.com/repos/$REPO/releases}"
DOWNLOAD_BASE="${SNIP_DOWNLOAD_BASE:-https://github.com/$REPO/releases/download}"

[ -n "$HOME_DIR" ] || exit 0
command -v curl >/dev/null 2>&1 || exit 0

# --- platform -> target triple + archive format ----------------------------
case "$(uname -s)" in
  Darwin)               OS_T="apple-darwin";       EXT="tar.gz"; BINF="snip" ;;
  Linux)                OS_T="unknown-linux-musl"; EXT="tar.gz"; BINF="snip" ;;
  MINGW*|MSYS*|CYGWIN*) OS_T="pc-windows-msvc";    EXT="zip";    BINF="snip.exe" ;;
  *) exit 0 ;;
esac
case "$(uname -m)" in
  x86_64|amd64)  ARCH_T="x86_64" ;;
  arm64|aarch64) ARCH_T="aarch64" ;;
  *) exit 0 ;;
esac
TARGET="${ARCH_T}-${OS_T}"

# Only these four targets are built; bail on anything else (e.g. aarch64 Linux).
case "$TARGET" in
  x86_64-apple-darwin|aarch64-apple-darwin|x86_64-unknown-linux-musl|x86_64-pc-windows-msvc) ;;
  *) exit 0 ;;
esac

# --- resolve version --------------------------------------------------------
if [ -z "$VERSION" ]; then
  VERSION="$(curl -fsSL "$RELEASES_API/latest" 2>/dev/null \
    | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"v\{0,1\}\([^"]*\)".*/\1/p' | head -n1)"
fi
[ -n "$VERSION" ] || exit 0
# Already on the resolved version? Nothing to do (only when the caller told us
# the installed version — first-install passes an explicit <version> and no CURRENT).
[ -z "$CURRENT" ] || [ "$VERSION" != "$CURRENT" ] || exit 0

ASSET="snip-${TARGET}.${EXT}"
URL="$DOWNLOAD_BASE/v${VERSION}/${ASSET}"

TMP="$(mktemp -d 2>/dev/null || printf '%s' "${TMPDIR:-/tmp}/snip-bootstrap.$$")"
mkdir -p "$TMP" || exit 0
trap 'rm -rf "$TMP"' EXIT

# --- download + checksum-verify (fail-CLOSED) ------------------------------
# Integrity is the one place best-effort does NOT apply: a missing sidecar, an
# unavailable hasher, or a mismatch each installs nothing. The sidecar shares the
# binary's TLS channel, so this guards corruption / a stale-or-forged binary with
# a correct sidecar — not a full on-path MITM; release signing (minisign/cosign)
# is the proper next step (see audit C1).
curl -fsSL "$URL" -o "$TMP/$ASSET" 2>/dev/null || exit 0
curl -fsSL "$URL.sha256" -o "$TMP/$ASSET.sha256" 2>/dev/null || exit 0  # no sidecar -> install nothing
[ -s "$TMP/$ASSET.sha256" ] || exit 0
want="$(awk '{print $1; exit}' "$TMP/$ASSET.sha256" | tr 'A-Z' 'a-z')"
[ -n "$want" ] || exit 0
got=""
if command -v shasum >/dev/null 2>&1; then
  got="$(shasum -a 256 "$TMP/$ASSET" | awk '{print $1}')"
elif command -v sha256sum >/dev/null 2>&1; then
  got="$(sha256sum "$TMP/$ASSET" | awk '{print $1}')"
elif command -v certutil >/dev/null 2>&1; then
  got="$(certutil -hashfile "$TMP/$ASSET" SHA256 2>/dev/null | sed -n 2p | tr -d ' \r' | tr 'A-Z' 'a-z')"
fi
[ -n "$got" ] || exit 0          # no working hasher -> do NOT install unverified
[ "$got" = "$want" ] || exit 0   # mismatch -> install nothing

# --- extract ----------------------------------------------------------------
case "$EXT" in
  tar.gz) tar -xzf "$TMP/$ASSET" -C "$TMP" 2>/dev/null || exit 0 ;;
  zip)
    if command -v unzip >/dev/null 2>&1; then
      unzip -oq "$TMP/$ASSET" -d "$TMP" 2>/dev/null || exit 0
    elif command -v powershell >/dev/null 2>&1; then
      powershell -NoProfile -Command "Expand-Archive -Force -Path '$TMP/$ASSET' -DestinationPath '$TMP'" 2>/dev/null || exit 0
    else
      exit 0
    fi ;;
esac
[ -f "$TMP/$BINF" ] || exit 0

# --- install atomically into the data dir ----------------------------------
mkdir -p "$HOME_DIR/bin" || exit 0
mv -f "$TMP/$BINF" "$HOME_DIR/bin/$BINF" 2>/dev/null || cp -f "$TMP/$BINF" "$HOME_DIR/bin/$BINF" || exit 0
chmod +x "$HOME_DIR/bin/$BINF" 2>/dev/null || true
exit 0
