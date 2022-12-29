import sys
from pathlib import Path
import socketserver
import http.server
import os

print(f"{sys.argv = }")
os.chdir(sys.argv[1])

content_type = {
    "js": "application/javascript",
    "wasm": "application/wasm",
    "html": "text/html",
    "ico": "image/vnd.microsoft.icon",
}

class Handler(http.server.SimpleHTTPRequestHandler):
    def do_GET(self):
        path = self.path
        if path.endswith("/"):
            path += "index.html"
        assert path[0] == "/"
        path = Path("."+path)
        if not path.exists():
            self.send_response(404)
            self.end_headers()
            return
        self.send_response(200)
        self.send_header("Cross-Origin-Embedder-Policy", "require-corp")
        self.send_header("Cross-Origin-Opener-Policy", "same-origin")
        extension = str(path).split(".")[-1]
        self.send_header("Content-Type", content_type[extension]);
        self.end_headers()
        self.wfile.write(open(path, "rb").read())
        print(f"GET {self.path} {content_type[extension]}")

ADDR = ("0.0.0.0", 8080)
socketserver.TCPServer.allow_reuse_address = True
with socketserver.TCPServer(ADDR, Handler) as httpd:
    print(f"Serving on {ADDR}")
    httpd.serve_forever()
