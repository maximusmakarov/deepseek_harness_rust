#!/usr/bin/env python3
"""End-to-end smoke test for the Cordis client-half bridge.

Starts a real `dshr serve` with two throwaway plugins (a lazy multi-slot half
and an eager `immediately` half), drives the Web UI in headless Chrome over the
DevTools protocol and checks the parts unit tests cannot see:

  * client-half discovery and slot tabs (``panel`` and ``settings.section``)
  * slot switching and the ``resize`` message
  * live theme propagation into the sandboxed frame
  * ``ctx.action`` round-trips (unsupported action, ``close``)
  * an eager half stays mounted after the panel closes

Only the Python standard library is used. Chrome and node are required; the
script skips with exit code 0 when either is missing.

Usage:
    python3 scripts/plugin-smoke.py [--binary target/debug/dshr] [--keep]
"""

from __future__ import annotations

import argparse
import base64
import json
import os
import shutil
import socket
import struct
import subprocess
import sys
import tempfile
import time
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TOKEN = "smoke-token"


# --------------------------------------------------------------------------
# Minimal CDP client (Chrome DevTools Protocol over WebSocket, stdlib only)
# --------------------------------------------------------------------------


class DevTools:
    def __init__(self, ws_url: str) -> None:
        assert ws_url.startswith("ws://"), ws_url
        rest = ws_url[len("ws://"):]
        host_port, path = rest.split("/", 1)
        host, port = host_port.split(":")
        self.sock = socket.create_connection((host, int(port)), timeout=20)
        key = base64.b64encode(os.urandom(16)).decode()
        request = (
            f"GET /{path} HTTP/1.1\r\nHost: {host_port}\r\n"
            "Upgrade: websocket\r\nConnection: Upgrade\r\n"
            f"Sec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n\r\n"
        )
        self.sock.sendall(request.encode())
        buffer = b""
        while b"\r\n\r\n" not in buffer:
            chunk = self.sock.recv(4096)
            if not chunk:
                raise RuntimeError("DevTools handshake failed")
            buffer += chunk
        self.buffer = buffer.split(b"\r\n\r\n", 1)[1]
        self.message_id = 0

    def _read(self, count: int) -> bytes:
        while len(self.buffer) < count:
            chunk = self.sock.recv(1 << 16)
            if not chunk:
                raise RuntimeError("DevTools connection closed")
            self.buffer += chunk
        data, self.buffer = self.buffer[:count], self.buffer[count:]
        return data

    def _send(self, payload: dict) -> None:
        body = json.dumps(payload).encode()
        mask = os.urandom(4)
        header = bytearray([0x81])
        if len(body) < 126:
            header.append(0x80 | len(body))
        elif len(body) < 1 << 16:
            header.append(0x80 | 126)
            header += struct.pack(">H", len(body))
        else:
            header.append(0x80 | 127)
            header += struct.pack(">Q", len(body))
        header += mask
        masked = bytes(byte ^ mask[index % 4] for index, byte in enumerate(body))
        self.sock.sendall(bytes(header) + masked)

    def _recv(self) -> dict:
        _first, second = self._read(2)
        length = second & 0x7F
        if length == 126:
            length = struct.unpack(">H", self._read(2))[0]
        elif length == 127:
            length = struct.unpack(">Q", self._read(8))[0]
        return json.loads(self._read(length))

    def call(self, method: str, params: dict | None = None, timeout: float = 20.0) -> dict:
        self.message_id += 1
        message_id = self.message_id
        self._send({"id": message_id, "method": method, "params": params or {}})
        deadline = time.time() + timeout
        while time.time() < deadline:
            message = self._recv()
            if message.get("id") == message_id:
                if "error" in message:
                    raise RuntimeError(f"{method}: {message['error']}")
                return message.get("result", {})
        raise TimeoutError(method)

    def eval(self, expression: str, await_promise: bool = False):
        result = self.call(
            "Runtime.evaluate",
            {
                "expression": expression,
                "returnByValue": True,
                "awaitPromise": await_promise,
            },
        )
        remote = result.get("result", {})
        if "exceptionDetails" in result:
            raise RuntimeError(f"evaluate failed: {result['exceptionDetails']}")
        return remote.get("value")

    def close(self) -> None:
        try:
            self.sock.close()
        except OSError:
            pass


