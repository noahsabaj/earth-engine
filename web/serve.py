#!/usr/bin/env python3
"""
Earth Engine WebGPU Development Server
Serves the data-oriented JavaScript implementation with proper ES module support
"""

import http.server
import socketserver
import os
import sys

class WebGPUDevServer(http.server.SimpleHTTPRequestHandler):
    """HTTP server configured for WebGPU and ES modules"""
    
    def end_headers(self):
        # CORS headers for development
        self.send_header('Access-Control-Allow-Origin', '*')
        self.send_header('Access-Control-Allow-Methods', 'GET, POST, OPTIONS')
        self.send_header('Access-Control-Allow-Headers', 'Content-Type')
        
        # Prevent caching for development
        self.send_header('Cache-Control', 'no-cache, no-store, must-revalidate')
        self.send_header('Pragma', 'no-cache')
        self.send_header('Expires', '0')
        
        super().end_headers()
    
    def guess_type(self, path):
        """Set correct MIME types for modern web development"""
        # Remove query string if present
        path = path.split('?')[0]
        
        # Get base MIME type
        mimetype = super().guess_type(path)
        
        # Override for specific file types
        if path.endswith('.js'):
            return 'application/javascript'
        elif path.endswith('.wasm'):
            return 'application/wasm'
        elif path.endswith('.wgsl'):
            return 'text/plain'
        
        return mimetype
    
    def do_GET(self):
        # Strip query parameters for file serving (cache busting)
        self.path = self.path.split('?')[0]
        return super().do_GET()

def main():
    PORT = 8080
    
    # Change to web directory
    web_dir = os.path.dirname(os.path.abspath(__file__))
    os.chdir(web_dir)
    
    print("╔═══════════════════════════════════════════════════════════╗")
    print("║        Earth Engine WebGPU - Data-Oriented Edition        ║")
    print("╚═══════════════════════════════════════════════════════════╝")
    print()
    print(f"🚀 Server running at: http://localhost:{PORT}")
    print(f"📦 Serving from: {web_dir}")
    print()
    print("📋 Requirements:")
    print("  • Chrome Canary or Chrome 113+")
    print("  • WebGPU enabled (chrome://flags/#enable-unsafe-webgpu)")
    print()
    print("🎮 Controls:")
    print("  • WASD - Move")
    print("  • Mouse - Look (click to lock pointer)")
    print("  • Space - Up")
    print("  • Shift - Down")
    print()
    print("🔧 Architecture: 100% Data-Oriented Programming")
    print("  • No classes, no OOP")
    print("  • Pure functions + data structures")
    print("  • GPU buffers as single source of truth")
    print()
    print("Press Ctrl+C to stop the server")
    print("─" * 60)
    
    try:
        with socketserver.TCPServer(("", PORT), WebGPUDevServer) as httpd:
            httpd.serve_forever()
    except KeyboardInterrupt:
        print("\n\n✅ Server stopped")
        sys.exit(0)
    except Exception as e:
        print(f"\n❌ Error: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()