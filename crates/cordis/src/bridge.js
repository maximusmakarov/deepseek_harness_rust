// dshr client-half bridge: runs inside the sandboxed plugin iframe.
//
// Contract (protocol v1.2), see docs/PLUGINS.md:
//   iframe -> shell: ready | slots | action | resize | error | log
//   shell -> iframe: activate | deactivate | theme | action-result
//
// One plugin can own several frames. The *registry frame* publishes every
// registration and renders on demand; each *slot frame* renders exactly one
// registration and is mounted by the shell in the region its slot name maps
// to (ADR-013 §5). The host page injects the frame config as
// `window.__DSHR_FRAME__`; URL query parameters are the standalone-debugging
// fallback so the same page still works when opened directly.
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
  const frameConfig =
    window.__DSHR_FRAME__ && typeof window.__DSHR_FRAME__ === "object"
      ? window.__DSHR_FRAME__
      : null;

  // Injected config wins over the URL query. The query is only a fallback for
  // debugging the host page without the shell; every value is optional.
  function setting(key) {
    if (frameConfig && typeof frameConfig[key] === "string" && frameConfig[key] !== "") {
      return frameConfig[key];
    }
    const value = params.get(key);
    return value === null ? "" : value;
  }

  const plugin =
    (typeof window.__DSHR_PLUGIN__ === "string" && window.__DSHR_PLUGIN__) || setting("plugin");
  const frame = setting("frame");
  const mode = setting("mode") === "slot" ? "slot" : "registry";
  // Slot frames target one registration; the server validates both fields.
  const requestedSlot = mode === "slot" ? setting("slot") : "";
  const requestedId = mode === "slot" ? setting("id") : "";
  const rawContext = mode === "slot" ? setting("context") : "";
  const shellOrigin = setting("origin") || "*";
  let theme = setting("theme") === "light" ? "light" : "dark";

  // `context` is a JSON string in the host-page config; slot render props get
  // the parsed object, or null when it is missing or not an object.
  let context = null;
  let contextProblem = "";
  if (rawContext) {
    try {
      const parsed = JSON.parse(rawContext);
      if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) context = parsed;
      else contextProblem = "frame context must be a JSON object";
    } catch (error) {
      contextProblem =
        "invalid frame context: " + (error && error.message ? error.message : String(error));
    }
  }

  // Tier B / C-lite: when the host module table provides React, slot
  // registrations are React components (the reference renderer contract)
  // instead of imperative `render(element, props)` callbacks. Detection is lazy:
  // the bridge script runs before the shared runtime script in the host page,
  // so `window.__DSHR_MODULES__` only exists by the time the bundle activates.
  let reactRuntime = null;
  function reactRuntimeNow() {
    if (reactRuntime) return reactRuntime;
    const modules = window.__DSHR_MODULES__ || null;
    const React = modules && modules["react"];
    const ReactDOM = modules && (modules["react-dom/client"] || modules["react-dom"]);
    if (!React || !ReactDOM || !ReactDOM.createRoot) return null;
    reactRuntime = { React: React, ReactDOM: ReactDOM };
    return reactRuntime;
  }

  // Small host-state store shared by `ctx.get('sessions')` and the hook props.
  // The shell pushes it as a `host-state` message on ready and whenever the
  // current session, the session list or the theme changes.
  const hostState = { theme: theme, wide: true, session: null, sessions: [] };
  const hostStateListeners = new Set();

  function subscribeHostState(listener) {
    if (typeof listener !== "function") return function () {};
    hostStateListeners.add(listener);
    return function () {
      hostStateListeners.delete(listener);
    };
  }

  function notifyHostState() {
    hostStateListeners.forEach(function (listener) {
      try {
        listener(hostState);
      } catch (error) {
        fail(error);
      }
    });
    rerenderActive();
  }

  function setHostState(next) {
    if (!next || typeof next !== "object") return;
    if (typeof next.theme === "string") {
      hostState.theme = next.theme === "light" ? "light" : "dark";
    }
    if (typeof next.wide === "boolean") hostState.wide = next.wide;
    if (next.session === null || (next.session && typeof next.session === "object")) {
      hostState.session = next.session;
    }
    if (Array.isArray(next.sessions)) hostState.sessions = next.sessions;
    notifyHostState();
  }

  // The reference `sessions` service surface that a simple half uses. It is
  // populated from host-state messages, so it is empty at apply() time and
  // becomes live once the shell answers `ready`.
  const sessionsService = {
    currentProvideInfo: {
      getSnapshot: function () {
        return hostState.session;
      },
      subscribe: subscribeHostState,
    },
    get: function (id) {
      return (
        hostState.sessions.filter(function (candidate) {
          return candidate && candidate.id === id;
        })[0] || null
      );
    },
    list: function () {
      return hostState.sessions.slice();
    },
  };

  // The reference `slots` service (`ctx.get('slots')`) used by some halves: the
  // same registry as `ctx.slots`, plus `entries(key)` for introspection.
  const slotsService = {
    inject: function (key, factory) {
      return ctx.slots.inject(key, factory);
    },
    register: function (entry, component) {
      return ctx.slots.register(entry, component);
    },
    remove: function (id) {
      return ctx.slots.remove(id);
    },
    list: function () {
      return ctx.slots.list();
    },
    entries: function (key) {
      return sorted()
        .filter(function (entry) {
          return !key || entry.name === key;
        })
        .map(described);
    },
  };

  // Minimal `locale` service: the dshr shell is English-only for plugin frames.
  const localeService = {
    getSnapshot: function () {
      return { active: "en", locale: "en" };
    },
    subscribe: function (listener) {
      if (typeof listener === "function") {
        try {
          listener({ active: "en", locale: "en" });
        } catch (error) {
          fail(error);
        }
      }
      return function () {};
    },
  };

  // Hook props handed to React slot components (the documented dshr subset).
  // Built lazily from the same host-state store as `ctx.get('sessions')`.
  let cachedHostProps = null;

  function hostPropsNow() {
    const runtime = reactRuntimeNow();
    if (!runtime) return {};
    if (cachedHostProps) return cachedHostProps;
    const React = runtime.React;

    function useHostState(selector) {
      const read = function () {
        return typeof selector === "function" ? selector(hostState) : hostState;
      };
      const pair = React.useState(read);
      const value = pair[0];
      const setValue = pair[1];
      React.useEffect(function () {
        return subscribeHostState(function () {
          setValue(read());
        });
      }, []);
      return value;
    }

    cachedHostProps = {
      useHostState: useHostState,
      useTheme: function () {
        return useHostState(function (state) {
          return state.theme;
        });
      },
      useSession: function () {
        return useHostState(function (state) {
          return state.session;
        });
      },
      useSessions: function (selector) {
        return useHostState(function (state) {
          return typeof selector === "function" ? selector(state.sessions) : state.sessions;
        });
      },
    };
    return cachedHostProps;
  }

  // DSH slot name -> shell region id. Exhaustive over the 116-key DSH catalog
  // (crates/cordis/tests/slot-catalog.json) plus the two legacy dshr keys;
  // unknown names fall back to `panel` so older plugins keep rendering.
  const SLOT_REGIONS = Object.freeze({
    "contextMenu.content": "panel",
    "contextMenu.icon": "panel",
    "conversation.approval.detail": "approval.detail",
    "conversation.chat.assistant-actions": "message.action",
    "conversation.chat.commandview": "message.action",
    "conversation.chat.flow": "panel",
    "conversation.chat.markdown.fence": "panel",
    "conversation.chat.message.images": "panel",
    "conversation.chat.node": "panel",
    "conversation.chat.reasoning.body": "panel",
    "conversation.chat.text-interaction": "message.action",
    "conversation.chat.turnTail": "message.action",
    "conversation.chat.user.before": "message.action",
    "conversation.chat.user.images": "panel",
    "conversation.composer": "composer.actions",
    "conversation.composer.bar": "composer.actions",
    "conversation.composer.dock": "composer.actions",
    "conversation.header": "conversation.header",
    "conversation.header.leading": "conversation.header",
    "conversation.hero.agentPreset": "panel",
    "conversation.hero.brand.mark": "panel",
    "conversation.hero.frame": "panel",
    "conversation.hero.options": "panel",
    "conversation.hero.workspace": "panel",
    "conversation.hero.workspace.directoryFlow": "panel",
    "conversation.input.activity": "composer.actions",
    "conversation.input.attachments": "composer.actions",
    "conversation.input.dock": "composer.actions",
    "conversation.input.left": "composer.actions",
    "conversation.input.model": "composer.actions",
    "conversation.input.model.label": "composer.actions",
    "conversation.input.model.options": "composer.actions",
    "conversation.input.overlay": "composer.actions",
    "conversation.input.permission": "composer.actions",
    "conversation.input.plan": "composer.actions",
    "conversation.input.right": "composer.actions",
    "conversation.plan-review.actions": "composer.actions",
    "conversation.session": "panel",
    "conversation.session.header": "conversation.header",
    "conversation.session.header.actions": "conversation.header",
    "conversation.session.header.actions.subagent.footer": "conversation.header",
    "conversation.session.header.corner": "conversation.header",
    "conversation.session.header.lineage": "conversation.header",
    "conversation.session.header.utilities": "conversation.header",
    "conversation.session.header.views.actions": "conversation.header",
    "conversation.trajectory.images": "panel",
    "conversation.user.text.node": "message.action",
    "conversation.view": "panel",
    "deliverables.file.actions": "panel",
    "deliverables.review.file.actions": "panel",
    "main": "panel",
    "main.conversation": "panel",
    "message.action": "message.action",
    "panel": "panel",
    "plugins.add.actions": "plugins.add",
    "plugins.bundle.activation": "plugins.detail",
    "plugins.bundle.config": "plugins.detail",
    "plugins.detail.actions": "plugins.detail",
    "plugins.detail.badge": "plugins.detail",
    "plugins.detail.section": "plugins.detail",
    "plugins.item": "plugins.detail",
    "plugins.row.config": "plugins.detail",
    "plugins.tab": "plugins.detail",
    "rightbar": "panel",
    "rightbar.session": "panel",
    "root": "panel",
    "settings.action": "settings.action",
    "settings.close": "settings.action",
    "settings.general.item": "settings.section",
    "settings.header": "settings.section",
    "settings.launcher": "shell.leading",
    "settings.launcher.notifications": "shell.leading",
    "settings.models.footer": "settings.action",
    "settings.models.provider-card": "settings.section",
    "settings.models.sign-in": "settings.section",
    "settings.onboarding": "settings.section",
    "settings.plugins.tab": "settings.section",
    "settings.section": "settings.section",
    "settings.section.icon": "settings.icon",
    "settings.trigger": "shell.leading",
    "shell.bottom": "shell.bottom",
    "shell.leading": "shell.leading",
    "shell.overlay": "shell.overlay",
    "shell.quota-notice": "shell.bottom",
    "sidebar": "panel",
    "sidebar.brand.mark": "sidebar.top",
    "sidebar.brand.name": "sidebar.top",
    "sidebar.chat.conversation": "panel",
    "sidebar.footer.action": "sidebar.footer",
    "sidebar.panellist": "sidebar.footer",
    "sidebar.panellist.actions": "sidebar.footer",
    "sidebar.right.pane.tab": "panel",
    "sidebar.right.pane.tab.title": "panel",
    "sidebar.right.tab.document": "panel",
    "sidebar.right.tab.document.action": "panel",
    "sidebar.right.tab.document.actions": "panel",
    "sidebar.right.tab.document.office.pdf": "panel",
    "sidebar.right.tab.document.unpreviewable": "panel",
    "sidebar.right.tab.files.actions": "panel",
    "sidebar.right.tab.guide": "panel",
    "sidebar.right.tab.guide.actions": "panel",
    "sidebar.right.tab.guide.entry": "panel",
    "sidebar.right.tab.menu.item": "panel",
    "sidebar.session.action": "sidebar.session.action",
    "sidebar.session.row.hover": "sidebar.session.action",
    "sidebar.session.row.leading": "sidebar.session.action",
    "sidebar.session.row.trailing": "sidebar.session.action",
    "sidebar.settings": "sidebar.footer",
    "sidebar.toggle.badge": "sidebar.top",
    "sidebar.workspaces": "sidebar.top",
    "sidebar.workspaces.directoryFlow": "sidebar.top",
    "sidebar.workspaces.session.menu.item": "sidebar.footer",
    "sidebar.workspaces.session.row.action": "sidebar.session.action",
    "tool.call.images": "tool.card",
    "tool.call.toolview": "tool.card",
    "tool.view.cordis": "tool.card",
    "workspace.picker.directoryFlow": "panel",
    "workspace.toast": "shell.bottom",
  });

  function slotRegion(name) {
    return Object.prototype.hasOwnProperty.call(SLOT_REGIONS, name) ? SLOT_REGIONS[name] : "panel";
  }

  const slots = new Map();
  const disposers = [];
  const themeListeners = new Set();
  const actionWaiters = new Map();
  let actionSeq = 0;
  let autoActivated = false;
  let readySent = false;
  let slotStarted = false;
  let slotPending = false;
  let active = null;
  // Set once the document starts navigating away. A shell reload bumps every
  // frame version at once, and Chrome rejects the in-flight `import(url)` of the
  // old document while it is being torn down (observed as
  // "Failed to resolve module specifier ''"). The new document loads fine, so
  // reporting that rejection would only paint a spurious error in the shell.
  let unloading = false;

  const root = document.createElement("div");
  root.id = "dshr-slot-root";
  document.body.appendChild(root);

  function post(message) {
    const payload = Object.assign(
      { v: 1, ns: "dshr-client", plugin: plugin, frame: frame, mode: mode },
      message,
    );
    parent.postMessage(payload, shellOrigin);
  }

  if (contextProblem) {
    post({ type: "log", message: contextProblem });
    if (window.console) window.console.warn("[dshr-client]", contextProblem);
  }

  function fail(error) {
    if (unloading) return;
    const message = error && error.message ? error.message : String(error);
    // Keep the failure inspectable from the frame (debugging a real plugin's
    // compatibility gap without a shell round-trip).
    window.__DSHR_LAST_ERROR__ = {
      message: message,
      stack: error && error.stack ? String(error.stack) : "",
    };
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

  // One wire shape for registrations, so `slots` messages and `slots.list()`
  // agree and the shell can place every entry without recomputing the region.
  function described(entry) {
    return {
      name: entry.name,
      slot: entry.name,
      id: entry.id,
      title: entry.title,
      order: entry.order,
      region: slotRegion(entry.name),
    };
  }

  function publish() {
    post({
      type: "slots",
      slots: sorted().map(described),
    });
    // A registration may arrive after the frame reported ready (async setup);
    // slot frames render as soon as their registration shows up.
    trySlotRender();
    // Standalone debugging: opened as a top-level page, render the first slot
    // so the bundle can be developed without the shell. Slot frames are driven
    // by the requested registration instead.
    if (mode === "registry" && !autoActivated && window.parent === window && slots.size > 0) {
      autoActivated = true;
      const first = sorted()[0];
      activate(first.name, first.id);
    }
  }

  // React slot rendering (C-lite): one root per frame, re-rendered on theme,
  // host-state and context changes. A reference-style error boundary keeps a
  // throwing component from blanking the region and reports it to the shell.
  let reactRoot = null;
  let reactEntry = null;
  let reactProps = null;
  let cachedBoundary = null;

  function slotBoundary() {
    const runtime = reactRuntimeNow();
    if (!runtime) return null;
    if (cachedBoundary) return cachedBoundary;
    const React = runtime.React;
    cachedBoundary = class DshrSlotBoundary extends React.Component {
      constructor(props) {
        super(props);
        this.state = { error: null };
      }

      static getDerivedStateFromError(error) {
        return { error: error };
      }

      componentDidCatch(error) {
        fail(error);
      }

      render() {
        if (this.state.error) {
          const message =
            this.state.error && this.state.error.message
              ? this.state.error.message
              : String(this.state.error);
          return React.createElement(
            "div",
            { className: "dshr-plugin-error" },
            "Plugin render error: " + message,
          );
        }
        return this.props.children;
      }
    };
    return cachedBoundary;
  }

  function mountReact(entry, props) {
    const runtime = reactRuntimeNow();
    if (!runtime) return false;
    const React = runtime.React;
    if (!reactRoot) reactRoot = runtime.ReactDOM.createRoot(root);
    reactEntry = entry;
    reactProps = props;
    reactRoot.render(
      React.createElement(
        slotBoundary(),
        null,
        React.createElement(
          entry.render,
          Object.assign({}, props, hostPropsNow()),
        ),
      ),
    );
    return true;
  }

  function rerenderActive() {
    if (!reactRoot || !reactEntry || !reactProps) return;
    mountReact(reactEntry, reactProps);
  }

  function clear() {
    if (reactRoot) {
      try {
        reactRoot.unmount();
      } catch (error) {
        // The frame may already be torn down; nothing to report.
      }
      reactRoot = null;
      reactEntry = null;
      reactProps = null;
    }
    while (root.firstChild) root.removeChild(root.firstChild);
    root.removeAttribute("data-dshr-slot");
    active = null;
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
    clear();
  }

  function measure() {
    return Math.ceil(root.getBoundingClientRect().height);
  }

  function reportResize(entry) {
    post({
      type: "resize",
      height: measure(),
      slot: entry.name,
      id: entry.id,
      region: slotRegion(entry.name),
    });
  }

  async function mount(entry, props) {
    try {
      const reactPropsHost = Object.assign({}, props, {
        theme: theme,
        wide: hostState.wide,
        context: props && props.context !== undefined ? props.context : null,
      });
      if (mountReact(entry, reactPropsHost)) {
        root.dataset.dshrSlot = entry.id;
        active = entry;
      } else {
        // `clear()` also resets `active`, so set the slot identity after it.
        clear();
        root.dataset.dshrSlot = entry.id;
        active = entry;
        await entry.render(root, props);
      }
      reportResize(entry);
    } catch (error) {
      report("render " + entry.id, error);
    }
  }

  async function activate(name, id) {
    const entry =
      slots.get(id) ||
      sorted().find(function (candidate) {
        return candidate.name === name;
      });
    if (!entry) {
      post({ type: "error", message: "unknown slot '" + id + "'" });
      return;
    }
    await mount(entry, { theme: theme, slot: entry.name, id: entry.id, plugin: plugin });
  }

  function deactivate() {
    // Drop the rendered view but keep registrations and effects: the shell may
    // reopen the panel (or the plugin may be mounted eagerly).
    clear();
  }

  // The registration a slot frame was created for: id is authoritative, the
  // slot name is the fallback for bundles that only registered one candidate.
  function matchRequested() {
    if (!requestedSlot && !requestedId) return null;
    if (requestedId && slots.has(requestedId)) return slots.get(requestedId);
    if (!requestedSlot) return null;
    return (
      sorted().find(function (candidate) {
        return candidate.name === requestedSlot;
      }) || null
    );
  }

  function trySlotRender() {
    if (mode !== "slot" || !slotStarted || !slotPending) return;
    const entry = matchRequested();
    if (!entry) return;
    slotPending = false;
    mount(entry, {
      theme: theme,
      slot: entry.name,
      id: entry.id,
      plugin: plugin,
      context: context,
    });
  }

  // Called once the bundle's activation function settles. The registration is
  // usually already there; the timeout only covers async registration and then
  // reports the slot frame as unusable.
  function startSlotMode() {
    if (mode !== "slot" || slotStarted) return;
    slotStarted = true;
    slotPending = true;
    trySlotRender();
    if (!slotPending) return;
    setTimeout(function () {
      if (!slotPending) return;
      trySlotRender();
      if (!slotPending) return;
      slotPending = false;
      const message =
        "slot '" + requestedSlot + "' (" + requestedId + ") was not registered";
      post({ type: "error", message: message });
      if (window.console) window.console.warn("[dshr-client]", message);
    }, 1500);
  }

  function setTheme(next) {
    theme = next === "light" ? "light" : "dark";
    hostState.theme = theme;
    document.documentElement.dataset.theme = theme;
    notifyHostState();
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

  function postReady() {
    if (readySent) return;
    readySent = true;
    post({ type: "ready" });
  }

  // Called by the host page when the bundle's activation function settles:
  // ready after activation (so the shell no longer races the registrations),
  // and slot frames then resolve their requested registration.
  function frameActivated() {
    postReady();
    startSlotMode();
  }

  window.addEventListener("message", function (event) {
    if (event.source !== parent) return;
    const data = event.data;
    if (!data || data.v !== 1 || data.ns !== "dshr-shell") return;
    if (data.type === "activate") activate(data.slot || data.name, data.id);
    else if (data.type === "deactivate") deactivate();
    else if (data.type === "theme") setTheme(data.theme);
    else if (data.type === "host-state") setHostState(data.state);
    else if (data.type === "action-result") {
      const settle = actionWaiters.get(data.id);
      if (settle) {
        actionWaiters.delete(data.id);
        settle({ ok: Boolean(data.ok), result: data.result, error: data.error });
      }
    }
  });

  // The rendered slot reports its height again when the shell window resizes.
  window.addEventListener("resize", function () {
    if (active) reportResize(active);
  });

  window.addEventListener("pagehide", function () {
    unloading = true;
  });
  window.addEventListener("unload", function () {
    unloading = true;
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
    // Reference `ctx.get(service)` subset: `sessions` is populated from
    // host-state messages; anything else is unknown in the dshr bridge.
    get: function (name) {
      if (name === "sessions") return sessionsService;
      if (name === "slots") return slotsService;
      if (name === "locale") return localeService;
      return undefined;
    },
    // Reference Cordis fiber helper: run `callback(inner)` only when every
    // requested service exists. Optional integrations (betterSidebar,
    // sidebarRight, webTerminals, ...) are simply not mounted in dshr, which is
    // exactly the degradation DSH plugins are written to expect.
    inject: function (dependencies, callback) {
      if (typeof callback !== "function") return function () {};
      const list = Array.isArray(dependencies) ? dependencies : [dependencies];
      const resolved = list.every(function (name) {
        return ctx.get(name) !== undefined;
      });
      if (!resolved) return function () {};
      try {
        const result = callback(ctx);
        if (typeof result === "function") return result;
        return function () {};
      } catch (error) {
        fail(error);
        return function () {};
      }
    },
    get theme() {
      return theme;
    },
    onTheme: subscribeTheme,
    slotRegion: slotRegion,
    SLOT_REGIONS: SLOT_REGIONS,
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
        const added = Array.from(slots.keys()).filter(function (id) {
          return !before.has(id);
        });
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
        return sorted().map(described);
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
    ready: postReady,
    dispose: dispose,
    __frameActivated: frameActivated,
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
})();
