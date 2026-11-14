#!/usr/bin/env python3
"""Build a deterministic .deb with the CLI and desktop binaries."""

import argparse
import gzip
import io
import os
import struct
import tarfile


def add_file(tar, path, data, mode=0o644):
    info = tarfile.TarInfo(path)
    info.size = len(data)
    info.mode = mode
    info.mtime = 0
    info.uid = 0
    info.gid = 0
    info.uname = ""
    info.gname = ""
    tar.addfile(info, io.BytesIO(data))


def targz(files):
    raw = io.BytesIO()
    with tarfile.open(fileobj=raw, mode="w", format=tarfile.PAX_FORMAT) as tar:
        for path, (data, mode) in sorted(files.items()):
            add_file(tar, path, data, mode)
    data = raw.getvalue()
    compressed = gzip.compress(data, compresslevel=9, mtime=0)
    return compressed


def ar_member(name, data):
    header = (
        f"{name:<16}{0:<12}{0:<6}{0:<6}{0o100644:<8o}{len(data):<10}`\n".encode()
    )
    assert len(header) == 60
    return header + data + (b"\n" if len(data) % 2 else b"")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("version")
    parser.add_argument("arch", choices=["amd64", "arm64"])
    parser.add_argument("binary")
    parser.add_argument("desktop")
    parser.add_argument("output")
    args = parser.parse_args()

    binary = open(args.binary, "rb").read()
    desktop_binary = open(args.desktop, "rb").read()
    icon_path = os.path.join(os.path.dirname(__file__), "..", "crates/desktop/icons/icon.png")
    icon = open(icon_path, "rb").read()

    control = f"""Package: dshr
Version: {args.version}
Section: devel
Priority: optional
Architecture: {args.arch}
Maintainer: dshr <dshr@localhost>
Depends: libwebkit2gtk-4.1-0, libgtk-3-0
Description: Rust-native agent harness (CLI, TUI, HTTP/SSE, ACP, desktop)
"""
    control_files = {
        "control": (control.encode(), 0o644),
    }
    data_files = {
        "usr/bin/dshr": (binary, 0o755),
        "usr/bin/dshr-desktop": (desktop_binary, 0o755),
        "usr/share/applications/dshr.desktop": (
            b"[Desktop Entry]\nType=Application\nName=dshr\nExec=dshr-desktop\nTerminal=false\nCategories=Development;\n",
            0o644,
        ),
        "usr/share/icons/hicolor/128x128/apps/dshr.png": (icon, 0o644),
    }
    deb = (
        b"!<arch>\n"
        + ar_member("debian-binary", b"2.0\n")
        + ar_member("control.tar.gz", targz(control_files))
        + ar_member("data.tar.gz", targz(data_files))
    )
    with open(args.output, "wb") as handle:
        handle.write(deb)
    print(args.output)


if __name__ == "__main__":
    main()
