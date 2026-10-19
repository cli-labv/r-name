#!/usr/bin/env sh
# Compila renamr (release) y deja el binario en dist/. No instala nada en el PATH.
set -eu

DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$DIR"

echo "· compilando (release)…"
cargo build --release

mkdir -p "$DIR/dist"
cp "$DIR/target/release/renamr" "$DIR/dist/renamr-linux-x86_64"
chmod +x "$DIR/start" 2>/dev/null || true

echo "✓ listo. Usá:  ./start --help"