def chrome_binary() -> str | None:
    candidates = [
        os.environ.get("CHROME"),
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        shutil.which("google-chrome"),
        shutil.which("chromium"),
        shutil.which("chromium-browser"),
    ]
    return next((path for path in candidates if path and Path(path).exists()), None)


def free_port() -> int:
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


# --------------------------------------------------------------------------
# Fixtures
# --------------------------------------------------------------------------

DEMO_MANIFEST = {
    "name": "@smoke/demo",
    "version": "1.0.0",
    "main": "plugin.mjs",
    "dsh": {"client": {"platform": "web", "entry": "client.js"}},
}

DEMO_PLUGIN = """
export default {
  apply(ctx) {
    ctx.tool({ name: "echo", description: "echo", inputSchema: { type: "object" }, handler: () => "ok" });
  },
};
"""

DEMO_CLIENT = """
export default function activate(ctx) {
  const panel = (el) => {
    el.innerHTML = `
      <h3>Smoke panel v1</h3>
      <p id="slot-label">slot: panel · theme: ${ctx.theme}</p>
      <p id="action-status">action: idle</p>
      <button id="close-btn">close</button>
      <button id="nope-btn">nope</button>
      <div style="height:260px">tall content</div>`;
    el.querySelector("#close-btn").onclick = () => ctx.action("close");
    el.querySelector("#nope-btn").onclick = () => {
      ctx.action("nope").then((result) => {
        el.querySelector("#action-status").textContent =
          "action: " + (result.ok ? "ok" : result.error);
      });
    };
  };
  const settings = (el) => {
    el.innerHTML = "<h3>Smoke settings</h3><p id=\\"slot-label\\">slot: settings</p>";
  };
  ctx.slots.register({ name: "panel", id: "smoke-panel", title: "Smoke panel", order: 10 }, panel);
  ctx.slots.register({ name: "settings.section", id: "smoke-settings", title: "Smoke settings", order: 20 }, settings);
  ctx.onTheme((theme) => { document.body.dataset.pluginTheme = theme; });
  ctx.effect(() => {
    const timer = setInterval(() => { document.body.dataset.tick = String(Date.now()); }, 500);
    return () => clearInterval(timer);
  });
}
"""

# A half that uses the DSH slot names for the subset the bridge promises
# (ADR-013 §5): a panel plus a message action and a settings action.
DSH_MANIFEST = {
    "name": "@smoke/dsh-slots",
    "version": "1.0.0",
    "main": "plugin.mjs",
    "dsh": {"client": {"platform": "web", "entry": "client.js"}},
}

DSH_CLIENT = """
export default function activate(ctx) {
  const render = (label) => (el) => {
    el.innerHTML = "<p id=\\"slot-label\\">slot: " + label + "</p>";
  };
  ctx.slots.register({ name: "panel", id: "dsh-panel", title: "DSH panel", order: 30 }, render("panel"));
  ctx.slots.register({ name: "message.action", id: "dsh-message", title: "Message action", order: 31 }, render("message.action"));
  ctx.slots.register({ name: "settings.action", id: "dsh-settings-action", title: "Settings action", order: 32 }, render("settings.action"));
}
"""

ASSETS_MANIFEST = {
    "name": "@smoke/assets",
    "version": "1.0.0",
    "main": "plugin.mjs",
    "dsh": {"client": {"platform": "web", "entry": "client.js"}},
}

ASSETS_PLUGIN = """
export default {
  apply(ctx) {
    ctx.tool({ name: "assets", description: "assets", inputSchema: { type: "object" }, handler: () => "ok" });
  },
};
"""

ASSETS_CLIENT = """
import { render } from "./lib/panel.js";
export default function activate(ctx) {
  ctx.slots.register({ name: "panel", id: "assets-panel", title: "Assets panel" }, render);
}
"""

ASSETS_CHUNK = """
export function render(element) {
  element.innerHTML = "<h3>Multi-file bundle</h3><p id='asset-status'>panel from a chunk</p>";
}
"""

