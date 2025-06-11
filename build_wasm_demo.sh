#!/bin/bash

# Build script for minimal WASM demo

set -e

echo "Building Earth Engine WASM Demo..."

# Install wasm-pack if not already installed
if ! command -v wasm-pack &> /dev/null; then
    echo "Installing wasm-pack..."
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
fi

# Build the demo
cd wasm-demo
wasm-pack build --target web --out-dir pkg

# Copy files to web directory
echo "Copying files to web directory..."
cp pkg/earth_engine_wasm_demo_bg.wasm ../web/
cp pkg/earth_engine_wasm_demo.js ../web/

echo "Build complete!"
echo ""
echo "To test the WASM demo:"
echo "  1. cd web"
echo "  2. python3 serve.py"
echo "  3. Open http://localhost:8080/index-wasm.html"