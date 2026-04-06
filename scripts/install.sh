#!/bin/sh
# frogbite installer (POSIX sh)
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/Dilgo-dev/frogbite/main/scripts/install.sh | sh
#   FROGBITE_VERSION=v0.1.0 sh install.sh
#   FROGBITE_INSTALL_DIR=/usr/local/bin sh install.sh

set -eu

REPO="Dilgo-dev/frogbite"
BIN="frogbite"
INSTALL_DIR="${FROGBITE_INSTALL_DIR:-$HOME/.local/bin}"

err() { printf 'error: %s\n' "$*" >&2; exit 1; }
info() { printf '==> %s\n' "$*"; }

need() { command -v "$1" >/dev/null 2>&1 || err "missing required tool: $1"; }

need uname
need tar
need mkdir
need mv
if command -v curl >/dev/null 2>&1; then
  DL='curl -fsSL -o'
elif command -v wget >/dev/null 2>&1; then
  DL='wget -qO'
else
  err "need curl or wget"
fi

OS=$(uname -s)
ARCH=$(uname -m)
case "$OS" in
  Linux)  os=unknown-linux-gnu ;;
  Darwin) os=apple-darwin ;;
  *) err "unsupported OS: $OS (use the PowerShell installer on Windows)" ;;
esac
case "$ARCH" in
  x86_64|amd64) arch=x86_64 ;;
  arm64|aarch64) arch=aarch64 ;;
  *) err "unsupported arch: $ARCH" ;;
esac
TARGET="${arch}-${os}"

VERSION="${FROGBITE_VERSION:-}"
if [ -z "$VERSION" ]; then
  info "resolving latest release"
  VERSION=$(
    curl -fsSL -o /dev/null -w '%{url_effective}' \
      "https://github.com/${REPO}/releases/latest" \
      | sed 's|.*/tag/||'
  )
  [ -n "$VERSION" ] || err "failed to resolve latest version"
fi
info "installing frogbite ${VERSION} (${TARGET})"

ARCHIVE="frogbite-${VERSION}-${TARGET}.tar.gz"
URL="https://github.com/${REPO}/releases/download/${VERSION}/${ARCHIVE}"
SUM_URL="${URL}.sha256"

TMP=$(mktemp -d 2>/dev/null || mktemp -d -t frogbite)
trap 'rm -rf "$TMP"' EXIT INT TERM

info "downloading $ARCHIVE"
$DL "$TMP/$ARCHIVE"        "$URL"
$DL "$TMP/$ARCHIVE.sha256" "$SUM_URL"

info "verifying checksum"
( cd "$TMP" && \
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum -c "$ARCHIVE.sha256"
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 -c "$ARCHIVE.sha256"
  else
    err "no sha256 tool found (sha256sum or shasum)"
  fi
) || err "checksum verification failed"

info "extracting"
tar -C "$TMP" -xzf "$TMP/$ARCHIVE"

mkdir -p "$INSTALL_DIR"
mv "$TMP/frogbite-${VERSION}-${TARGET}/$BIN" "$INSTALL_DIR/$BIN"
chmod +x "$INSTALL_DIR/$BIN"

info "installed $BIN -> $INSTALL_DIR/$BIN"

case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *) printf '\nNote: %s is not in your PATH. Add this to your shell rc:\n  export PATH="%s:$PATH"\n' "$INSTALL_DIR" "$INSTALL_DIR" ;;
esac
