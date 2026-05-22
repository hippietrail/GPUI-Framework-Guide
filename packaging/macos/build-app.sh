#!/bin/sh
# Build NumNum.app with a universal (fat) binary, and a NumNum-<ver>.dmg.
#
# The binary is built for both x86_64 and aarch64 and merged with lipo, so a
# single download runs on Intel and Apple Silicon. The bundle is UNSIGNED;
# public distribution also needs codesign with a Developer ID plus
# notarization via `xcrun notarytool`.
set -eu

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
cd "$ROOT"

[ "$(uname)" = "Darwin" ] || { echo "macOS only"; exit 1; }
command -v cargo >/dev/null || { echo "cargo not found"; exit 1; }

VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
OUT="$ROOT/dist/out"
APP="$OUT/NumNum.app"
DMG="$OUT/NumNum-$VERSION.dmg"
mkdir -p "$OUT"

echo "==> Building both architectures"
rustup target add x86_64-apple-darwin aarch64-apple-darwin
cargo build --release --locked --target x86_64-apple-darwin
cargo build --release --locked --target aarch64-apple-darwin

echo "==> Merging into a universal binary"
lipo -create -output "$OUT/numnum-universal" \
    target/x86_64-apple-darwin/release/numnum \
    target/aarch64-apple-darwin/release/numnum
lipo -info "$OUT/numnum-universal"

echo "==> Assembling NumNum.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
install -m 755 "$OUT/numnum-universal" "$APP/Contents/MacOS/numnum"
sed "s/@VERSION@/$VERSION/g" packaging/macos/Info.plist > "$APP/Contents/Info.plist"

echo "==> Generating icon (.icns)"
RSVG=$(command -v rsvg-convert || true)
if [ -n "$RSVG" ]; then
    ICONSET=$(mktemp -d)/numnum.iconset
    mkdir -p "$ICONSET"
    for s in 16 32 128 256 512; do
        d=$((s * 2))
        "$RSVG" -w "$s" -h "$s" assets/icons/numnum.svg -o "$ICONSET/icon_${s}x${s}.png"
        "$RSVG" -w "$d" -h "$d" assets/icons/numnum.svg -o "$ICONSET/icon_${s}x${s}@2x.png"
    done
    iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/numnum.icns"
else
    echo "   rsvg-convert not found (brew install librsvg) - app uses default icon"
fi

echo "==> Building $DMG"
rm -f "$DMG"
hdiutil create -volname "NumNum" -srcfolder "$APP" -ov -format UDZO "$DMG"
rm -f "$OUT/numnum-universal"

echo "==> Done:"
ls -ld "$APP" "$DMG"
