#!/bin/bash
# Build WASM and copy output to www/
# Requires: wasm-pack (https://rustwasm.github.io/wasm-pack/)
#   Install: cargo install wasm-pack

set -e

cd "$(dirname "$0")/.."

echo "Building WASM with wasm-pack..."
wasm-pack build --target web --out-dir www/pkg --features wasm

echo ""
echo "Done! Output in www/pkg/"
echo ""
echo "To serve locally:"
echo "  cd www && python3 -m http.server 8080"
echo "  open http://localhost:8080"