LEGACY_MANIFEST = {
    "name": "@smoke/legacy",
    "version": "1.0.0",
    "main": "plugin.mjs",
    "dsh": {"client": {"platform": "web", "entry": "client.js"}},
}

LEGACY_PLUGIN = """
export default {
  apply(ctx) {
    ctx.tool({ name: "legacy", description: "legacy", inputSchema: { type: "object" }, handler: () => "ok" });
  },
};
"""

LEGACY_CLIENT = """
window.__ModuleLoader__.load({
  id: "@smoke/legacy",
  factory(require) {
    return {
      inject: ["slots"],
      apply(ctx) {
        ctx.slots.register({ name: "panel", id: "legacy-panel", title: "Legacy" }, (el) => {
          el.innerHTML = "<h3>Legacy bundle</h3><p id='legacy-status'>module table bundle</p>";
        });
      },
    };
  },
});
"""

ORDERED_MANIFEST = {
    "name": "@smoke/ordered",
    "version": "1.0.0",
    "main": "plugin.mjs",
    "dsh": {"client": {"platform": "web", "entry": "client.js", "inject": ["demo6"]}},
}

ORDERED_PLUGIN = """
export default {
  apply(ctx) {
    ctx.tool({ name: "ordered", description: "ordered", inputSchema: { type: "object" }, handler: () => "ok" });
  },
};
"""

ORDERED_CLIENT = """
export default function activate(ctx) {
  ctx.slots.register({ name: "panel", id: "ordered-panel", title: "Ordered" }, (el) => {
    el.innerHTML = "<h3>Ordered panel</h3>";
  });
}
"""

ERROR_MANIFEST = {
    "name": "@smoke/error",
    "version": "1.0.0",
    "main": "plugin.mjs",
    "dsh": {"client": {"platform": "web", "entry": "client.js"}},
}

ERROR_PLUGIN = """
export default {
  apply(ctx) {
    ctx.tool({ name: "broken", description: "broken", inputSchema: { type: "object" }, handler: () => "ok" });
  },
};
"""

ERROR_CLIENT = """
export default function activate() {
  throw new Error("boom from plugin");
}
"""

EXPORTS_MANIFEST = {
    "name": "@smoke/exports",
    "version": "1.0.0",
    "main": "plugin.mjs",
    "dsh": {"client": {"platform": "web"}},
    "exports": {"./client": {"browser": "./browser.js", "default": "./browser.js"}},
}

EXPORTS_PLUGIN = """
export default {
  apply(ctx) {
    ctx.tool({ name: "exports", description: "exports", inputSchema: { type: "object" }, handler: () => "ok" });
  },
};
"""

EXPORTS_CLIENT = """
export default function activate(ctx) {
  ctx.slots.register({ name: "panel", id: "exports-panel", title: "Exports" }, (el) => {
    el.innerHTML = "<h3>Exports map panel</h3>";
  });
}
"""

EAGER_MANIFEST = {
    "name": "@smoke/eager",
    "version": "1.0.0",
    "main": "plugin.mjs",
    "dsh": {"client": {"platform": "web", "entry": "client.js", "immediately": True}},
}

EAGER_PLUGIN = """
export default {
  apply(ctx) {
    ctx.tool({ name: "eager", description: "eager", inputSchema: { type: "object" }, handler: () => "ok" });
  },
};
"""

EAGER_CLIENT = """
export default function activate(ctx) {
  document.documentElement.dataset.eager = "1";
  ctx.slots.register({ name: "panel", id: "eager-panel", title: "Eager panel" }, (el) => {
    el.innerHTML = "<h3>Eager panel</h3>";
  });
}
"""


