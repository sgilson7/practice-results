#!/usr/bin/env bash
# Builds the wasm module from crates/wasm (which forwards to crates/core) and
# assembles the static site in dist/web/. Needs: Rust stable with the
# wasm32-unknown-unknown target, and wasm-bindgen-cli 0.2.127 on PATH.
set -euo pipefail
cd "$(dirname "$0")"

cargo build --release --target wasm32-unknown-unknown -p practice_wasm

rm -rf dist/web
mkdir -p dist/web/pkg dist/web/data
wasm-bindgen --target web --no-typescript \
  --out-dir dist/web/pkg \
  target/wasm32-unknown-unknown/release/practice_wasm.wasm
cp web/index.html web/app.js web/style.css dist/web/
cp data/*.json dist/web/data/

echo "Built dist/web:"
find dist/web -type f | sort
