#!/usr/bin/env python3
"""Publish Homebrew bottles to GHCR using the OCI layout brew expects.

The image path is <owner>/<repo>/<formula>. One OCI index per version is tagged
<version>; it holds one child manifest per bottle platform. Child manifests
carry sh.brew.bottle.digest, sh.brew.tab and
org.opencontainers.image.ref.name annotations.

All bottles of a version are pushed together on purpose: the tag points at the
index, so publishing platform by platform would replace the index each time and
leave only the last platform installable.
"""

import argparse
import base64
import hashlib
import json
import os
import re
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


def tag_from_filename(path: str, version: str) -> str:
    """Derive the bottle tag from dshr--<version>.<tag>.bottle.tar.gz."""
    base = os.path.basename(path)
    match = re.fullmatch(
        rf"dshr--{re.escape(version)}\.(?P<tag>[A-Za-z0-9_]+)\.bottle\.tar\.gz", base
    )
    if match is None:
        raise SystemExit(
            f"cannot derive a bottle tag from {base!r}; pass --tag explicitly"
        )
    return match.group("tag")


def manifest(version: str, tag: str, bottle: bytes, receipt: str):
    """Return (raw child manifest, index descriptor) for one platform."""
    if tag not in PLATFORMS:
        raise SystemExit(
            f"unknown bottle tag {tag!r}; expected one of {', '.join(sorted(PLATFORMS))}"
        )
    layer_digest = sha256(bottle)
    ref = f"{version}.{tag}"
    annotations = {
        "org.opencontainers.image.ref.name": ref,
        "sh.brew.bottle.digest": layer_digest.removeprefix("sha256:"),
        "sh.brew.tab": receipt,
    }
    child = {
        "schemaVersion": 2,
        "mediaType": MANIFEST_MEDIA,
        "config": {
            "mediaType": EMPTY_MEDIA,
            "digest": sha256(EMPTY),
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
    raw_child = json.dumps(child, separators=(",", ":")).encode()
    os_name, arch, os_version = PLATFORMS[tag]
    platform = {"architecture": arch, "os": os_name}
    if os_version:
        platform["os.version"] = os_version
    descriptor = {
        "mediaType": MANIFEST_MEDIA,
        "digest": sha256(raw_child),
        "size": len(raw_child),
        "platform": platform,
        "annotations": annotations,
    }
    return raw_child, descriptor


def index(descriptors) -> bytes:
    """Return the OCI index that lists every platform of one version."""
    ordered = sorted(descriptors, key=lambda d: d["annotations"]["org.opencontainers.image.ref.name"])
    body = {
        "schemaVersion": 2,
        "mediaType": INDEX_MEDIA,
        "manifests": ordered,
    }
    return json.dumps(body, separators=(",", ":")).encode()


def collect(version: str, bottles, tags):
    """Read the bottles and return [(tag, bottle bytes, raw child, descriptor)]."""
    if tags is None:
        tags = [tag_from_filename(path, version) for path in bottles]
    seen = set()
    entries = []
    for tag, path in zip(tags, bottles, strict=True):
        if tag in seen:
            raise SystemExit(f"duplicate bottle tag {tag!r}")
        seen.add(tag)
        bottle = open(path, "rb").read()
        raw_child, descriptor = manifest(version, tag, bottle, receipt_from_bottle(path, version))
        entries.append((tag, bottle, raw_child, descriptor))
    return entries


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


def push(registry, owner, repo, version, entries, actor, token):
    base = f"{registry}/v2/{owner}/{repo}/dshr"
    bearer = auth_token(registry, owner, repo, actor, token)
    ensure_blob(registry, base, bearer, EMPTY, sha256(EMPTY))
    for tag, bottle, raw_child, _ in entries:
        ensure_blob(registry, base, bearer, bottle, sha256(bottle))
        status, _, _ = request(
            "PUT",
            f"{base}/manifests/{sha256(raw_child)}",
            bearer,
            data=raw_child,
            content_type=MANIFEST_MEDIA,
        )
        if status not in (200, 201):
            raise SystemExit(f"manifest push failed for {tag}: {status}")
    raw_index = index([descriptor for _, _, _, descriptor in entries])
    status, _, _ = request(
        "PUT",
        f"{base}/manifests/{version}",
        bearer,
        data=raw_index,
        content_type=INDEX_MEDIA,
    )
    if status not in (200, 201):
        raise SystemExit(f"index push failed: {status}")
    platforms = ",".join(sorted(tag for tag, *_ in entries))
    print(f"ghcr.io/{owner}/{repo}/dshr:{version} ({platforms})")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--version", required=True)
    parser.add_argument(
        "--bottle",
        required=True,
        action="append",
        help="bottle tarball; repeat for every platform of the version",
    )
    parser.add_argument(
        "--tag",
        action="append",
        help="bottle tag per --bottle in order; inferred from the file name when omitted",
    )
    parser.add_argument("--registry", default="https://ghcr.io")
    parser.add_argument("--owner", required=True)
    parser.add_argument("--repo", required=True)
    parser.add_argument("--actor", default=os.environ.get("GITHUB_ACTOR", ""))
    parser.add_argument("--token", default=os.environ.get("GITHUB_TOKEN", ""))
    parser.add_argument("--dry-run", default="")
    parser.add_argument("--push", action="store_true")
    args = parser.parse_args()

    if args.tag is not None and len(args.tag) != len(args.bottle):
        parser.error("--tag must be repeated once per --bottle, or omitted")
    entries = collect(args.version, args.bottle, args.tag)
    raw_index = index([descriptor for _, _, _, descriptor in entries])
    if args.dry_run:
        os.makedirs(args.dry_run, exist_ok=True)
        for tag, _, raw_child, _ in entries:
            with open(os.path.join(args.dry_run, f"manifest-{tag}.json"), "wb") as handle:
                handle.write(raw_child)
        path = os.path.join(args.dry_run, "index.json")
        with open(path, "wb") as handle:
            handle.write(raw_index)
        print(path)
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
        entries,
        args.actor,
        args.token,
    )


if __name__ == "__main__":
    main()
