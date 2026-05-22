#!/bin/sh
# Build .deb and .rpm packages for NumNum on Linux.
#
# Layout/metadata come from [package.metadata.deb] and
# [package.metadata.generate-rpm] in the root Cargo.toml. Runtime
# dependencies are resolved automatically by each tool from the binary.
set -eu

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
cd "$ROOT"

[ "$(uname)" = "Linux" ] || { echo "deb/rpm packaging must run on Linux"; exit 1; }
command -v cargo >/dev/null || { echo "cargo not found"; exit 1; }

OUT="$ROOT/dist/out"
mkdir -p "$OUT"

echo "==> Building release binary"
cargo build --release --locked

if command -v cargo-deb >/dev/null; then
    echo "==> Building .deb"
    cargo deb --no-build --output "$OUT"
else
    echo "!! cargo-deb missing - skipping .deb  (cargo install cargo-deb)"
fi

if command -v cargo-generate-rpm >/dev/null; then
    echo "==> Building .rpm"
    cargo generate-rpm
    cp target/generate-rpm/*.rpm "$OUT"/
else
    echo "!! cargo-generate-rpm missing - skipping .rpm  (cargo install cargo-generate-rpm)"
fi

echo "==> Done:"
ls -la "$OUT"
