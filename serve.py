#!/usr/bin/env python3
"""
Simple HTTP server for Earth Engine WebGPU development
"""

import http.server
import socketserver
import os

class ESModuleHTTPRequestHandler(http.server.SimpleHTTPRequestHandler):
    """HTTP request handler with proper MIME types for ES modules"""
    
    def end_headers(self):
        # Add CORS headers for development
        self.send_header('Access-Control-Allow-Origin', '*')
        super().end_headers()
    
    def guess_type(self, path):
        """Ensure JavaScript files are served with module MIME type"""
        mimetype, _ = super().guess_type(path)
        
        if path.endswith('.js'):
            mimetype = 'application/javascript'
        elif path.endswith('.wasm'):
            mimetype = 'application/wasm'
            
        return mimetype, None

def main():
    PORT = 8080
    os.chdir(os.path.dirname(os.path.abspath(__file__)))
    
    print(f"Earth Engine Development Server")
    print(f"Server: http://localhost:{PORT}")
    print(f"WebGPU Demo: http://localhost:{PORT}/web/")
    print(f"\nRequirements:")
    print(f"- Chrome Canary with WebGPU enabled")
    print(f"- chrome://flags/#enable-unsafe-webgpu")
    
    with socketserver.TCPServer(("", PORT), ESModuleHTTPRequestHandler) as httpd:
        httpd.serve_forever()

if __name__ == "__main__":
    main()