/* dshr shared host runtime (tier B PoC): the module table every opted-in
 * client-half frame gets before its bundle. React/ReactDOM UMD builds are
 * concatenated above this file by the server route; this glue publishes the
 * reference module ids in window.__DSHR_MODULES__, including a jsx-runtime shim
 * built on React.createElement (the UMD build ships no jsx-runtime module).
 * A plugin-shipped dsh.client.runtime runs afterwards and may override entries. */
(function () {
  var React = window.React;
  var ReactDOM = window.ReactDOM;
  if (!React || !ReactDOM) {
    throw new Error("dshr shared runtime: React UMD did not publish window.React/window.ReactDOM");
  }
  function jsx(type, props, key) {
    var next = {};
    if (props) {
      for (var name in props) next[name] = props[name];
    }
    if (key !== undefined) next.key = key;
    return React.createElement(type, next);
  }
  var jsxRuntime = { jsx: jsx, jsxs: jsx, Fragment: React.Fragment };
  // @deepseek-ai/dsh-client-store subset: framework-free snapshot stores and the
  // defineStore shell used by slot store seats (reference packages/client/store).
  // zustand/immer semantics are reproduced with a plain deep-copied draft; the
  // persist key is accepted but not persisted (documented limitation).
  function shallowEqual(a, b) {
    if (Object.is(a, b)) return true;
    if (typeof a !== "object" || typeof b !== "object" || a === null || b === null) return false;
    var aKeys = Object.keys(a);
    var bKeys = Object.keys(b);
    if (aKeys.length !== bKeys.length) return false;
    return aKeys.every(function (key) {
      return Object.prototype.hasOwnProperty.call(b, key) && Object.is(a[key], b[key]);
    });
  }

  function createSnapshotStore(initial) {
    var state = initial;
    var listeners = new Set();
    return {
      getSnapshot: function () {
        return state;
      },
      subscribe: function (listener) {
        if (typeof listener !== "function") return function () {};
        listeners.add(listener);
        return function () {
          listeners.delete(listener);
        };
      },
      set: function (next) {
        state = next;
        listeners.forEach(function (listener) {
          try {
            listener();
          } catch (error) {
            console.error("[dshr-store] subscriber failed:", error);
          }
        });
      },
      update: function (mutator) {
        var draft;
        try {
          draft = JSON.parse(JSON.stringify(state));
        } catch (error) {
          draft = Object.assign({}, state);
        }
        mutator(draft);
        state = draft;
        listeners.forEach(function (listener) {
          try {
            listener();
          } catch (error) {
            console.error("[dshr-store] subscriber failed:", error);
          }
        });
      },
    };
  }

  function defineStore(spec) {
    var shared = new Map();
    function create(scopeKey) {
      var cacheKey = scopeKey === undefined ? "root" : String(scopeKey);
      if (shared.has(cacheKey)) return shared.get(cacheKey);
      var store = createSnapshotStore(spec.init());
      var actions = {};
      Object.keys(spec.actions || {}).forEach(function (name) {
        actions[name] = function () {
          var args = Array.prototype.slice.call(arguments);
          store.update(function (draft) {
            spec.actions[name].apply(null, [draft].concat(args));
          });
        };
      });
      var instance = {
        actions: actions,
        getSnapshot: store.getSnapshot,
        subscribe: store.subscribe,
        clearPersisted: function () {},
      };
      shared.set(cacheKey, instance);
      return instance;
    }
    return { spec: spec, create: create };
  }

  window.__DSHR_MODULES__ = Object.assign(window.__DSHR_MODULES__ || {}, {
    "react": React,
    "react/jsx-runtime": jsxRuntime,
    "react/jsx-dev-runtime": { jsx: jsx, jsxs: jsx, jsxDEV: jsx, Fragment: React.Fragment },
    "react-dom": ReactDOM,
    "react-dom/client": ReactDOM,
    "@deepseek-ai/dsh-client-store": {
      shallowEqual: shallowEqual,
      createSnapshotStore: createSnapshotStore,
      defineStore: defineStore,
    },
  });
})();
