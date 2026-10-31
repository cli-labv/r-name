#!/usr/bin/env sh
# Compila r-name (release) y deja el binario en dist/. No instala nada en el PATH.
set -eu

DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$DIR"

echo "· compilando (release)…"
cargo build --release

mkdir -p "$DIR/dist"
cp "$DIR/target/release/r-name" "$DIR/dist/r-name-linux-x86_64"
chmod +x "$DIR/start" 2>/dev/null || true

echo "✓ listo. Usá:  ./start        (menú interactivo)"
echo "             ./start --set     (renombrado directo por temporada desde .env, sin preguntar)"