def write_fixtures(home: Path, workspace: Path) -> None:
    (home / "profiles" / "default").mkdir(parents=True, exist_ok=True)
    (home / "config.yaml").write_text("version: 1\n", encoding="utf-8")
    profile = ["version: 1", "plugins:", "  cordis:"]
    for name, manifest, plugin, client in [
        ("demo", DEMO_MANIFEST, DEMO_PLUGIN, DEMO_CLIENT),
        ("demo2", EAGER_MANIFEST, EAGER_PLUGIN, EAGER_CLIENT),
        ("demo3", ASSETS_MANIFEST, ASSETS_PLUGIN, ASSETS_CLIENT),
        ("demo4", LEGACY_MANIFEST, LEGACY_PLUGIN, LEGACY_CLIENT),
        ("demo5", ERROR_MANIFEST, ERROR_PLUGIN, ERROR_CLIENT),
        ("demo6", EXPORTS_MANIFEST, EXPORTS_PLUGIN, EXPORTS_CLIENT),
        ("demo7", ORDERED_MANIFEST, ORDERED_PLUGIN, ORDERED_CLIENT),
        ("demo8", DSH_MANIFEST, DEMO_PLUGIN, DSH_CLIENT),
    ]:
        directory = workspace / name
        directory.mkdir(parents=True, exist_ok=True)
        (directory / "package.json").write_text(json.dumps(manifest), encoding="utf-8")
        (directory / "plugin.mjs").write_text(plugin, encoding="utf-8")
        (directory / "client.js").write_text(client, encoding="utf-8")
        if name == "demo6":
            # Resolved through exports["./client"], not dsh.client.entry.
            (directory / "browser.js").write_text(client, encoding="utf-8")
        if name == "demo3":
            chunk = directory / "lib" / "panel.js"
            chunk.parent.mkdir(parents=True, exist_ok=True)
            chunk.write_text(ASSETS_CHUNK, encoding="utf-8")
        profile += [
            f"    - name: {name}",
            f"      entry: {directory}",
            "      enabled: true",
        ]
    (home / "profiles" / "default" / "profile.yaml").write_text("\n".join(profile) + "\n", encoding="utf-8")


# --------------------------------------------------------------------------
# Server + Chrome
# --------------------------------------------------------------------------


