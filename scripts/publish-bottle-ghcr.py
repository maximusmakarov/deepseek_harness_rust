#!/usr/bin/env python3
"""Publish a Homebrew bottle to GHCR using the OCI layout brew expects.

The image path is <owner>/<repo>/<formula>; each version is an OCI index
tagged <version>. Child manifests carry sh.brew.bottle.digest,
sh.brew.tab and org.opencontainers.image.ref.name annotations.
"""

import argparse
import base64
import hashlib
import io
import json
import os
import tarfile
import urllib.error
import urllib.parse
import urllib.request

EMPTY = b"{}"
EMPTY_MEDIA = "application/vnd.oci.empty.v1+json"
MANIFEST_MEDIA = "application/vnd.oci.image.manifest.v1+json"
INDEX_MEDIA = "application/vnd.oci.image.index.v1+json"
LAYER_MEDIA = "application/vnd.oci.image.layer.v1.tar+gzip"

PLATFORMS = {
    "arm64_tahoe": ("darwin", "arm64", "26"),
    "arm64_sequoia": ("darwin", "arm64", "15"),
    "sonoma": ("darwin", "amd64", "14"),
    "x86_64_linux": ("linux", "amd64", None),
    "aarch64_linux": ("linux", "arm64", None),
}


def sha256(data: bytes) -> str:
    return "sha256:" + hashlib.sha256(data).hexdigest()


def receipt_from_bottle(path: str, version: str) -> str:
    with tarfile.open(path, "r:gz") as tar:
        name = f"dshr/{version}/INSTALL_RECEIPT.json"
        member = tar.extractfile(name)
        if member is None:
            return "{}"
        return member.read().decode()


def manifests(version: str, tag: str, bottle: bytes, receipt: str):
    config_digest = sha256(EMPTY)
    layer_digest = sha256(bottle)
    ref = f"{version}.{tag}"
    annotations = {
        "org.opencontainers.image.ref.name": ref,
        "sh.brew.bottle.digest": layer_digest.removeprefix("sha256:"),
        "sh.brew.tab": receipt,
    }
    manifest = {
        "schemaVersion": 2,
        "mediaType": MANIFEST_MEDIA,
        "config": {
            "mediaType": EMPTY_MEDIA,
            "digest": config_digest,
            "size": len(EMPTY),
        },
        "layers": [
            {
                "mediaType": LAYER_MEDIA,
                "digest": layer_digest,
                "size": len(bottle),
            }
        ],
        "annotations": annotations,
    }
    raw_manifest = json.dumps(manifest, separators=(",", ":")).encode()
    os_name, arch, os_version = PLATFORMS[tag]
    platform = {"architecture": arch, "os": os_name}
    if os_version:
        platform["os.version"] = os_version
    descriptor = {
        "mediaType": MANIFEST_MEDIA,
        "digest": sha256(raw_manifest),
        "size": len(raw_manifest),
        "platform": platform,
        "annotations": annotations,
    }
    index = {
        "schemaVersion": 2,
        "mediaType": INDEX_MEDIA,
        "manifests": [descriptor],
    }
    return raw_manifest, json.dumps(index, separators=(",", ":")).encode()


def auth_token(registry: str, owner: str, repo: str, actor: str, token: str) -> str:
    query = urllib.parse.urlencode(
        {
            "service": "ghcr.io",
            "scope": f"repository:{owner}/{repo}/dshr:pull,push",
        }
    )
    request = urllib.request.Request(f"{registry}/token?{query}")
    basic = base64.b64encode(f"{actor}:{token}".encode()).decode()
    request.add_header("Authorization", f"Basic {basic}")
    with urllib.request.urlopen(request, timeout=30) as response:
        return json.load(response)["token"]


def request(method, url, token, data=None, content_type=None):
    req = urllib.request.Request(url, data=data, method=method)
    req.add_header("Authorization", f"Bearer {token}")
    if content_type:
        req.add_header("Content-Type", content_type)
    try:
        with urllib.request.urlopen(req, timeout=120) as response:
            return response.status, response.headers, response.read()
    except urllib.error.HTTPError as error:
        if error.code == 404:
            return 404, error.headers, b""
        raise


def ensure_blob(registry, base, token, data, digest):
    status, _, _ = request("HEAD", f"{base}/blobs/{digest}", token)
    if status == 200:
        return
    status, headers, _ = request("POST", f"{base}/blobs/uploads/", token)
    location = headers["Location"]
    if location.startswith("/"):
        location = registry + location
    separator = "&" if "?" in location else "?"
    request(
        "PUT",
        f"{location}{separator}digest={urllib.parse.quote(digest)}",
        token,
        data=data,
        content_type="application/octet-stream",
    )


def push(registry, owner, repo, version, tag, bottle_path, actor, token):
    bottle = open(bottle_path, "rb").read()
    receipt = receipt_from_bottle(bottle_path, version)
    raw_manifest, raw_index = manifests(version, tag, bottle, receipt)
    base = f"{registry}/v2/{owner}/{repo}/dshr"
    bearer = auth_token(registry, owner, repo, actor, token)
    ensure_blob(registry, base, bearer, EMPTY, sha256(EMPTY))
    ensure_blob(registry, base, bearer, bottle, sha256(bottle))
    manifest_digest = sha256(raw_manifest)
    status, _, _ = request(
        "PUT",
        f"{base}/manifests/{manifest_digest}",
        bearer,
        data=raw_manifest,
        content_type=MANIFEST_MEDIA,
    )
    if status not in (200, 201):
        raise SystemExit(f"manifest push failed: {status}")
    status, _, _ = request(
        "PUT",
        f"{base}/manifests/{version}",
        bearer,
        data=raw_index,
        content_type=INDEX_MEDIA,
    )
    if status not in (200, 201):
        raise SystemExit(f"index push failed: {status}")
    print(f"ghcr.io/{owner}/{repo}/dshr:{version} ({tag})")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--version", required=True)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--bottle", required=True)
    parser.add_argument("--registry", default="https://ghcr.io")
    parser.add_argument("--owner", required=True)
    parser.add_argument("--repo", required=True)
    parser.add_argument("--actor", default=os.environ.get("GITHUB_ACTOR", ""))
    parser.add_argument("--token", default=os.environ.get("GITHUB_TOKEN", ""))
    parser.add_argument("--dry-run", default="")
    parser.add_argument("--push", action="store_true")
    args = parser.parse_args()

    bottle = open(args.bottle, "rb").read()
    receipt = receipt_from_bottle(args.bottle, args.version)
    raw_manifest, raw_index = manifests(args.version, args.tag, bottle, receipt)
    if args.dry_run:
        os.makedirs(args.dry_run, exist_ok=True)
        with open(os.path.join(args.dry_run, "manifest.json"), "wb") as handle:
            handle.write(raw_manifest)
        with open(os.path.join(args.dry_run, "index.json"), "wb") as handle:
            handle.write(raw_index)
        print(os.path.join(args.dry_run, "index.json"))
        return
    if not args.push:
        parser.error("pass --push or --dry-run")
    if not args.actor or not args.token:
        raise SystemExit("GITHUB_ACTOR and GITHUB_TOKEN are required for --push")
    push(
        args.registry.rstrip("/"),
        args.owner,
        args.repo,
        args.version,
        args.tag,
        args.bottle,
        args.actor,
        args.token,
    )


if __name__ == "__main__":
    main()
