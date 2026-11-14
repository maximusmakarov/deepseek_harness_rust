// Bridge semantics harness: runs bridge.js in a stub DOM and asserts the
// protocol/lifecycle contract. Invoked by crates/cordis/tests/bridge.rs.
//
// Usage: node bridge_harness.mjs <path-to-bridge.js>
import fs from "node:fs";
import vm from "node:vm";

const bridgePath = process.argv[2];
if (!bridgePath) {
  console.error("usage: node bridge_harness.mjs <bridge.js>");
  process.exit(2);
}
const source = fs.readFileSync(bridgePath, "utf8");

const failures = [];
function check(label, condition, detail) {
  if (condition) {
    console.log(`ok   ${label}`);
  } else {
    console.log(`FAIL ${label}${detail === undefined ? "" : `: ${JSON.stringify(detail)}`}`);
    failures.push(label);
  }
}

function element() {
  const children = [];
  const node = {
    dataset: {},
    style: {},
    get textContent() {
      return children.map((child) => child.textContent || "").join("");
    },
    set textContent(value) {
      children.length = 0;
      node.firstChild = null;
      const text = String(value);
      if (text !== "") {
        const textNode = { nodeType: 3, textContent: text };
        children.push(textNode);
        node.firstChild = textNode;
      }
    },
    innerHTML: "",
    firstChild: null,
    appendChild(child) {
      children.push(child);
      node.firstChild = children[0];
      return child;
    },
    removeChild(child) {
      const index = children.indexOf(child);
      if (index >= 0) children.splice(index, 1);
      node.firstChild = children[0] || null;
      return child;
    },
    setAttribute(name, value) {
      node[name] = value;
    },
    removeAttribute(name) {
      delete node[name];
    },
    getBoundingClientRect() {
      return { height: 200 };
    },
    querySelector() {
      return null;
    },
  };
  return node;
}

const posted = [];
const listeners = [];
const root = element();
const documentElement = element();

const window = {
  parent: null,
  console,
  location: { search: "?plugin=harness&theme=dark" },
  addEventListener(type, handler) {
    if (type === "message") listeners.push(handler);
  },
  __DSHR_PLUGIN__: "harness",
};
window.parent = window; // top-level page: auto-activation runs render once

const document = {
  documentElement,
  body: element(),
  createElement() {
    return root;
  },
};
window.document = document;
window.parent.postMessage = (message) => {
  posted.push(message);
};

const context = vm.createContext({
  window,
  document,
  parent: window.parent,
  location: window.location,
  console,
  URLSearchParams,
  Promise,
  setTimeout,
  clearTimeout,
  setInterval,
  clearInterval,
  JSON,
  Object,
  Array,
  String,
  Number,
  Boolean,
  Math,
  Date,
  Error,
});

vm.runInContext(source, context, { filename: "bridge.js" });

const dshr = window.dshr;
check("bridge exposes the context", Boolean(dshr));
check("plugin name comes from the host page", dshr.name === "harness", dshr.name);

const render = (container, props) => {
  container.textContent = `rendered:${props.slot}:${props.id}:${props.theme}`;
};

let injectedCleanup = 0;
const group = dshr.slots.inject("owner", () => {
  dshr.slots.register({ name: "settings.section", id: "settings", title: "Settings", order: 20 }, render);
  dshr.slots.register({ name: "panel", id: "first", title: "First", order: 10 }, render);
  return () => {
    injectedCleanup += 1;
  };
});
check("slots.inject registers both slots", dshr.slots.list().length === 2, dshr.slots.list());
check(
  "slot list is ordered by order then id",
  JSON.stringify(dshr.slots.list().map((slot) => slot.id)) === JSON.stringify(["first", "settings"]),
  dshr.slots.list(),
);
check(
  "slots are reported with the reference field name",
  dshr.slots.list()[0].name === "panel" && dshr.slots.list()[0].slot === "panel",
  dshr.slots.list()[0],
);
group();
check("inject disposer unregisters the group", dshr.slots.list().length === 0, dshr.slots.list());
check("inject disposer runs the factory cleanup", injectedCleanup === 1, injectedCleanup);

let effects = 0;
let effectCleanup = 0;
dshr.effect(() => {
  effects += 1;
  return () => {
    effectCleanup += 1;
  };
});
check("ctx.effect runs its factory once", effects === 1, effects);

const themes = [];
dshr.onTheme((theme) => themes.push(theme));
check("onTheme is called immediately with the current theme", themes[0] === "dark", themes);

function deliver(data) {
  for (const handler of listeners) handler({ source: window.parent, data });
}

dshr.slots.register({ name: "panel", id: "main", title: "Main", order: 0 }, render);
deliver({ v: 1, ns: "dshr-shell", type: "activate", slot: "panel", id: "main" });
check(
  "activate renders the registered slot",
  root.textContent === "rendered:panel:main:dark",
  root.textContent,
);

deliver({ v: 1, ns: "dshr-shell", type: "deactivate" });
check("deactivate clears the rendered view", root.textContent === "", root.textContent);
check("deactivate keeps registrations", dshr.slots.list().length === 1, dshr.slots.list());

deliver({ v: 1, ns: "dshr-shell", type: "theme", theme: "light" });
check("theme message updates the surface", documentElement.dataset.theme === "light", documentElement.dataset.theme);
check("theme listeners receive the new value", themes[themes.length - 1] === "light", themes);

const pending = dshr.action("reload", { plugin: "harness" });
const action = posted.filter((message) => message.type === "action").pop();
check("ctx.action posts the action with an id", Boolean(action && action.id), action);
deliver({ v: 1, ns: "dshr-shell", type: "action-result", id: action.id, ok: true });
let settled = null;
pending.then((result) => {
  settled = result;
});

const slotsMessage = posted.filter((message) => message.type === "slots").pop();
check("slots messages carry name/order/title", Boolean(slotsMessage && slotsMessage.slots[0].order === 0), slotsMessage);

setTimeout(() => {
  const resize = posted.filter((message) => message.type === "resize").pop();
  check("activate reports the measured height", Boolean(resize) && resize.height === 200, resize);
  check("ctx.action resolves with the shell result", settled && settled.ok === true, settled);
  dshr.dispose();
  check("dispose clears every slot", dshr.slots.list().length === 0, dshr.slots.list());
  check("dispose runs effect cleanup", effectCleanup === 1, effectCleanup);
  if (failures.length > 0) {
    console.log(`${failures.length} bridge checks failed`);
    process.exit(1);
  }
  console.log(`bridge harness: ${posted.length} messages, all checks passed`);
}, 10);
