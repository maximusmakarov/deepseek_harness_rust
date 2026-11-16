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
import os
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import threading
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
SECOND_PROMPT = "fix the notes file"

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
        [
            {"type": "start", "model": "deepseek-flash"},
            {"type": "tool_use_start", "index": 0, "id": "call_2", "name": "edit"},
            {"type": "tool_input_delta", "index": 0, "json": json.dumps({
                "path": "notes.txt", "old_string": "old line", "new_string": "new line"})},
            {"type": "done", "stop_reason": "tool_use"},
        ],
        [
            {"type": "start", "model": "deepseek-flash"},
            {"type": "text_delta", "text": "Edited the notes."},
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


def start_model_endpoint() -> tuple[ThreadingHTTPServer, str]:
    """A tiny OpenAI-compatible listing the discovery button can ask."""

    class Handler(BaseHTTPRequestHandler):
        def do_GET(self) -> None:  # noqa: N802 - http.server API
            if not self.path.endswith("/models"):
                self.send_response(404)
                self.send_header("content-length", "0")
                self.end_headers()
                return
            body = json.dumps(
                {
                    "data": [
                        {"id": "acme-one", "name": "Acme One"},
                        {"id": "acme-two", "name": "Acme Two"},
                    ]
                }
            ).encode()
            self.send_response(200)
            self.send_header("content-type", "application/json")
            self.send_header("content-length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, *_args: object) -> None:
            pass

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return server, f"http://127.0.0.1:{server.server_address[1]}/v1"


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
            # Nested sandboxes (a CI container or an agent sandbox) cannot run
            # macOS seatbelt; local verification may opt into danger-full-access.
            *(["--danger"] if os.environ.get("DSHR_E2E_DANGER") == "1" else []),
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
    (workspace / "notes.txt").write_text("old line\n")
    profile = Path(tempfile.mkdtemp(prefix="dshr-web-chrome-"))
    cassette = home / "cassette.json"
    cassette.write_text(json.dumps(CASSETTE))

    serve_port = smoke.free_port()
    chrome_port = smoke.free_port()
    while chrome_port == serve_port:
        chrome_port = smoke.free_port()
    smoke_server = smoke_chrome = None
    model_server = None
    models_url = ""
    failures: list[str] = []
    checks = 0

    def send_prompt(text: str) -> None:
        # Wait for the send button: it is disabled while a turn is running.
        smoke.wait_until(
            lambda: evaluate(smoke, dt, "!document.querySelector('button[type=submit]')?.disabled"),
            timeout=STEP_TIMEOUT,
            label="composer idle",
        )
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
            })()""" % json.dumps(text),
        )

    def check(label: str, condition: bool, detail: object = "") -> None:
        nonlocal checks
        checks += 1
        if condition:
            print(f"PASS {label}")
        else:
            print(f"FAIL {label} {detail}")
            failures.append(label)

    try:
        (home / "config.yaml").write_text(
            "version: 1\nmodel:\n  provider: deepseek-account\n  name: deepseek-flash\n"
        )
        smoke_server = start_serve(binary, home, workspace, serve_port, cassette)
        model_server, models_url = start_model_endpoint()
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
        send_prompt(PROMPT)

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

        # Chat/turn model: the shell tags each message with its turn ordinal and
        # keeps the composer/message slot regions mounted for chain entries.
        turns = evaluate(
            smoke,
            dt,
            """(() => [...document.querySelectorAll('#messages .message')]
                .map(node => node.dataset.turn ?? null))()""",
        )
        check(
            "messages carry the shell turn ordinal",
            isinstance(turns, list)
            and len(turns) >= 2
            and turns[0] is not None
            and turns[0] == turns[1],
            turns,
        )
        regions = evaluate(
            smoke,
            dt,
            """(() => ({
                composer: !!document.querySelector('[data-region="composer.actions"]'),
                messageAction: !!document.querySelector('[data-region="message.action"]'),
            }))()""",
        )
        check(
            "shell mounts composer and message action regions",
            isinstance(regions, dict) and regions.get("composer") is True and regions.get("messageAction") is True,
            regions,
        )

        messages = evaluate(smoke, dt, "document.querySelector('#messages')?.textContent || ''")
        check("tool card is rendered for bash", "bash" in messages, messages)

        # Models settings: provider rows, the editor, a saved profile and a
        # provider added from the catalog, all persisted in models.yaml.
        evaluate(
            smoke,
            dt,
            """(() => {
                const button = [...document.querySelectorAll('header button')]
                    .find(b => b.textContent.trim() === 'Settings');
                if (!button) return false;
                button.click();
                return true;
            })()""",
        )
        smoke.wait_until(
            lambda: evaluate(
                smoke,
                dt,
                "!!document.querySelector('dialog h3') && document.querySelector('dialog h3').textContent.trim() === 'Settings'",
            ),
            timeout=STEP_TIMEOUT,
            label="settings dialog",
        )
        evaluate(
            smoke,
            dt,
            """(() => {
                const tab = [...document.querySelectorAll('.settings-nav button')]
                    .find(b => b.textContent.trim() === 'Models');
                if (!tab) return false;
                tab.click();
                return true;
            })()""",
        )
        smoke.wait_until(
            lambda: evaluate(
                smoke,
                dt,
                "!!document.querySelector('.models-section')",
            ),
            timeout=STEP_TIMEOUT,
            label="models section",
        )
        smoke.wait_until(
            lambda: evaluate(
                smoke,
                dt,
                "!!document.querySelector('.models-row[data-provider=\"deepseek-account\"]')",
            ),
            timeout=STEP_TIMEOUT,
            label="catalog provider row",
        )
        smoke.wait_until(
            lambda: evaluate(smoke, dt, "!!document.querySelector('.models-welcome')"),
            timeout=STEP_TIMEOUT,
            label="welcome notice",
        )
        evaluate(
            smoke,
            dt,
            """(() => {
                [...document.querySelectorAll('.models-welcome button')]
                    .find(b => b.textContent.trim() === 'Continue').click();
                return true;
            })()""",
        )
        smoke.wait_until(
            lambda: evaluate(smoke, dt, "!document.querySelector('.models-welcome')"),
            timeout=STEP_TIMEOUT,
            label="welcome acknowledged",
        )
        check(
            "the first-run notice is acknowledged durably",
            "welcome_version: 1" in (home / "models.yaml").read_text()
            if (home / "models.yaml").exists()
            else False,
        )
        check(
            "models mounts its child slot regions",
            bool(
                evaluate(
                    smoke,
                    dt,
                    """!!document.querySelector('[data-region="settings.models.card"]')
                        && !!document.querySelector('[data-region="settings.models.footer"]')""",
                )
            ),
        )
        check(
            "models lists the active provider with its catalog models",
            evaluate(
                smoke,
                dt,
                """(() => {
                    const row = document.querySelector('.models-row[data-provider="deepseek-account"]');
                    return row ? row.textContent : '';
                })()""",
            ).find("2 model(s)") >= 0,
            evaluate(smoke, dt, "document.querySelector('.models-section')?.textContent || ''"),
        )

        # Edit the active provider: endpoint override and a renamed model.
        evaluate(
            smoke,
            dt,
            """(() => {
                const row = document.querySelector('.models-row[data-provider="deepseek-account"]');
                const edit = row && [...row.querySelectorAll('button')]
                    .find(b => b.textContent.trim() === 'Edit');
                if (!edit) return false;
                edit.click();
                return true;
            })()""",
        )
        smoke.wait_until(
            lambda: evaluate(
                smoke,
                dt,
                """!!document.querySelector('.models-editor-card[data-provider="deepseek-account"]')""",
            ),
            timeout=STEP_TIMEOUT,
            label="provider editor",
        )
        evaluate(
            smoke,
            dt,
            """(() => {
                const editor = document.querySelector('.models-editor-card[data-provider="deepseek-account"]');
                const fields = editor.querySelectorAll('.models-field input');
                const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
                setter.call(fields[0], 'https://gateway.example/anthropic');
                fields[0].dispatchEvent(new Event('input', {bubbles: true}));
                const nameInput = editor.querySelectorAll('.models-model-row > input')[1];
                setter.call(nameInput, 'E2E Flash');
                nameInput.dispatchEvent(new Event('input', {bubbles: true}));
                const save = [...editor.querySelectorAll('button')]
                    .find(b => b.textContent.trim() === 'Save');
                save.click();
                return true;
            })()""",
        )
        smoke.wait_until(
            lambda: evaluate(
                smoke,
                dt,
                "document.querySelector('.models-status')?.textContent.includes('saved')",
            ),
            timeout=STEP_TIMEOUT,
            label="models profile saved",
        )
        models_file = home / "models.yaml"
        check(
            "the Models profile is written to models.yaml",
            models_file.exists() and "gateway.example" in models_file.read_text() and "E2E Flash" in models_file.read_text(),
            models_file.read_text() if models_file.exists() else "missing",
        )

        # Add a catalog provider and give it a key reference.
        evaluate(
            smoke,
            dt,
            """(() => {
                const select = document.querySelector('.models-add select');
                select.value = 'openai';
                select.dispatchEvent(new Event('change', {bubbles: true}));
                const add = [...document.querySelectorAll('.models-add button')]
                    .find(b => b.textContent.trim() === 'Add');
                add.click();
                return true;
            })()""",
        )
        smoke.wait_until(
            lambda: evaluate(
                smoke,
                dt,
                "!!document.querySelector('.models-editor-card[data-provider=\"openai\"]')",
            ),
            timeout=STEP_TIMEOUT,
            label="openai draft editor",
        )
        evaluate(
            smoke,
            dt,
            """(() => {
                const editor = document.querySelector('.models-editor-card[data-provider="openai"]');
                const fields = editor.querySelectorAll('.models-field input');
                const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
                setter.call(fields[0], 'https://openai.example/v1');
                fields[0].dispatchEvent(new Event('input', {bubbles: true}));
                setter.call(fields[1], 'E2E_OPENAI_KEY');
                fields[1].dispatchEvent(new Event('input', {bubbles: true}));
                [...editor.querySelectorAll('button')]
                    .find(b => b.textContent.trim() === 'Save').click();
                return true;
            })()""",
        )
        smoke.wait_until(
            lambda: evaluate(
                smoke,
                dt,
                "document.querySelector('.models-status')?.textContent.includes('openai')",
            ),
            timeout=STEP_TIMEOUT,
            label="openai profile saved",
        )
        written = models_file.read_text()
        check(
            "the added provider is persisted next to the active one",
            "openai:" in written and "E2E_OPENAI_KEY" in written,
            written,
        )

        # The row offers a deletion confirmation and the write removes the
        # stored profile, restoring the catalog defaults.
        evaluate(
            smoke,
            dt,
            """(() => {
                const row = document.querySelector('.models-row[data-provider="openai"]');
                const remove = row && [...row.querySelectorAll('.models-row-head button')]
                    .find(b => b.textContent.trim() === 'Delete');
                if (!remove) return false;
                remove.click();
                return true;
            })()""",
        )
        smoke.wait_until(
            lambda: evaluate(
                smoke,
                dt,
                "!!document.querySelector('.models-modal')",
            ),
            timeout=STEP_TIMEOUT,
            label="delete confirmation",
        )
        evaluate(
            smoke,
            dt,
            """(() => {
                const modal = document.querySelector('.models-modal');
                [...modal.querySelectorAll('button')]
                    .find(b => b.textContent.trim() === 'Delete').click();
                return true;
            })()""",
        )
        smoke.wait_until(
            lambda: evaluate(
                smoke,
                dt,
                "!!document.querySelector('.models-status') && document.querySelector('.models-status').textContent.includes('removed')",
            ),
            timeout=STEP_TIMEOUT,
            label="provider removed",
        )
        check(
            "deleting a provider restores the catalog defaults",
            "openai:" not in models_file.read_text(),
            models_file.read_text(),
        )

        # A hand-declared provider: id, protocol, endpoint, key reference and
        # one model, all persisted through the same revision-fenced write.
        evaluate(smoke, dt, "window.__MODELS_URL = %s" % json.dumps(models_url))
        evaluate(
            smoke,
            dt,
            """(() => {
                [...document.querySelectorAll('.models-mode button')]
                    .find(b => b.textContent.trim() === 'Custom model API').click();
                return true;
            })()""",
        )
        smoke.wait_until(
            lambda: evaluate(
                smoke,
                dt,
                "!!document.querySelector('.models-editor-card[data-provider=\"\"]')",
            ),
            timeout=STEP_TIMEOUT,
            label="custom provider card",
        )
        evaluate(
            smoke,
            dt,
            """(() => {
                const card = document.querySelector('.models-editor-card[data-provider=""]');
                const fields = card.querySelectorAll('.models-field input');
                const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
                const set = (input, value) => {
                    setter.call(input, value);
                    input.dispatchEvent(new Event('input', {bubbles: true}));
                };
                set(fields[0], 'acme-gateway');
                set(fields[1], 'Acme Gateway');
                set(fields[2], window.__MODELS_URL);
                set(fields[3], 'ACME_GATEWAY_KEY');
                [...card.querySelectorAll('button')]
                    .find(b => b.textContent.trim() === 'Fetch available models').click();
                return true;
            })()""",
        )
        smoke.wait_until(
            lambda: evaluate(smoke, dt, "!!document.querySelector('.models-picker')"),
            timeout=STEP_TIMEOUT,
            label="discovered model picker",
        )
        candidates = evaluate(smoke, dt, "document.querySelector('.models-picker')?.textContent || ''")
        check(
            "discovery lists the endpoint models",
            "acme-one" in candidates and "acme-two" in candidates,
            candidates,
        )
        evaluate(
            smoke,
            dt,
            """(() => {
                [...document.querySelectorAll('.models-picker button')]
                    .find(b => b.textContent.trim() === 'Update list').click();
                return true;
            })()""",
        )
        smoke.wait_until(
            lambda: evaluate(
                smoke,
                dt,
                "document.querySelectorAll('.models-editor-card[data-provider=\"acme-gateway\"] .models-model-row').length === 2",
            ),
            timeout=STEP_TIMEOUT,
            label="custom model rows from discovery",
        )
        evaluate(
            smoke,
            dt,
            """(() => {
                const rows = document.querySelectorAll('.models-editor-card[data-provider="acme-gateway"] .models-model-block');
                rows[1].querySelector('button[title="remove this row"]').click();
                return true;
            })()""",
        )
        smoke.wait_until(
            lambda: evaluate(smoke, dt, "!!document.querySelector('.models-modal')"),
            timeout=STEP_TIMEOUT,
            label="model deletion confirmation",
        )
        modal_text = evaluate(smoke, dt, "document.querySelector('.models-modal')?.textContent || ''")
        check("model deletion names the model", "acme-two" in modal_text, modal_text)
        evaluate(
            smoke,
            dt,
            """(() => {
                [...document.querySelectorAll('.models-modal button')]
                    .find(b => b.textContent.trim() === 'Cancel').click();
                return true;
            })()""",
        )
        smoke.wait_until(
            lambda: evaluate(
                smoke,
                dt,
                "document.querySelectorAll('.models-editor-card[data-provider=\"acme-gateway\"] .models-model-row').length === 2",
            ),
            timeout=STEP_TIMEOUT,
            label="model deletion cancelled",
        )
        evaluate(
            smoke,
            dt,
            """(() => {
                const card = document.querySelector('.models-editor-card[data-provider="acme-gateway"]');
                const input = card.querySelector('.models-model-row > input');
                const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
                setter.call(input, 'acme-one');
                input.dispatchEvent(new Event('input', {bubbles: true}));
                // The reasoning switch starts from off/low/medium/high.
                card.querySelector('.models-reasoning input[type=checkbox]').click();
                // Choose the explicit normal retry policy; the draft starts
                // from the upstream defaults.
                card.querySelector('.models-retry summary').click();
                const mode = card.querySelector('.models-retry select');
                mode.value = 'normal';
                mode.dispatchEvent(new Event('change', {bubbles: true}));
                [...card.querySelectorAll('button')]
                    .find(b => b.textContent.trim() === 'Save').click();
                return true;
            })()""",
        )
        smoke.wait_until(
            lambda: evaluate(
                smoke,
                dt,
                "document.querySelector('.models-status')?.textContent.includes('acme-gateway')",
            ),
            timeout=STEP_TIMEOUT,
            label="custom provider saved",
        )
        written = models_file.read_text()
        check(
            "a custom provider is persisted with its protocol, model, reasoning levels and retry policy",
            "acme-gateway:" in written
            and "ACME_GATEWAY_KEY" in written
            and "acme-one" in written
            and "acme-two" in written
            and "reasoning_efforts" in written
            and "high" in written
            and "retry:" in written
            and "max_retries: 5" in written
            and "RATE_LIMIT" in written,
            written,
        )

        evaluate(
            smoke,
            dt,
            """[...document.querySelectorAll('dialog button')]
                .find(b => b.textContent.trim() === 'Close')?.click()""",
        )

        # Second turn: `edit` must show a -old/+new preview and change the file.
        send_prompt(SECOND_PROMPT)
        smoke.wait_until(
            lambda: evaluate(
                smoke,
                dt,
                "document.querySelector('#messages')?.textContent.includes('Edited the notes.')",
            ),
            timeout=STEP_TIMEOUT,
            label="second answer",
        )
        transcript = evaluate(smoke, dt, "document.querySelector('#messages')?.textContent || ''")
        check("diff preview shows the removed line", "-old line" in transcript, transcript)
        check("diff preview shows the added line", "+new line" in transcript, transcript)
        notes = workspace / "notes.txt"
        check(
            "the edit really changed the file",
            notes.exists() and notes.read_text() == "new line\n",
            notes.read_text() if notes.exists() else "missing",
        )

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
        if model_server is not None:
            model_server.shutdown()
            model_server.server_close()
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
