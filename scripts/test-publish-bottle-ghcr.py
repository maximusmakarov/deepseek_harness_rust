#!/usr/bin/env python3
"""Smoke-test the OCI publisher against a local in-memory registry.

Usage: test-publish-bottle-ghcr.py <version> <tag> <bottle> [<tag> <bottle> ...]

Every platform of one version must land in a single OCI index, otherwise brew
can only install whichever platform was pushed last.
"""

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
    version = sys.argv[1]
    pairs = sys.argv[2:]
    if len(pairs) % 2 != 0 or not pairs:
        raise SystemExit(__doc__)
    platforms = [(pairs[i], pairs[i + 1]) for i in range(0, len(pairs), 2)]

    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    registry = f"http://127.0.0.1:{server.server_port}"
    with tempfile.TemporaryDirectory():
        env = dict(os.environ, GITHUB_ACTOR="local", GITHUB_TOKEN="local")
        command = [
            sys.executable,
            "scripts/publish-bottle-ghcr.py",
            "--registry",
            registry,
            "--version",
            version,
        ]
        for tag, bottle in platforms:
            command += ["--bottle", bottle, "--tag", tag]
        command += ["--owner", "local", "--repo", "repo", "--push"]
        subprocess.run(command, check=True, env=env)

    index = json.loads(stored[version])
    assert index["mediaType"] == "application/vnd.oci.image.index.v1+json"
    expected = {
        tag: None for tag, _ in platforms
    }
    for child in index["manifests"]:
        ref_name = child["annotations"]["org.opencontainers.image.ref.name"]
        assert ref_name.startswith(f"{version}."), ref_name
        tag = ref_name.removeprefix(f"{version}.")
        assert tag in expected, f"unexpected platform {tag} in the index"
        assert expected[tag] is None, f"duplicate platform {tag} in the index"
        expected[tag] = child
        assert len(child["annotations"]["sh.brew.bottle.digest"]) == 64
        manifest = json.loads(stored[child["digest"]])
        assert manifest["layers"][0]["digest"].startswith("sha256:")
        assert manifest["annotations"]["sh.brew.tab"].strip()
        assert child["platform"]["os"] in {"darwin", "linux"}
        assert child["platform"]["architecture"] in {"arm64", "amd64"}
    missing = [tag for tag, child in expected.items() if child is None]
    assert not missing, f"platforms missing from the index: {missing}"
    print(f"ghcr publisher smoke ok ({len(index['manifests'])} platforms)")
    server.shutdown()


if __name__ == "__main__":
    main()
