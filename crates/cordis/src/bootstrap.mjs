// dshr Cordis sidecar: exposes a JS plugin's ctx.tool() registrations as MCP.
import readline from "node:readline";
import path from "node:path";
import fs from "node:fs/promises";
import http from "node:http";
import { randomBytes } from "node:crypto";
import { pathToFileURL } from "node:url";
import { createRequire } from "node:module";

const entry = process.argv[2];
const root = path.resolve(process.env.DSHR_CORDIS_ROOT || process.cwd());
const tools = new Map();
const grants = (() => {
  try {
    return JSON.parse(process.env.DSHR_CORDIS_GRANTS || "{}");
  } catch {
    return {};
  }
})();
const extraRead = (grants.read || []).map((entry) => path.resolve(root, entry));
const extraWrite = (grants.write || []).map((entry) => path.resolve(root, entry));

function within(name, roots) {
  const absolute = path.resolve(root, name);
  if (absolute === root || absolute.startsWith(root + path.sep)) return true;
  return roots.some(
    (allowed) => absolute === allowed || absolute.startsWith(allowed + path.sep),
  );
}

function confined(name, roots = []) {
  const absolute = path.resolve(root, name);
  if (!within(name, roots)) {
    throw new Error(`path escapes the workspace: ${name}`);
  }
  return absolute;
}

function addTool(definition) {
  const name = definition && definition.name;
  if (typeof name !== "string" || name.length === 0) {
    throw new Error("tool.name is required");
  }
  if (typeof definition.handler !== "function") {
    throw new Error(`tool ${name}: handler is required`);
  }
  tools.set(name, {
    name,
    description: definition.description || "",
    inputSchema: definition.inputSchema || { type: "object" },
    handler: definition.handler,
  });
}

// ---------------------------------------------------------------------------
// Host HTTP routes (C1): DSH host halves call
//   ctx.inject(['webServer'], (scope) => scope.webServer.register({ name, kind, path, handler }))
// with a plain Node `(req, res)` handler. The sidecar runs a loopback-only HTTP
// server, and the dshr server proxies only the registered paths to it.
// ---------------------------------------------------------------------------
let httpServer = null;
let httpPort = 0;
// Reference `registerUpgrade` table: exact-path protocol owners. A WebSocket (or
// any upgrade) route owns negotiation and the raw socket; dshr tunnels the
// upgraded connection to the sidecar and never parses the protocol itself.
const upgradeRoutes = new Map();
// Reference webServer owns exactly one fallback seat (the shipped composition
// puts the SPA dist server there). A second registration throws.
let fallbackHandler = null;
const httpToken = randomBytes(16).toString("hex");
// Paths that collide with the dshr HTTP API/embedded UI must never be proxied.
const reservedPrefixes = [
  "/v1",
  "/dshr",
  "/openapi",
  "/manifest.webmanifest",
  "/style.css",
  "/icon-192.png",
  "/icon-512.png",
];

function validRoutePath(candidate) {
  if (typeof candidate !== "string" || candidate.length === 0 || candidate.length > 400) return false;
  // Reference contract: an absolute pathname with no trailing slash, query or
  // hash. "/" itself is the fallback seat, not a named route.
  if (!candidate.startsWith("/") || candidate === "/") return false;
  if (candidate.endsWith("/") || candidate.includes("?") || candidate.includes("#")) return false;
  if (candidate.includes("..")) return false;
  return !reservedPrefixes.some(
    (prefix) => candidate === prefix || candidate.startsWith(prefix + "/"),
  );
}