def start_server(binary: Path, home: Path, workspace: Path, port: int) -> subprocess.Popen:
    environment = dict(os.environ, DEEPSEEK_API_KEY="dummy")
    process = subprocess.Popen(
        [
            str(binary),
            "--home",
            str(home),
            "--profile",
            "default",
            "serve",
            "--port",
            str(port),
            "--token",
            TOKEN,
        ],
        cwd=workspace,
        env=environment,
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


def start_chrome(binary: str, port: int, profile: Path) -> subprocess.Popen:
    process = subprocess.Popen(
        [
            binary,
            "--headless=new",
            "--disable-gpu",
            "--no-sandbox",
            "--disable-dev-shm-usage",
            "--disable-extensions",
            "--hide-scrollbars",
            "--window-size=960,760",
            f"--remote-debugging-port={port}",
            f"--user-data-dir={profile}",
            "about:blank",
        ],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    deadline = time.time() + 30
    while time.time() < deadline:
        try:
            urllib.request.urlopen(f"http://127.0.0.1:{port}/json/version", timeout=1)
            return process
        except OSError:
            time.sleep(0.2)
    raise RuntimeError("chrome did not expose DevTools")


def page_target(port: int) -> str:
    with urllib.request.urlopen(f"http://127.0.0.1:{port}/json/list") as response:
        targets = json.load(response)
    page = next(target for target in targets if target["type"] == "page")
    return page["webSocketDebuggerUrl"]


def frame_targets(port: int, plugin: str) -> list[str]:
    with urllib.request.urlopen(f"http://127.0.0.1:{port}/json/list") as response:
        targets = json.load(response)
    return [
        target["webSocketDebuggerUrl"]
        for target in targets
        if target["type"] == "iframe" and f"/plugins/{plugin}/" in target["url"]
    ]


def frame_target(port: int, plugin: str) -> str | None:
    targets = frame_targets(port, plugin)
    return targets[0] if targets else None


def wait_for_frame_text(port: int, plugin: str, needle: str, timeout: float = 20.0) -> bool:
    """Polls the plugin frame text, reconnecting when the frame reloads."""
    deadline = time.time() + timeout
    while time.time() < deadline:
        for target in frame_targets(port, plugin):
            frame = DevTools(target)
            try:
                frame.call("Runtime.enable")
                text = str(frame.eval("document.getElementById('dshr-slot-root')?.textContent"))
            except Exception:  # noqa: BLE001 - the target may vanish mid-reload
                text = ""
            finally:
                frame.close()
            if needle in text:
                return True
        time.sleep(0.25)
    return False


def wait_until(predicate, timeout: float = 20.0, interval: float = 0.2, label: str = "condition"):
    deadline = time.time() + timeout
    last_error = None
    while time.time() < deadline:
        try:
            if predicate():
                return
        except Exception as error:  # noqa: BLE001 - polled predicates may race with DOM updates
            last_error = error
        time.sleep(interval)
    raise TimeoutError(f"timed out waiting for {label} ({last_error})")


# --------------------------------------------------------------------------
# Checks
# --------------------------------------------------------------------------


def run(args: argparse.Namespace) -> int:
    if not shutil.which("node"):
        print("SKIP: node is not installed")
        return 0
    chrome = chrome_binary()
    if chrome is None:
        print("SKIP: chrome was not found (set CHROME)")
        return 0

    binary = Path(args.binary)
    if not binary.is_absolute():
        binary = ROOT / binary
    if not binary.exists():
        print(f"building {binary}…")
        subprocess.run(["cargo", "build", "-p", "dshr-cli"], cwd=ROOT, check=True)

    workspace = Path(tempfile.mkdtemp(prefix="dshr-plugin-smoke-"))
    home = workspace / "home"
    plugins = workspace / "workspace"
    plugins.mkdir()
    write_fixtures(home, plugins)
    server = None
    chrome_process = None
    failures: list[str] = []
    checks: list[str] = []

    def check(label: str, condition: bool, detail: object = "") -> None:
        checks.append(label)
        if condition:
            print(f"  ok   {label}")
        else:
            print(f"  FAIL {label}{(': ' + str(detail)) if detail else ''}")
            failures.append(label)

    try:
        port = free_port()
        print(f"starting dshr on 127.0.0.1:{port}")
        server = start_server(binary, home, plugins, port)
        debug_port = free_port()
        chrome_process = start_chrome(chrome, debug_port, workspace / "chrome-profile")
        shell = DevTools(page_target(debug_port))
        shell.call("Runtime.enable")
        shell.call("Page.enable")

        request = urllib.request.Request(
            f"http://127.0.0.1:{port}/v1/plugins/clients",
            headers={"Authorization": f"Bearer {TOKEN}"},
        )
        with urllib.request.urlopen(request, timeout=10) as response:
            clients = json.load(response)["clients"]
        check("lists every client half", len(clients) == 8, str(clients))
        modes = {client["name"]: client["bundle"] for client in clients}
        check("single-file halves are inlined", modes.get("demo") == "inline", modes)
        check("multi-file halves use capability assets", modes.get("demo3") == "assets", modes)
        check(
            "eager half is marked immediately",
            any(client["name"] == "demo2" and client["immediately"] for client in clients),
        )

        shell.call(
            "Page.navigate",
            {"url": f"http://127.0.0.1:{port}/?token={TOKEN}&plugin=demo"},
        )
        wait_until(
            lambda: shell.eval("document.getElementById('plugin-stage')?.className === 'plugin-stage open'"),
            label="panel stage opens",
        )

        demo_ws = frame_target(debug_port, "demo")
        check("lazy panel frame is mounted", demo_ws is not None)
        demo = DevTools(demo_ws)
        demo.call("Runtime.enable")

        wait_until(
            lambda: "slot: panel" in str(demo.eval("document.getElementById('dshr-slot-root').textContent")),
            label="panel slot renders",
        )
        check("panel slot renders", True)
        check(
            "both slots are offered as tabs",
            "settings.section" in str(shell.eval("document.querySelector('.plugin-tabs').textContent")),
        )
        height = shell.eval("document.getElementById('plugin-frame-demo').style.height")
        check("resize message sets the frame height", bool(height) and height != "auto", str(height))

        eager_ws = frame_target(debug_port, "demo2")
        check("eager half is mounted at startup", eager_ws is not None)
        if eager_ws:
            eager = DevTools(eager_ws)
            eager.call("Runtime.enable")
            check("eager half ran its bundle", eager.eval("document.documentElement.dataset.eager") == "1")
            eager.close()

        shell.eval("[...document.querySelectorAll('.plugin-tabs button')][1].click()")
        wait_until(
            lambda: "slot: settings" in str(demo.eval("document.getElementById('dshr-slot-root').textContent")),
            label="settings slot renders",
        )
        check("slot switching activates the other slot", True)

        shell.eval("[...document.querySelectorAll('.plugin-tabs button')][0].click()")
        wait_until(
            lambda: "slot: panel" in str(demo.eval("document.getElementById('dshr-slot-root').textContent")),
            label="panel slot re-activates",
        )
        shell.eval("[...document.querySelectorAll('header button')].find(b => b.textContent === 'Theme').click()")
        wait_until(
            lambda: demo.eval("document.body.dataset.pluginTheme") in ("light", "dark"),
            label="theme reaches the frame",
        )
        theme = demo.eval("document.body.dataset.pluginTheme")
        shell_theme = shell.eval("document.documentElement.dataset.theme")
        check("theme propagates live into the frame", theme == shell_theme, f"{theme} vs {shell_theme}")

        demo.eval("document.getElementById('nope-btn').click()")
        wait_until(
            lambda: "unsupported action" in str(demo.eval("document.getElementById('action-status').textContent")),
            label="action round-trip",
        )
        check("ctx.action reports an unsupported action", True)

        demo.eval("document.getElementById('close-btn').click()")
        wait_until(
            lambda: shell.eval("document.getElementById('plugin-stage').className") == "plugin-stage",
            label="panel closes from the frame",
        )
        frame_ids = [item for item in str(shell.eval("[...document.querySelectorAll('iframe')].map(f => f.id).join(',')")).split(",") if item]
        check("close unmounts the lazy frame", "plugin-frame-demo" not in frame_ids, str(frame_ids))
        check("eager frame stays mounted", "plugin-frame-demo2" in frame_ids, str(frame_ids))
        demo.close()

        # A bundle with relative imports loads its module graph through the
        # capability asset route and renders a chunked component.
        shell.call(
            "Page.navigate",
            {"url": f"http://127.0.0.1:{port}/?token={TOKEN}&plugin=demo3"},
        )
        assets_ws = None
        deadline = time.time() + 20
        while time.time() < deadline and assets_ws is None:
            assets_ws = frame_target(debug_port, "demo3")
            time.sleep(0.2)
        check("asset-mode frame is mounted", assets_ws is not None)
        if assets_ws:
            assets = DevTools(assets_ws)
            assets.call("Runtime.enable")
            wait_until(
                lambda: "panel from a chunk" in str(assets.eval("document.getElementById('dshr-slot-root').textContent")),
                label="multi-file chunk renders",
            )
            check("multi-file bundle renders a chunk", True)
            assets.close()

        # A bundle built for the reference module table runs through the
        # __ModuleLoader__ shim and registers a slot from its apply().
        shell.call(
            "Page.navigate",
            {"url": f"http://127.0.0.1:{port}/?token={TOKEN}&plugin=demo4"},
        )
        legacy_ws = None
        deadline = time.time() + 20
        while time.time() < deadline and legacy_ws is None:
            legacy_ws = frame_target(debug_port, "demo4")
            time.sleep(0.2)
        check("module-table frame is mounted", legacy_ws is not None)
        if legacy_ws:
            legacy = DevTools(legacy_ws)
            legacy.call("Runtime.enable")
            wait_until(
                lambda: "module table bundle" in str(legacy.eval("document.getElementById('dshr-slot-root').textContent")),
                label="module-table bundle renders",
            )
            check("module-table bundle activates through apply(ctx)", True)
            legacy.close()

        # A bundle that throws surfaces its error in the shell, not a blank panel.
        shell.call(
            "Page.navigate",
            {"url": f"http://127.0.0.1:{port}/?token={TOKEN}&plugin=demo5"},
        )
        wait_until(
            lambda: "boom from plugin" in str(shell.eval("document.querySelector('.plugin-stage-head .tool')?.textContent")),
            label="plugin error surfaces",
        )
        check("a failing bundle reports its error to the shell", True)

        # exports["./client"] resolution renders too.
        shell.call(
            "Page.navigate",
            {"url": f"http://127.0.0.1:{port}/?token={TOKEN}&plugin=demo6"},
        )
        wait_until(
            lambda: frame_target(debug_port, "demo6") is not None,
            label="exports frame mounts",
        )
        check(
            "exports-map bundle renders",
            wait_for_frame_text(debug_port, "demo6", "Exports map panel"),
        )

        # inject ordering: opening demo7 mounts its dependency (demo6) first.
        shell.call(
            "Page.navigate",
            {"url": f"http://127.0.0.1:{port}/?token={TOKEN}&plugin=demo7"},
        )
        wait_until(
            lambda: frame_target(debug_port, "demo7") is not None,
            label="ordered frame mounts",
        )
        order = str(shell.eval("[...document.querySelectorAll('iframe')].map(f => f.id).join(',')"))
        check(
            "inject dependency is mounted before the panel that needs it",
            order.index("plugin-frame-demo6") < order.index("plugin-frame-demo7"),
            order,
        )

        # Host actions: a plugin can compose into the prompt and notify.
        shell.call(
            "Page.navigate",
            {"url": f"http://127.0.0.1:{port}/?token={TOKEN}&plugin=demo"},
        )
        wait_until(
            lambda: frame_target(debug_port, "demo") is not None,
            label="demo frame after navigation",
        )
        actions_ws = frame_target(debug_port, "demo")
        actions = DevTools(actions_ws)
        actions.call("Runtime.enable")
        wait_until(
            lambda: "Smoke panel v1" in str(actions.eval("document.getElementById('dshr-slot-root').textContent")),
            label="demo panel renders again",
        )
        result = actions.eval(
            "window.dshr.action('compose', {text: 'hello from plugin'})",
            await_promise=True,
        )
        check("compose action resolves", result.get("ok") is True, result)
        prompt_value = str(shell.eval("document.querySelector('textarea').value"))
        check("compose reaches the prompt", "hello from plugin" in prompt_value, prompt_value)
        result = actions.eval(
            "window.dshr.action('notify', {message: 'plugin says hi'})",
            await_promise=True,
        )
        check("notify action resolves", result.get("ok") is True, result)
        check(
            "notify reaches the status bar",
            "plugin says hi" in str(shell.eval("document.getElementById('status').textContent")),
        )

        # Editing the client bundle and asking for a reload must refresh the
        # open panel instead of requiring a reopen.
        (plugins / "demo" / "client.js").write_text(
            DEMO_CLIENT.replace("Smoke panel v1", "Smoke panel v2"), encoding="utf-8"
        )
        result = actions.eval("window.dshr.action('reload')", await_promise=True)
        check("reload action resolves", result.get("ok") is True, result)
        check(
            "an open panel picks up a new client bundle",
            wait_for_frame_text(debug_port, "demo", "Smoke panel v2"),
        )
        # DSH subset slot names (ADR-013 §5): a half that registers panel plus
        # message.action and settings.action must have all of them surfaced.
        shell.call(
            "Page.navigate",
            {"url": f"http://127.0.0.1:{port}/?token={TOKEN}&plugin=demo8"},
        )
        wait_until(
            lambda: "message.action" in str(shell.eval("document.querySelector('.plugin-tabs')?.textContent || ''")),
            label="DSH slot names render",
        )
        dsh_tabs = str(shell.eval("document.querySelector('.plugin-tabs').textContent"))
        check(
            "DSH subset slot names are offered (panel/message.action/settings.action)",
            all(name in dsh_tabs for name in ("panel", "message.action", "settings.action")),
            dsh_tabs,
        )

        actions.close()
        shell.close()
    finally:
        if chrome_process:
            chrome_process.terminate()
            chrome_process.wait(timeout=10)
        if server:
            server.terminate()
            try:
                server.wait(timeout=10)
            except subprocess.TimeoutExpired:
                server.kill()
        if args.keep:
            print(f"workdir kept at {workspace}")
        else:
            shutil.rmtree(workspace, ignore_errors=True)

    print(f"\n{len(checks) - len(failures)}/{len(checks)} checks passed")
    if failures:
        print("failed: " + ", ".join(failures))
        return 1
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", default="target/debug/dshr")
    parser.add_argument("--keep", action="store_true", help="keep the temporary workspace")
    return run(parser.parse_args())


if __name__ == "__main__":
    sys.exit(main())
