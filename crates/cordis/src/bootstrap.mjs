// dshr Cordis sidecar: exposes a JS plugin's ctx.tool() registrations as MCP.
import readline from "node:readline";
import path from "node:path";
import fs from "node:fs/promises";
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

const ctx = {
  tool: (definition) => addTool(definition),
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
