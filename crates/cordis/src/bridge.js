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

  // ---------------------------------------------------------------------------
  // DSH client slot catalog (generated from the reference repository:
  // deepseek-ai/deepseek-harness, packages/extensions/cordis-client-runner/
  // src/client/slot-catalog.ts). The host page publishes it as
  // `window.__DSHR_SLOT_META__`; registration validation, cardinality, scope and
  // the framework standard-kit seats are derived from it exactly like the
  // reference SlotCore does. Slots outside the catalog are only accepted from
  // the dshr extension namespaces (`panel`/`message.action` legacy keys and
  // `custom.*`/`x-*`/`dshr.*` test/extension keys).
  // ---------------------------------------------------------------------------
  const SLOT_META =
    window.__DSHR_SLOT_META__ && typeof window.__DSHR_SLOT_META__ === "object"
      ? window.__DSHR_SLOT_META__
      : null;
  const SLOT_SPECS = SLOT_META && SLOT_META.slots ? SLOT_META.slots : null;
  const EXTENSION_SLOT = /^(panel$|message\.action$|custom\.|x-|dshr\.)/;

  // Runtime declaration ledger (SlotCore parity): the shipped catalog seeds the
  // declared keys, and a live registration may declare child slots with
  // `children: { key: { kind, scope } }`. A child declaration lives exactly as
  // long as the declaring entry (both cascade away on unload).
  const declaredSlots = new Map();
  const slotInjectionWaiters = new Map();
  if (SLOT_SPECS) {
    for (const [key, spec] of Object.entries(SLOT_SPECS)) {
      declaredSlots.set(key, {
        kind: spec.kind,
        scope: spec.scope,
        std: Array.isArray(spec.std) ? spec.std : [],
        inject: Boolean(spec.inject),
        declaredByEntry: null,
      });
    }
  }

  function slotSpec(name) {
    return declaredSlots.get(name) || null;
  }

  function declareChildSlots(entry, children) {
    if (!children || typeof children !== "object") return;
    const declared = [];
    for (const [childName, childSpec] of Object.entries(children)) {
      if (declaredSlots.has(childName)) {
        throw new Error(
          'slot "' + childName + '" is already declared (by ' +
            (declaredSlots.get(childName).declaredByEntry || "the shipped catalog") + ")",
        );
      }
      if (!childSpec || typeof childSpec !== "object" || typeof childSpec.kind !== "string") {
        throw new Error('child slot "' + childName + '" needs a kind');
      }
      const record = {
        kind: childSpec.kind,
        scope: typeof childSpec.scope === "string" ? childSpec.scope : "root",
        std: [],
        inject: Boolean(childSpec.inject),
        declaredByEntry: entry.id,
      };
      declaredSlots.set(childName, record);
      declared.push({ name: childName, record });
      runSlotWaiters(childName);
    }
    entry.childSlots = declared;
  }

  function undeclareChildSlots(entry) {
    if (!entry.childSlots) return;
    for (const child of entry.childSlots) {
      // Cascade: registrations into the child slot and pending injections die
      // with the declaration that created it.
      for (const candidate of Array.from(slots.values())) {
        if (candidate.name === child.name) {
          slots.delete(candidate.id);
          undeclareChildSlots(candidate);
        }
      }
      const waiters = child.record.injectionDisposers;
      if (waiters) {
        for (const dispose of waiters.slice().reverse()) {
          try {
            dispose();
          } catch (error) {
            fail(error);
          }
        }
      }
      declaredSlots.delete(child.name);
    }
    entry.childSlots = [];
  }

  function runSlotWaiters(name) {
    const waiters = slotInjectionWaiters.get(name);
    if (!waiters || waiters.length === 0) return;
    slotInjectionWaiters.delete(name);
    const record = declaredSlots.get(name);
    const disposers = record ? (record.injectionDisposers = record.injectionDisposers || []) : [];
    for (const waiter of waiters) {
      if (waiter.disposed) continue;
      try {
        const result = waiter.factory();
        const disposer = cordisDisposer(result);
        if (disposer) {
          disposers.push(disposer);
          waiter.disposers.push(disposer);
        }
      } catch (error) {
        report("inject", error);
      }
    }
  }

  function isExtensionSlot(name) {
    return EXTENSION_SLOT.test(name);
  }

  function slotKindOf(name) {
    const spec = slotSpec(name);
    return spec && typeof spec.kind === "string" ? spec.kind : "list";
  }

  function slotScopeOf(name) {
    const spec = slotSpec(name);
    return spec && typeof spec.scope === "string" ? spec.scope : "root";
  }

  function slotStandardNames(name) {
    const spec = slotSpec(name);
    return spec && Array.isArray(spec.std) ? spec.std : [];
  }

  function slotNeedsInject(name) {
    const spec = slotSpec(name);
    return Boolean(spec && spec.inject);
  }

  // ---------------------------------------------------------------------------
  // Owner props (reference `renderSlot` owner share). The shell can only hand
  // JSON to a frame, while real render sites also pass callbacks
  // (`settings.section.close`, `openFile`, `inspectCall`, ...). Those travel as
  // `{ "$callback": "<name>" }` markers (server-side they stay a plain JSON
  // object) and are materialized into functions here; calling one posts an
  // `action: "callback"` to the shell, which dispatches by name and answers
  // through `action-result`, so `openFile(path).then(...)` keeps the reference
  // promise shape. Callbacks keep their marker name, so a component forwarding
  // owner props into a child slot re-serializes them instead of losing them.
  // ---------------------------------------------------------------------------
  const CALLBACK_MARKER = "$callback";

  function ownerCallback(name) {
    const callback = function () {
      const args = Array.prototype.slice.call(arguments);
      return ctx.action("callback", { name: name, args: args });
    };
    callback[CALLBACK_MARKER] = name;
    return callback;
  }

  function materializeOwnerProps(value) {
    if (!value || typeof value !== "object") return value;
    if (Array.isArray(value)) return value.map(materializeOwnerProps);
    if (typeof value[CALLBACK_MARKER] === "string") return ownerCallback(value[CALLBACK_MARKER]);
    const out = {};
    for (const key of Object.keys(value)) out[key] = materializeOwnerProps(value[key]);
    return out;
  }

  // JSON-safe projection of owner props for a child slot anchor: materialized
  // callbacks go back to their marker form, plain functions are dropped (a
  // frame cannot clone arbitrary closures into another frame).
  function serializeOwnerProps(value) {
    const seen = new Set();
    function walk(node) {
      if (node === null) return null;
      const type = typeof node;
      if (type === "function") {
        return typeof node[CALLBACK_MARKER] === "string"
          ? { [CALLBACK_MARKER]: node[CALLBACK_MARKER] }
          : undefined;
      }
      if (type !== "object") return type === "undefined" ? undefined : node;
      if (seen.has(node)) return undefined;
      seen.add(node);
      if (Array.isArray(node)) {
        const list = node.map(walk);
        seen.delete(node);
        return list;
      }
      const out = {};
      for (const key of Object.keys(node)) {
        const next = walk(node[key]);
        if (next !== undefined) out[key] = next;
      }
      seen.delete(node);
      return out;
    }
    try {
      return JSON.stringify(walk(value) || {});
    } catch (error) {
      return "{}";
    }
  }

  let frameOwnerProps = {};
  try {
    const configured = frameConfig ? frameConfig.ownerProps : undefined;
    if (configured && typeof configured === "object") {
      frameOwnerProps = materializeOwnerProps(configured);
    } else if (typeof configured === "string" && configured !== "") {
      const parsed = JSON.parse(configured);
      if (parsed && typeof parsed === "object") frameOwnerProps = materializeOwnerProps(parsed);
    }
  } catch (error) {
    frameOwnerProps = {};
  }
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
    subscribe: subscribeSlots,
    getVersion: function (region) {
      return slotVersionNow(region);
    },
    entriesOfSlot: function (region) {
      return slotsService.entries(region);
    },
    entries: function (key) {
      return sorted()
        .filter(function (entry) {
          return !key || entry.name === key;
        })
        .map(function (entry) {
          return Object.assign({}, described(entry), {
            component: entry.render,
            options: {
              name: entry.name,
              id: entry.id,
              key: entry.key,
              title: entry.title,
              order: entry.order,
              priority: entry.priority,
              locale: entry.locale,
            },
          });
        });
    },
  };

  // Minimal `locale` service: the dshr shell is English-only for plugin frames,
  // but `register(namespace, dicts)` is supported so components that declare
  // `locale` get a working `t()` prop.
  const localeDictionaries = new Map();
  // Browser-side `configForms` (C1.2), persisted by C5.6b: the form is a view of
  // the server's settings store (`/v1/config-forms/{entry}`). The frame asks the
  // shell — it holds the user token — to read or mutate an entry and applies the
  // returned snapshot, so edits survive a frame reload.
  const configFormStates = new Map();
  const configFormWaiters = new Map();
  let configFormSeq = 0;

  function requestConfigForm(type, payload) {
    const id = "config-form-" + ++configFormSeq;
    return new Promise(function (resolve) {
      configFormWaiters.set(id, resolve);
      post(Object.assign({ type: type, id: id }, payload));
    });
  }

  function browserConfigForm(entryId) {
    const existing = configFormStates.get(entryId);
    if (existing) return existing;
    const state = {
      entryId,
      status: "ready",
      writable: true,
      value: {},
      base: {},
      user: {},
      // Reference ConfigFormSnapshot.revision is a number; dshr starts at 0
      // until the shell answers with the stored revision.
      revision: 0,
      listeners: new Set(),
      // `useSyncExternalStore` requires a stable snapshot object between
      // notifications; a fresh object every read causes an update loop.
      snapshot: null,
      getSnapshot: function () {
        if (!state.snapshot) {
          state.snapshot = {
            status: state.status,
            value: state.value,
            base: state.base,
            user: state.user,
            revision: state.revision,
            writable: state.writable,
          };
        }
        return state.snapshot;
      },
      subscribe: function (listener) {
        if (typeof listener !== "function") return function () {};
        state.listeners.add(listener);
        return function () {
          state.listeners.delete(listener);
        };
      },
      applySnapshot: function (snapshot) {
        if (!snapshot || typeof snapshot !== "object") return false;
        state.status = typeof snapshot.status === "string" ? snapshot.status : "ready";
        state.value = snapshot.value && typeof snapshot.value === "object" ? snapshot.value : {};
        state.base = snapshot.base && typeof snapshot.base === "object" ? snapshot.base : {};
        state.user =
          snapshot.user && typeof snapshot.user === "object" ? snapshot.user : state.value;
        state.revision = Number(snapshot.revision) || 0;
        state.writable = snapshot.writable !== false;
        state.snapshot = null;
        state.listeners.forEach(function (listener) {
          try {
            listener();
          } catch (error) {
            fail(error);
          }
        });
        return true;
      },
      mutate: async function (operations, expectedRevision) {
        if (!Array.isArray(operations)) return false;
        if (expectedRevision !== undefined && Number(expectedRevision) !== Number(state.revision)) {
          return false;
        }
        const reply = await requestConfigForm("config-form-mutate", {
          entry: entryId,
          ops: operations,
          expected_revision: expectedRevision === undefined ? null : Number(expectedRevision),
        });
        if (!reply || reply.error || typeof reply.snapshot !== "string") return false;
        try {
          return state.applySnapshot(JSON.parse(reply.snapshot));
        } catch (error) {
          fail(error);
          return false;
        }
      },
      set: function (field, value) {
        return state.mutate([{ op: "set", path: [field], value: value }]);
      },
      unset: function (field) {
        return state.mutate([{ op: "unset", path: [field] }]);
      },
    };
    configFormStates.set(entryId, state);
    // Hydrate without blocking the synchronous handle contract.
    requestConfigForm("config-form-read", { entry: entryId }).then(function (reply) {
      if (!reply || reply.error || typeof reply.snapshot !== "string") return;
      try {
        state.applySnapshot(JSON.parse(reply.snapshot));
      } catch (error) {
        fail(error);
      }
    });
    return state;
  }
  const configFormsService = {
    get: function (entryId) {
      return browserConfigForm(String(entryId));
    },
  };

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
    register: function (namespace, dictionaries) {
      if (typeof namespace !== "string" || !namespace) return function () {};
      localeDictionaries.set(namespace, dictionaries || {});
      return function () {
        localeDictionaries.delete(namespace);
      };
    },
    // Reference `locale.bind(namespace)` returns the `t(key, params)` function
    // plugins use in non-component code.
    bind: function (namespace) {
      return translatorFor(typeof namespace === "string" ? namespace : "");
    },
  };

  function translate(namespace, key, params) {
    const dictionaries = localeDictionaries.get(namespace);
    const table = dictionaries && (dictionaries.en || dictionaries);
    let text = table && typeof table[key] === "string" ? table[key] : String(key);
    if (params && typeof params === "object") {
      text = text.replace(/\{(\w+)\}/g, function (match, name) {
        return params[name] === undefined ? match : String(params[name]);
      });
    }
    return text;
  }

  function cordisDisposer(result) {
    if (result === undefined || result === null) return null;
    if (typeof result === "function") return result;
    // Cordis inject/effect factories may be generators: call the factory body,
    // then run yielded disposers (in reverse) on teardown.
    if (typeof result === "object" && typeof result.next === "function") {
      const yielded = [];
      let step = result.next();
      while (!step.done) {
        if (typeof step.value === "function") yielded.push(step.value);
        step = result.next();
      }
      return function () {
        yielded
          .slice()
          .reverse()
          .forEach(function (dispose) {
            try {
              dispose();
            } catch (error) {
              fail(error);
            }
          });
      };
    }
    if (Array.isArray(result)) {
      const disposers = result.filter(function (item) {
        return typeof item === "function";
      });
      return function () {
        disposers
          .slice()
          .reverse()
          .forEach(function (dispose) {
            try {
              dispose();
            } catch (error) {
              fail(error);
            }
          });
      };
    }
    return null;
  }

  function injectedProps(entry) {
    if (!entry.inject) return {};
    try {
      const args = injectArguments(entry);
      const value =
        typeof entry.inject === "function" ? entry.inject.apply(null, args) : entry.inject;
      return value && typeof value === "object" ? value : {};
    } catch (error) {
      // A seat's injected props may legitimately reference host services dshr
      // does not provide; render the component degraded instead of failing the
      // whole plugin.
      log("seat inject skipped: " + (error && error.message ? error.message : String(error)));
      return {};
    }
  }

  // Framework standard-kit seats per the reference scope contract: names the
  // catalog lists for a slot become props (`useX` selector hooks / plain values).
  // dshr implements the subset it can source from host state; any other declared
  // hook stays callable and returns undefined so real components do not crash on
  // a missing host service, and plain values stay undefined.
  const standardCache = new Map();
  function slotStandardProps(name) {
    if (standardCache.has(name)) return standardCache.get(name);
    const out = {};
    const host = hostPropsNow();
    for (const propName of slotStandardNames(name)) {
      if (Object.prototype.hasOwnProperty.call(host, propName)) {
        out[propName] = host[propName];
        continue;
      }
      if (propName === "sessionId") {
        out.sessionId = hostState && hostState.session ? hostState.session.id : undefined;
        continue;
      }
      if (/^use[A-Z]/.test(propName)) {
        out[propName] = function () {
          return undefined;
        };
      }
    }
    // dshr extension hooks stay available on every entry (C-lite contract).
    Object.assign(out, host);
    standardCache.set(name, out);
    return out;
  }

  // Store seats (reference packages/client/store): a registration may declare a
  // shared handle or a factory; the framework owns one instance per scope key and
  // hands the component a selector hook plus the baked action set.
  const storeInstances = new Map();

  function currentSessionId() {
    return hostState && hostState.session ? hostState.session.id : undefined;
  }

  function resolveStoreInstance(entry) {
    if (!entry.store) return null;
    const scopeKey =
      entry.scope === "session" || entry.scope === "session-maybe"
        ? String(currentSessionId() || "root")
        : "root";
    const cacheKey = entry.id + "\u0000" + scopeKey;
    const existing = storeInstances.get(cacheKey);
    if (existing) return existing;
    try {
      const handle = typeof entry.store === "function" ? entry.store() : entry.store;
      if (!handle || typeof handle.create !== "function") return null;
      const instance = handle.create(scopeKey === "root" ? undefined : scopeKey);
      if (!instance || typeof instance.getSnapshot !== "function") return null;
      storeInstances.set(cacheKey, instance);
      return instance;
    } catch (error) {
      report("store", error);
      return null;
    }
  }

  function storeProps(entry) {
    const instance = resolveStoreInstance(entry);
    if (!instance) return {};
    const runtime = reactRuntimeNow();
    const actions = instance.actions && typeof instance.actions === "object" ? instance.actions : {};
    if (!runtime) return { actions: actions };
    const React = runtime.React;
    function useStore(selector, equality) {
      const read = function () {
        const snapshot = instance.getSnapshot();
        return typeof selector === "function" ? selector(snapshot) : snapshot;
      };
      const pair = React.useState(read);
      const value = pair[0];
      const setValue = pair[1];
      const last = React.useRef(value);
      React.useEffect(
        function () {
          return instance.subscribe(function () {
            const next = read();
            const equal = typeof equality === "function" ? equality : Object.is;
            if (!equal(last.current, next)) {
              last.current = next;
              setValue(next);
            }
          });
        },
        [instance, selector, equality],
      );
      return value;
    }
    return { useStore: useStore, actions: actions };
  }

  function injectArguments(entry) {
    const sessionId = currentSessionId();
    const instance = entry.store ? resolveStoreInstance(entry) : null;
    const actions = instance && instance.actions ? instance.actions : undefined;
    if (entry.scope === "session") {
      return entry.store ? [sessionId, actions] : [sessionId];
    }
    if (entry.scope === "session-maybe") {
      return entry.store ? [sessionId, actions] : [sessionId];
    }
    return entry.store ? [actions] : [];
  }

  function translatorFor(namespace) {
    return function (key, params) {
      return translate(namespace || "", key, params);
    };
  }

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

  // DSH slot name -> shell region id. Exhaustive over the 122-slot reference
  // catalog (crates/cordis/src/slot-catalog.json) plus the two legacy dshr keys;
  // unknown names fall back to `panel` so older plugins keep rendering.
  const SLOT_REGIONS = Object.freeze({
    "contextMenu.content": "panel",
    "contextMenu.icon": "panel",
    "conversation.approval.detail": "approval.detail",
    "conversation.chat.assistant-actions": "chat.assistant-actions",
    "conversation.chat.commandview": "chat.commandview",
    "conversation.chat.flow": "chat.flow",
    "conversation.chat.markdown.fence": "chat.markdown.fence",
    "conversation.chat.message.images": "chat.images",
    "conversation.chat.node": "chat.node",
    "conversation.chat.reasoning.body": "chat.reasoning",
    "conversation.chat.text-interaction": "chat.text-interaction",
    "conversation.chat.turnTail": "chat.turnTail",
    "conversation.chat.user.before": "chat.user.before",
    "conversation.chat.user.images": "chat.user.images",
    "conversation.composer": "composer.chain",
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
    "conversation.input.model.pane": "composer.actions",
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
    "forge.change-request.card.actions": "panel",
    "forge.changes.file.menu": "panel",
    "forge.changes.toolbar": "panel",
    "forge.diff.comment.actions": "panel",
    "forge.diff.line.annotation": "panel",
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
  // Region change notifications: `ctx.slots.subscribe(region, listener)` and
  // `ctx.slots.getVersion(region)` are part of the reference slots surface used
  // by feature plugins that reconcile their own shadow registrations.
  const slotVersions = new Map();
  const slotSubscribers = new Set();

  function slotVersionNow(region) {
    return slotVersions.get(region) || 0;
  }

  function notifySlotChange(region) {
    slotVersions.set(region, slotVersionNow(region) + 1);
    slotSubscribers.forEach(function (subscriber) {
      if (subscriber.region !== "" && subscriber.region !== region) return;
      try {
        subscriber.listener(region);
      } catch (error) {
        fail(error);
      }
    });
  }

  function subscribeSlots(region, listener) {
    if (typeof region === "function") {
      listener = region;
      region = "";
    }
    if (typeof listener !== "function") return function () {};
    const subscriber = { region: region || "", listener: listener };
    slotSubscribers.add(subscriber);
    return function () {
      slotSubscribers.delete(subscriber);
    };
  }
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

  let registrationSeq = 0;

  // DSH SlotCore.register parity: an undeclared slot throws, cardinality
  // requirements are enforced, and a same-cell/same-priority second registration
  // throws (a different priority shadows instead).
  function normalize(entry, renderArgument) {
    if (!entry || typeof entry !== "object") throw new Error("slot entry must be an object");
    const render =
      typeof renderArgument === "function" || (renderArgument && typeof renderArgument === "object")
        ? renderArgument
        : entry.render;
    // React component types are functions, but React.memo/forwardRef wrap them in
    // an object carrying `$$typeof`; a host-React frame must accept those too.
    const reactType =
      render && typeof render === "object" && Boolean(render.$$typeof) && Boolean(reactRuntimeNow());
    if (typeof render !== "function" && !reactType) throw new Error("slot: render is required");
    const name = String(entry.name || entry.slot || "panel");
    const spec = slotSpec(name);
    const extension = isExtensionSlot(name);
    if (!spec && !extension) {
      throw new Error(
        'slot "' +
          name +
          '" is not declared in the DSH client slot catalog (register into a declared slot key)',
      );
    }
    const key = typeof entry.key === "string" ? entry.key : "";
    // Extension slots keep the dshr registration contract (a key means keyed,
    // otherwise a list entry); declared slots follow the reference rules.
    const kind = spec ? slotKindOf(name) : key !== "" ? "keyed" : "list";
    if (kind === "keyed" && key === "") {
      throw new Error('keyed slot "' + name + '" requires options.key');
    }
    const id = entry.id || (key ? name + ":" + key : name);
    if ((kind === "list" || kind === "hoc" || kind === "chain") && !spec && !entry.id) {
      // dshr extension convenience: extension keys may omit an explicit id.
    } else if (kind !== "single" && kind !== "keyed" && (typeof entry.id !== "string" || !entry.id) && !extension) {
      throw new Error('slot "' + name + '" requires options.id');
    }
    if (typeof id !== "string" || !id) throw new Error("slot entry needs a string id");
    const select = typeof entry.select === "function" ? entry.select : null;
    if (kind === "chain" && !select) {
      throw new Error('chain slot "' + name + '" requires options.select');
    }
    const priority = Number.isFinite(entry.priority) ? Number(entry.priority) : 0;
    const order = Number.isFinite(entry.order) ? Number(entry.order) : 0;
    // Conflict: same cell (single = the slot; keyed = key; others = id) at the
    // same priority. A different priority shadows (lowest renders).
    for (const occupant of slots.values()) {
      if (occupant.name !== name || occupant.priority !== priority) continue;
      const sameCell =
        kind === "single"
          ? true
          : kind === "keyed"
            ? occupant.key === key
            : occupant.id === id;
      if (sameCell) {
        throw new Error(
          (kind === "single" ? "single" : kind) +
            ' slot "' +
            name +
            '" already has a registration at priority ' +
            priority +
            (occupant.registrant ? " (registered by " + occupant.registrant + ")" : "") +
            " — register at a different priority to shadow it (lowest renders)",
        );
      }
    }
    const normalized = {
      name: name,
      id: id,
      key: key,
      kind: kind,
      scope: spec ? slotScopeOf(name) : "root",
      seq: ++registrationSeq,
      title: typeof entry.title === "string" && entry.title ? entry.title : id,
      order: order,
      priority: priority,
      locale: typeof entry.locale === "string" ? entry.locale : "",
      select: select,
      registrant: typeof entry.registrant === "string" ? entry.registrant : "",
      // DSH slot registrations may provide the component's extra props with
      // `inject: () => ({...})` (a framework seat); resolve it at mount time.
      inject: typeof entry.inject === "function" || (entry.inject && typeof entry.inject === "object") ? entry.inject : null,
      // Store seat declaration: a shared StoreHandle or a factory returning one.
      store: typeof entry.store === "function" || (entry.store && typeof entry.store === "object") ? entry.store : null,
      render: render,
      children: entry.children && typeof entry.children === "object" ? entry.children : null,
    };
    if (normalized.children) {
      try {
        declareChildSlots(normalized, normalized.children);
      } catch (error) {
        undeclareChildSlots(normalized);
        throw error;
      }
    }
    return normalized;
  }

  // Raw ledger order, mirroring SlotCore: priority ascending for every kind,
  // list/hoc refine equal priorities by explicit `order`, ties keep registration
  // order (stable sort).
  function dropEntry(entry) {
    undeclareChildSlots(entry);
    return slots.delete(entry.id);
  }

  // Run a slot-level inject factory now that the slot is declared.
  function injectNow(factory) {
    const before = new Set(slots.keys());
    let result;
    try {
      result = factory();
    } catch (error) {
      report("inject", error);
      return function () {};
    }
    const added = Array.from(slots.keys())
      .filter(function (id) {
        return !before.has(id);
      })
      .map(function (id) {
        return slots.get(id);
      })
      .filter(Boolean);
    const inner = cordisDisposer(result);
    const disposeGroup = function () {
      added.forEach(function (entry) {
        dropEntry(entry);
      });
      publish();
      if (inner) {
        try {
          inner();
        } catch (error) {
          fail(error);
        }
      }
    };
    disposers.push(disposeGroup);
    return disposeGroup;
  }

  function ordered(entries) {
    return entries
      .slice()
      .sort(function (left, right) {
        const byPriority = left.priority - right.priority;
        if (byPriority !== 0) return byPriority;
        if (left.kind === "list" || left.kind === "hoc") {
          const byOrder = left.order - right.order;
          if (byOrder !== 0) return byOrder;
        }
        return left.seq - right.seq;
      });
  }

  function sorted() {
    return ordered(Array.from(slots.values()));
  }

  // Projection to shadowing winners: single keeps the lowest-priority entry,
  // keyed keeps one winner per key, list/chain/hoc keep every layer (their
  // dispatch/nesting runs at render time in the reference; dshr mounts one
  // frame per winner and renders chain/hoc layers in ledger order).
  function winners() {
    const byName = new Map();
    for (const entry of sorted()) {
      const bucket = byName.get(entry.name);
      if (!bucket) {
        byName.set(entry.name, [entry]);
        continue;
      }
      bucket.push(entry);
    }
    const out = [];
    for (const [name, bucket] of byName) {
      const kind = slotKindOf(name);
      if (kind === "single") {
        out.push(bucket[0]);
      } else if (kind === "keyed") {
        const byKey = new Map();
        for (const entry of bucket) {
          if (!byKey.has(entry.key)) byKey.set(entry.key, entry);
        }
        out.push(...byKey.values());
      } else {
        out.push(...bucket);
      }
    }
    return ordered(out);
  }

  // One wire shape for registrations, so `slots` messages and `slots.list()`
  // agree and the shell can place every entry without recomputing the region.
  // `kind`/`priority`/`key` drive the shell's owner-renderer (chain election,
  // keyed occurrences) and `children` is the child-slot declaration, so the
  // shell mounts nested slots through their parent entry instead of flatly.
  function described(entry) {
    return {
      name: entry.name,
      slot: entry.name,
      id: entry.id,
      key: entry.key,
      kind: entry.kind,
      title: entry.title,
      order: entry.order,
      priority: entry.priority,
      region: slotRegion(entry.name),
      children: entry.children ? Object.keys(entry.children) : [],
    };
  }

  function publish() {
    const wireSlots = winners().map(described);
    post({
      type: "slots",
      slots: wireSlots,
    });
    // Reference subscriptions are keyed by slot name (not by the mount region).
    const names = new Set(
      wireSlots.map(function (entry) {
        return entry.name;
      }),
    );
    names.forEach(function (name) {
      notifySlotChange(name);
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
  let reactExtra = null;
  let cachedBoundary = null;
  let cachedAnchorComponent = null;

  // ---------------------------------------------------------------------------
  // Nested slot anchors (reference `renderSlot`/`renderSlotChain` children).
  // A parent entry cannot mount another plugin's frame inside its own document,
  // so it renders a placeholder anchor element, reports the anchor's box to the
  // shell, and the shell mounts the declared child slot frames absolutely over
  // it — the children nest inside the parent instead of being flattened into a
  // shell region.
  // ---------------------------------------------------------------------------
  let anchorSeq = 0;
  // Anchor elements by id: the shell reports the mounted child's height back so
  // the parent reserves that space in its own layout (nested slots are not just
  // absolutely positioned overlays).
  const anchorElements = new Map();

  function reportAnchor(anchor, element, slotKey, ownerProps, hostId) {
    if (!element || typeof element.getBoundingClientRect !== "function") return;
    const rect = element.getBoundingClientRect();
    post({
      type: "anchor",
      anchor: anchor,
      host: hostId,
      slot: slotKey,
      ownerProps: serializeOwnerProps(ownerProps),
      rect: {
        x: Math.round((Number(rect.x) || 0) * 100) / 100,
        y: Math.round((Number(rect.y) || 0) * 100) / 100,
        width: Math.round((Number(rect.width) || 0) * 100) / 100,
        height: Math.round((Number(rect.height) || 0) * 100) / 100,
      },
    });
  }

  function anchorComponent() {
    const runtime = reactRuntimeNow();
    if (!runtime) return null;
    if (cachedAnchorComponent) return cachedAnchorComponent;
    const React = runtime.React;
    cachedAnchorComponent = function DshrSlotAnchor(props) {
      const ref = React.useRef(null);
      React.useLayoutEffect(
        function () {
          const element = ref.current;
          if (!element) return undefined;
          anchorElements.set(props.anchor, element);
          reportAnchor(props.anchor, element, props.slotKey, props.ownerProps, props.hostId);
          let disconnect = null;
          if (typeof window.ResizeObserver === "function") {
            const observer = new window.ResizeObserver(function () {
              reportAnchor(props.anchor, element, props.slotKey, props.ownerProps, props.hostId);
            });
            observer.observe(element);
            disconnect = function () {
              observer.disconnect();
            };
          }
          return function () {
            if (disconnect) disconnect();
            anchorElements.delete(props.anchor);
            post({ type: "anchor-clear", anchor: props.anchor, host: props.hostId });
          };
        },
        [props.anchor],
      );
      // Owner props are re-projected on every render: a child seat sees the
      // parent's latest share (node data, selection, forwarded callbacks).
      React.useEffect(function () {
        reportAnchor(props.anchor, ref.current, props.slotKey, props.ownerProps, props.hostId);
      });
      return React.createElement("div", {
        className: "dshr-slot-anchor",
        "data-dshr-slot-anchor": props.slotKey,
        ref: ref,
      });
    };
    return cachedAnchorComponent;
  }

  // One binding per entry (reference `boundRenderSlot`): only slots the entry
  // declared in `children` may be rendered, and the binding dies with the entry
  // because the frame is torn down with it.
  function boundRenderSlot(entry) {
    return function renderSlot(key, owner) {
      const declared = entry.children && entry.children[key];
      if (!declared) {
        throw new Error('slot "' + key + '" is not declared by this entry\'s children');
      }
      const anchor = entry.id + "#" + key + "#" + ++anchorSeq;
      const runtime = reactRuntimeNow();
      if (runtime) {
        return runtime.React.createElement(anchorComponent(), {
          anchor: anchor,
          slotKey: key,
          ownerProps: owner || {},
          hostId: entry.id,
        });
      }
      const element = document.createElement("div");
      element.className = "dshr-slot-anchor";
      if (element.dataset) element.dataset.dshrSlotAnchor = key;
      anchorElements.set(anchor, element);
      reportAnchor(anchor, element, key, owner || {}, entry.id);
      return element;
    };
  }

  // Entry-stable bindings: a memoized parent must not rebuild its renderSlot
  // props on an unrelated host-state re-render (reference caches the same way).
  const renderSlotBindings = new WeakMap();

  function anchorProps(entry) {
    if (!entry.children) return {};
    let renderSlot = renderSlotBindings.get(entry);
    if (!renderSlot) {
      renderSlot = boundRenderSlot(entry);
      renderSlotBindings.set(entry, renderSlot);
    }
    // Reference: `renderSlotChain` exists for chain children; dshr renders the
    // same anchor and lets the shell run the chain election for the child key.
    return { renderSlot: renderSlot, renderSlotChain: renderSlot };
  }

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

  function mountReact(entry, props, extra) {
    const runtime = reactRuntimeNow();
    if (!runtime) return false;
    const React = runtime.React;
    if (!reactRoot) reactRoot = runtime.ReactDOM.createRoot(root);
    reactEntry = entry;
    reactProps = props;
    reactExtra = extra || {};
    reactRoot.render(
      React.createElement(
        slotBoundary(),
        null,
        React.createElement(
          entry.render,
          Object.assign(
            {},
            props,
            slotStandardProps(entry.name),
            injectedProps(entry),
            storeProps(entry),
            frameOwnerProps,
            reactExtra,
            anchorProps(entry),
            { t: translatorFor(entry.locale) },
          ),
        ),
      ),
    );
    return true;
  }

  function rerenderActive() {
    if (!reactRoot || !reactEntry || !reactProps) return;
    mountReact(reactEntry, reactProps, reactExtra);
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
      reactExtra = null;
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

  // Chain election (reference chain slots): `select(ownerProps)` returns the
  // matched owner — the first entry whose selector does not decline wins, and
  // its component receives `{...ownerProps, matched}`. The shell mounts one
  // frame per chain slot at a time (owner-renderer): a declining frame reports
  // `chain-unselected` and the shell advances to the next candidate, so several
  // matching registrations never render several frames.
  //
  // Compatibility: the round-102 dshr contract documented boolean selectors, so
  // `false`/`undefined` also decline here; reference selectors return `null` to
  // decline, which is the same path.
  function chainElection(entry) {
    if (entry.kind !== "chain" || typeof entry.select !== "function") {
      return { elected: true, matched: undefined };
    }
    try {
      const matched = entry.select(frameOwnerProps, { slot: entry.name, id: entry.id });
      if (matched === null || matched === undefined || matched === false) {
        return { elected: false, matched: matched };
      }
      return { elected: true, matched: matched };
    } catch (error) {
      report("select " + entry.id, error);
      return { elected: false, matched: null };
    }
  }

  async function mount(entry, props) {
    const election = chainElection(entry);
    if (!election.elected) {
      clear();
      root.dataset.dshrSlot = entry.id;
      active = null;
      post({ type: "chain-unselected", slot: entry.name, id: entry.id });
      return;
    }
    const electionProps = entry.kind === "chain" ? { matched: election.matched } : {};
    try {
      const reactPropsHost = Object.assign({}, props, {
        theme: theme,
        wide: hostState.wide,
        context: props && props.context !== undefined ? props.context : null,
      });
      if (mountReact(entry, reactPropsHost, electionProps)) {
        root.dataset.dshrSlot = entry.id;
        active = entry;
      } else {
        // `clear()` also resets `active`, so set the slot identity after it.
        clear();
        root.dataset.dshrSlot = entry.id;
        active = entry;
        await entry.render(
          root,
          Object.assign(
            {},
            props,
            injectedProps(entry),
            storeProps(entry),
            frameOwnerProps,
            electionProps,
            anchorProps(entry),
          ),
        );
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
    else if (data.type === "anchor-size") {
      // The shell mounted the child slot and reports its height: reserve it in
      // the parent's layout (the anchor element is the placeholder).
      const element = anchorElements.get(data.anchor);
      const height = Number(data.height);
      if (element && element.style && Number.isFinite(height) && height > 0) {
        element.style.height = height + "px";
      }
    }
    else if (data.type === "owner-props") {
      // Big owner shares (a whole chat node) cannot ride a URL; the shell sends
      // them once the frame is ready and the active slot re-renders with them.
      if (typeof data.ownerProps === "string" && data.ownerProps !== "") {
        try {
          const parsed = JSON.parse(data.ownerProps);
          if (parsed && typeof parsed === "object") {
            frameOwnerProps = materializeOwnerProps(parsed);
            rerenderActive();
          }
        } catch (error) {
          report("owner-props", error);
        }
      }
    }
    else if (data.type === "config-form-snapshot") {
      const settle = configFormWaiters.get(data.id);
      if (settle) {
        configFormWaiters.delete(data.id);
        settle(data);
      }
    }
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
    locale: localeService,
    configForms: configFormsService,
    // Fiber-style timers: registered in the frame so dispose() tears them down.
    interval: function (callback, milliseconds) {
      if (typeof callback !== "function") return function () {};
      const handle = setInterval(callback, Number(milliseconds) || 0);
      const stop = function () {
        clearInterval(handle);
      };
      disposers.push(stop);
      return stop;
    },
    timeout: function (callback, milliseconds) {
      if (typeof callback !== "function") return function () {};
      const handle = setTimeout(callback, Number(milliseconds) || 0);
      const stop = function () {
        clearTimeout(handle);
      };
      disposers.push(stop);
      return stop;
    },
    // Reference `ctx.get(service)` subset: `sessions` is populated from
    // host-state messages; anything else is unknown in the dshr bridge.
    get: function (name) {
      if (name === "sessions") return sessionsService;
      if (name === "slots") return slotsService;
      if (name === "locale") return localeService;
      if (name === "configForms") return configFormsService;
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
        const disposer = cordisDisposer(callback(ctx));
        return disposer || function () {};
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
          if (dropEntry(normalized)) publish();
        };
        disposers.push(unregister);
        return unregister;
      },
      // Reference-shaped: the factory's registrations are owned as one group
      // and can be torn down together.
      inject: function (key, factory) {
        if (typeof factory !== "function") return function () {};
        if (declaredSlots.has(key) || isExtensionSlot(key)) return injectNow(factory);
        // The slot exists only while its declaring entry is mounted (SlotCore):
        // defer until the declaration ledger receives the key.
        const waiter = { factory: factory, disposed: false, disposers: [] };
        const list = slotInjectionWaiters.get(key) || [];
        list.push(waiter);
        slotInjectionWaiters.set(key, list);
        const stop = function () {
          waiter.disposed = true;
          const current = slotInjectionWaiters.get(key);
          if (current) {
            const index = current.indexOf(waiter);
            if (index >= 0) current.splice(index, 1);
          }
          waiter.disposers
            .slice()
            .reverse()
            .forEach(function (dispose) {
              try {
                dispose();
              } catch (error) {
                fail(error);
              }
            });
        };
        disposers.push(stop);
        return stop;
      },
      remove: function (id) {
        const entry = slots.get(id);
        if (entry && dropEntry(entry)) publish();
      },
      list: function () {
        return sorted().map(described);
      },
      // Reference helpers used by feature plugins for reconciliation.
      subscribe: subscribeSlots,
      getVersion: function (region) {
        return slotVersionNow(region);
      },
      entriesOfSlot: function (region) {
        return slotsService.entries(region);
      },
      entries: function (key) {
        return slotsService.entries(key);
      },
      // Reference-callable slot declaration lookup (kind/scope/standard kit),
      // used by plugins that branch on the resident owner, e.g. mnemon's chain.
      spec: function (key) {
        const spec = slotSpec(key);
        if (!spec) return undefined;
        return {
          key: key,
          kind: spec.kind,
          scope: spec.scope,
          standardProps: slotStandardNames(key),
          inject: slotNeedsInject(key),
        };
      },
    },
    effect: function (factory) {
      const group = [];
      try {
        const cleanup = cordisDisposer(factory());
        if (cleanup) track(cleanup, group);
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
