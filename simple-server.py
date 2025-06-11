#!/usr/bin/env python3
import http.server
import socketserver

PORT = 8080

class MyHandler(http.server.SimpleHTTPRequestHandler):
    def end_headers(self):
        self.send_header('Cache-Control', 'no-cache, no-store, must-revalidate')
        self.send_header('Pragma', 'no-cache')
        self.send_header('Expires', '0')
        super().end_headers()

    def do_GET(self):
        # Strip query parameters for file serving
        self.path = self.path.split('?')[0]
        return super().do_GET()

with socketserver.TCPServer(("", PORT), MyHandler) as httpd:
    print(f"Server at http://localhost:{PORT}")
    print(f"Try: http://localhost:{PORT}/web/run.html")
    httpd.serve_forever()