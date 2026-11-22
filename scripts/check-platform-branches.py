#!/usr/bin/env python3
"""Type-check the platform branches a host build never compiles.

`#[cfg(windows)]` and `#[cfg(not(unix))]` code is compiled only by the Windows
CI leg, so a host-only `cargo clippy` cannot see it. That blind spot hid a real
`E0308` in `crates/npm/src/install.rs` for a week: every local run was green and
only `test (windows-latest)` failed.

Two rules keep that from recurring, because neither needs a Windows runner:

1. **Width.** A platform-gated *function body* may not exceed
   `MAX_BLOCK_LINES`. Long bodies are exactly where type errors hide; a stub
   that returns `Ok(())`, `None`, or an error stays short and stays reviewable.
   When the platform-specific work is genuinely non-trivial, factor it into a
   function that is compiled on every host and gate only the thin wrapper.
2. **Type-check what can be type-checked.** Where the platform body is a
   one-liner over shared logic, the shared logic must be reachable host-side.

The checker is deliberately syntactic: it parses the attribute, the signature,
and the brace-matched body. It is not a Rust parser and does not try to be.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MAX_BLOCK_LINES = 6

GATE = re.compile(r"#\[cfg\((?:not\(unix\)|windows)\)\]")

# Files outside the Cargo workspace roots: gated code here is either a separate
# workspace (desktop, built by its own job) or a test file gated wholesale.
SKIP_PARTS = {"target", ".git", "vendor"}


def gated_items(path: Path) -> tuple[int, list[int]]:
    """Return (line count, body line indexes) for each gated item in `path`."""
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except (OSError, UnicodeDecodeError):
        return 0, []
    found = 0
    long_bodies: list[int] = []
    for index, line in enumerate(lines):
        if not GATE.search(line):
            continue
        found += 1
        cursor = index
        # Walk to the first `{` that opens the item's body, or to a `;` that
        # ends a gated declaration (a `const`, a `use`, an attribute-only item).
        while cursor < len(lines) and "{" not in lines[cursor]:
            if ";" in lines[cursor] and cursor > index:
                cursor = len(lines)
                break
            cursor += 1
        if cursor >= len(lines):
            continue
        depth = 0
        end = cursor
        while end < len(lines):
            depth += lines[end].count("{") - lines[end].count("}")
            if depth <= 0:
                break
            end += 1
        body = end - cursor
        if body > MAX_BLOCK_LINES:
            long_bodies.append(index + 1)
    return found, long_bodies


def main() -> int:
    roots = [ROOT / "crates"]
    failures: list[str] = []
    gated = 0
    for root in roots:
        for path in sorted(root.rglob("*.rs")):
            if SKIP_PARTS & set(path.parts):
                continue
            count, long_bodies = gated_items(path)
            gated += count
            relative = path.relative_to(ROOT)
            for line in long_bodies:
                failures.append(
                    f"{relative}:{line}: platform-gated body exceeds "
                    f"{MAX_BLOCK_LINES} lines; gate only a thin wrapper around "
                    "host-compiled logic"
                )
    if failures:
        print("\n".join(failures))
        print(
            f"\ncheck-platform-branches: {len(failures)} over-long gated "
            "body/bodies. Windows-only code is never compiled on the host, so "
            "keep it small enough to review by eye.",
            file=sys.stderr,
        )
        return 1
    print(
        f"check-platform-branches: {gated} platform-gated items, "
        f"all bodies <= {MAX_BLOCK_LINES} lines"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
