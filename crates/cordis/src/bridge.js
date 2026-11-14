// dshr client-half bridge: runs inside the sandboxed plugin iframe.
//
// Contract (protocol v1), see docs/PLUGINS.md:
//   iframe -> shell: ready | slots | action | resize | error | log
//   shell -> iframe: activate | deactivate | theme | action-result
//
// The bundle is loaded by the host page *after* this file and calls
// `window.dshr.slots.register({ slot, id, title, render })` (or exports a
// default `activate(ctx)`). The iframe owns no shell DOM: the shell decides
// which slot is visible and sends `activate` with its id.
(function () {
  "use strict";
  const params = new URLSearchParams(window.location.search);
  const plugin = params.get("plugin") || "";
  const token = params.get("token") || "";
  const shellOrigin = "*";
  const slots = new Map();
  let theme = params.get("theme") || "dark";
  let actionId = 0;
  const pending = new Map();
  const root = document.createElement("div");
  root.id = "dshr-slot-root";
  document.body.appendChild(root);

  function post(message) {
    parent.postMessage(Object.assign({ v: 1, ns: "dshr-client", plugin: plugin }, message), shellOrigin);
  }

  function fail(error) {
    const message = error && error.message ? error.message : String(error);
    const stack = error && error.stack ? String(error.stack) : "";
    post({ type: "error", message: message, stack: stack });
    root.textContent = "Plugin error: " + message;
    if (window.console) console.error("[dshr-client]", error);
  }

  function normalize(entry) {
    if (!entry || typeof entry !== "object") throw new Error("slot entry must be an object");
    const id = entry.id;
    if (typeof id !== "string" || !id) throw new Error("slot entry needs a string id");
    if (typeof entry.render !== "function") throw new Error("slot " + id + ": render is required");
    return {
      slot: typeof entry.slot === "string" && entry.slot ? entry.slot : "panel",
      id: id,
      title: typeof entry.title === "string" && entry.title ? entry.title : id,
      render: entry.render,
    };
  }

  let autoActivated = false;

  function publish() {
    post({
      type: "slots",
      slots: Array.from(slots.values()).map(function (entry) {
        return { slot: entry.slot, id: entry.id, title: entry.title };
      }),
    });
    // Standalone debugging: opened as a top-level page, render the first slot
    // so the bundle can be developed without the shell.
    if (!autoActivated && window.parent === window && slots.size > 0) {
      autoActivated = true;
      const first = slots.values().next().value;
      activate(first.slot, first.id);
    }
  }

  function clear() {
    while (root.firstChild) root.removeChild(root.firstChild);
  }

  async function activate(slot, id) {
    const entry = slots.get(id);
    if (!entry) {
      post({ type: "error", message: "unknown slot '" + id + "'" });
      return;
    }
    try {
      clear();
      await entry.render(root, { theme: theme, slot: slot || entry.slot, id: id });
      post({ type: "resize", height: Math.ceil(root.getBoundingClientRect().height) });
    } catch (error) {
      fail(error);
    }
  }

  function deactivate() {
    clear();
  }

  window.addEventListener("message", function (event) {
    if (event.source !== parent) return;
    const data = event.data;
    if (!data || data.v !== 1 || data.ns !== "dshr-shell") return;
    if (data.type === "activate") activate(data.slot, data.id);
    else if (data.type === "deactivate") deactivate();
    else if (data.type === "theme") {
      theme = data.theme || theme;
      document.documentElement.dataset.theme = theme;
    } else if (data.type === "action-result") {
      const settle = pending.get(data.id);
      if (settle) {
        pending.delete(data.id);
        settle({ ok: Boolean(data.ok), result: data.result, error: data.error });
      }
    }
  });

  window.addEventListener("error", function (event) {
    fail(event.error || new Error(event.message));
  });
  window.addEventListener("unhandledrejection", function (event) {
    fail(event.reason);
  });

  document.documentElement.dataset.theme = theme;

  window.dshr = {
    protocol: 1,
    name: plugin,
    get theme() {
      return theme;
    },
    slots: {
      register: function (entry) {
        const normalized = normalize(entry);
        slots.set(normalized.id, normalized);
        publish();
        return function () {
          slots.delete(normalized.id);
          publish();
        };
      },
      remove: function (id) {
        slots.delete(id);
        publish();
      },
      list: function () {
        return Array.from(slots.values()).map(function (entry) {
          return { slot: entry.slot, id: entry.id, title: entry.title };
        });
      },
    },
    // Ask the shell to perform a host-side action (command, settings change).
    // Resolves with the shell's result; the host may refuse.
    action: function (action, payload) {
      const id = "act_" + ++actionId;
      return new Promise(function (resolve) {
        pending.set(id, resolve);
        post({ type: "action", action: action, payload: payload, id: id });
      });
    },
    ready: function () {
      publish();
      post({ type: "ready", token: token ? 1 : 0 });
    },
    log: function () {
      post({
        type: "log",
        message: Array.prototype.map.call(arguments, String).join(" "),
      });
    },
    __fail: fail,
  };

  window.dshr.ready();
})();
