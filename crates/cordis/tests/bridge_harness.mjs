// Bridge semantics harness: runs bridge.js in a stub DOM and asserts the
// protocol/lifecycle contract (registry mode, v1.2 frame config, region map,
// slot mode). Invoked by crates/cordis/tests/bridge.rs.
//
// Usage: node bridge_harness.mjs <path-to-bridge.js> <path-to-slot-catalog.json>
import fs from "node:fs";
import vm from "node:vm";

const bridgePath = process.argv[2];
const catalogPath = process.argv[3];
if (!bridgePath || !catalogPath) {
  console.error("usage: node bridge_harness.mjs <bridge.js> <slot-catalog.json>");
  process.exit(2);
}
const source = fs.readFileSync(bridgePath, "utf8");
const catalog = JSON.parse(fs.readFileSync(catalogPath, "utf8"));

let checks = 0;
const failures = [];
function check(label, condition, detail) {
  checks += 1;
  if (condition) {
    console.log(`ok   ${label}`);
  } else {
    console.log(`FAIL ${label}${detail === undefined ? "" : `: ${JSON.stringify(detail)}`}`);
    failures.push(label);
  }
}

// The 17 shell region ids (the fallback `panel` included).
const REGION_IDS = new Set([
  "panel",
  "settings.section",
  "settings.icon",
  "settings.action",
  "message.action",
  "sidebar.top",
  "sidebar.footer",
  "sidebar.session.action",
  "shell.leading",
  "shell.bottom",
  "shell.overlay",
  "conversation.header",
  "composer.actions",
  "plugins.detail",
  "plugins.add",
  "tool.card",
  "approval.detail",
]);

// Resolves multiple microtask generations (the bridge awaits render results).
const tick = () => new Promise((resolve) => setTimeout(resolve, 0));

function element() {
  const children = [];
  const node = {
    children,
    dataset: {},
    style: {},
    boundingHeight: 200,
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
      return { height: node.boundingHeight };
    },
    querySelector() {
      return null;
    },
  };
  return node;
}

// Builds one stub page (window + document) and runs the bridge inside it.
// `frameConfig` populates window.__DSHR_FRAME__; `search` is the URL fallback.
// setTimeout inside the bridge is captured so the 1.5 s slot timeout can be
// fired deterministically instead of sleeping.
function createPage(options = {}) {
  const posted = [];
  const listeners = new Map();
  const timers = [];
  const documentElement = element();
  const body = element();

  const window = {
    console,
    location: { search: options.search || "" },
    addEventListener(type, handler) {
      const handlers = listeners.get(type) || [];
      handlers.push(handler);
      listeners.set(type, handlers);
    },
  };
  if (options.plugin !== undefined) window.__DSHR_PLUGIN__ = options.plugin;
  if (options.frameConfig !== undefined) window.__DSHR_FRAME__ = options.frameConfig;
  if (options.topLevel) {
    window.parent = window; // top-level page: registry auto-activation runs
    window.postMessage = (message) => {
      posted.push(message);
    };
  } else {
    window.parent = {
      postMessage(message) {
        posted.push(message);
      },
    };
  }

  const document = {
    documentElement,
    body,
    createElement() {
      return element();
    },
  };
  window.document = document;

  const context = vm.createContext({
    window,
    document,
    parent: window.parent,
    location: window.location,
    console,
    URLSearchParams,
    Promise,
    JSON,
    Object,
    Array,
    String,
    Number,
    Boolean,
    Math,
    Date,
    Error,
    setTimeout(fn, delay) {
      timers.push({ fn, delay, fired: false });
      return timers.length;
    },
    clearTimeout(id) {
      if (timers[id - 1]) timers[id - 1].cancelled = true;
    },
  });
  vm.runInContext(source, context, { filename: "bridge.js" });

  const root = body.children[0];
  return {
    window,
    document,
    documentElement,
    root,
    posted,
    timers,
    get dshr() {
      return window.dshr;
    },
    deliver(data) {
      for (const handler of listeners.get("message") || []) {
        handler({ source: window.parent, data });
      }
    },
    dispatch(type) {
      for (const handler of listeners.get(type) || []) handler({});
    },
    messages(type) {
      return posted.filter((message) => message.type === type);
    },
    fireTimer(delay) {
      const timer = timers.find((candidate) => candidate.delay === delay && !candidate.fired);
      if (!timer) return false;
      timer.fired = true;
      timer.fn();
      return true;
    },
  };
}