function ensureHttpServer() {
  if (httpServer) return;
  httpServer = http.createServer(async (req, res) => {
    if (req.headers["x-dshr-sidecar-token"] !== httpToken) {
      res.writeHead(403, { "content-type": "application/json" });
      res.end(JSON.stringify({ ok: false, error: "forbidden" }));
      return;
    }
    let route;
    try {
      const pathname = new URL(req.url || "/", "http://127.0.0.1").pathname;
      // Exact routes win; a `kind: "prefix"` route matches itself and children,
      // longest prefix first (DSH asset/sub-path routes).
      route = webRoutes.exact.get(pathname);
      if (!route) {
        let best = null;
        for (const candidate of webRoutes.prefix.values()) {
          const matches = pathname === candidate.path || pathname.startsWith(candidate.path + "/");
          if (matches && (best === null || candidate.path.length > best.path.length)) {
            best = candidate;
          }
        }
        route = best || undefined;
      }
    } catch {
      route = undefined;
    }
    if (!route) {
      if (fallbackHandler === null) {
        res.writeHead(404, { "content-type": "application/json" });
        res.end(JSON.stringify({ ok: false, error: "no such plugin route" }));
        return;
      }
      try {
        await fallbackHandler(req, res);
      } catch (error) {
        if (!res.headersSent) {
          res.writeHead(500, { "content-type": "application/json" });
          res.end(JSON.stringify({ ok: false, error: String(error?.message ?? error) }));
        } else if (!res.writableEnded) {
          res.end();
        }
      }
      return;
    }
    try {
      await route.handler(req, res);
    } catch (error) {
      if (!res.headersSent) {
        res.writeHead(500, { "content-type": "application/json" });
        res.end(JSON.stringify({ ok: false, error: String(error?.message ?? error) }));
      } else if (!res.writableEnded) {
        res.end();
      }
    }
  });
  httpServer.on("listening", () => {
    httpPort = httpServer.address().port;
  });
  httpServer.keepAliveTimeout = 5_000;
  httpServer.on("upgrade", (req, socket, head) => {
    const refuse = (status) => {
      try {
        socket.end(`HTTP/1.1 ${status}\r\nConnection: close\r\n\r\n`);
      } catch (error) {
        void error;
      }
      socket.destroy();
    };
    if (req.headers["x-dshr-sidecar-token"] !== httpToken) {
      refuse("403 Forbidden");
      return;
    }
    let route;
    try {
      route = upgradeRoutes.get(new URL(req.url || "/", "http://127.0.0.1").pathname);
    } catch (error) {
      void error;
      refuse("400 Bad Request");
      return;
    }
    if (!route) {
      refuse("404 Not Found");
      return;
    }
    try {
      Promise.resolve(route.handler(req, socket, head)).catch((error) => {
        console.error("[dshr-sidecar] upgrade handler failed:", error);
        socket.destroy();
      });
    } catch (error) {
      console.error("[dshr-sidecar] upgrade handler failed:", error);
      socket.destroy();
    }
  });
  httpServer.requestTimeout = 30_000;
  httpServer.listen(0, "127.0.0.1");
}

// Reference-shaped route tables: exact and prefix are independent namespaces,
// and a duplicate (kind, path) is a composition error, not an override.
const webRoutes = { exact: new Map(), prefix: new Map() };
let bridgeTarget = null;

function readBridgeTarget() {
  if (bridgeTarget) return bridgeTarget;
  try {
    const parsed = JSON.parse(require("node:fs").readFileSync(hostBridgeFile, "utf8"));
    if (parsed && typeof parsed.url === "string") bridgeTarget = parsed;
  } catch {
    bridgeTarget = null;
  }
  return bridgeTarget;
}

