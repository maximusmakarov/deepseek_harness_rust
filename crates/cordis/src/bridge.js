// dshr client-half bridge: runs inside the sandboxed plugin iframe.
//
// Contract (protocol v1.1), see docs/PLUGINS.md:
//   iframe -> shell: ready | slots | action | resize | error | log
//   shell -> iframe: activate | deactivate | theme | action-result
//
// The registration shape follows the reference client API so a DSH client
// bundle's `apply(ctx)` body ports with minimal edits:
//
//   ctx.slots.inject(ownerKey, () => ctx.slots.register(
//     { name: "panel", id: "demo", title: "Demo", order: 5 }, render))
//   ctx.effect(() => { const timer = setInterval(...); return () => clearInterval(timer); })
//
// `render(container, props)` replaces React: the iframe owns no shell DOM and
// the shell decides which registered slot is visible.
(function () {
  "use strict";
  const params = new URLSearchParams(window.location.search);
  const plugin =
    (typeof window.__DSHR_PLUGIN__ === "string" && window.__DSHR_PLUGIN__) ||
    params.get("plugin") ||
    "";
  const shellOrigin = params.get("origin") || "*";
  const slots = new Map();
  const disposers = [];
  const themeListeners = new Set();
  const actionWaiters = new Map();
  let theme = params.get("theme") || "dark";
  let actionSeq = 0;
  let autoActivated = false;

  const root = document.createElement("div");
  root.id = "dshr-slot-root";
  document.body.appendChild(root);

  function post(message) {
    const payload = Object.assign({ v: 1, ns: "dshr-client", plugin: plugin }, message);
    parent.postMessage(payload, shellOrigin);
  }

  function fail(error) {
    const message = error && error.message ? error.message : String(error);
    post({ type: "error", message: message, stack: error && error.stack ? String(error.stack) : "" });
    root.textContent = "Plugin error: " + message;
    if (window.console) window.console.error("[dshr-client]", error);
  }

  function report(step, error) {
    fail(new Error(step + ": " + (error && error.message ? error.message : String(error))));
  }

  function normalize(entry, renderArgument) {
    if (!entry || typeof entry !== "object") throw new Error("slot entry must be an object");
    const render = typeof renderArgument === "function" ? renderArgument : entry.render;
    if (typeof render !== "function") throw new Error("slot: render is required");
    const name = entry.name || entry.slot || "panel";
    const id = entry.id || name;
    if (typeof id !== "string" || !id) throw new Error("slot entry needs a string id");
    return {
      name: String(name),
      id: id,
      title: typeof entry.title === "string" && entry.title ? entry.title : id,
      order: Number.isFinite(entry.order) ? Number(entry.order) : 0,
      render: render,
    };
  }

  function sorted() {
    return Array.from(slots.values()).sort(function (left, right) {
      return left.order - right.order || left.id.localeCompare(right.id);
    });
  }

  function publish() {
    post({
      type: "slots",
      slots: sorted().map(function (entry) {
        return { name: entry.name, slot: entry.name, id: entry.id, title: entry.title, order: entry.order };
      }),
    });
    // Standalone debugging: opened as a top-level page, render the first slot
    // so the bundle can be developed without the shell.
    if (!autoActivated && window.parent === window && slots.size > 0) {
      autoActivated = true;
      const first = sorted()[0];
      activate(first.name, first.id);
    }
  }

  function clear() {
    while (root.firstChild) root.removeChild(root.firstChild);
    root.removeAttribute("data-dshr-slot");
  }

  function dispose() {
    while (disposers.length > 0) {
      const cleanup = disposers.pop();
      try {
        cleanup();
      } catch (error) {
        fail(error);
      }
    }
    slots.clear();
  }

  async function activate(name, id) {
    const entry = slots.get(id) || sorted().find(function (candidate) { return candidate.name === name; });
    if (!entry) {
      post({ type: "error", message: "unknown slot '" + id + "'" });
      return;
    }
    try {
      clear();
      root.dataset.dshrSlot = entry.id;
      await entry.render(root, { theme: theme, slot: entry.name, id: entry.id, plugin: plugin });
      post({
        type: "resize",
        height: Math.ceil(root.getBoundingClientRect().height),
        slot: entry.name,
        id: entry.id,
      });
    } catch (error) {
      report("render " + entry.id, error);
    }
  }

  function deactivate() {
    // Drop the rendered view but keep registrations and effects: the shell may
    // reopen the panel (or the plugin may be mounted eagerly).
    clear();
  }

  function setTheme(next) {
    theme = next === "light" ? "light" : "dark";
    document.documentElement.dataset.theme = theme;
    themeListeners.forEach(function (listener) {
      try {
        listener(theme);
      } catch (error) {
        fail(error);
      }
    });
  }

  function subscribeTheme(listener) {
    if (typeof listener !== "function") return function () {};
    themeListeners.add(listener);
    try {
      listener(theme);
    } catch (error) {
      fail(error);
    }
    return function () {
      themeListeners.delete(listener);
    };
  }

  window.addEventListener("message", function (event) {
    if (event.source !== parent) return;
    const data = event.data;
    if (!data || data.v !== 1 || data.ns !== "dshr-shell") return;
    if (data.type === "activate") activate(data.slot || data.name, data.id);
    else if (data.type === "deactivate") deactivate();
    else if (data.type === "theme") setTheme(data.theme);
    else if (data.type === "action-result") {
      const settle = actionWaiters.get(data.id);
      if (settle) {
        actionWaiters.delete(data.id);
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

  function track(cleanup, group) {
    if (typeof cleanup !== "function") return;
    const entry = function () {
      const index = group.indexOf(entry);
      if (index >= 0) group.splice(index, 1);
      cleanup();
    };
    group.push(entry);
  }

  const ctx = {
    protocol: 1,
    name: plugin,
    get theme() {
      return theme;
    },
    onTheme: subscribeTheme,
    slots: {
      register: function (entry, renderArgument) {
        const normalized = normalize(entry, renderArgument);
        slots.set(normalized.id, normalized);
        publish();
        const unregister = function () {
          if (slots.delete(normalized.id)) publish();
        };
        disposers.push(unregister);
        return unregister;
      },
      // Reference-shaped: the factory's registrations are owned as one group
      // and can be torn down together.
      inject: function (_ownerKey, factory) {
        const before = new Set(slots.keys());
        let result;
        try {
          result = factory();
        } catch (error) {
          report("inject", error);
          return function () {};
        }
        const added = Array.from(slots.keys()).filter(function (id) { return !before.has(id); });
        const disposeGroup = function () {
          added.forEach(function (id) {
            if (slots.delete(id)) publish();
          });
          if (typeof result === "function") {
            try {
              result();
            } catch (error) {
              fail(error);
            }
          }
        };
        disposers.push(disposeGroup);
        return disposeGroup;
      },
      remove: function (id) {
        if (slots.delete(id)) publish();
      },
      list: function () {
        return sorted().map(function (entry) {
          return { name: entry.name, slot: entry.name, id: entry.id, title: entry.title, order: entry.order };
        });
      },
    },
    effect: function (factory) {
      const group = [];
      try {
        const cleanup = factory();
        track(cleanup, group);
      } catch (error) {
        report("effect", error);
      }
      const disposeGroup = function () {
        while (group.length > 0) group.pop()();
      };
      // Also owned by the frame so dispose() and a torn-down page clean up.
      disposers.push(disposeGroup);
      return disposeGroup;
    },
    // Ask the shell to perform a host-side action (reload, close, ...).
    action: function (action, payload) {
      const id = "act_" + ++actionSeq;
      return new Promise(function (resolve) {
        actionWaiters.set(id, resolve);
        post({ type: "action", action: action, payload: payload, id: id });
      });
    },
    log: function () {
      post({ type: "log", message: Array.prototype.map.call(arguments, String).join(" ") });
    },
    ready: function () {
      publish();
      post({ type: "ready" });
    },
    dispose: dispose,
    __fail: fail,
  };

  // Minimal stand-in for the reference browser module table: a bundle built for
  // DSH calls `__ModuleLoader__.load({ id, factory })`; we run the factory and
  // activate the returned plugin object. `require` only exposes what the bundle
  // brought itself, so React-based components still need porting (ADR-013 §5).
  const loaded = [];
  window.__ModuleLoader__ = {
    load: function (definition) {
      loaded.push(definition);
    },
  };
  window.dshr = ctx;
  window.dshrBridge = ctx;
  window.dshr.__loaded = loaded;
  window.dshr.__dispose = dispose;

  window.dshr.ready();
})();
