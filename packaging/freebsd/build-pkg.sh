#!/bin/sh
# Build a FreeBSD binary package for NumNum, plus a plain binary tarball.
#
# Modern pkg(8) produces a .pkg file (a tar archive, zstd-compressed by
# default). No ports tree is needed: we stage the files, generate a
# +MANIFEST from the template, and let pkg-create compute sizes/checksums
# from the plist.
set -eu

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
cd "$ROOT"

command -v cargo >/dev/null || { echo "cargo not found"; exit 1; }
command -v pkg   >/dev/null || { echo "pkg not found - run this on FreeBSD"; exit 1; }

VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
STAGE="$ROOT/target/freebsd-pkg/stage"
META="$ROOT/target/freebsd-pkg/meta"
OUT="$ROOT/dist/out"

echo "==> Building release binary"
cargo build --release --locked

echo "==> Staging files"
rm -rf "$STAGE" "$META"
mkdir -p "$STAGE/usr/local/bin" \
         "$STAGE/usr/local/share/applications" \
         "$STAGE/usr/local/share/icons/hicolor/scalable/apps" \
         "$META" "$OUT"
install -m 755 target/release/numnum    "$STAGE/usr/local/bin/numnum"
install -m 644 packaging/numnum.desktop "$STAGE/usr/local/share/applications/numnum.desktop"
install -m 644 assets/icons/numnum.svg  "$STAGE/usr/local/share/icons/hicolor/scalable/apps/numnum.svg"

echo "==> Resolving runtime dependencies (ldd + pkg which)"
: > "$META/deps.ucl"
seen=" "
for lib in $(ldd target/release/numnum | awk '/=>/ {print $3}' | sort -u); do
    [ -e "$lib" ] || continue
    pkgname=$(pkg which -q "$lib" 2>/dev/null) || continue
    [ -n "$pkgname" ] || continue
    name=$(pkg query '%n' "$pkgname" 2>/dev/null) || continue
    case "$seen" in *" $name "*) continue ;; esac
    seen="$seen$name "
    origin=$(pkg query '%o' "$pkgname")
    ver=$(pkg query '%v' "$pkgname")
    printf '  %s: { origin: "%s", version: "%s" }\n' "$name" "$origin" "$ver" >> "$META/deps.ucl"
done

echo "==> Generating +MANIFEST"
{
    sed "s/@VERSION@/$VERSION/" packaging/freebsd/MANIFEST.ucl.in
    printf 'abi: "%s"\n' "$(pkg config ABI)"
    echo 'deps: {'
    cat "$META/deps.ucl"
    echo '}'
} > "$META/+MANIFEST"

echo "==> pkg create"
pkg create -o "$OUT" -r "$STAGE" -m "$META" -p packaging/freebsd/pkg-plist
mv "$OUT/numnum-$VERSION.pkg" "$OUT/NumNum-$VERSION-freebsd-x86_64.pkg"

echo "==> Building plain binary tarball"
TARDIR="$ROOT/target/freebsd-pkg/NumNum-$VERSION-freebsd-x86_64"
rm -rf "$TARDIR"
mkdir -p "$TARDIR"
install -m 755 target/release/numnum "$TARDIR/numnum"
install -m 644 README.md LICENSE "$TARDIR/"
tar -cJf "$OUT/NumNum-$VERSION-freebsd-x86_64.tar.xz" \
    -C "$(dirname "$TARDIR")" "$(basename "$TARDIR")"

echo "==> Done:"
ls -la "$OUT"/NumNum-"$VERSION"-freebsd-x86_64.*
