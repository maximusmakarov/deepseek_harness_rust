#!/usr/bin/env python3
"""Run the Cordis conformance matrix and write docs/CONFORMANCE.md.

For every published DSH plugin in the set it creates a throwaway home, installs
the package with `dshr plugin npm add`, mounts it through `dshr prompt` and
records which tools and commands the host half actually registered, plus any
warning. Output is a Markdown table and `docs/conformance.json`.

Needs network (npm registry) and a node runtime pack in the home: pass
`--node-pack <dir>` (typically `<home>/runtimes`) or rely on a system node via
DSHR_NODE.

    python3 scripts/conformance.py --dshr ./target/debug/dshr \
        --node-pack /tmp/node-runtimes --out /tmp/conformance
"""

import argparse
import json
import os
import re
import shutil
import subprocess
import sys

PLUGINS = [
    "dsh-remote",
    "@kt11/dsh-deepseek-balance",
    "@ruoyang/dsh-plugin-deepseek-balance",
    "dsh-genui",
    "dsh-session-superagent",
    "dsh-mnemon",
    "dsh-plugin-market",
    "dsh-plugin-codegraph-panel",
    "dsh-plugin-notify",
    "dsh-plugin-debugging",
    "dsh-plugin-studio",
]


def slug(spec: str) -> str:
    return re.sub(r"[^a-zA-Z0-9._-]+", "_", spec)


def run(dshr: str, args, home: str, timeout: int, log: str) -> tuple[int, str]:
    with open(log, "w") as out:
        try:
            code = subprocess.run(
                [dshr, "--home", home, *args],
                stdout=out,
                stderr=subprocess.STDOUT,
                timeout=timeout,
            ).returncode
        except subprocess.TimeoutExpired:
            out.write(f"\nTIMEOUT after {timeout}s\n")
            code = -9
    return code, open(log, errors="ignore").read()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--dshr", default="./target/debug/dshr")
    parser.add_argument("--out", default="/tmp/conformance")
    parser.add_argument("--node-pack", help="directory copied into each home as runtimes/")
    parser.add_argument("--install-timeout", type=int, default=600)
    parser.add_argument("--mount-timeout", type=int, default=180)
    parser.add_argument("--plugins", nargs="*", default=PLUGINS)
    args = parser.parse_args()

    shutil.rmtree(args.out, ignore_errors=True)
    os.makedirs(args.out, exist_ok=True)
    rows = []
    for spec in args.plugins:
        name = slug(spec)
        home = os.path.join(args.out, name)
        os.makedirs(home, exist_ok=True)
        if args.node_pack:
            shutil.copytree(args.node_pack, os.path.join(home, "runtimes"))
        install_code, install_log = run(
            args.dshr, ["plugin", "npm", "add", spec], home, args.install_timeout,
            os.path.join(home, "install.log"),
        )
        mount_code, mount_log = run(
            args.dshr, ["prompt"], home, args.mount_timeout, os.path.join(home, "prompt.log")
        )
        packages = re.search(r"\((\d+) packages\)", install_log)
        tools = sorted(set(re.findall(r"cordis__[A-Za-z0-9._-]+__[A-Za-z0-9._-]+", mount_log)))
        commands = sorted(set(re.findall(r"cmd__[A-Za-z0-9._-]+__[A-Za-z0-9._-]+", mount_log)))
        warnings = [
            line for line in mount_log.splitlines()
            if '"level":"WARN"' in line or "Error:" in line or "error[" in line
        ]
        row = {
            "spec": spec,
            "install_code": install_code,
            "mount_code": mount_code,
            "packages": int(packages.group(1)) if packages else 0,
            "tools": tools,
            "commands": commands,
            "warnings": warnings[:3],
            "note": "",
        }
        if install_code != 0:
            tail = [line for line in install_log.strip().splitlines() if line.strip()]
            row["note"] = tail[-1][:180] if tail else "install failed"
        elif mount_code != 0:
            row["note"] = "prompt failed"
        elif warnings:
            row["note"] = warnings[0][:180]
        rows.append(row)
        print(f"{spec}: install={install_code} tools={len(tools)} commands={len(commands)}", flush=True)

    with open(os.path.join(args.out, "conformance.json"), "w") as handle:
        json.dump(rows, handle, ensure_ascii=False, indent=2)
    print(json.dumps(rows, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
