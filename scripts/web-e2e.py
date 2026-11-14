#!/usr/bin/env python3
"""Browser e2e for the dshr Web UI (ROADMAP 6.4/6.8).

Drives headless Chrome over the DevTools protocol, with no npm dependency:
the same harness `scripts/plugin-smoke.py` uses for the client-half bridge.
The server replays a cassette, so the scenario is deterministic:

  chat message -> approval dialog (bash) -> Allow -> tool execution ->
  streamed answer -> the file really exists in the workspace.

Chrome is required; the script skips with exit code 0 when it is missing.

Usage:
    python3 scripts/web-e2e.py [--binary target/debug/dshr] [--keep]
"""

from __future__ import annotations

import argparse
import importlib.util
import json
import shutil
import subprocess
import sys
import tempfile
import time
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TOKEN = "web-e2e-token"
# A cold CI runner compiles the 700 KB wasm bundle and paints the first frame
# well after 20 s; the local deadline must not decide whether CI is green.
LOAD_TIMEOUT = 90.0
STEP_TIMEOUT = 60.0
PROMPT = "write the artifact file and report back"

CASSETTE = {
    "model": "deepseek-flash",
    "calls": [
        [
            {"type": "start", "model": "deepseek-flash"},
            {"type": "tool_use_start", "index": 0, "id": "call_1", "name": "bash"},
            {"type": "tool_input_delta", "index": 0, "json": json.dumps({"command": "echo e2e > artifact.txt"})},
            {"type": "done", "stop_reason": "tool_use"},
        ],
        [
            {"type": "start", "model": "deepseek-flash"},
            {"type": "text_delta", "text": "Wrote the artifact."},
            {"type": "done", "stop_reason": "end_turn"},
        ],
    ],
}


