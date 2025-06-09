#!/bin/bash

# Build script for Earth Engine WebGPU version

set -e

echo "Building Earth Engine for WebGPU..."

# Install wasm-pack if not already installed
if ! command -v wasm-pack &> /dev/null; then
    echo "Installing wasm-pack..."
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
fi

# Clean previous builds
rm -rf pkg/
rm -rf web/earth_engine_web*

# Build with wasm-pack
echo "Building WASM module..."
wasm-pack build --target web --out-dir pkg --features web

# Copy generated files to web directory
echo "Copying files to web directory..."
cp pkg/earth_engine_web_bg.wasm web/
cp pkg/earth_engine_web.js web/

# Create a simple server script
cat > web/serve.py << 'EOF'
#!/usr/bin/env python3
import http.server
import socketserver
import os

class MyHTTPRequestHandler(http.server.SimpleHTTPRequestHandler):
    def end_headers(self):
        # Add headers for SharedArrayBuffer and COOP/COEP
        self.send_header('Cross-Origin-Embedder-Policy', 'require-corp')
        self.send_header('Cross-Origin-Opener-Policy', 'same-origin')
        self.send_header('Access-Control-Allow-Origin', '*')
        super().end_headers()
    
    def guess_type(self, path):
        mimetype = super().guess_type(path)
        if path.endswith('.wasm'):
            return 'application/wasm'
        return mimetype

PORT = 8080
os.chdir(os.path.dirname(os.path.abspath(__file__)))

with socketserver.TCPServer(("", PORT), MyHTTPRequestHandler) as httpd:
    print(f"Server running at http://localhost:{PORT}/")
    print(f"Open http://localhost:{PORT}/ in a WebGPU-enabled browser")
    print("Press Ctrl+C to stop the server")
    httpd.serve_forever()
EOF

chmod +x web/serve.py

echo "Build complete!"
echo ""
echo "To run the web version:"
echo "  cd web && python3 serve.py"
echo ""
echo "Then open http://localhost:8080/ in a WebGPU-enabled browser (Chrome/Edge)"