const webServer = {
  register: (route) => {
    if (!route || typeof route !== "object" || typeof route.handler !== "function") {
      throw new Error("webServer.register: handler is required");
    }
    if (!validRoutePath(route.path)) {
      throw new Error(`webserver: invalid route path "${route.path}"`);
    }
    const kind = route.kind || "exact";
    if (kind !== "exact" && kind !== "prefix") {
      throw new Error(`webserver: unsupported route kind "${kind}"`);
    }
    const table = webRoutes[kind];
    if (table.has(route.path)) {
      throw new Error(`webserver: duplicate ${kind} route "${route.path}"`);
    }
    table.set(route.path, { ...route, kind, name: route.name || route.path });
    ensureHttpServer();
    return function unregister() {
      table.delete(route.path);
    };
  },
  // Reference introspection: the origin local clients use. The sidecar serves
  // loopback from its own listener, but the dshr server origin is what host
  // halves need (e.g. for absolute URLs), so it comes from the host bridge file.
  getOrigin: () => {
    const target = readBridgeTarget();
    if (!target) throw new Error("webserver: origin read before the host bridge is available");
    return target.url;
  },
  get port() {
    const target = readBridgeTarget();
    if (!target) return undefined;
    try {
      return Number(new URL(target.url).port);
    } catch {
      return undefined;
    }
  },
  get protocol() {
    const target = readBridgeTarget();
    try {
      return target ? new URL(target.url).protocol : "http:";
    } catch {
      return "http:";
    }
  },
  // Reference `registerFallback`: one owner for every unmatched request. The
  // handler receives the normal Node (req, res) pair; dshr only proxies paths it
  // does not reserve for its own API/UI.
  registerFallback: (fallback) => {
    if (fallbackHandler !== null) {
      throw new Error("webserver: fallback already registered");
    }
    const handler =
      typeof fallback === "function"
        ? fallback
        : fallback && typeof fallback.handler === "function"
          ? fallback.handler
          : null;
    if (handler === null) throw new Error("webServer.registerFallback: handler is required");
    fallbackHandler = handler;
    ensureHttpServer();
    return function unregister() {
      if (fallbackHandler === handler) fallbackHandler = null;
    };
  },
  // Reference `registerUpgrade`: an exact-path socket protocol owner (the route
  // owns negotiation and the socket). Duplicate paths throw, like routes.
  registerUpgrade: (route) => {
    if (!route || typeof route !== "object" || typeof route.handler !== "function") {
      throw new Error("webServer.registerUpgrade: handler is required");
    }
    if (!validRoutePath(route.path)) {
      throw new Error(`webserver: invalid upgrade route path "${route.path}"`);
    }
    if (upgradeRoutes.has(route.path)) {
      throw new Error(`webserver: duplicate upgrade route "${route.path}"`);
    }
    upgradeRoutes.set(route.path, { ...route, name: route.name || route.path });
    ensureHttpServer();
    return function unregister() {
      upgradeRoutes.delete(route.path);
    };
  },
};

// DSH credential seam: environment only (the sidecar inherits the harness env).
const credentialsService = {
  resolve: async (name) => {
    const value = process.env[name];
    return typeof value === "string" && value.trim() !== "" ? { value: value.trim() } : null;
  },
};

// ---------------------------------------------------------------------------
// C1.1 host bridge: the server publishes `{url, token}` in
// <home>/cordis/host.json and serves /v1/internal/session-snapshot. The sidecar
// translates that payload into the DSH-shaped `sessions`/`sessionProjections`
// services that design-system plugins resolve lazily in their route handlers.
// A snapshot (not a live proxy) keeps the service methods synchronous; it is
// refreshed every few seconds, so usage data can lag by that refresh interval.
// ---------------------------------------------------------------------------
const hostBridgeFile = process.env.DSHR_HOST_BRIDGE || "";
let hostSessions = [];

