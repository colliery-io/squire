#!/bin/sh
# Squire one-line installer (SQUIRE-T-0091/0092 / ADR SQUIRE-A-0012).
#
#   curl -fsSL https://raw.githubusercontent.com/colliery-io/squire/main/install.sh | sh
#
# On macOS this installs the native Squire app (a real window — the home server is embedded) into
# ~/Applications. Elsewhere it installs the headless `squire-serve` binary to ~/.local/bin. First run
# has no admin — you create one in the window/Keep ("First run? Create the admin Knight"). No
# code-signing needed: curl-fetched files aren't quarantined, so this sidesteps Gatekeeper. Honors
# SQUIRE_BIN_DIR / SQUIRE_APP_DIR for testing.
set -eu

REPO="colliery-io/squire"
BIN_DIR="${SQUIRE_BIN_DIR:-$HOME/.local/bin}"
APP_DIR="${SQUIRE_APP_DIR:-$HOME/Applications}"
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS-$ARCH" in
  Darwin-arm64)  TARGET="aarch64-apple-darwin" ;;
  Darwin-x86_64) TARGET="x86_64-apple-darwin" ;;
  Linux-x86_64)  TARGET="x86_64-unknown-linux-gnu" ;;
  *) echo "Unsupported platform $OS-$ARCH — see https://github.com/$REPO/releases" >&2; exit 1 ;;
esac

TAG="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
  | grep '"tag_name"' | head -1 | sed -E 's/.*"([^"]+)".*/\1/')"
[ -n "$TAG" ] || { echo "No release found in $REPO." >&2; exit 1; }
VER="${TAG#v}"
BASE="https://github.com/$REPO/releases/download/$TAG"

# macOS: prefer the native Squire.app (server embedded) when a build exists for this arch.
if [ "$OS" = "Darwin" ]; then
  APP_ZIP="Squire-$VER-$TARGET.app.zip"
  if curl -fsSL -I "$BASE/$APP_ZIP" >/dev/null 2>&1; then
    echo "Installing the native Squire app ($TARGET, $TAG)…"
    TMP="$(mktemp -d)"
    curl -fsSL "$BASE/$APP_ZIP" -o "$TMP/Squire.app.zip"
    mkdir -p "$APP_DIR"
    rm -rf "$APP_DIR/Squire.app"
    ditto -x -k "$TMP/Squire.app.zip" "$APP_DIR"
    rm -rf "$TMP"
    xattr -dr com.apple.quarantine "$APP_DIR/Squire.app" 2>/dev/null || true
    echo "Installed → $APP_DIR/Squire.app"
    echo ""
    echo "Done. Open “Squire” from $APP_DIR (double-click)."
    echo "On first run, create your admin account in the window, then pair phones with the QR it shows."
    exit 0
  fi
  echo "(no native app for $TARGET yet — installing the headless server instead)"
fi

# Headless server binary (Linux, or a macOS arch without an app build).
ASSET="squire-serve-$VER-$TARGET.tar.gz"
echo "Installing squire-serve ($TARGET, $TAG)…"
TMP="$(mktemp -d)"
curl -fsSL "$BASE/$ASSET" -o "$TMP/$ASSET"
tar -xzf "$TMP/$ASSET" -C "$TMP"
mkdir -p "$BIN_DIR"
mv "$TMP/squire-serve" "$BIN_DIR/squire-serve"
chmod +x "$BIN_DIR/squire-serve"
rm -rf "$TMP"
echo "Installed → $BIN_DIR/squire-serve"
echo ""
echo "Done. Run the home server: $BIN_DIR/squire-serve"
echo "Then open the Keep it prints, create your admin account, and pair phones with the QR."
