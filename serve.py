#!/usr/bin/env python3
"""
Simple HTTP server for Earth Engine WebGPU development
Serves with proper MIME types for ES modules
"""

import http.server
import socketserver
import os
import sys
from pathlib import Path

class ESModuleHTTPRequestHandler(http.server.SimpleHTTPRequestHandler):
    """HTTP request handler with proper MIME types for ES modules"""
    
    def end_headers(self):
        # Add CORS headers for development
        self.send_header('Access-Control-Allow-Origin', '*')
        self.send_header('Access-Control-Allow-Methods', 'GET, POST, OPTIONS')
        self.send_header('Access-Control-Allow-Headers', 'Content-Type')
        super().end_headers()
    
    def guess_type(self, path):
        """Guess MIME type with special handling for .js files"""
        mimetype, _ = super().guess_type(path)
        
        # Ensure JavaScript files are served with correct module MIME type
        if path.endswith('.js'):
            mimetype = 'application/javascript'
        elif path.endswith('.mjs'):
            mimetype = 'application/javascript'
        elif path.endswith('.wasm'):
            mimetype = 'application/wasm'
            
        return mimetype, None
    
    def translate_path(self, path):
        """Translate URL path to file system path"""
        # Remove query string
        path = path.split('?', 1)[0]
        path = path.split('#', 1)[0]
        
        # Handle earth-engine-js module imports
        if path.startswith('/earth-engine-js/'):
            # Map to JavaScript source directory
            rel_path = path[len('/earth-engine-js/'):]
            return os.path.join(os.getcwd(), 'earth-engine-js', rel_path)
        
        # Default behavior for other paths
        return super().translate_path(path)

def main():
    PORT = 8080
    
    # Change to repository root
    script_dir = Path(__file__).parent.absolute()
    os.chdir(script_dir)
    
    print(f"Earth Engine Development Server")
    print(f"Serving from: {os.getcwd()}")
    print(f"")
    print(f"WebGPU Demo: http://localhost:{PORT}/web/")
    print(f"")
    print(f"Requirements:")
    print(f"- Chrome Canary or Edge Canary")
    print(f"- Enable WebGPU: chrome://flags/#enable-unsafe-webgpu")
    print(f"")
    print(f"Press Ctrl+C to stop the server")
    
    with socketserver.TCPServer(("", PORT), ESModuleHTTPRequestHandler) as httpd:
        try:
            httpd.serve_forever()
        except KeyboardInterrupt:
            print("\nShutting down server...")
            sys.exit(0)

if __name__ == "__main__":
    main()