function dshSessionFromSnapshot(raw) {
  const log = [];
  const createdMs = Number.isFinite(Date.parse(raw.created_at)) ? Date.parse(raw.created_at) : 0;
  if (typeof raw.model === "string" && raw.model !== "") {
    log.push({ type: "request/context", time: createdMs, data: { model: raw.model } });
  }
  if (typeof raw.title === "string" && raw.title.trim() !== "") {
    log.push({ type: "session/title", time: createdMs, data: { title: raw.title.trim() } });
  }
  const tokenUsage = {
    uncachedInputTokens: 0,
    cacheReadTokens: 0,
    cacheWriteTokens: 0,
    outputTokens: 0,
  };
  const events = Array.isArray(raw.usage_events) ? raw.usage_events : [];
  events.forEach((event, index) => {
    const time = Number.isFinite(Date.parse(event.at)) ? Date.parse(event.at) : createdMs;
    const input = Number(event.input_tokens) || 0;
    const cacheRead = Number(event.cache_read_input_tokens) || 0;
    const cacheWrite = Number(event.cache_creation_input_tokens) || 0;
    const output = Number(event.output_tokens) || 0;
    log.push({
      type: "assistant/message",
      time,
      data: {
        turn: event.turn,
        step: Number.isFinite(Number(event.seq)) ? Number(event.seq) : index,
        usage: {
          inputTokens: input,
          outputTokens: output,
          cacheReadTokens: cacheRead,
          cacheWriteTokens: cacheWrite,
        },
      },
    });
    tokenUsage.uncachedInputTokens += Math.max(0, input - cacheRead);
    tokenUsage.cacheReadTokens += cacheRead;
    tokenUsage.cacheWriteTokens += cacheWrite;
    tokenUsage.outputTokens += output;
  });
  return {
    id: raw.id,
    header: { createdAt: createdMs, title: raw.title || "" },
    log,
    projections: {
      values: {
        sessionStats: {
          turns: Number(raw.turns) || 0,
          steps: events.length,
          // C5.4b: real step latency summed by the server from the journal.
          llmMs: Number(raw.llm_ms) || 0,
        },
        tokenUsage,
      },
    },
  };
}

async function readHostBridge() {
  if (!hostBridgeFile) return null;
  try {
    const text = await fs.readFile(hostBridgeFile, "utf8");
    const bridge = JSON.parse(text);
    if (!bridge || typeof bridge.url !== "string" || typeof bridge.token !== "string") return null;
    return bridge;
  } catch {
    return null;
  }
}

async function refreshHostSessions() {
  const bridge = await readHostBridge();
  if (!bridge) return;
  try {
    const response = await fetch(bridge.url + "/v1/internal/session-snapshot", {
      headers: { "x-dshr-internal-token": bridge.token },
      signal: AbortSignal.timeout(5000),
    });
    if (!response.ok) return;
    const payload = await response.json();
    if (!payload || !Array.isArray(payload.sessions)) return;
    hostSessions = payload.sessions.map(dshSessionFromSnapshot);
  } catch {
    // The server may not be listening yet (bridge file is absent/stale); the
    // feed loop retries, and plugins see an empty session list until it succeeds.
  }
}

/// C5.3: replaces or prepends one session from the live feed (same shape as the
/// snapshot entry, so projections re-derive exactly as after a hydration).
function mergeHostSession(raw) {
  const entry = dshSessionFromSnapshot(raw);
  const index = hostSessions.findIndex((item) => item.id === entry.id);
  if (index >= 0) hostSessions[index] = entry;
  else hostSessions.unshift(entry);
}

function parseSseFrame(frame) {
  let event = "message";
  const data = [];
  for (const line of frame.split("\n")) {
    if (line.startsWith("event:")) event = line.slice(6).trim();
    else if (line.startsWith("data:")) data.push(line.slice(5).trim());
  }
  return { event, data: data.join("\n") };
}

async function streamHostSessions(bridge) {
  const response = await fetch(bridge.url + "/v1/internal/session-events", {
    headers: {
      "x-dshr-internal-token": bridge.token,
      accept: "text/event-stream",
    },
  });
  if (!response.ok || !response.body) throw new Error("session feed " + response.status);
  const decoder = new TextDecoder();
  let buffer = "";
  for await (const chunk of response.body) {
    buffer += decoder.decode(chunk, { stream: true });
    let boundary = buffer.indexOf("\n\n");
    while (boundary >= 0) {
      const frame = buffer.slice(0, boundary);
      buffer = buffer.slice(boundary + 2);
      const { event, data } = parseSseFrame(frame);
      if (event === "resync") {
        await refreshHostSessions();
      } else if (event === "session" && data !== "") {
        try {
          const payload = JSON.parse(data);
          if (payload && payload.session) mergeHostSession(payload.session);
        } catch {
          // A malformed frame is skipped; the next event or a resync repairs it.
        }
      }
      boundary = buffer.indexOf("\n\n");
    }
  }
}

