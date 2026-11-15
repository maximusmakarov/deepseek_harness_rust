#!/usr/bin/env python3
"""End-to-end smoke test for the Cordis client-half bridge (protocol v1.2).

Starts a real `dshr serve` with twenty-four throwaway plugins (a multi-region half, a
lazy panel half, an eager `immediately` half, asset/module-table/exports
delivery variants, a throwing bundle, ...), drives the Web UI in headless
Chrome over the DevTools protocol and checks the parts unit tests cannot see:

  * client-half discovery (inline / assets / `immediately` / `inject`)
  * eager registry frames: one hidden `mode=registry` iframe per half at
    startup, in `inject` order
  * regional mounting: each registered slot gets its own `iframe.slot-frame`
    inside the `.slot-region[data-region=...]` its DSH slot name maps to, with
    the rendered text visible inside the frame
  * the unknown-name fallback into the panel stage, panel tabs and switching
  * dialogs: settings.* frames mount with the dialog and unmount with it,
    plugins.add / plugins.detail frames mount with the Plugins dialog
  * per-message `message.action` frames for a replayed session
  * live theme propagation into every frame (`documentElement.dataset.theme`
    plus `ctx.onTheme`)
  * `ctx.action` round-trips (compose / notify / unsupported / reload / close)
  * a throwing bundle and a throwing render surfaced in the shell
  * asset (multi-file), module-table and `exports["./client"]` bundles

The session transcript is a deterministic `--cassette` replay, so per-message
regions render without a provider. Only the Python standard library is used.
Chrome and node are required; the script skips with exit code 0 when either is
missing.

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
import urllib.parse
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TOKEN = "smoke-token"
# A cold CI runner compiles the wasm bundle and paints the first frame well
# after 20 s; the local deadline must not decide whether CI is green.
LOAD_TIMEOUT = 90.0
FRAME_TIMEOUT = 45.0


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


_RAW_EVAL = DevTools.eval


def _resilient_eval(self, expression, await_promise=False, attempts: int = 40):
    """Evaluate, retrying while a navigation swaps the execution context.

    `Page.navigate`, a plugin `reload` action and a bumped frame version all
    destroy the current JS context, so an evaluation that races with them fails
    with Chrome -32000 "Inspected target navigated or closed". Every call site
    gets the retry, because any evaluation can race with a navigation.
    """
    last_error = None
    for _ in range(attempts):
        try:
            return _RAW_EVAL(self, expression, await_promise)
        except Exception as error:  # noqa: BLE001 - retried below
            message = str(error)
            if "navigated or closed" not in message and "Cannot find context" not in message:
                raise
            last_error = error
            time.sleep(0.25)
    raise TimeoutError(f"evaluate after navigation failed: {last_error}")


DevTools.eval = _resilient_eval


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


def wait_until(predicate, timeout: float = 60.0, interval: float = 0.2, label: str = "condition"):
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
  ctx.slots.register({ name: "panel", id: "smoke-panel", title: "Smoke panel", order: 10 }, (el) => {
    el.innerHTML = "<h3>Smoke panel</h3>";
  });
  ctx.onTheme((theme) => { document.body.dataset.pluginTheme = theme; });
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

# A bundle that fails in both frame roles the shell has to survive:
#   * registry frame: it throws after publishing a panel registration, so the
#     shell still knows the slot and the error surfaces in the status bar;
#   * slot frame: it registers a render that throws, so the error surfaces in
#     the panel-stage head.
ERROR_CLIENT = """
const mode = new URLSearchParams(location.search).get("mode");
export default function activate(ctx) {
  if (mode === "slot") {
    ctx.slots.register({ name: "panel", id: "error-panel", title: "Error panel" }, () => {
      throw new Error("render blew up");
    });
    return;
  }
  ctx.slots.register({ name: "panel", id: "error-panel", title: "Error panel" }, (el) => {
    el.textContent = "never rendered";
  });
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

# The DSH subset half from the previous architecture: its slot names resolve to
# message.action / settings.action regions (not to stage tabs any more).
DSH_MANIFEST = {
    "name": "@smoke/dsh-slots",
    "version": "1.0.0",
    "main": "plugin.mjs",
    "dsh": {"client": {"platform": "web", "entry": "client.js"}},
}

DSH_CLIENT = """
const render = (name) => (element) => {
  element.innerHTML = "<p class='slot-label'>dsh slot: " + name + "</p>";
};
export default function activate(ctx) {
  ctx.slots.register({ name: "panel", id: "dsh-panel", title: "DSH panel", order: 30 }, render("panel"));
  ctx.slots.register({ name: "message.action", id: "dsh-message", title: "Message action", order: 31 }, render("message.action"));
  ctx.slots.register({ name: "settings.action", id: "dsh-settings-action", title: "Settings action", order: 32 }, render("settings.action"));
}
"""

# One registration per shell region (plus an unknown name that must fall back
# to the panel stage). Every render prints the region it was mounted into so the
# smoke test can verify placement from inside the sandboxed frame.
FRAMEWORK_MANIFEST = {
    "name": "@smoke/framework",
    "version": "1.0.0",
    "main": "plugin.mjs",
    "dsh": {"client": {"platform": "web", "entry": "client.js", "runtime": "runtime/framework.js"}},
}

FRAMEWORK_PLUGIN = """
export default {
  apply(ctx) {
    ctx.tool({ name: "framework", description: "framework", inputSchema: { type: "object" }, handler: () => "ok" });
  },
};
"""

FRAMEWORK_RUNTIME = """
// A plugin-shipped module table (`dsh.client.runtime`). The half brings its own
// mini framework instead of relying on modules from the shell; a real plugin
// bundles React into this file the same way (verified manually with the React
// 18.3.1 UMD build).
window.__DSHR_MODULES__ = {
  mini: {
    mount: function (element, text) {
      element.innerHTML = "";
      const title = document.createElement("h3");
      title.id = "mini-framework";
      title.textContent = "mini framework: " + text;
      element.appendChild(title);
    },
  },
};
"""

FRAMEWORK_CLIENT = """
window.__ModuleLoader__.load({
  id: "@smoke/framework",
  factory(require) {
    const mini = require("mini");
    return {
      inject: ["slots"],
      apply(ctx) {
        ctx.slots.register(
          { name: "panel", id: "framework-panel", title: "Framework panel", order: 60 },
          (element, props) => mini.mount(element, props.theme),
        );
        ctx.onTheme((theme) => { document.body.dataset.pluginTheme = theme; });
      },
    };
  },
});
"""

HOSTED_MANIFEST = {
    "name": "@smoke/hosted",
    "version": "1.0.0",
    "main": "plugin.mjs",
    "dsh": {"client": {
        "platform": "web",
        "entry": "client.js",
        "host_modules": ["react", "react-dom/client"],
    }},
}

HOSTED_PLUGIN = """
export default {
  apply(ctx) {
    ctx.tool({ name: "hosted", description: "hosted", inputSchema: { type: "object" }, handler: () => "ok" });
  },
};
"""

# Tier B PoC: the half ships no React of its own — it opts into the host module
# table (`dsh.client.host_modules`) and the frame loads the shared, immutable
# runtime from the server origin.
HOSTED_CLIENT = """
window.__ModuleLoader__.load({
  id: "@smoke/hosted",
  factory(require) {
    const React = require("react");
    function Counter(props) {
      const [clicks, setClicks] = React.useState(0);
      const theme = props.useTheme ? props.useTheme() : props.theme;
      return React.createElement(
        "div",
        { id: "hosted-react" },
        React.createElement("h3", null, "React from the host"),
        React.createElement("button", { id: "hosted-counter", onClick: () => setClicks(clicks + 1) }, "clicks: " + clicks),
        React.createElement("p", { id: "hosted-theme" }, "theme: " + theme),
      );
    }
    return {
      inject: ["slots"],
      apply(ctx) {
        ctx.slots.register(
          { name: "panel", id: "hosted-panel", title: "Hosted panel", order: 70 },
          Counter,
        );
      },
    };
  },
});
"""

UIKIT_MANIFEST = {
    "name": "@smoke/ui-kit",
    "version": "1.0.0",
    "main": "plugin.mjs",
    "dsh": {"client": {
        "platform": "web",
        "entry": "client.js",
        "host_modules": ["react", "@deepseek-ai/dsh-client-ui-primitives"],
    }},
}

UIKIT_PLUGIN = """
export default {
  apply(ctx) {
    ctx.tool({ name: "ui-kit", description: "ui-kit", inputSchema: { type: "object" }, handler: () => "ok" });
  },
};
"""

# C-lite design-system shim: the bundle imports the reference primitives id and
# renders the shim components through host React.
UIKIT_CLIENT = """
window.__ModuleLoader__.load({
  id: "@smoke/ui-kit",
  factory(require) {
    const React = require("react");
    const primitives = require("@deepseek-ai/dsh-client-ui-primitives");
    function Panel() {
      return React.createElement(
        "div",
        { id: "ui-kit" },
        React.createElement(primitives.Button, null, "Kit button"),
        React.createElement(primitives.StateDot, { state: "success", label: "ok" }),
        React.createElement(primitives.Tooltip, { label: "tip" }, "hover me"),
      );
    }
    return {
      inject: ["slots"],
      apply(ctx) {
        ctx.slots.register(
          { name: "panel", id: "ui-kit-panel", title: "UI kit", order: 80 },
          Panel,
        );
      },
    };
  },
});
"""

HOOKS_MANIFEST = {
    "name": "@smoke/hooks",
    "version": "1.0.0",
    "main": "plugin.mjs",
    "dsh": {"client": {
        "platform": "web",
        "entry": "client.js",
        "host_modules": ["react", "react-dom/client"],
    }},
}

HOOKS_PLUGIN = """
export default {
  apply(ctx) {
    ctx.tool({ name: "hooks", description: "hooks", inputSchema: { type: "object" }, handler: () => "ok" });
  },
};
"""

# C-lite host state: the component uses the hook props the bridge supplies
# (useTheme/useSession/wide) instead of the plugin's own data sources.
HOOKS_CLIENT = """
window.__ModuleLoader__.load({
  id: "@smoke/hooks",
  factory(require) {
    const React = require("react");
    function Status(props) {
      const theme = props.useTheme();
      const session = props.useSession();
      return React.createElement(
        "p",
        { id: "hooks-status" },
        "theme: " + theme + " | session: " + (session && session.id ? session.id : "none") + " | wide: " + String(props.wide),
      );
    }
    return {
      inject: ["slots"],
      apply(ctx) {
        ctx.slots.register(
          { name: "sidebar.footer.action", id: "hooks-status", title: "Hooks status", order: 90 },
          Status,
        );
      },
    };
  },
});
"""

REGIONS_MANIFEST = {
    "name": "@smoke/regions",
    "version": "1.0.0",
    "main": "plugin.mjs",
    "dsh": {"client": {"platform": "web", "entry": "client.js"}},
}

REGIONS_PLUGIN = """
export default {
  apply(ctx) {
    ctx.tool({ name: "regions", description: "regions", inputSchema: { type: "object" }, handler: () => "ok" });
  },
};
"""

REGIONS_CLIENT = """
const VERSION = "regions-v1";
const NAMES = [
  "panel",
  "settings.section",
  "settings.section.icon",
  "settings.action",
  "message.action",
  "sidebar.footer.action",
  "sidebar.workspaces",
  "sidebar.session.row.trailing",
  "shell.overlay",
  "shell.leading",
  "shell.bottom",
  "conversation.session.header.actions",
  "conversation.input.right",
  "plugins.add.actions",
  "plugins.detail.actions",
  "tool.call.toolview",
  "conversation.approval.detail",
  "custom.unknown",
];
export default function activate(ctx) {
  const render = (name) => (element, props) => {
    const region = ctx.slotRegion(name);
    const context = (props && props.context) || {};
    const role = context.role || "-";
    const index = context.index == null ? "-" : context.index;
    element.innerHTML =
      '<p class="slot-label">' + VERSION + ' region:' + region + ' slot:' + name + '</p>' +
      '<p class="slot-context">context:' + role + '/' + index + '</p>';
  };
  NAMES.forEach((name, index) => {
    const id = "regions-" + name.replace(/[^A-Za-z0-9]+/g, "-");
    ctx.slots.register({ name: name, id: id, title: "Regions " + name, order: index + 10 }, render(name));
  });
  ctx.onTheme((theme) => { document.body.dataset.pluginTheme = theme; });
}
"""

# Slot name -> region the bridge must route it to (protocol v1.2 mapping for the
# slot names the `regions` fixture registers).
REGION_MAPPING = {
    "panel": "panel",
    "settings.section": "settings.section",
    "settings.section.icon": "settings.icon",
    "settings.action": "settings.action",
    "message.action": "message.action",
    "sidebar.footer.action": "sidebar.footer",
    "sidebar.workspaces": "sidebar.top",
    "sidebar.session.row.trailing": "sidebar.session.action",
    "shell.overlay": "shell.overlay",
    "shell.leading": "shell.leading",
    "shell.bottom": "shell.bottom",
    "conversation.session.header.actions": "conversation.header",
    "conversation.input.right": "composer.actions",
    "plugins.add.actions": "plugins.add",
    "plugins.detail.actions": "plugins.detail",
    "tool.call.toolview": "tool.card",
    "conversation.approval.detail": "approval.detail",
    "custom.unknown": "panel",
}

ALL_REGIONS = sorted({
    "panel", "settings.section", "settings.icon", "settings.action", "message.action",
    "sidebar.top", "sidebar.footer", "sidebar.session.action", "shell.leading",
    "shell.bottom", "shell.overlay", "conversation.header", "composer.actions",
    "plugins.add", "plugins.detail", "tool.card", "approval.detail",
})

# Regions mounted at plain-page startup for the `regions` fixture: (slot, region).
STARTUP_REGIONS = [
    ("shell.leading", "shell.leading"),
    ("sidebar.workspaces", "sidebar.top"),
    ("sidebar.footer.action", "sidebar.footer"),
    ("conversation.session.header.actions", "conversation.header"),
    ("conversation.input.right", "composer.actions"),
    ("shell.bottom", "shell.bottom"),
    ("shell.overlay", "shell.overlay"),
]

CASSETTE = {
    "model": "deepseek-flash",
    "calls": [
        [
            {"type": "start", "model": "deepseek-flash"},
            {"type": "text_delta", "text": "replayed answer for the smoke test"},
            {"type": "done", "stop_reason": "end_turn"},
        ],
    ],
}


def slot_id(slot: str) -> str:
    return "regions-" + "".join(ch if ch.isalnum() else "-" for ch in slot).strip("-").lower()


def write_fixtures(home: Path, workspace: Path) -> None:
    (home / "profiles" / "default").mkdir(parents=True, exist_ok=True)
    (home / "config.yaml").write_text("version: 1\n", encoding="utf-8")
    profile = ["version: 1", "plugins:", "  cordis:"]
    entries = [
        ("demo", DEMO_MANIFEST, DEMO_PLUGIN, DEMO_CLIENT),
        ("demo2", EAGER_MANIFEST, EAGER_PLUGIN, EAGER_CLIENT),
        ("demo3", ASSETS_MANIFEST, ASSETS_PLUGIN, ASSETS_CLIENT),
        ("demo4", LEGACY_MANIFEST, LEGACY_PLUGIN, LEGACY_CLIENT),
        ("demo5", ERROR_MANIFEST, ERROR_PLUGIN, ERROR_CLIENT),
        ("demo6", EXPORTS_MANIFEST, EXPORTS_PLUGIN, EXPORTS_CLIENT),
        ("demo7", ORDERED_MANIFEST, ORDERED_PLUGIN, ORDERED_CLIENT),
        ("demo8", DSH_MANIFEST, DEMO_PLUGIN, DSH_CLIENT),
        ("regions", REGIONS_MANIFEST, REGIONS_PLUGIN, REGIONS_CLIENT),
        ("framework", FRAMEWORK_MANIFEST, FRAMEWORK_PLUGIN, FRAMEWORK_CLIENT),
        ("hosted", HOSTED_MANIFEST, HOSTED_PLUGIN, HOSTED_CLIENT),
        ("ui-kit", UIKIT_MANIFEST, UIKIT_PLUGIN, UIKIT_CLIENT),
        ("hooks", HOOKS_MANIFEST, HOOKS_PLUGIN, HOOKS_CLIENT),
    ]
    for name, manifest, plugin, client in entries:
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
        if name == "framework":
            runtime = directory / "runtime" / "framework.js"
            runtime.parent.mkdir(parents=True, exist_ok=True)
            runtime.write_text(FRAMEWORK_RUNTIME, encoding="utf-8")
        profile += [
            f"    - name: {name}",
            f"      entry: {directory}",
            "      enabled: true",
        ]
    # A real third-party plugin (MIT, vendored fixture): its client half is the
    # upstream file, unmodified; the manifest only opts into the host runtime.
    for fixture_name, plugin_name in [
        ("deepseek-balance", "deepseek-balance"),
        ("dsh-remote", "dsh-remote"),
        ("ruoyang-balance", "ruoyang-balance"),
        ("dsh-genui", "dsh-genui"),
        ("dsh-session-superagent", "dsh-session-superagent"),
        ("dsh-mnemon", "dsh-mnemon"),
        ("dsh-plugin-market", "dsh-plugin-market"),
        ("dsh-plugin-codegraph-panel", "dsh-plugin-codegraph-panel"),
        ("dsh-plugin-notify", "dsh-plugin-notify"),
        ("dsh-plugin-debugging", "dsh-plugin-debugging"),
        ("dsh-plugin-studio", "dsh-plugin-studio"),
    ]:
        source = ROOT / "crates" / "cordis" / "tests" / "fixtures" / fixture_name
        target = workspace / fixture_name
        shutil.copytree(source, target)
        profile += [
            f"    - name: {plugin_name}",
            f"      entry: {target}",
            "      enabled: true",
        ]
    (home / "profiles" / "default" / "profile.yaml").write_text("\n".join(profile) + "\n", encoding="utf-8")


# --------------------------------------------------------------------------
# Server + Chrome
# --------------------------------------------------------------------------


def start_server(
    binary: Path, home: Path, workspace: Path, port: int, cassette: Path
) -> subprocess.Popen:
    environment = dict(os.environ)
    # The real C1 host halves must not reach the network or a developer's
    # credentials file during tests: an empty key makes them answer with their own
    # deterministic "not configured" JSON, and an isolated HOME hides ~/.dsh.
    environment["DEEPSEEK_API_KEY"] = ""
    environment["DEEPSEEK_PLATFORM_TOKEN"] = ""
    environment["HOME"] = str(home)
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
            "--cassette",
            str(cassette),
            "--model",
            "deepseek-flash",
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


# --------------------------------------------------------------------------
# Frame helpers: every plugin frame is a separate DevTools target whose URL
# carries the protocol tunnel parameters (plugin path, mode, slot, id).
# --------------------------------------------------------------------------


def iframe_targets(port: int) -> list[dict]:
    with urllib.request.urlopen(f"http://127.0.0.1:{port}/json/list") as response:
        targets = json.load(response)
    return [target for target in targets if target["type"] == "iframe"]


def frame_plugin(url: str) -> str:
    parts = urllib.parse.urlparse(url).path.split("/")
    # /v1/plugins/<name>/host
    return urllib.parse.unquote(parts[3]) if len(parts) > 4 else ""


def frame_params(url: str) -> dict[str, str]:
    query = urllib.parse.parse_qs(urllib.parse.urlparse(url).query)
    return {key: values[0] for key, values in query.items() if values}


def frame_target(
    port: int,
    plugin: str | None = None,
    mode: str | None = None,
    slot: str | None = None,
    slot_id: str | None = None,
) -> dict | None:
    for target in iframe_targets(port):
        params = frame_params(target["url"])
        if plugin is not None and frame_plugin(target["url"]) != plugin:
            continue
        if mode is not None and params.get("mode") != mode:
            continue
        if slot is not None and params.get("slot") != slot:
            continue
        if slot_id is not None and params.get("id") != slot_id:
            continue
        return target
    return None


def poll_frame(
    port: int,
    expression: str,
    predicate,
    *,
    plugin: str,
    mode: str = "slot",
    slot: str | None = None,
    slot_id: str | None = None,
    timeout: float = FRAME_TIMEOUT,
    interval: float = 0.2,
    label: str = "frame",
):
    """Poll a frame expression, reconnecting when the target is recreated.

    `reload` and a bumped frame version recreate the DevTools target, so a
    cached websocket cannot be reused; the connection is rebuilt whenever the
    target URL changes.
    """
    deadline = time.time() + timeout
    target_url = None
    frame = None
    last_error = None
    try:
        while time.time() < deadline:
            target = frame_target(port, plugin, mode, slot, slot_id)
            if target is not None and target["webSocketDebuggerUrl"] != target_url:
                if frame is not None:
                    frame.close()
                target_url = target["webSocketDebuggerUrl"]
                try:
                    frame = DevTools(target_url)
                    frame.call("Runtime.enable")
                except Exception as error:  # noqa: BLE001 - the target may vanish mid-reload
                    last_error = error
                    frame = None
                    target_url = None
                    time.sleep(interval)
                    continue
            if frame is not None:
                try:
                    value = frame.eval(expression)
                except Exception as error:  # noqa: BLE001 - target navigated; reconnect
                    last_error = error
                    frame.close()
                    frame = None
                    target_url = None
                    time.sleep(interval)
                    continue
                if predicate(value):
                    return value
            time.sleep(interval)
    finally:
        if frame is not None:
            frame.close()
    raise TimeoutError(f"timed out waiting for {label} ({last_error})")


FRAME_TEXT_EXPRESSION = "String(document.getElementById('dshr-slot-root')?.textContent || '')"


def wait_frame_text(
    port: int,
    needle: str,
    plugin: str,
    slot: str | None = None,
    slot_id: str | None = None,
    timeout: float = FRAME_TIMEOUT,
) -> str:
    return str(
        poll_frame(
            port,
            FRAME_TEXT_EXPRESSION,
            lambda value: needle in str(value),
            plugin=plugin,
            slot=slot,
            slot_id=slot_id,
            timeout=timeout,
            label=f"{plugin}/{slot or slot_id} text {needle!r}",
        )
    )


def collect_frame_texts(
    port: int,
    plugin: str,
    slot: str,
    expected: int,
    timeout: float = FRAME_TIMEOUT,
) -> list[str]:
    """Reads the text of every frame of one registration (per-occurrence slots).

    Several frames share the same plugin/slot pair (one per message), so each
    DevTools target is read separately and the collection retries while the
    frames are still loading.
    """
    deadline = time.time() + timeout
    texts: list[str] = []
    while time.time() < deadline:
        targets = [
            target
            for target in iframe_targets(port)
            if frame_plugin(target["url"]) == plugin
            and frame_params(target["url"]).get("slot") == slot
        ]
        texts = []
        for target in targets:
            frame = None
            try:
                frame = DevTools(target["webSocketDebuggerUrl"])
                frame.call("Runtime.enable")
                texts.append(str(frame.eval(FRAME_TEXT_EXPRESSION)))
            except Exception:  # noqa: BLE001 - the target may vanish mid-load
                pass
            finally:
                if frame is not None:
                    frame.close()
        if len(texts) >= expected and all(texts):
            return texts
        time.sleep(0.25)
    return texts


def wait_registry_ready(port: int, count: int, timeout: float = LOAD_TIMEOUT) -> None:
    wait_until(
        lambda: len([target for target in iframe_targets(port)
                     if frame_params(target["url"]).get("mode") == "registry"]) >= count,
        timeout=timeout,
        label="eager registry frames",
    )


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
    cassette = home / "cassette.json"
    cassette.write_text(json.dumps(CASSETTE), encoding="utf-8")

    server = None
    chrome_process = None
    shell = None
    failures: list[str] = []
    checks: list[str] = []

    def check(label: str, condition: bool, detail: object = "") -> None:
        checks.append(label)
        if condition:
            print(f"  ok   {label}")
        else:
            # `::error::` surfaces as a check-run annotation, so a red CI run is
            # diagnosable without downloading job logs.
            print(f"::error::plugin-smoke: {label}{(': ' + str(detail)) if detail else ''}")
            print(f"  FAIL {label}{(': ' + str(detail)) if detail else ''}")
            failures.append(label)

    def api(port: int, method: str, path: str, body: dict | None = None) -> dict:
        request = urllib.request.Request(
            f"http://127.0.0.1:{port}{path}",
            method=method,
            data=json.dumps(body).encode() if body is not None else None,
            headers={"Authorization": f"Bearer {TOKEN}", "Content-Type": "application/json"},
        )
        with urllib.request.urlopen(request, timeout=10) as response:
            return json.load(response)

    def click(selector: str, label: str) -> bool:
        return bool(
            shell.eval(
                """(() => {
                    const button = [...document.querySelectorAll(%s)]
                        .find(b => b.textContent.trim() === %s);
                    if (!button) return false;
                    button.click();
                    return true;
                })()""" % (json.dumps(selector), json.dumps(label))
            )
        )

    print(f"chrome: {chrome}")
    for tool in ("node", "google-chrome"):
        print(f"{tool}: {shutil.which(tool) or 'not found'}")
    try:
        port = free_port()
        print(f"starting dshr on 127.0.0.1:{port}")
        server = start_server(binary, home, plugins, port, cassette)

        # Discovery ---------------------------------------------------------
        clients = api(port, "GET", "/v1/plugins/clients")["clients"]
        names = [client["name"] for client in clients]
        check("lists every configured client half", len(clients) == 24, names)
        modes = {client["name"]: client["bundle"] for client in clients}
        check("single-file halves are inlined", modes.get("demo") == "inline", modes)
        check("multi-file halves use capability assets", modes.get("demo3") == "assets", modes)
        check(
            "eager half is marked immediately and demo7 injects demo6",
            any(client["name"] == "demo2" and client["immediately"] for client in clients)
            and any(client["name"] == "demo7" and client["inject"] == ["demo6"] for client in clients),
        )
        framework_half = next((client for client in clients if client["name"] == "framework"), None)
        check(
            "plugin-shipped runtime is discovered with the half",
            framework_half is not None and framework_half.get("runtime") == "runtime/framework.js",
            framework_half,
        )
        runtime_page = urllib.request.urlopen(
            urllib.request.Request(
                f"http://127.0.0.1:{port}/v1/plugins/framework/host?token={TOKEN}",
                headers={"Authorization": f"Bearer {TOKEN}"},
            ),
            timeout=10,
        ).read().decode()
        runtime_at = runtime_page.find("dshr plugin runtime")
        bundle_at = runtime_page.find("const url = ")
        check(
            "host page runs the plugin runtime before the bundle",
            runtime_at >= 0 and bundle_at > runtime_at and "mini framework" in runtime_page,
            (runtime_at, bundle_at, len(runtime_page)),
        )
        hosted_half = next((client for client in clients if client["name"] == "hosted"), None)
        check(
            "a half can opt into the shared host module table",
            hosted_half is not None
            and hosted_half.get("host_modules") == ["react", "react-dom/client"],
            hosted_half,
        )
        runtime_route = urllib.request.urlopen(
            f"http://127.0.0.1:{port}/v1/runtime/react-18.3.1.js", timeout=10
        )
        runtime_js = runtime_route.read().decode()
        check(
            "shared host runtime is one immutable versioned artifact",
            "immutable" in runtime_route.headers.get("Cache-Control", "")
            and "window.__DSHR_MODULES__" in runtime_js
            and '"react/jsx-runtime"' in runtime_js
            and '"@deepseek-ai/dsh-client-ui-primitives"' in runtime_js,
            runtime_route.headers.get("Cache-Control"),
        )
        hosted_page = urllib.request.urlopen(
            urllib.request.Request(
                f"http://127.0.0.1:{port}/v1/plugins/hosted/host?token={TOKEN}",
                headers={"Authorization": f"Bearer {TOKEN}"},
            ),
            timeout=10,
        ).read().decode()
        check(
            "host frames reference the shared runtime instead of embedding React",
            "/v1/runtime/react-18.3.1.js" in hosted_page
            and "@license React" not in hosted_page,
            (len(hosted_page), "@license React" in hosted_page),
        )
        ui_kit_half = next((client for client in clients if client["name"] == "ui-kit"), None)
        check(
            "a half can opt into the design-system primitives shim",
            ui_kit_half is not None
            and "@deepseek-ai/dsh-client-ui-primitives" in ui_kit_half.get("host_modules", []),
            ui_kit_half,
        )
        balance_half = next((client for client in clients if client["name"] == "deepseek-balance"), None)
        check(
            "the real DSH plugin is discovered as a client half",
            balance_half is not None and "react" in balance_half.get("host_modules", []),
            balance_half,
        )
        remote_half = next((client for client in clients if client["name"] == "dsh-remote"), None)
        check(
            "a second real DSH plugin is discovered with the primitives shim",
            remote_half is not None
            and "@deepseek-ai/dsh-client-ui-primitives" in remote_half.get("host_modules", []),
            remote_half,
        )
        ruoyang_half = next((client for client in clients if client["name"] == "ruoyang-balance"), None)
        genui_half = next((client for client in clients if client["name"] == "dsh-genui"), None)
        check(
            "two more real DSH plugins are discovered as client halves",
            ruoyang_half is not None
            and ruoyang_half.get("host_modules") == ["react"]
            and genui_half is not None
            and "@deepseek-ai/dsh-client-ui-primitives" in genui_half.get("host_modules", []),
            (ruoyang_half, genui_half),
        )
        conformance_halves = {
            name: next((client for client in clients if client["name"] == name), None)
            for name in ("dsh-session-superagent", "dsh-mnemon")
        }
        check(
            "conformance reaches six real DSH client halves",
            all(client is not None for client in conformance_halves.values()),
            conformance_halves,
        )
        extra_halves = {
            name: next((client for client in clients if client["name"] == name), None)
            for name in (
                "dsh-plugin-market",
                "dsh-plugin-codegraph-panel",
                "dsh-plugin-notify",
            )
        }
        check(
            "conformance reaches nine real DSH client halves",
            all(client is not None for client in extra_halves.values()),
            extra_halves,
        )
        c2_halves = {
            name: next((client for client in clients if client["name"] == name), None)
            for name in ("dsh-plugin-debugging", "dsh-plugin-studio")
        }
        check(
            "conformance reaches eleven real DSH client halves (C2)",
            all(client is not None for client in c2_halves.values())
            and all(
                "@deepseek-ai/dsh-client-ui-primitives" in client.get("host_modules", [])
                for client in c2_halves.values()
            ),
            c2_halves,
        )

        # A deterministic transcript: create a session over REST and replay one
        # assistant turn, so per-message regions have rows to mount into.
        session = api(port, "POST", "/v1/sessions", {})["session"]["id"]
        api(port, "POST", f"/v1/sessions/{session}/turns", {"prompt": "smoke prompt"})
        wait_until(
            lambda: len(api(port, "GET", f"/v1/sessions/{session}").get("messages", [])) >= 2,
            timeout=30,
            label="replayed transcript",
        )

        debug_port = free_port()
        chrome_process = start_chrome(chrome, debug_port, workspace / "chrome-profile")
        shell = DevTools(page_target(debug_port))
        shell.call("Runtime.enable")
        shell.call("Page.enable")

        # Remount the page from the start for the checks below.
        shell.call("Page.navigate", {"url": f"http://127.0.0.1:{port}/?token={TOKEN}"})
        wait_until(
            lambda: bool(shell.eval("document.getElementById('status') !== null")),
            timeout=LOAD_TIMEOUT,
            label="web ui loaded",
        )
        wait_registry_ready(debug_port, 24)
        # Registry frames load in inject order; wait until the multi-region half
        # has published all of its startup registrations before snapshotting the
        # mounted slot frames (otherwise the snapshot races the last frame).
        wait_until(
            lambda: (
                shell.eval(
                    'document.querySelectorAll(\'iframe.slot-frame[data-plugin="regions"]\').length'
                )
                or 0
            )
            >= 7,
            timeout=LOAD_TIMEOUT,
            label="regions startup slot frames",
        )

        # Eager registry ----------------------------------------------------
        registry = shell.eval(
            """JSON.stringify([...document.querySelectorAll('#plugin-registry iframe.registry-frame')]
                .map(f => ({plugin: f.dataset.plugin, hidden: f.hidden,
                            display: getComputedStyle(f).display,
                            parentHidden: f.parentElement.hidden, src: f.src})))"""
        )
        registry = json.loads(str(registry))
        check(
            "one hidden registry frame per client half at startup",
            len(registry) == 24
            and {item["plugin"] for item in registry} == set(names)
            and all("mode=registry" in item["src"] for item in registry),
            [item["plugin"] for item in registry],
        )
        check(
            "registry frames are hidden",
            all(item["hidden"] and item["parentHidden"] and item["display"] == "none" for item in registry),
            [(item["plugin"], item["display"]) for item in registry],
        )
        order = [item["plugin"] for item in registry]
        immediately = {
            client["name"] for client in clients if client.get("immediately")
        }
        check(
            "registry mount order follows inject/immediately",
            order.index("demo6") < order.index("demo7")
            and immediately <= set(order)
            and all(
                order.index(name) < order.index("demo")
                for name in immediately
                if name in order and "demo" in order
            ),
            order,
        )
        eager_state = json.loads(
            str(
                poll_frame(
                    debug_port,
                    """JSON.stringify({
                        mode: window.__DSHR_FRAME__ && window.__DSHR_FRAME__.mode,
                        plugin: window.__DSHR_FRAME__ && window.__DSHR_FRAME__.plugin,
                        eager: document.documentElement.dataset.eager,
                    })""",
                    lambda value: bool(value),
                    plugin="demo2",
                    mode="registry",
                    label="eager registry frame",
                )
            )
        )
        check(
            "eager half's bundle runs inside its registry frame",
            eager_state == {"mode": "registry", "plugin": "demo2", "eager": "1"},
            eager_state,
        )

        # Slot -> region mapping the bridge publishes ------------------------
        regions_registry = json.loads(
            str(
                poll_frame(
                    debug_port,
                    "JSON.stringify(window.dshr.slots.list())",
                    lambda value: bool(value) and len(json.loads(str(value))) >= 18,
                    plugin="regions",
                    mode="registry",
                    label="regions registry slots",
                )
            )
        )
        published = {entry["name"]: entry["region"] for entry in regions_registry}
        check(
            "bridge routes every fixture slot to its region",
            published == REGION_MAPPING,
            {
                key: [published.get(key), REGION_MAPPING[key]]
                for key in REGION_MAPPING
                if published.get(key) != REGION_MAPPING[key]
            } or published,
        )
        check(
            "every shell region id is covered",
            set(published.values()) == set(ALL_REGIONS),
            sorted(set(published.values())),
        )
        vocabulary = json.loads(
            str(
                poll_frame(
                    debug_port,
                    """JSON.stringify({
                        keys: Object.keys(window.dshr.SLOT_REGIONS).length,
                        unknown: window.dshr.slotRegion("custom.unknown"),
                    })""",
                    lambda value: bool(value),
                    plugin="regions",
                    mode="registry",
                    label="slot vocabulary",
                )
            )
        )
        check(
            "bridge exposes the full DSH slot vocabulary and a panel fallback",
            vocabulary == {"keys": 118, "unknown": "panel"},
            vocabulary,
        )

        # Regional mounting on the plain shell ------------------------------
        static_anchors = shell.eval(
            "JSON.stringify([...document.querySelectorAll('.slot-region')].map(r => r.dataset.region))"
        )
        static_anchors = set(json.loads(str(static_anchors)))
        check(
            "static shell region anchors exist",
            set(region for _, region in STARTUP_REGIONS) <= static_anchors,
            sorted(static_anchors),
        )
        mounted = json.loads(
            str(
                shell.eval(
                    """JSON.stringify([...document.querySelectorAll('iframe.slot-frame')].map(f => ({
                        plugin: f.dataset.plugin, slot: f.dataset.slot, slotId: f.dataset.slotId,
                        region: f.dataset.region, height: f.style.height,
                        inRegion: !!f.closest('.slot-region[data-region="' + f.dataset.region + '"]'),
                        occurrence: f.closest('.slot-region') ? f.closest('.slot-region').dataset.occurrence : '',
                    })))"""
                )
            )
        )
        for slot, region in STARTUP_REGIONS:
            entry = next(
                (item for item in mounted if item["plugin"] == "regions" and item["slot"] == slot), None
            )
            text = ""
            if entry is not None:
                text = wait_frame_text(debug_port, "region:" + region, "regions", slot)
            check(
                f"{slot} frame mounts in .slot-region[data-region={region}]",
                entry is not None
                and entry["region"] == region
                and entry["slotId"] == slot_id(slot)
                and entry["inRegion"]
                and bool(entry["height"])
                and "region:%s slot:%s" % (region, slot) in text,
                entry or "not mounted",
            )

        # Per-session-row region (the fixture registers it for repeated rows).
        session_row_state = shell.eval(
            """JSON.stringify([...document.querySelectorAll('.session-row')].map(row => ({
                region: row.querySelector('.slot-region')?.dataset.region || '',
                occurrence: row.querySelector('.slot-region')?.dataset.occurrence || '',
                frames: [...row.querySelectorAll('iframe.slot-frame')].map(f => ({
                    plugin: f.dataset.plugin, slot: f.dataset.slot,
                })),
            })))"""
        )
        session_rows = json.loads(str(session_row_state))
        session_row = next(
            (row for row in session_rows if row["occurrence"] == session), None
        )
        session_text = ""
        if session_row is not None:
            session_text = wait_frame_text(
                debug_port, "region:sidebar.session.action", "regions", "sidebar.session.row.trailing"
            )
        check(
            "sidebar.session.action frame mounts once per session row",
            session_row is not None
            and session_row["region"] == "sidebar.session.action"
            and any(
                frame["plugin"] == "regions"
                and frame["slot"] == "sidebar.session.row.trailing"
                for frame in session_row["frames"]
            )
            and "region:sidebar.session.action slot:sidebar.session.row.trailing" in session_text,
            session_row or session_rows,
        )

        # Messages: open the replayed session so per-message rows exist. The
        # row itself is already covered by the sidebar.session.action check.
        wait_until(
            lambda: bool(shell.eval("[...document.querySelectorAll('.session')].length")),
            timeout=30,
            label="session row listed",
        )
        shell.eval("[...document.querySelectorAll('.session')][0].click()")
        wait_until(
            lambda: (shell.eval("[...document.querySelectorAll('.message')].length") or 0) >= 2,
            timeout=30,
            label="session messages",
        )
        message_frames = json.loads(
            str(
                shell.eval(
                    """JSON.stringify([...document.querySelectorAll('.message .slot-region[data-region="message.action"]')]
                        .map(r => ({occurrence: r.dataset.occurrence,
                                    frames: r.querySelectorAll('iframe.slot-frame[data-plugin="regions"]').length})))"""
                )
            )
        )
        check(
            "message.action frames mount once per message",
            len(message_frames) >= 2 and all(item["frames"] == 1 for item in message_frames),
            message_frames,
        )
        # Every message gets its own frame; collect both and read the props
        # context out of the rendered text.
        message_texts = collect_frame_texts(debug_port, "regions", "message.action", 2)
        check(
            "message action frames render per-occurrence context",
            all("region:message.action slot:message.action" in text for text in message_texts)
            and any("context:user/0" in text for text in message_texts)
            and any("context:assistant/1" in text for text in message_texts),
            message_texts,
        )

        # Theme propagation into every mounted frame -----------------------
        previous_theme = str(shell.eval("document.documentElement.dataset.theme"))
        shell.eval(
            "[...document.querySelectorAll('header button')].find(b => b.textContent.trim() === 'Theme').click()"
        )
        wait_until(
            lambda: str(shell.eval("document.documentElement.dataset.theme")) != previous_theme,
            timeout=10,
            label="theme toggled",
        )
        shell_theme = str(shell.eval("document.documentElement.dataset.theme"))
        inspected = []
        for plugin, mode, slot in [
            ("regions", "slot", "sidebar.workspaces"),
            ("regions", "slot", "message.action"),
            ("regions", "slot", "shell.leading"),
            ("regions", "registry", None),
            ("demo2", "registry", None),
        ]:
            inspected.append(
                str(
                    poll_frame(
                        debug_port,
                        "JSON.stringify({theme: document.documentElement.dataset.theme, "
                        "pluginTheme: document.body.dataset.pluginTheme || ''})",
                        lambda value: json.loads(str(value))["theme"] == shell_theme,
                        plugin=plugin,
                        mode=mode,
                        slot=slot,
                        label=f"{plugin}/{slot} theme",
                    )
                )
            )
        check(
            "theme toggle reaches every inspected frame",
            all(json.loads(value)["theme"] == shell_theme for value in inspected),
            (shell_theme, inspected),
        )
        plugin_themes = [
            json.loads(value)["pluginTheme"]
            for value in inspected
            if json.loads(value)["pluginTheme"]
        ]
        check(
            "ctx.onTheme listeners receive the new theme",
            bool(plugin_themes) and all(theme == shell_theme for theme in plugin_themes),
            (shell_theme, plugin_themes),
        )

        # C-lite: host-provided React slot components, host services (hooks) and
        # the design-system primitives shim. ---------------------------------
        hooks_text = wait_frame_text(
            debug_port, "theme: ", "hooks", "sidebar.footer.action", timeout=20
        )
        check(
            "React slot components receive host theme/session hooks",
            f"theme: {shell_theme}" in hooks_text
            and f"session: {session}" in hooks_text
            and "wide: true" in hooks_text,
            hooks_text,
        )
        balance_text = wait_frame_text(
            debug_port, "API 余额", "deepseek-balance", "sidebar.footer.action", timeout=20
        )
        check(
            "a real DSH plugin renders its React card in the sidebar footer",
            "API 余额" in balance_text,
            balance_text,
        )
        balance_ui = json.loads(
            str(
                poll_frame(
                    debug_port,
                    """JSON.stringify({
                        css: !!document.querySelector('style[data-plugin="@kt11/dsh-deepseek-balance"]'),
                        button: !!document.querySelector('.dshb-refresh'),
                        text: (document.getElementById('dshr-slot-root')?.textContent || '').slice(0, 120),
                    })""",
                    lambda value: json.loads(str(value))["css"]
                    and json.loads(str(value))["button"],
                    plugin="deepseek-balance",
                    slot="sidebar.footer.action",
                    label="balance card ui",
                )
            )
        )
        check(
            "the real plugin's own CSS and DOM are applied inside the slot",
            balance_ui["css"] and balance_ui["button"],
            balance_ui,
        )
        # Click once (a side-effectful expression inside the poll loop would
        # toggle the popup closed on every retry), then poll the portal text.
        balance_target = frame_target(
            debug_port, "deepseek-balance", "slot", "sidebar.footer.action"
        )
        if balance_target is not None:
            balance_frame = DevTools(balance_target["webSocketDebuggerUrl"])
            try:
                balance_frame.call("Runtime.enable")
                balance_frame.eval(
                    """(() => {
                        const wrap = document.querySelector('#dshr-slot-root > div');
                        const card = wrap ? wrap.firstElementChild || wrap : null;
                        if (card) card.click();
                    })()"""
                )
            finally:
                balance_frame.close()
        popup_text = str(
            poll_frame(
                debug_port,
                "String(document.body.innerText || '')",
                lambda value: "查询失败" in str(value),
                plugin="deepseek-balance",
                slot="sidebar.footer.action",
                label="balance popup",
            )
        )
        check(
            "the real plugin's popup interaction works",
            # Its data fetches 404 against dshr, so the popup renders its error
            # branch — opening it still proves the React click handler runs.
            "查询失败" in popup_text,
            popup_text[:200],
        )

        # Second real plugin: dsh-remote registers React slots in several
        # supported regions; it must mount without a bridge error even though its
        # optional DSH services (betterSidebar/sidebarRight/webTerminals) are
        # absent — that is the graceful degradation its code is written for.
        remote_registry = poll_frame(
            debug_port,
            "JSON.stringify({ready: document.documentElement.dataset.dshrReady, error: window.__DSHR_LAST_ERROR__ || null})",
            lambda value: json.loads(str(value))["ready"] == "1",
            plugin="dsh-remote",
            mode="registry",
            label="dsh-remote registry frame",
        )
        check(
            "the second real plugin's registry frame activates without errors",
            json.loads(str(remote_registry))["ready"] == "1"
            and json.loads(str(remote_registry))["error"] is None,
            remote_registry,
        )
        remote_slot = poll_frame(
            debug_port,
            "JSON.stringify({ready: document.documentElement.dataset.dshrReady, error: window.__DSHR_LAST_ERROR__ || null})",
            lambda value: json.loads(str(value))["ready"] == "1",
            plugin="dsh-remote",
            slot="conversation.session.header.actions",
            label="dsh-remote header slot",
        )
        remote_frame = shell.eval(
            """JSON.stringify([...document.querySelectorAll(
                '.slot-region[data-region="conversation.header"] iframe.slot-frame[data-plugin="dsh-remote"]'
            )].map(f => ({slot: f.dataset.slot, region: f.dataset.region})))"""
        )
        check(
            "the second real plugin's slot mounts in the conversation header region",
            bool(json.loads(str(remote_frame)))
            and json.loads(str(remote_slot))["error"] is None,
            {"frame": remote_frame, "slot": remote_slot},
        )

        # C-lite conformance n=4: a floating badge in shell.overlay and a
        # composer snippet using React.memo/keyed registrations.
        ruoyang_slot = poll_frame(
            debug_port,
            "JSON.stringify({ready: document.documentElement.dataset.dshrReady, error: window.__DSHR_LAST_ERROR__ || null, text: String(document.getElementById('dshr-slot-root')?.textContent || '')})",
            lambda value: json.loads(str(value))["ready"] == "1"
            and json.loads(str(value))["text"].strip() != "",
            plugin="ruoyang-balance",
            slot="shell.overlay",
            label="ruoyang-balance overlay",
        )
        ruoyang_state = json.loads(str(ruoyang_slot))
        check(
            "a floating real plugin renders in the shell overlay",
            ruoyang_state["error"] is None
            and "Balance unavailable" in ruoyang_state["text"],
            ruoyang_state,
        )
        genui_slot = poll_frame(
            debug_port,
            "JSON.stringify({ready: document.documentElement.dataset.dshrReady, error: window.__DSHR_LAST_ERROR__ || null, text: String(document.getElementById('dshr-slot-root')?.textContent || '')})",
            lambda value: json.loads(str(value))["ready"] == "1"
            and "GenUI" in json.loads(str(value))["text"],
            plugin="dsh-genui",
            slot="conversation.input.left",
            label="genui prompt toggle",
        )
        genui_state = json.loads(str(genui_slot))
        check(
            "a real plugin using React.memo/keyed slots renders in the composer",
            genui_state["error"] is None and "GenUI" in genui_state["text"],
            genui_state,
        )

        # Conformance n=6: a session superagent composer dock and a large
        # multi-region plugin (mnemon) that reconciles its own shadow registrations
        # through ctx.slots.subscribe/getVersion/entriesOfSlot.
        superagent_slot = poll_frame(
            debug_port,
            "JSON.stringify({ready: document.documentElement.dataset.dshrReady, error: window.__DSHR_LAST_ERROR__ || null})",
            lambda value: json.loads(str(value))["ready"] == "1",
            plugin="dsh-session-superagent",
            slot="conversation.composer.dock",
            label="superagent composer dock",
        )
        check(
            "a real plugin mounts its composer dock without errors",
            json.loads(str(superagent_slot))["error"] is None,
            superagent_slot,
        )
        mnemon_registry = poll_frame(
            debug_port,
            "JSON.stringify({ready: document.documentElement.dataset.dshrReady, error: window.__DSHR_LAST_ERROR__ || null, slots: window.dshr.slots.list().length})",
            lambda value: json.loads(str(value))["ready"] == "1",
            plugin="dsh-mnemon",
            mode="registry",
            label="mnemon registry",
        )
        mnemon_state = json.loads(str(mnemon_registry))
        check(
            "a six-plugin conformance half activates and publishes slots",
            mnemon_state["error"] is None and mnemon_state["slots"] > 0,
            mnemon_state,
        )

        # Conformance n=9: three more real halves (market sidebar entry,
        # settings-page panel, plugins-page rows) must activate and publish slots.
        extra_states = {}
        for plugin in (
            "dsh-plugin-market",
            "dsh-plugin-codegraph-panel",
            "dsh-plugin-notify",
        ):
            extra_states[plugin] = json.loads(
                str(
                    poll_frame(
                        debug_port,
                        "JSON.stringify({ready: document.documentElement.dataset.dshrReady, error: window.__DSHR_LAST_ERROR__ || null, slots: window.dshr.slots.list().length})",
                        lambda value: json.loads(str(value))["ready"] == "1",
                        plugin=plugin,
                        mode="registry",
                        label=f"{plugin} registry",
                    )
                )
            )
        check(
            "three more real plugins activate and publish slots",
            all(
                state["error"] is None and state["slots"] > 0
                for state in extra_states.values()
            ),
            extra_states,
        )
        # C2: primitives-heavy halves activate cleanly; host-service-dependent UI
        # stays unmounted instead of crashing (graceful degradation).
        c2_states = {}
        for plugin in ("dsh-plugin-debugging", "dsh-plugin-studio"):
            c2_states[plugin] = json.loads(
                str(
                    poll_frame(
                        debug_port,
                        "JSON.stringify({ready: document.documentElement.dataset.dshrReady, error: window.__DSHR_LAST_ERROR__ || null})",
                        lambda value: json.loads(str(value))["ready"] == "1",
                        plugin=plugin,
                        mode="registry",
                        label=f"{plugin} registry",
                    )
                )
            )
        check(
            "primitives-heavy conformance halves activate without errors (C2)",
            all(state["error"] is None for state in c2_states.values()),
            c2_states,
        )
        market_frame = json.loads(
            str(
                shell.eval(
                    """JSON.stringify([...document.querySelectorAll(
                        'iframe.slot-frame[data-plugin="dsh-plugin-market"]'
                    )].map(f => ({slot: f.dataset.slot, region: f.dataset.region})))"""
                )
            )
        )
        check(
            "the market plugin mounts its sidebar entry frame",
            any(item["slot"] == "sidebar.panellist" for item in market_frame),
            market_frame,
        )

        # C1: the real host half registers its own routes (ctx.inject(['webServer']));
        # they are proxied to the sidecar and answered as the plugin's JSON, not a
        # dshr 404 page. Evaluate inside the plugin frame to cover frame CSP + CORS.
        balance_route_target = frame_target(
            debug_port, "deepseek-balance", "slot", "sidebar.footer.action"
        )
        balance_route_state = {"error": "no balance frame"}
        if balance_route_target is not None:
            balance_route_frame = DevTools(balance_route_target["webSocketDebuggerUrl"])
            try:
                balance_route_frame.call("Runtime.enable")
                raw = balance_route_frame.eval(
                    "fetch('/dsh-balance/deepseek').then(r => r.json().then(body => JSON.stringify({status: r.status, body})))",
                    True,
                )
                balance_route_state = json.loads(str(raw))
            except Exception as error:  # noqa: BLE001 - reported as a check detail
                balance_route_state = {"error": str(error)}
            finally:
                balance_route_frame.close()
        route_body = balance_route_state.get("body") or {}
        check(
            "a plugin host route is proxied with its own JSON (C1)",
            balance_route_state.get("status") == 500
            and route_body.get("ok") is False
            and route_body.get("code") == "not-configured",
            balance_route_state,
        )
        usage_route_state = {"error": "no balance frame"}
        if balance_route_target is not None:
            usage_frame = DevTools(balance_route_target["webSocketDebuggerUrl"])
            try:
                usage_frame.call("Runtime.enable")
                raw = usage_frame.eval(
                    "fetch('/dsh-usage/sessions?tz=UTC').then(r => r.json().then(body => JSON.stringify({status: r.status, body})))",
                    True,
                )
                usage_route_state = json.loads(str(raw))
            except Exception as error:  # noqa: BLE001 - reported as a check detail
                usage_route_state = {"error": str(error)}
            finally:
                usage_frame.close()
        usage_body = usage_route_state.get("body") or {}
        check(
            "a plugin host route keeps query strings and status codes",
            usage_route_state.get("status") == 200
            and usage_body.get("ok") is True
            and isinstance(usage_body.get("sessions"), list),
            usage_route_state,
        )
        unknown_route = shell.eval(
            """(() => {
                window.__c1Unknown = null;
                fetch('/dsh-balance/unknown-route').then(r => r.json().then(body => {
                    window.__c1Unknown = {status: r.status, body};
                })).catch(error => { window.__c1Unknown = {error: String(error)}; });
            })()"""
        )
        wait_until(
            lambda: shell.eval("window.__c1Unknown !== null"),
            timeout=FRAME_TIMEOUT,
            label="unknown plugin route",
        )
        unknown_state = json.loads(str(shell.eval("JSON.stringify(window.__c1Unknown)")))
        check(
            "unknown plugin routes keep the dshr 404",
            unknown_state.get("status") == 404
            and (unknown_state.get("body") or {}).get("error") == "not found",
            unknown_state,
        )

        shell.call(
            "Page.navigate",
            {"url": f"http://127.0.0.1:{port}/?token={TOKEN}&plugin=ui-kit"},
        )
        wait_until(
            lambda: shell.eval("document.getElementById('plugin-stage')?.className === 'plugin-stage open'"),
            timeout=LOAD_TIMEOUT,
            label="ui-kit panel opens",
        )
        ui_text = wait_frame_text(debug_port, "Kit button", "ui-kit", "panel", timeout=20)
        ui_dom = json.loads(
            str(
                poll_frame(
                    debug_port,
                    """JSON.stringify({
                        button: !!document.querySelector('.dshr-button'),
                        dot: !!document.querySelector('.dshr-state-dot-success'),
                        text: (document.getElementById('dshr-slot-root')?.textContent || ''),
                    })""",
                    lambda value: json.loads(str(value))["button"],
                    plugin="ui-kit",
                    slot="panel",
                    label="ui-kit dom",
                )
            )
        )
        check(
            "design-system primitives shim renders through host React",
            ui_dom["button"] and ui_dom["dot"] and "Kit button" in ui_dom["text"],
            {"text": ui_text, **ui_dom},
        )

        # C2: the expanded design-system shim must render every modelled primitive
        # with host React, including the pieces real plugins probe for
        # (Menu/Modal/Toast/Segmented*/DisclosureRow/hooks/icons).
        ui_kit_target = frame_target(debug_port, "ui-kit", "slot", "panel")
        c2_state = {"error": "no ui-kit frame"}
        if ui_kit_target is not None:
            c2_frame = DevTools(ui_kit_target["webSocketDebuggerUrl"])
            try:
                c2_frame.call("Runtime.enable")
                raw = c2_frame.eval(
                    """new Promise((resolve) => {
                        try {
                            const primitives = window.__DSHR_MODULES__[
                                "@deepseek-ai/dsh-client-ui-primitives"
                            ];
                            const React = window.React;
                            const ReactDOM = window.__DSHR_MODULES__["react-dom/client"];
                            const e = React.createElement;
                            const host = document.createElement("div");
                            host.id = "dshr-c2-probe";
                            document.body.appendChild(host);
                            ReactDOM.createRoot(host).render(e("div", null,
                                e(primitives.Button, { variant: "primary" }, "C2 button"),
                                e(primitives.Input, {
                                    placeholder: "C2 input",
                                    icon: e(primitives.IconSearchOutlineRegular, { size: 12 }),
                                }),
                                e(primitives.Switch, { checked: true, label: "C2 switch", onChange() {} }),
                                e(primitives.Checkbox, { checked: true, label: "C2 checkbox", onChange() {} }),
                                e(primitives.Tag, { tone: "info" }, "C2 tag"),
                                e(primitives.Pill, { active: true, onClick() {} }, "C2 pill"),
                                e(primitives.SegmentedControl, {
                                    options: [{ value: "a", label: "C2 segment" }],
                                    value: "a",
                                    onChange() {},
                                }),
                                e(primitives.SegmentedTabs, {
                                    items: [{ value: "a", label: "C2 tab" }],
                                    value: "a",
                                    onChange() {},
                                }),
                                e(primitives.DisclosureRow, {
                                    title: "C2 disclosure",
                                    open: false,
                                    expandable: false,
                                }),
                                e(primitives.PathLabel, { path: "/tmp/c2-path" }),
                                e(primitives.TextShimmer, { active: true }, "C2 shimmer"),
                                e(primitives.Tooltip, { content: "C2 tooltip" }, "C2 tt"),
                                e(primitives.Modal, { open: false }),
                                e(primitives.Toast, { text: "C2 toast", holdMs: 100000, onDone() {} }),
                                e(primitives.Menu, {
                                    open: true,
                                    items: [{ id: "x", label: "C2 menu item" }],
                                    anchor: e(primitives.Button, null, "C2 menu anchor"),
                                }),
                                e(primitives.IconChevronDownOutlineRegular, { size: 12 })
                            ));
                            setTimeout(() => resolve(JSON.stringify({
                                text: document.body.innerText,
                                error: window.__DSHR_LAST_ERROR__ || null,
                            })), 400);
                        } catch (error) {
                            resolve(JSON.stringify({ error: String(error) }));
                        }
                    })""",
                    True,
                )
                c2_state = json.loads(str(raw))
            except Exception as error:  # noqa: BLE001 - reported as a check detail
                c2_state = {"error": str(error)}
            finally:
                c2_frame.close()
        c2_text = c2_state.get("text") or ""
        c2_markers = [
            "C2 button",
            "C2 switch",
            "C2 checkbox",
            "C2 tag",
            "C2 pill",
            "C2 segment",
            "C2 tab",
            "C2 disclosure",
            "/tmp/c2-path",
            "C2 shimmer",
            "C2 toast",
            "C2 menu item",
        ]
        check(
            "expanded design-system primitives render with host React (C2)",
            all(marker in c2_text for marker in c2_markers),
            {"missing": [marker for marker in c2_markers if marker not in c2_text], **c2_state},
        )

        # Settings dialog: region frames exist only while it is open. ------
        click("header button", "Settings")
        wait_until(
            lambda: bool(shell.eval("document.querySelector('dialog h3') !== null")),
            timeout=FRAME_TIMEOUT,
            label="settings dialog",
        )
        settings_state = shell.eval(
            """JSON.stringify([...document.querySelectorAll('dialog .slot-region')]
                .map(r => ({region: r.dataset.region, slots: [...r.querySelectorAll('iframe.slot-frame')]
                    .map(f => f.dataset.slot)})))"""
        )
        settings_regions = json.loads(str(settings_state))
        settings_by_region = {item["region"]: item["slots"] for item in settings_regions}
        check(
            "Settings mounts the settings.section frame",
            "settings.section" in settings_by_region.get("settings.section", []),
            settings_regions,
        )
        check(
            "Settings mounts the settings.icon and settings.action frames",
            "settings.section.icon" in settings_by_region.get("settings.icon", [])
            and "settings.action" in settings_by_region.get("settings.action", []),
            settings_regions,
        )
        for slot, needle in [
            ("settings.section", "region:settings.section"),
            ("settings.section.icon", "region:settings.icon"),
            ("settings.action", "region:settings.action"),
        ]:
            wait_frame_text(debug_port, needle, "regions", slot)
        click("dialog button", "Close")
        wait_until(
            lambda: not shell.eval("!!document.querySelector('dialog h3')"),
            timeout=FRAME_TIMEOUT,
            label="settings dialog closed",
        )
        check(
            "closing Settings unmounts its region frames",
            not shell.eval(
                """[...document.querySelectorAll('iframe.slot-frame')]
                    .some(f => ['settings.section', 'settings.section.icon', 'settings.action']
                        .includes(f.dataset.slot))"""
            ),
        )

        # Plugins dialog ----------------------------------------------------
        click("header button", "Plugins")
        wait_until(
            lambda: "Plugins"
            in str(shell.eval("document.querySelector('dialog h3')?.textContent || ''")),
            timeout=FRAME_TIMEOUT,
            label="plugins dialog",
        )
        wait_frame_text(debug_port, "region:plugins.add", "regions", "plugins.add.actions")
        check("Plugins mounts the plugins.add frame", True)
        wait_frame_text(debug_port, "region:plugins.detail", "regions", "plugins.detail.actions")
        check("Plugins mounts the plugins.detail frame", True)
        # Real plugins that use the primitives shim (notify: Tag/Icon rows,
        # mnemon: Settings controls) must mount their contributions without errors.
        wait_until(
            lambda: bool(
                shell.eval(
                    """[...document.querySelectorAll('dialog iframe.slot-frame')]
                        .some(f => ['dsh-plugin-notify', 'dsh-mnemon']
                            .includes(f.dataset.plugin))"""
                )
            ),
            timeout=FRAME_TIMEOUT,
            label="primitives-based plugin frames in the Plugins dialog",
        )
        dialog_plugin_frames = json.loads(
            str(
                shell.eval(
                    """JSON.stringify([...document.querySelectorAll('dialog iframe.slot-frame')]
                        .filter(f => ['dsh-plugin-notify', 'dsh-mnemon'].includes(f.dataset.plugin))
                        .map(f => ({plugin: f.dataset.plugin, slot: f.dataset.slot})))"""
                )
            )
        )
        dialog_errors = {}
        for plugin_name in {item["plugin"] for item in dialog_plugin_frames}:
            for frame in [
                target
                for target in iframe_targets(debug_port)
                if frame_plugin(target["url"]) == plugin_name
            ]:
                frame_client = DevTools(frame["webSocketDebuggerUrl"])
                try:
                    frame_client.call("Runtime.enable")
                    dialog_errors[plugin_name] = json.loads(
                        str(
                            frame_client.eval(
                                "JSON.stringify(window.__DSHR_LAST_ERROR__ || null)"
                            )
                        )
                    )
                except Exception as error:  # noqa: BLE001 - reported as a check detail
                    dialog_errors[plugin_name] = str(error)
                finally:
                    frame_client.close()
        check(
            "real plugins render primitives-based contributions without errors",
            bool(dialog_plugin_frames)
            and all(error is None for error in dialog_errors.values()),
            {"frames": dialog_plugin_frames, "errors": dialog_errors},
        )
        click("dialog button", "Close")
        wait_until(
            lambda: not shell.eval("!!document.querySelector('dialog h3')"),
            timeout=FRAME_TIMEOUT,
            label="plugins dialog closed",
        )

        # Panel stage: tabs, fallback, switching, resize --------------------
        shell.call("Page.navigate", {"url": f"http://127.0.0.1:{port}/?token={TOKEN}&plugin=regions"})
        wait_until(
            lambda: shell.eval("document.getElementById('plugin-stage')?.className === 'plugin-stage open'"),
            timeout=LOAD_TIMEOUT,
            label="panel stage opens",
        )
        wait_registry_ready(debug_port, 24)
        check("deep link ?plugin= opens the panel stage", True)
        panel_frame = poll_frame(
            debug_port,
            """JSON.stringify({
                slot: window.__DSHR_FRAME__ && window.__DSHR_FRAME__.slot,
                text: String(document.getElementById('dshr-slot-root')?.textContent || ''),
            })""",
            lambda value: "region:panel slot:panel" in str(value),
            plugin="regions",
            slot="panel",
            label="panel frame",
        )
        panel_state = json.loads(str(panel_frame))
        panel_element = json.loads(
            str(
                shell.eval(
                    """(() => {
                        const frame = document.querySelector('#plugin-stage .slot-region[data-region="panel"] iframe.slot-frame');
                        if (!frame) return 'null';
                        return JSON.stringify({slot: frame.dataset.slot, slotId: frame.dataset.slotId,
                            region: frame.dataset.region, plugin: frame.dataset.plugin,
                            height: frame.style.height});
                    })()"""
                )
            )
        )
        panel_element = panel_element or {}
        check(
            "panel region mounts the first panel slot frame",
            panel_state["slot"] == "panel"
            and panel_element.get("slot") == "panel"
            and panel_element.get("slotId") == "regions-panel"
            and panel_element.get("region") == "panel"
            and panel_element.get("plugin") == "regions",
            panel_element,
        )
        tabs = json.loads(
            str(
                shell.eval(
                    "JSON.stringify([...document.querySelectorAll('.plugin-tabs button')].map(b => b.textContent.trim()))"
                )
            )
        )
        check(
            "unknown slot name falls back into the panel stage tabs",
            "panel · Regions panel" in tabs
            and "custom.unknown · Regions custom.unknown" in tabs,
            tabs,
        )
        shell.eval(
            """[...document.querySelectorAll('.plugin-tabs button')]
                .find(b => b.textContent.includes('custom.unknown')).click()"""
        )
        fallback_text = wait_frame_text(debug_port, "region:panel slot:custom.unknown", "regions", "custom.unknown")
        check(
            "switching tabs swaps the mounted panel frame",
            "region:panel slot:custom.unknown" in fallback_text
            and shell.eval(
                """document.querySelector('#plugin-stage .slot-region[data-region="panel"] iframe.slot-frame')
                    ?.dataset.slotId === 'regions-custom-unknown'"""
            ),
            fallback_text,
        )
        height = shell.eval(
            """(() => {
                const frame = document.querySelector('#plugin-stage .slot-region[data-region="panel"] iframe.slot-frame');
                return frame ? frame.style.height : '';
            })()"""
        )
        height_px = float(str(height).replace("px", "") or 0)
        footer_height = shell.eval(
            """(() => {
                const frame = document.querySelector('iframe.slot-frame[data-slot="sidebar.footer.action"]');
                return frame ? frame.style.height : '';
            })()"""
        )
        check(
            "resize messages set the per-frame height (clamped 24..640)",
            24 <= height_px <= 640 and bool(footer_height) and footer_height != "auto",
            (height, footer_height),
        )

        # Plugin-shipped module table: a bundle built for the reference module
        # table resolves its own framework through require and renders it.
        shell.call(
            "Page.navigate",
            {"url": f"http://127.0.0.1:{port}/?token={TOKEN}&plugin=framework"},
        )
        wait_until(
            lambda: shell.eval("document.getElementById('plugin-stage')?.className === 'plugin-stage open'"),
            timeout=LOAD_TIMEOUT,
            label="framework panel opens",
        )
        framework_text = wait_frame_text(debug_port, "mini framework", "framework", "panel")
        check(
            "a plugin-shipped module table resolves through require",
            "mini framework: " in framework_text,
            framework_text,
        )

        # Tier B: React comes from the shared host runtime, the half ships none.
        shell.call(
            "Page.navigate",
            {"url": f"http://127.0.0.1:{port}/?token={TOKEN}&plugin=hosted"},
        )
        wait_until(
            lambda: shell.eval("document.getElementById('plugin-stage')?.className === 'plugin-stage open'"),
            timeout=LOAD_TIMEOUT,
            label="hosted panel opens",
        )
        hosted_text = poll_frame(
            debug_port,
            "String(document.getElementById('hosted-react')?.textContent || '')",
            lambda value: "React from the host" in str(value),
            plugin="hosted",
            slot="panel",
            label="hosted React panel",
        )
        check(
            "React from the shared host runtime renders in a slot",
            "React from the host" in str(hosted_text),
            hosted_text,
        )
        clicked_text = poll_frame(
            debug_port,
            """(() => {
                const button = document.getElementById('hosted-counter');
                if (!button) return '';
                button.click();
                return String(document.getElementById('hosted-react')?.textContent || '');
            })()""",
            lambda value: "clicks: 1" in str(value),
            plugin="hosted",
            slot="panel",
            label="hosted React interaction",
        )
        check(
            "the host-provided React is interactive (useState)",
            "clicks: 1" in str(clicked_text),
            clicked_text,
        )
        # Back to the regions panel for the action round-trips below.
        shell.call(
            "Page.navigate",
            {"url": f"http://127.0.0.1:{port}/?token={TOKEN}&plugin=regions"},
        )
        wait_until(
            lambda: shell.eval("document.getElementById('plugin-stage')?.className === 'plugin-stage open'"),
            timeout=LOAD_TIMEOUT,
            label="regions panel reopens",
        )
        wait_registry_ready(debug_port, 24)
        # React/multi-plugin loading can delay the stage's first tab selection;
        # wait for the active panel frame before driving actions through it.
        wait_until(
            lambda: bool(
                shell.eval(
                    "document.querySelector('#plugin-stage iframe.slot-frame')?.dataset.slot || ''"
                )
            ),
            timeout=LOAD_TIMEOUT,
            label="active panel frame",
        )

        # ctx.action round-trips from a panel frame -------------------------
        def panel_eval(expression: str, await_promise: bool = False):
            # The active tab can change (the registry re-publishes on reload) and
            # a freshly created frame target can lag /json/list, so poll for it.
            deadline = time.time() + FRAME_TIMEOUT
            active = ""
            target = None
            while time.time() < deadline:
                active = str(
                    shell.eval(
                        """document.querySelector('#plugin-stage iframe.slot-frame')?.dataset.slot || ''"""
                    )
                )
                target = (
                    frame_target(debug_port, "regions", "slot", active)
                    if active
                    else None
                )
                if target is not None:
                    break
                time.sleep(0.2)
            if target is None:
                raise RuntimeError(f"panel frame disappeared (active slot: {active!r})")
            frame = DevTools(target["webSocketDebuggerUrl"])
            try:
                frame.call("Runtime.enable")
                return frame.eval(expression, await_promise)
            finally:
                frame.close()

        result = panel_eval("window.dshr.action('compose', {text: 'hello from regions'})", True)
        prompt = str(shell.eval("document.querySelector('textarea')?.value || ''"))
        check(
            "compose action resolves and reaches the prompt",
            result.get("ok") is True and "hello from regions" in prompt,
            (result, prompt),
        )
        result = panel_eval("window.dshr.action('notify', {message: 'plugin says hi'})", True)
        status = str(shell.eval("document.getElementById('status')?.textContent || ''"))
        check(
            "notify action resolves and reaches the status bar",
            result.get("ok") is True and "plugin says hi" in status,
            (result, status),
        )
        result = panel_eval("window.dshr.action('nope')", True)
        check(
            "unknown action is rejected with an error reply",
            result.get("ok") is False and "unsupported action" in str(result.get("error")),
            result,
        )

        # reload: edit the bundle, ask for a reload, every frame of the plugin
        # must fetch the new bundle. The frame navigates before the promise can
        # be observed, so the round-trip is asserted through its side effects.
        (plugins / "regions" / "client.js").write_text(
            REGIONS_CLIENT.replace("regions-v1", "regions-v2"), encoding="utf-8"
        )

        def reload_bundle() -> tuple[str, str]:
            """Fires `reload` and waits until the frames show the new bundle.

            Regression guard: a reload navigates every frame of the plugin at
            once. The bridge suppresses the teardown rejection of the old
            document's in-flight import (v1.2), but a frame navigation can still
            race the DevTools target, so one extra reload is allowed here.
            """
            last_error: Exception | None = None
            for attempt in range(1, 4):
                targets = [
                    target
                    for target in iframe_targets(debug_port)
                    if frame_plugin(target["url"]) == "regions"
                    and frame_params(target["url"]).get("mode") == "slot"
                ]
                if not targets:
                    raise RuntimeError("no regions slot frame to reload from")
                frame = DevTools(targets[0]["webSocketDebuggerUrl"])
                try:
                    frame.call("Runtime.enable")
                    frame.eval("window.dshr.action('reload'); true")
                finally:
                    frame.close()
                try:
                    wait_until(
                        lambda: shell.eval(
                            """(document.querySelector('iframe.slot-frame[data-plugin="regions"][data-slot="sidebar.footer.action"]')?.src || '').includes('v=%d')"""
                            % attempt
                        ),
                        timeout=FRAME_TIMEOUT,
                        label=f"reload attempt {attempt} bumps the frame version",
                    )
                    footer_text = wait_frame_text(
                        debug_port, "regions-v2", "regions", "sidebar.footer.action", timeout=20
                    )
                    # The stage panel frame is remounted while the registry
                    # re-publishes the slots, so wait for it before reading.
                    active = ""
                    remount_deadline = time.time() + FRAME_TIMEOUT
                    while time.time() < remount_deadline and not active:
                        active = str(
                            shell.eval(
                                """document.querySelector('#plugin-stage iframe.slot-frame')?.dataset.slot || ''"""
                            )
                        )
                        time.sleep(0.25)
                    if not active:
                        raise TimeoutError("stage panel frame did not remount")
                    panel_text = wait_frame_text(
                        debug_port, "regions-v2", "regions", active, timeout=20
                    )
                    return panel_text, footer_text
                except TimeoutError as error:  # noqa: PERF203 - one retry on a browser flake
                    last_error = error
                    print(f"  retry: frame reload attempt {attempt} was not visible in time")
            raise TimeoutError(f"reload did not refresh the frames: {last_error}")

        panel_text, footer_text = reload_bundle()
        check(
            "reload refreshes every frame of the plugin",
            "regions-v2" in panel_text
            and "region:sidebar.footer slot:sidebar.footer.action" in footer_text,
            (panel_text, footer_text),
        )

        panel_eval("window.dshr.action('close'); true")
        wait_until(
            lambda: shell.eval("document.getElementById('plugin-stage').className") == "plugin-stage",
            timeout=FRAME_TIMEOUT,
            label="panel closes from a frame",
        )
        check(
            "close action closes the stage and unmounts the panel frame",
            not shell.eval(
                """!!document.querySelector('#plugin-stage .slot-region[data-region="panel"] iframe')"""
            ),
        )

        # Failure paths -----------------------------------------------------
        shell.call("Page.navigate", {"url": f"http://127.0.0.1:{port}/?token={TOKEN}&plugin=demo5"})
        wait_until(
            lambda: "boom from plugin" in str(shell.eval("document.getElementById('status')?.textContent || ''")),
            timeout=LOAD_TIMEOUT,
            label="throwing bundle surfaces in the status bar",
        )
        check("a throwing bundle reports its message to the status bar", True)
        wait_until(
            lambda: "render blew up"
            in str(shell.eval("document.querySelector('.plugin-stage-head .tool')?.textContent || ''")),
            timeout=FRAME_TIMEOUT,
            label="throwing render surfaces in the stage head",
        )
        check("a throwing slot render surfaces in the panel-stage head", True)
        wait_registry_ready(debug_port, 24)

        def surviving_frames() -> int:
            count = 0
            for slot, region in STARTUP_REGIONS:
                target = frame_target(debug_port, "regions", "slot", slot)
                if target is None:
                    continue
                frame = None
                try:
                    frame = DevTools(target["webSocketDebuggerUrl"])
                    frame.call("Runtime.enable")
                    if "region:" + region in str(frame.eval(FRAME_TEXT_EXPRESSION)):
                        count += 1
                except Exception:  # noqa: BLE001 - an unrelated frame must not matter here
                    pass
                finally:
                    if frame is not None:
                        frame.close()
            return count

        wait_until(
            lambda: surviving_frames() == len(STARTUP_REGIONS),
            timeout=FRAME_TIMEOUT,
            label="surviving region frames",
        )
        check("one failing plugin does not break the other frames", True)

        # Delivery variants still render their panels -----------------------
        for plugin, panel_id, needle, label in [
            ("demo3", "assets-panel", "panel from a chunk", "multi-file (assets) bundle renders its panel"),
            ("demo4", "legacy-panel", "module table bundle", "module-table bundle renders its panel"),
            ("demo6", "exports-panel", "Exports map panel", 'exports["./client"] bundle renders its panel'),
        ]:
            shell.call(
                "Page.navigate", {"url": f"http://127.0.0.1:{port}/?token={TOKEN}&plugin={plugin}"}
            )
            wait_registry_ready(debug_port, 24)
            wait_until(
                lambda: shell.eval(
                    "document.getElementById('plugin-stage')?.className === 'plugin-stage open'"
                ),
                timeout=LOAD_TIMEOUT,
                label=f"{plugin} stage",
            )
            text = wait_frame_text(debug_port, needle, plugin, "panel", panel_id)
            check(label, needle in text, text)

        shell.close()
    except Exception as error:  # noqa: BLE001 - report the failure instead of a bare traceback
        failures.append(f"exception: {error}")
        print(f"::error::plugin-smoke: exception: {error}")
        import traceback

        traceback.print_exc()
        try:
            state = None if shell is None else shell.eval(
                """JSON.stringify({
                    href: location.href,
                    ready: document.readyState,
                    status: document.querySelector('#status')?.textContent || '',
                    stage: document.querySelector('#plugin-stage')?.className || '',
                    body: document.body ? document.body.innerText.slice(0, 400) : '',
                })"""
            )
            print(f"  state={state}")
        except Exception:  # noqa: BLE001 - best effort diagnostics
            print("  diagnostics unavailable (page or DevTools target is gone)")
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
