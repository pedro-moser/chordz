#!/bin/bash
set -e

cd "$(dirname "$0")/.."

echo "Building WASM..."
wasm-pack build --target web --features wasm
mv pkg web/pkg

echo "Building Svelte..."
cd web
npm run build

echo "Done! Output in web/build/"