// Sleeps without keeping the sidecar process alive: an unref'd timer does not
// hold the event loop, so stdin close still shuts the sidecar down.
function hostFeedDelay(ms) {
  return new Promise((resolve) => {
    const timer = setTimeout(resolve, ms);
    if (typeof timer.unref === "function") timer.unref();
  });
}

/// Keeps the session list live: subscribe to the server feed, and on any failure
/// re-hydrate from the snapshot before reconnecting (a 404 from an older server
/// degrades to the snapshot with backoff instead of a fixed 5 s poll).
async function runHostFeed() {
  let backoff = 500;
  for (;;) {
    const bridge = await readHostBridge();
    if (!bridge) {
      await hostFeedDelay(1000);
      continue;
    }
    // Hydrate before subscribing: the feed only carries updates, so a sidecar
    // that starts after sessions exist must not serve an empty list meanwhile.
    await refreshHostSessions();
    try {
      await streamHostSessions(bridge);
      backoff = 500;
    } catch {
      backoff = Math.min(backoff * 2, 10000);
    }
    await refreshHostSessions();
    await hostFeedDelay(backoff);
  }
}

// ---------------------------------------------------------------------------
// C1.2 `configForms` host service: plugins register settings cards that read and
// mutate a form snapshot. dshr has no DSH configuration engine, so this shim
// backs each entry with a small JSON file under <home>/cordis/config-forms/
// (writable, revision-checked). It is enough for real settings cards to render,
// toggle and persist; it is not the DSH configuration model.
// ---------------------------------------------------------------------------
const configFormsDir = hostBridgeFile
  ? path.join(path.dirname(hostBridgeFile), "config-forms")
  : "";
const configForms = new Map();
const configFormListeners = new Map();

function configFormPath(entryId) {
  if (!configFormsDir) return "";
  return path.join(configFormsDir, encodeURIComponent(entryId) + ".json");
}

function loadConfigForm(entryId) {
  if (configForms.has(entryId)) return configForms.get(entryId);
  let value = {};
  let revision = "0";
  try {
    const raw = JSON.parse(require("node:fs").readFileSync(configFormPath(entryId), "utf8"));
    if (raw && typeof raw === "object") {
      value = raw.value && typeof raw.value === "object" ? raw.value : value;
      revision = String(raw.revision ?? revision);
    }
  } catch {
    // No file yet: start from an empty writable form.
  }
  const state = {
    entryId,
    status: "ready",
    writable: true,
    value,
    base: {},
    user: value,
    revision,
    listeners: new Set(),
    snapshot: null,
    getSnapshot: () => {
      if (!state.snapshot) {
        state.snapshot = {
          status: state.status,
          value: state.value,
          base: state.base,
          user: state.user,
          revision: Number(state.revision) || 0,
          writable: state.writable,
        };
      }
      return state.snapshot;
    },
    subscribe: (listener) => {
      if (typeof listener !== "function") return () => {};
      state.listeners.add(listener);
      return () => state.listeners.delete(listener);
    },
    mutate: async (operations, expectedRevision) => {
      if (!Array.isArray(operations)) return false;
      if (
        expectedRevision !== undefined &&
        Number(expectedRevision) !== (Number(state.revision) || 0)
      ) {
        return false;
      }
      let next;
      try {
        next = JSON.parse(JSON.stringify(state.value));
      } catch {
        next = {};
      }
      for (const operation of operations) {
        if (!operation || (operation.op !== "set" && operation.op !== "unset")) return false;
        const pathSegments = Array.isArray(operation.path) ? operation.path : [operation.path];
        if (pathSegments.length === 0) return false;
        let cursor = next;
        for (let index = 0; index < pathSegments.length - 1; index += 1) {
          const segment = String(pathSegments[index]);
          if (typeof cursor[segment] !== "object" || cursor[segment] === null) cursor[segment] = {};
          cursor = cursor[segment];
        }
        const last = String(pathSegments[pathSegments.length - 1]);
        if (operation.op === "unset") delete cursor[last];
        else cursor[last] = operation.value;
      }
      state.value = next;
      state.user = next;
      state.revision = (Number(state.revision) || 0) + 1;
      state.snapshot = null;
      try {
        if (configFormPath(entryId)) {
          require("node:fs").mkdirSync(configFormsDir, { recursive: true });
          require("node:fs").writeFileSync(
            configFormPath(entryId),
            JSON.stringify({ value: state.value, revision: state.revision }, null, 2),
          );
        }
      } catch (error) {
        console.error("[cordis] configForms persist failed:", error);
      }
      state.listeners.forEach((listener) => {
        try {
          listener();
        } catch (error) {
          console.error("[cordis] configForms listener failed:", error);
        }
      });
      return true;
    },
    set: (field, value) =>
      state.mutate([{ op: "set", path: [field], value: value }]),
    unset: (field) => state.mutate([{ op: "unset", path: [field] }]),
  };
  configForms.set(entryId, state);
  return state;
}