def load_smoke():
    """Reuse the stdlib CDP client from plugin-smoke.py."""
    spec = importlib.util.spec_from_file_location("plugin_smoke", ROOT / "scripts" / "plugin-smoke.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)  # type: ignore[union-attr]
    return module


def start_serve(binary: Path, home: Path, workspace: Path, port: int, cassette: Path) -> subprocess.Popen:
    process = subprocess.Popen(
        [
            str(binary),
            "--home",
            str(home),
            "serve",
            "--port",
            str(port),
            "--token",
            TOKEN,
            "--cassette",
            str(cassette),
            "--model",
            "deepseek-flash",
        ],
        cwd=workspace,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    deadline = time.time() + 60
    while time.time() < deadline:
        if process.poll() is not None:
            output = process.stdout.read() if process.stdout else ""
            raise RuntimeError(f"server exited early:\n{output}")
        try:
            with urllib.request.urlopen(f"http://127.0.0.1:{port}/v1/health", timeout=1):
                return process
        except OSError:
            time.sleep(0.2)
    raise RuntimeError("server did not become healthy")


def evaluate(smoke, dt, expression: str, attempts: int = 8):
    """Evaluate in the page, retrying while a navigation swaps contexts."""
    last = None
    for _ in range(attempts):
        try:
            return dt.eval(expression)
        except Exception as error:  # noqa: BLE001 - retried below
            last = error
            time.sleep(0.25)
    raise TimeoutError(f"evaluate failed: {last}")


def run(args: argparse.Namespace) -> int:
    smoke = load_smoke()
    chrome = smoke.chrome_binary()
    if not chrome:
        print("SKIP: chrome was not found (set CHROME)")
        return 0
    # Resolve before spawning: the child runs with `cwd=workspace`, so a
    # relative `--binary target/debug/dshr` would be resolved inside that temp
    # directory and fail with ENOENT (this broke CI once).
    binary = Path(args.binary).resolve()
    if not binary.is_file():
        print(f"SKIP: binary not built: {binary}")
        return 0

    home = Path(tempfile.mkdtemp(prefix="dshr-web-home-"))
    workspace = Path(tempfile.mkdtemp(prefix="dshr-web-work-"))
    profile = Path(tempfile.mkdtemp(prefix="dshr-web-chrome-"))
    cassette = home / "cassette.json"
    cassette.write_text(json.dumps(CASSETTE))

    serve_port = smoke.free_port()
    chrome_port = smoke.free_port()
    while chrome_port == serve_port:
        chrome_port = smoke.free_port()
    smoke_server = smoke_chrome = None
    failures: list[str] = []
    checks = 0

    def check(label: str, condition: bool, detail: object = "") -> None:
        nonlocal checks
        checks += 1
        if condition:
            print(f"PASS {label}")
        else:
            print(f"FAIL {label} {detail}")
            failures.append(label)

    try:
        smoke_server = start_serve(binary, home, workspace, serve_port, cassette)
        smoke_chrome = smoke.start_chrome(chrome, chrome_port, profile)
        dt = smoke.DevTools(smoke.page_target(chrome_port))
        dt.call("Page.enable")
        dt.call("Runtime.enable")
        url = f"http://127.0.0.1:{serve_port}/?token={TOKEN}"
        dt.call("Page.navigate", {"url": url})

        def composer_ready() -> bool:
            return bool(evaluate(smoke, dt, "!!document.querySelector('textarea')"))

        try:
            smoke.wait_until(composer_ready, timeout=LOAD_TIMEOUT, label="web ui loaded")
        except Exception:  # noqa: BLE001 - one reload absorbs a cold start
            print("retrying the page load once")
            dt.call("Page.reload", {"ignoreCache": True})
            smoke.wait_until(composer_ready, timeout=LOAD_TIMEOUT, label="web ui loaded")
        check("web ui renders the chat composer", True)

        clicked_new = evaluate(
            smoke,
            dt,
            """(() => {
                const button = [...document.querySelectorAll('button')]
                    .find(b => b.textContent.trim() === 'New session');
                if (!button) return false;
                button.click();
                return true;
            })()""",
        )
        check("New session button starts a session", clicked_new is True)
        smoke.wait_until(
            lambda: evaluate(smoke, dt, "!!document.querySelector('button.session')"),
            timeout=STEP_TIMEOUT,
            label="session listed in the sidebar",
        )

        # Send the prompt through the real form path (native setter + input event
        # so the Leptos signal sees the change).
        evaluate(
            smoke,
            dt,
            """(() => {
                const ta = document.querySelector('textarea');
                const setter = Object.getOwnPropertyDescriptor(
                    window.HTMLTextAreaElement.prototype, 'value').set;
                setter.call(ta, %s);
                ta.dispatchEvent(new Event('input', {bubbles: true}));
                document.querySelector('button[type=submit]').click();
                return true;
            })()""" % json.dumps(PROMPT),
        )

        smoke.wait_until(
            lambda: evaluate(
                smoke,
                dt,
                "[...document.querySelectorAll('dialog h3')].some(h => h.textContent.includes('Approval'))",
            ),
            timeout=STEP_TIMEOUT,
            label="approval dialog",
        )
        check("approval dialog is shown for bash", True)
        approval_text = evaluate(smoke, dt, "document.querySelector('dialog')?.textContent || ''")
        check("approval names the tool", "bash" in approval_text, approval_text)

        clicked = evaluate(
            smoke,
            dt,
            """(() => {
                const button = [...document.querySelectorAll('dialog button')]
                    .find(b => b.textContent.trim() === 'Allow');
                if (!button) return false;
                button.click();
                return true;
            })()""",
        )
        check("Allow button resolves the approval", clicked is True)

        smoke.wait_until(
            lambda: evaluate(
                smoke,
                dt,
                "document.querySelector('#messages')?.textContent.includes('Wrote the artifact.')",
            ),
            timeout=STEP_TIMEOUT,
            label="streamed answer",
        )
        check("streamed answer is rendered", True)

        messages = evaluate(smoke, dt, "document.querySelector('#messages')?.textContent || ''")
        check("tool card is rendered for bash", "bash" in messages, messages)

        artifact = workspace / "artifact.txt"
        tool_ok = "✓ bash" in messages
        check(
            "the tool really wrote the file",
            artifact.exists() and artifact.read_text().strip() == "e2e",
            (artifact.read_text() if artifact.exists() else "missing")
            + ("" if tool_ok else " (the bash tool reported an error; see messages)"),
        )

        sessions = json.loads(
            urllib.request.urlopen(
                urllib.request.Request(
                    f"http://127.0.0.1:{serve_port}/v1/sessions",
                    headers={"Authorization": f"Bearer {TOKEN}"},
                )
            ).read()
        )
        check("session is journaled", len(sessions.get("sessions", sessions)) >= 1, sessions)

        dt.close()
    except Exception as error:  # noqa: BLE001 - e2e reports the failure verbatim
        failures.append(f"exception: {error}")
        print(f"::error::web-e2e: exception: {error}")
        print(f"FAIL exception: {error}")
        try:
            state = evaluate(
                smoke,
                dt,
                """JSON.stringify({
                    href: location.href,
                    ready: document.readyState,
                    status: document.querySelector('#status')?.textContent || '',
                    messages: document.querySelector('#messages')?.textContent || '',
                    body: document.body.innerText.slice(0, 400),
                })""",
            )
            print(f"  state={state}")
        except Exception:  # noqa: BLE001 - best effort diagnostics
            print("  diagnostics unavailable (page or DevTools target is gone)")
    finally:
        for process in (smoke_chrome, smoke_server):
            if process is not None and process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    process.kill()
        if not args.keep:
            for path in (home, workspace, profile):
                shutil.rmtree(path, ignore_errors=True)

    print(f"web-e2e: {checks - len(failures)}/{checks} checks passed")
    return 0 if not failures else 1


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", default=str(ROOT / "target" / "debug" / "dshr"))
    parser.add_argument("--keep", action="store_true", help="keep temp dirs for debugging")
    return run(parser.parse_args())


if __name__ == "__main__":
    sys.exit(main())
