#!/usr/bin/env python3
"""Simple HTTP server for testing Earth Engine web demo"""

import http.server
import socketserver
import os

PORT = 8080

class MyHTTPRequestHandler(http.server.SimpleHTTPRequestHandler):
    def end_headers(self):
        # Add CORS headers for local development
        self.send_header('Access-Control-Allow-Origin', '*')
        self.send_header('Access-Control-Allow-Methods', 'GET, POST, OPTIONS')
        self.send_header('Cross-Origin-Embedder-Policy', 'require-corp')
        self.send_header('Cross-Origin-Opener-Policy', 'same-origin')
        super().end_headers()

os.chdir(os.path.dirname(os.path.abspath(__file__)))

with socketserver.TCPServer(("", PORT), MyHTTPRequestHandler) as httpd:
    print(f"Server running at http://localhost:{PORT}/")
    print(f"Open http://localhost:{PORT}/index.html in your browser")
    print("Press Ctrl+C to stop")
    httpd.serve_forever()