const configFormsService = {
  get: (entryId) => loadConfigForm(String(entryId)),
  entries: () => [...configForms.keys()],
};

const sessionsService = {
  list: () => hostSessions,
  get: (id) => hostSessions.find((session) => session.id === id),
};
const sessionProjectionsService = {
  snapshot: (session) =>
    (session && session.projections) || {
      values: { sessionStats: null, tokenUsage: null },
      asOfSeq: null,
    },
};

const serviceMap = {
  webServer,
  credentials: credentialsService,
  sessions: sessionsService,
  sessionProjections: sessionProjectionsService,
  configForms: configFormsService,
};

const ctx = {
  // Reference loader context: the file URL the plugin package was loaded from.
  baseUrl: pathToFileURL(root + path.sep).href,
  tool: (definition) => addTool(definition),
  get: (name) => serviceMap[name],
  webServer,
  configForms: configFormsService,
  // Reference fiber helper: run the callback only when every dependency exists.
  inject: (dependencies, callback) => {
    if (typeof callback !== "function") return () => {};
    const list = Array.isArray(dependencies) ? dependencies : [dependencies];
    const ready = list.every((name) => serviceMap[name] !== undefined);
    if (!ready) return () => {};
    try {
      const result = callback(ctx);
      return typeof result === "function" ? result : () => {};
    } catch (error) {
      console.error("[cordis] inject callback failed:", error);
      return () => {};
    }
  },
  log: (...args) => console.error("[cordis]", ...args),
  fs: {
    readFile: async (name) => {
      if (!within(name, extraRead)) {
        throw new Error(`path escapes the workspace (no read grant): ${name}`);
      }
      return fs.readFile(confined(name, extraRead), "utf8");
    },
    writeFile: async (name, contents) => {
      if (!within(name, extraWrite)) {
        throw new Error(`path escapes the workspace (no write grant): ${name}`);
      }
      return fs.writeFile(confined(name, extraWrite), contents);
    },
    readdir: async (name) => {
      if (!within(name, extraRead)) {
        throw new Error(`path escapes the workspace (no read grant): ${name}`);
      }
      return fs.readdir(confined(name, extraRead));
    },
  },
  exec: () => {
    throw new Error("ctx.exec is disabled by the dshr capability bus");
  },
  net: () => {
    throw new Error("ctx.net is disabled by the dshr capability bus");
  },
};

