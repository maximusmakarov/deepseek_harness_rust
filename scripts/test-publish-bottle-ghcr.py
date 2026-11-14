#!/usr/bin/env python3
"""Smoke-test the OCI publisher against a local in-memory registry."""

import http.server
import json
import os
import subprocess
import sys
import tempfile
import threading
import urllib.parse

stored = {}


class Handler(http.server.BaseHTTPRequestHandler):
    def log_message(self, *_args):
        pass

    def _json(self, value, status=200):
        body = json.dumps(value).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        if self.path.startswith("/token"):
            self._json({"token": "local"})
        else:
            self.send_response(404)
            self.end_headers()

    def do_HEAD(self):
        digest = self.path.rsplit("/", 1)[-1]
        self.send_response(200 if digest in stored else 404)
        self.end_headers()

    def do_POST(self):
        self.send_response(202)
        self.send_header("Location", self.path.replace("/blobs/uploads/", "/uploads/u1"))
        self.end_headers()

    def do_PUT(self):
        length = int(self.headers.get("Content-Length", "0"))
        body = self.rfile.read(length)
        if "/manifests/" in self.path:
            stored[self.path.rsplit("/", 1)[-1]] = body
            self._json({"ok": True}, 201)
        else:
            stored[urllib.parse.parse_qs(self.path.split("?", 1)[1])["digest"][0]] = body
            self._json({"ok": True}, 201)


def main():
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    registry = f"http://127.0.0.1:{server.server_port}"
    bottle = sys.argv[1]
    with tempfile.TemporaryDirectory() as directory:
        env = dict(os.environ, GITHUB_ACTOR="local", GITHUB_TOKEN="local")
        subprocess.run(
            [
                sys.executable,
                "scripts/publish-bottle-ghcr.py",
                "--registry",
                registry,
                "--version",
                "0.1.0",
                "--tag",
                "arm64_sequoia",
                "--bottle",
                bottle,
                "--owner",
                "local",
                "--repo",
                "repo",
                "--push",
            ],
            check=True,
            env=env,
        )
    index = json.loads(stored["0.1.0"])
    assert index["mediaType"] == "application/vnd.oci.image.index.v1+json"
    child = index["manifests"][0]
    assert child["annotations"]["org.opencontainers.image.ref.name"] == "0.1.0.arm64_sequoia"
    assert len(child["annotations"]["sh.brew.bottle.digest"]) == 64
    manifest = json.loads(stored[child["digest"]])
    assert manifest["layers"][0]["digest"].startswith("sha256:")
    print("ghcr publisher smoke ok")
    server.shutdown()


if __name__ == "__main__":
    main()
