#!/bin/env bash
# Build WASM and copy output to www/
# Requires: wasm-pack (https://rustwasm.github.io/wasm-pack/)

set -e

cd "$(dirname "$0")/.."

# wasm-pack が見つからない場合、Nix dev shell 経由で再実行する
if ! command -v wasm-pack &>/dev/null; then
  if command -v nix &>/dev/null && [ -z "$IN_NIX_SHELL" ]; then
    echo "wasm-pack not found. Entering Nix dev shell..."
    exec nix develop "$(cd "$(dirname "$0")" && pwd)" --command bash "$0" "$@"
  fi
  echo "Error: wasm-pack is required."
  echo ""
  echo "Options:"
  echo "  Nix dev shell:  nix develop ./www"
  echo "  Manual install: cargo install wasm-pack"
  exit 1
fi

echo "Building WASM with wasm-pack..."
wasm-pack build --target web --out-dir www/pkg --features wasm

echo ""
echo "Done! Output in www/pkg/"
echo ""
echo "To serve locally:"
echo "  cd www && python3 -m http.server 8080"
echo "  open http://localhost:8080"