// Internal control tool (not served to the agent; the host filters `__dshr_`
// tools out of the registry). It reports the loopback port + route table.
addTool({
  name: "__dshr_routes",
  description: "internal: host HTTP route table",
  inputSchema: { type: "object" },
  handler: async () => {
    // `listen(0)` completes asynchronously; an early route query must not report
    // port 0 (the host would treat the plugin as route-less).
    if (httpServer && httpPort === 0) {
      await new Promise((resolve) => {
        if (httpServer.listening) resolve();
        else httpServer.once("listening", resolve);
      });
    }
    return {
      port: httpPort,
      token: httpToken,
      fallback: fallbackHandler !== null,
      upgrades: [...upgradeRoutes.keys()],
      routes: [
        ...[...webRoutes.exact.values()].map((route) => ({ path: route.path, kind: "exact" })),
        ...[...webRoutes.prefix.values()].map((route) => ({ path: route.path, kind: "prefix" })),
      ],
    };
  },
});

const require = createRequire(path.resolve(root, entry));
let resolvedEntry = path.resolve(root, entry);
try {
  resolvedEntry = require.resolve(resolvedEntry);
} catch {
  // Keep the explicit path; the import below reports the real error.
}
const module = await import(pathToFileURL(resolvedEntry).href);
const plugin = module.default ?? module;

let cordis = null;
try {
  cordis = await import(pathToFileURL(require.resolve("cordis")).href);
} catch {
  cordis = null;
}

let applied = false;
if (cordis && typeof cordis.Context === "function") {
  try {
    const app = new cordis.Context();
    app.tool = ctx.tool;
    app.log = ctx.log;
    app.get = ctx.get;
    app.inject = ctx.inject;
    app.webServer = ctx.webServer;
    app.dshr = { fs: ctx.fs, exec: ctx.exec, net: ctx.net };
    app.plugin(plugin);
    await new Promise((resolve) => setTimeout(resolve, 50));
    applied = true;
  } catch (error) {
    console.error("[cordis] real cordis failed, using direct apply:", error);
  }
}
if (!applied && typeof plugin.apply === "function") {
  await plugin.apply(ctx);
} else if (!applied && typeof plugin === "function") {
  await plugin(ctx);
} else if (!applied && plugin.tools) {
  for (const definition of plugin.tools) addTool(definition);
}

const send = (value) => process.stdout.write(`${JSON.stringify(value)}\n`);
const respond = (id, result) => send({ jsonrpc: "2.0", id, result });
const fail = (id, code, message) => send({ jsonrpc: "2.0", id, error: { code, message } });

const input = readline.createInterface({ input: process.stdin });
// Start the live session feed once stdin (the MCP channel) holds the event loop:
// an unref'd reconnect timer must never be the only pending handle.
if (hostBridgeFile) {
  refreshHostSessions();
  runHostFeed();
}
for await (const line of input) {
  if (!line.trim()) continue;
  let request;
  try {
    request = JSON.parse(line);
  } catch {
    continue;
  }
  if (!("id" in request)) continue;
  const id = request.id;
  const method = request.method || "";
  if (method === "initialize") {
    respond(id, {
      protocolVersion: "2024-11-05",
      capabilities: { tools: {} },
      serverInfo: { name: "dshr-cordis", version: "0.1.0" },
    });
  } else if (method === "tools/list") {
    respond(id, {
      tools: [...tools.values()].map(({ name, description, inputSchema }) => ({
        name,
        description,
        inputSchema,
      })),
    });
  } else if (method === "tools/call") {
    const name = request.params?.name;
    const tool = tools.get(name);
    if (!tool) {
      fail(id, -32602, `unknown tool: ${name}`);
      continue;
    }
    try {
      const output = await tool.handler(request.params?.arguments ?? {});
      respond(id, {
        content: [{ type: "text", text: typeof output === "string" ? output : JSON.stringify(output) }],
        isError: false,
      });
    } catch (error) {
      respond(id, {
        content: [{ type: "text", text: String(error) }],
        isError: true,
      });
    }
  } else if (method === "resources/list") {
    respond(id, { resources: [] });
  } else if (method === "prompts/list") {
    respond(id, { prompts: [] });
  } else {
    fail(id, -32601, "method not found");
  }
}