function checkEnvelope(label, page, frame, mode) {
  check(
    `${label}: every message is protocol v1 in the dshr-client namespace`,
    page.posted.length > 0 &&
      page.posted.every((message) => message.v === 1 && message.ns === "dshr-client"),
    page.posted,
  );
  check(
    `${label}: every message carries the frame token`,
    page.posted.every((message) => message.frame === frame),
    page.posted.map((message) => message.frame),
  );
  check(
    `${label}: every message carries the frame mode`,
    page.posted.every((message) => message.mode === mode),
    page.posted.map((message) => message.mode),
  );
}

async function registryScenario() {
  const page = createPage({
    topLevel: true,
    frameConfig: {
      plugin: "harness",
      frame: "registry:harness",
      mode: "registry",
      theme: "dark",
    },
  });
  const dshr = page.dshr;
  check("bridge exposes the context", Boolean(dshr));
  check("plugin name comes from the host page", dshr.name === "harness", dshr.name);
  check(
    "the frame config supplies the frame token",
    Boolean(dshr.SLOT_REGIONS) && Object.isFrozen(dshr.SLOT_REGIONS),
    dshr.SLOT_REGIONS,
  );

  // Region lookup (protocol v1.2): spec samples plus the unknown fallback.
  check("slotRegion maps sidebar.footer.action", dshr.slotRegion("sidebar.footer.action") === "sidebar.footer", dshr.slotRegion("sidebar.footer.action"));
  check("slotRegion maps settings.section.icon", dshr.slotRegion("settings.section.icon") === "settings.icon", dshr.slotRegion("settings.section.icon"));
  check("slotRegion maps shell.overlay", dshr.slotRegion("shell.overlay") === "shell.overlay", dshr.slotRegion("shell.overlay"));
  check("slotRegion maps the legacy message.action key", dshr.slotRegion("message.action") === "message.action", dshr.slotRegion("message.action"));
  check("slotRegion maps the legacy panel key", dshr.slotRegion("panel") === "panel", dshr.slotRegion("panel"));
  check("slotRegion falls back to panel for unknown keys", dshr.slotRegion("custom.key") === "panel", dshr.slotRegion("custom.key"));

  // Every catalog key has an explicit region; unknown keys are not in the map.
  const missingRegions = catalog.keys.filter((key) => !REGION_IDS.has(dshr.slotRegion(key)));
  check(
    "every slot-catalog key maps to one of the 17 regions",
    catalog.keys.length === 116 && missingRegions.length === 0,
    missingRegions,
  );
  const mapKeys = Object.keys(dshr.SLOT_REGIONS);
  check(
    "the region map covers the 116 catalog keys plus the 2 legacy keys",
    mapKeys.length === catalog.keys.length + 2 &&
      catalog.keys.every((key) => Object.prototype.hasOwnProperty.call(dshr.SLOT_REGIONS, key)),
    mapKeys.length,
  );
  check(
    "the region map only uses the 17 shell region ids",
    Object.values(dshr.SLOT_REGIONS).every((region) => REGION_IDS.has(region)) &&
      new Set(Object.values(dshr.SLOT_REGIONS)).size === REGION_IDS.size,
    [...new Set(Object.values(dshr.SLOT_REGIONS))],
  );

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
  check(
    "slot entries carry their region",
    dshr.slots.list().every((slot) => typeof slot.region === "string") &&
      dshr.slots.list().find((slot) => slot.id === "settings").region === "settings.section",
    dshr.slots.list(),
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

  dshr.slots.register({ name: "panel", id: "main", title: "Main", order: 0 }, render);
  page.deliver({ v: 1, ns: "dshr-shell", type: "activate", slot: "panel", id: "main" });
  await tick();
  check(
    "activate renders the registered slot",
    page.root.textContent === "rendered:panel:main:dark",
    page.root.textContent,
  );

  const slotsMessage = page.messages("slots").pop();
  check(
    "slots messages carry name/order/title/region",
    Boolean(slotsMessage) &&
      slotsMessage.slots[0].order === 0 &&
      slotsMessage.slots[0].region === "panel" &&
      slotsMessage.slots[0].name === "panel" &&
      slotsMessage.slots[0].slot === "panel",
    slotsMessage,
  );

  page.deliver({ v: 1, ns: "dshr-shell", type: "deactivate" });
  check("deactivate clears the rendered view", page.root.textContent === "", page.root.textContent);
  check("deactivate keeps registrations", dshr.slots.list().length === 1, dshr.slots.list());

  page.deliver({ v: 1, ns: "dshr-shell", type: "theme", theme: "light" });
  check("theme message updates the surface", page.documentElement.dataset.theme === "light", page.documentElement.dataset.theme);
  check("theme listeners receive the new value", themes[themes.length - 1] === "light", themes);

  const pending = dshr.action("reload", { plugin: "harness" });
  const action = page.messages("action").pop();
  check("ctx.action posts the action with an id", Boolean(action && action.id), action);
  page.deliver({ v: 1, ns: "dshr-shell", type: "action-result", id: action.id, ok: true });
  let settled = null;
  pending.then((result) => {
    settled = result;
  });

  // The host page signals that the bundle activation settled; only then does
  // the bridge report ready (protocol v1.2).
  check("ready is not posted before the activation settles", page.messages("ready").length === 0, page.messages("ready"));
  dshr.__frameActivated();
  dshr.__frameActivated();
  await tick();
  const ready = page.messages("ready").pop();
  check("ready arrives after activation settles", Boolean(ready), page.messages("ready"));
  check("ready is posted once", page.messages("ready").length === 1, page.messages("ready"));
  check(
    "ready carries the frame token and mode",
    Boolean(ready) && ready.frame === "registry:harness" && ready.mode === "registry" && ready.plugin === "harness",
    ready,
  );

  // Re-activate so the resize assertions below have a rendered frame.
  page.deliver({ v: 1, ns: "dshr-shell", type: "activate", slot: "panel", id: "main" });
  await tick();
  const resize = page.messages("resize").pop();
  check("activate reports the measured height", Boolean(resize) && resize.height === 200, resize);
  check(
    "resize messages carry slot/id/region",
    Boolean(resize) &&
      resize.slot === "panel" &&
      resize.id === "main" &&
      resize.region === "panel",
    resize,
  );
  check("ctx.action resolves with the shell result", settled && settled.ok === true, settled);

  // The rendered slot re-reports its height when the shell window resizes.
  page.root.boundingHeight = 321;
  page.dispatch("resize");
  const resized = page.messages("resize").pop();
  check("the frame re-reports its height on window resize", Boolean(resized) && resized.height === 321, resized);

  checkEnvelope("registry mode", page, "registry:harness", "registry");

  // C-lite host services: `ctx.get('sessions')` is populated by host-state
  // messages from the shell; the reference `sessions` surface is a live view.
  page.deliver({
    v: 1,
    ns: "dshr-shell",
    type: "host-state",
    state: {
      theme: "light",
      wide: false,
      session: { id: "s1", title: "Session one" },
      sessions: [{ id: "s1" }, { id: "s2" }],
    },
  });
  const sessions = dshr.get("sessions");
  check("ctx.get exposes the sessions service", Boolean(sessions), sessions);
  check(
    "the sessions snapshot follows host-state",
    sessions.currentProvideInfo.getSnapshot().id === "s1",
    sessions.currentProvideInfo.getSnapshot(),
  );
  check(
    "the sessions list follows host-state",
    sessions.list().length === 2 && sessions.get("s2").id === "s2",
    sessions.list(),
  );
  check("unknown services stay undefined", dshr.get("nope") === undefined, dshr.get("nope"));

  // Reference service shims used by real plugins: slots alias, locale, inject.
  const slotsAlias = dshr.get("slots");
  check(
    "ctx.get('slots') exposes the registry alias",
    Boolean(slotsAlias) && typeof slotsAlias.inject === "function",
    slotsAlias,
  );
  slotsAlias.register({ name: "panel", id: "alias-entry", title: "Alias" }, render);
  check(
    "ctx.get('slots').register publishes a slot",
    dshr.slots.list().some((slot) => slot.id === "alias-entry"),
    dshr.slots.list(),
  );
  check(
    "ctx.get('slots').entries filters by slot name",
    slotsAlias.entries("panel").some((entry) => entry.id === "alias-entry"),
    slotsAlias.entries("panel"),
  );
  const locale = dshr.get("locale");
  check(
    "ctx.get('locale') returns the English snapshot",
    Boolean(locale) && locale.getSnapshot().active === "en",
    locale && locale.getSnapshot(),
  );
  let injected = 0;
  let injectedInner = null;
  dshr.inject(["sessions"], (inner) => {
    injected += 1;
    injectedInner = inner;
  });
  dshr.inject(["missing-service"], () => {
    injected += 1;
  });
  check(
    "ctx.inject gates on available services",
    injected === 1 && injectedInner === dshr,
    { injected: injected, inner: injectedInner === dshr },
  );
  // Reference slots surface: subscribe/getVersion/entriesOfSlot + locale.bind.
  let slotChanges = 0;
  const stopSlots = dshr.slots.subscribe("sidebar.footer.action", () => {
    slotChanges += 1;
  });
  const versionBefore = dshr.slots.getVersion("sidebar.footer.action");
  dshr.slots.register(
    { name: "sidebar.footer.action", id: "surface-entry", title: "Surface" },
    render,
  );
  stopSlots();
  check(
    "ctx.slots.subscribe fires for the registered region",
    slotChanges > 0 && dshr.slots.getVersion("sidebar.footer.action") > versionBefore,
    { changes: slotChanges, version: dshr.slots.getVersion("sidebar.footer.action") },
  );
  check(
    "ctx.slots.entriesOfSlot returns reference-shaped entries",
    dshr.slots
      .entriesOfSlot("sidebar.footer.action")
      .some((entry) => entry.id === "surface-entry" && Boolean(entry.options)),
    dshr.slots.entriesOfSlot("sidebar.footer.action"),
  );
  dshr.locale.register("probe", { en: { hi: "Hi {name}" } });
  check(
    "ctx.locale.bind returns a working t()",
    dshr.locale.bind("probe")("hi", { name: "dshr" }) === "Hi dshr",
    dshr.locale.bind("probe")("hi", { name: "dshr" }),
  );

  dshr.__fail(new Error("diagnostic probe"));
  check(
    "__DSHR_LAST_ERROR__ records the failure for diagnostics",
    page.window.__DSHR_LAST_ERROR__.message === "diagnostic probe",
    page.window.__DSHR_LAST_ERROR__,
  );

  // A shell reload bumps every frame version at once; the old document is torn
  // down while its module import is in flight and Chrome rejects that import.
  // The rejection must not be painted in the shell as a plugin error.
  const errorsBeforeUnload = page.messages("error").length;
  page.dispatch("pagehide");
  dshr.__fail(new Error("Failed to resolve module specifier ''"));
  check(
    "a navigating frame does not report the in-flight import rejection",
    page.messages("error").length === errorsBeforeUnload,
    page.messages("error").slice(-2),
  );

  dshr.dispose();
  check("dispose clears every slot", dshr.slots.list().length === 0, dshr.slots.list());
  check("dispose runs effect cleanup", effectCleanup === 1, effectCleanup);
  check("dispose clears the rendered view", page.root.textContent === "", page.root.textContent);
}

// C-lite React mode: when the host module table provides React, registrations
// are React components (including React.memo/forwardRef types), receive host
// props/hooks and a `t()` seat for their locale namespace.
async function reactScenario() {
  const React = {
    createElement(type, props) {
      const children = Array.prototype.slice.call(arguments, 2);
      return {
        type,
        props: Object.assign({}, props || {}, children.length ? { children: children.length === 1 ? children[0] : children } : {}),
      };
    },
    memo(type) {
      return { $$typeof: Symbol.for("react.memo"), type };
    },
    forwardRef(render) {
      return { $$typeof: Symbol.for("react.forward_ref"), render };
    },
    useState(initial) {
      return [typeof initial === "function" ? initial() : initial, function () {}];
    },
    useEffect() {},
    Component: class Component {
      constructor(props) {
        this.props = props;
      }
    },
    Fragment: Symbol("Fragment"),
  };
  const ReactDOM = {
    createRoot(container) {
      return {
        render(element) {
          container.__reactElement = element;
        },
        unmount() {
          container.__reactElement = null;
        },
      };
    },
  };
  const MemoSlot = React.memo(function MemoSlot() {
    return null;
  });
  const page = createPage({
    frameConfig: {
      plugin: "harness",
      frame: "slot:harness:react:0",
      mode: "slot",
      slot: "sidebar.footer.action",
      id: "memo",
      theme: "dark",
    },
  });
  page.window.__DSHR_MODULES__ = { react: React, "react-dom/client": ReactDOM };
  const dshr = page.dshr;
  dshr.locale.register("harness", { en: { greet: "Hello {name}" } });
  dshr.slots.register(
    {
      name: "sidebar.footer.action",
      id: "memo",
      title: "Memo",
      locale: "harness",
      inject: () => ({ seatValue: "seat" }),
    },
    MemoSlot,
  );
  dshr.__frameActivated();
  await tick();

  const tree = page.root.__reactElement;
  const child = tree && tree.props && tree.props.children;
  check(
    "host React mode accepts a React.memo registration",
    Boolean(child) && child.type === MemoSlot,
    child && typeof child.type,
  );
  check(
    "React slot components receive reference-style host props",
    Boolean(child) &&
      child.props.wide === true &&
      typeof child.props.useTheme === "function" &&
      typeof child.props.useSession === "function",
    child && Object.keys(child.props),
  );
  check(
    "React slot components get a working t() seat for their locale",
    Boolean(child) && child.props.t("greet", { name: "dshr" }) === "Hello dshr",
    child && typeof child.props.t === "function" && child.props.t("greet", { name: "dshr" }),
  );

  check(
    "slot-level inject supplies seat props",
    Boolean(child) && child.props.seatValue === "seat",
    child && child.props.seatValue,
  );

  // Cordis generator factories: yielded disposers are registered and run.
  let generatedDisposed = 0;
  const generatorDisposer = dshr.slots.inject("panel", function* () {
    yield dshr.slots.register({ name: "panel", id: "generated" }, function Generated() {
      return null;
    });
    yield function () {
      generatedDisposed += 1;
    };
  });
  const generatedIds = dshr.slots.list().map((slot) => slot.id);
  generatorDisposer();
  check(
    "generator slots.inject registers and disposes registrations",
    generatedIds.includes("generated") && generatedDisposed === 1,
    { generatedIds, generatedDisposed },
  );

  // Cordis effect may also be a generator.
  let effectDisposed = 0;
  const effectDisposer = dshr.effect(function* () {
    yield function () {
      effectDisposed += 1;
    };
  });
  effectDisposer();
  check(
    "generator effect runs yielded cleanups on dispose",
    effectDisposed === 1,
    { effectDisposed },
  );

  const component = function KeyedSlot() {
    return null;
  };
  dshr.slots.register({ name: "panel", key: "a" }, component);
  dshr.slots.register({ name: "panel", key: "b" }, component);
  const ids = dshr.slots.list().map((slot) => slot.id);
  check(
    "keyed registrations without an id get distinct ids",
    ids.includes("panel:a") && ids.includes("panel:b"),
    ids,
  );
  const entries = dshr.get("slots").entries("panel");
  check(
    "ctx.get('slots').entries exposes component and options for reference code",
    entries.length >= 2 &&
      entries.every((entry) => typeof entry.component === "function" && Boolean(entry.options)),
    entries.map((entry) => entry.id),
  );
  dshr.dispose();
}

function urlFallbackScenario() {
  const page = createPage({
    topLevel: true,
    search: "?plugin=url-plugin&frame=url:frame&mode=registry&theme=light",
  });
  const dshr = page.dshr;
  check(
    "standalone URL fallback names the plugin when the page has no __DSHR_PLUGIN__",
    dshr.name === "url-plugin",
    dshr.name,
  );
  check("standalone URL fallback applies the theme", page.documentElement.dataset.theme === "light", page.documentElement.dataset.theme);
  dshr.__frameActivated();
  const ready = page.messages("ready").pop();
  check(
    "standalone URL fallback frame/mode reach the ready message",
    Boolean(ready) && ready.frame === "url:frame" && ready.mode === "registry",
    ready,
  );
}

async function slotAutoRenderScenario() {
  const page = createPage({
    frameConfig: {
      plugin: "harness",
      frame: "slot:harness:footer:0",
      mode: "slot",
      slot: "sidebar.footer.action",
      id: "footer-action",
      context: '{"index":2}',
      theme: "light",
    },
  });
  const dshr = page.dshr;
  const renders = [];
  dshr.slots.register({ name: "conversation.composer", id: "composer" }, (container) => {
    container.textContent = "composer";
  });
  dshr.slots.register(
    { name: "sidebar.footer.action", id: "footer-action", title: "Footer" },
    (container, props) => {
      renders.push(props);
      container.textContent = `rendered:${props.slot}:${props.id}:${props.theme}`;
    },
  );

  dshr.__frameActivated();
  await tick();
  check(
    "slot mode auto-renders the requested registration",
    page.root.textContent === "rendered:sidebar.footer.action:footer-action:light",
    page.root.textContent,
  );
  check(
    "slot render props carry the parsed context and plugin",
    renders.length === 1 &&
      JSON.stringify(renders[0].context) === '{"index":2}' &&
      renders[0].plugin === "harness" &&
      renders[0].theme === "light",
    renders,
  );
  const resize = page.messages("resize").pop();
  check(
    "slot mode reports a resize for the rendered slot",
    Boolean(resize) &&
      resize.slot === "sidebar.footer.action" &&
      resize.id === "footer-action" &&
      resize.region === "sidebar.footer" &&
      resize.height === 200,
    resize,
  );
  check("slot mode still publishes its registrations", (page.messages("slots").pop() || { slots: [] }).slots.length === 2, page.messages("slots"));

  const themes = [];
  dshr.onTheme((theme) => themes.push(theme));
  page.deliver({ v: 1, ns: "dshr-shell", type: "theme", theme: "dark" });
  check("slot mode applies the theme message", page.documentElement.dataset.theme === "dark", page.documentElement.dataset.theme);
  check("slot mode theme listeners receive the new value", themes[themes.length - 1] === "dark", themes);
  check(
    "slot mode does not re-render on a theme message",
    page.root.textContent === "rendered:sidebar.footer.action:footer-action:light",
    page.root.textContent,
  );

  page.root.boundingHeight = 310;
  page.dispatch("resize");
  const resized = page.messages("resize").pop();
  check("slot frame re-reports its height on window resize", Boolean(resized) && resized.height === 310, resized);

  page.deliver({ v: 1, ns: "dshr-shell", type: "deactivate" });
  check("deactivate clears the rendered slot view", page.root.textContent === "", page.root.textContent);

  checkEnvelope("slot auto-render", page, "slot:harness:footer:0", "slot");
}

async function slotByNameScenario() {
  const page = createPage({
    frameConfig: {
      plugin: "harness",
      frame: "slot:harness:name:0",
      mode: "slot",
      slot: "conversation.composer",
      id: "missing-registration-id",
    },
  });
  page.dshr.slots.register({ name: "conversation.composer", id: "actual-id" }, (container, props) => {
    container.textContent = props.id;
  });
  page.dshr.__frameActivated();
  await tick();
  check(
    "slot mode falls back to matching the slot name when the id is unknown",
    page.root.textContent === "actual-id",
    page.root.textContent,
  );
}

async function slotLateRegistrationScenario() {
  const page = createPage({
    frameConfig: {
      plugin: "harness",
      frame: "slot:harness:late:0",
      mode: "slot",
      slot: "conversation.composer",
      id: "late-id",
    },
  });
  page.dshr.__frameActivated();
  await tick();
  check("slot mode waits when the registration is not there yet", page.root.textContent === "", page.root.textContent);
  page.dshr.slots.register({ name: "conversation.composer", id: "late-id" }, (container) => {
    container.textContent = "late";
  });
  await tick();
  check("slot mode renders a late registration before the timeout", page.root.textContent === "late", page.root.textContent);
  page.fireTimer(1500);
  check("a rendered slot does not report an error when the timeout fires", page.messages("error").length === 0, page.messages("error"));
}

async function slotInvalidContextScenario() {
  const page = createPage({
    frameConfig: {
      plugin: "harness",
      frame: "slot:harness:context:0",
      mode: "slot",
      slot: "conversation.composer",
      id: "context-id",
      context: "not json",
    },
  });
  const renders = [];
  page.dshr.slots.register({ name: "conversation.composer", id: "context-id" }, (container, props) => {
    renders.push(props);
    container.textContent = "context";
  });
  page.dshr.__frameActivated();
  await tick();
  check(
    "invalid frame context is reported in a log message",
    page.messages("log").some((message) => message.message.includes("invalid frame context")),
    page.messages("log"),
  );
  check("invalid frame context renders with null props", renders.length === 1 && renders[0].context === null, renders);
}

function slotMissingScenario() {
  const page = createPage({
    frameConfig: {
      plugin: "harness",
      frame: "slot:harness:missing:0",
      mode: "slot",
      slot: "missing.slot",
      id: "missing-id",
    },
  });
  page.dshr.__frameActivated();
  check("slot mode does not error before the registration timeout", page.messages("error").length === 0, page.messages("error"));
  const timer = page.timers.find((candidate) => candidate.delay === 1500);
  check("slot mode waits 1.5 s for a late registration", Boolean(timer), page.timers.map((candidate) => candidate.delay));
  page.fireTimer(1500);
  const error = page.messages("error").pop();
  check(
    "slot mode reports an unregistered slot",
    Boolean(error) && error.message === "slot 'missing.slot' (missing-id) was not registered",
    error,
  );
  checkEnvelope("slot missing", page, "slot:harness:missing:0", "slot");
}

async function main() {
  await registryScenario();
  await reactScenario();
  urlFallbackScenario();
  await slotAutoRenderScenario();
  await slotByNameScenario();
  await slotLateRegistrationScenario();
  await slotInvalidContextScenario();
  slotMissingScenario();
}

try {
  await main();
} catch (error) {
  console.error(error);
  failures.push("harness threw");
}

if (failures.length > 0) {
  console.log(`${failures.length} bridge checks failed`);
  process.exit(1);
}
console.log(`bridge harness: ${checks} checks, all checks passed`);
