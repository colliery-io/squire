#!/bin/sh
# Squire one-line installer (SQUIRE-T-0091 / ADR SQUIRE-A-0012).
#
#   curl -fsSL https://raw.githubusercontent.com/colliery-io/squire/main/install.sh | sh
#
# Downloads the latest signed `squire-serve` for this machine from the public release repo, installs
# it to ~/.local/bin, and on macOS creates a double-clickable Squire.app (in ~/Applications) that
# starts the server and opens the Keep. No code-signing needed: curl-fetched files aren't quarantined,
# so this sidesteps Gatekeeper. First run has no admin — you create one in the browser (the Keep's
# "First run? Create the admin Knight" form). Honors SQUIRE_BIN_DIR / SQUIRE_APP_DIR for testing.
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
  *) echo "Unsupported platform $OS-$ARCH — grab a binary from https://github.com/$REPO/releases" >&2; exit 1 ;;
esac

echo "Squire installer — $TARGET"
TAG="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
  | grep '"tag_name"' | head -1 | sed -E 's/.*"([^"]+)".*/\1/')"
[ -n "$TAG" ] || { echo "No release found in $REPO." >&2; exit 1; }
VER="${TAG#v}"
ASSET="squire-serve-$VER-$TARGET.tar.gz"
URL="https://github.com/$REPO/releases/download/$TAG/$ASSET"

echo "Downloading $ASSET ($TAG)…"
TMP="$(mktemp -d)"
curl -fsSL "$URL" -o "$TMP/$ASSET"
tar -xzf "$TMP/$ASSET" -C "$TMP"
mkdir -p "$BIN_DIR"
mv "$TMP/squire-serve" "$BIN_DIR/squire-serve"
chmod +x "$BIN_DIR/squire-serve"
rm -rf "$TMP"
echo "Installed squire-serve $VER → $BIN_DIR/squire-serve"

if [ "$OS" = "Darwin" ]; then
  APP="$APP_DIR/Squire.app"
  rm -rf "$APP"
  mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
  curl -fsSL "https://raw.githubusercontent.com/$REPO/main/squire.icns" \
    -o "$APP/Contents/Resources/squire.icns" 2>/dev/null || true
  cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>Squire</string>
  <key>CFBundleDisplayName</key><string>Squire</string>
  <key>CFBundleIdentifier</key><string>io.colliery.squire</string>
  <key>CFBundleExecutable</key><string>Squire</string>
  <key>CFBundleIconFile</key><string>squire</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$VER</string>
</dict></plist>
PLIST
  cat > "$APP/Contents/MacOS/Squire" <<'LAUNCH'
#!/bin/bash
# Start the home server if it isn't already up, then open the Keep.
BIN="$HOME/.local/bin/squire-serve"
LOG="$HOME/Library/Logs/Squire.log"
KEEP="http://127.0.0.1:4920"
if ! curl -fsS "$KEEP/health" >/dev/null 2>&1; then
  nohup "$BIN" >"$LOG" 2>&1 &
  for _ in $(seq 1 40); do curl -fsS "$KEEP/health" >/dev/null 2>&1 && break; sleep 0.5; done
fi
open "$KEEP"
LAUNCH
  chmod +x "$APP/Contents/MacOS/Squire"
  touch "$APP" # nudge the icon cache
  echo "Created $APP"
fi

echo ""
echo "Done. To start Squire:"
[ "$OS" = "Darwin" ] && echo "  • Open “Squire” from $APP_DIR (double-click), or: open \"$APP_DIR/Squire.app\""
echo "  • Or run: $BIN_DIR/squire-serve"
echo "Then create your admin account in the Keep that opens, and pair phones with the QR it shows."
