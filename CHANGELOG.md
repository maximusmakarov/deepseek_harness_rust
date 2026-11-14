# Changelog

## 0.1.0 — current

- M0–M2: workspace, config, LLM adapters, JSONL sessions, tools, agent loop, TUI.
- M3: prompt/AGENTS, skills, compaction, fork/rewind/title/search.
- M4: MCP stdio, web tools, jobs, subagents.
- M5: Seatbelt/Landlock sandbox, MCP plugin manager.
- M6: HTTP/SSE server, Web UI, JSON-RPC.
- M7: workflows, teams (durable), worktrees, schedules, LSP, Office, CUA, FTS5, WASM, OTel, ACP.
- Hardening: Linux seccomp/network, tool allowlist, agent-tree budget, scheduler lock.
- Packaging: Dockerfile, release workflow, install script.
- Plugins: experimental `dsh.client.runtime` — a package-relative classic script
  embedded into the sandboxed host page and executed before the client bundle; the
  half may publish `window.__DSHR_MODULES__`, and module-table bundles
  (`__ModuleLoader__.load({id, factory})`) resolve `require(specifier)` from it.
  Validation: package-confined path, ≤ 8 MiB, `</script>` rejected (400), CSP stays
  network-free, no separate signature. This is the enabler for plugins that bring
  their own UI stack, React included (React 18.3.1 PoC verified manually).
- Plugins: tier B PoC of ADR-040 — shared host module table `dsh.client.host_modules`.
  A half lists ids from a fixed allowlist (`react`, `react/jsx-runtime`,
  `react/jsx-dev-runtime`, `react-dom`, `react-dom/client`); unknown ids are rejected
  at discovery. An opted-in host page loads one versioned immutable artifact,
  `GET /v1/runtime/react-18.3.1.js` (React + ReactDOM UMD + `bootstrap.js`, ~143 KB,
  `Cache-Control: public, max-age=31536000, immutable`), so the browser downloads
  React once per page instead of embedding ~140 KB into every host page. Order is
  shared runtime → plugin-shipped `dsh.client.runtime` (may override entries) → bundle;
  the only new CSP permission is `script-src <server origin>` for opt-in frames
  (`connect-src` stays `'none'`, frames without `host_modules` stay network-free).
  Vendored assets `crates/server/assets/runtime/` are checksum-gated by
  `scripts/check-runtime-assets.sh` (wired into `scripts/check.sh` and CI). Each
  iframe keeps its own JS realm, so "one version per page" means one artifact/download,
  not a shared JS context. Full React compatibility is the open C-full decision in
  ADR-040. Evidence: smoke 60/60, workspace tests 361 passed + 3 ignored (desktop 21,
  total 382), web-e2e 16/16. Also: WASM hot reload (fingerprint
  watcher, shared registry); Cordis hot reload (supervisor, file/config watcher,
  `/v1/plugins/reload`)
  and the client half: every `dsh.client` half gets an eager hidden registry
  frame, and every slot registration renders in its own sandboxed iframe mounted
  into the shell region its DSH slot key maps to — the full 116-key DSH 0.2.2-rc.2
  slot vocabulary plus legacy `panel`/`message.action` mapped to 17 regions, with
  unknown names falling back to the plugin panel; bridge protocol v1.2 (frame
  token, region, slot mode, occurrence context).
- Plugins: C-lite PoC of ADR-040 — React component slots over the shared host
  runtime. When a frame has host React (`host_modules` includes `react`),
  `ctx.slots.register(entry, Component)` is a React component (reference
  contract): one React root per frame, re-rendered on theme/host-state/context
  changes, wrapped in an error boundary that reports render errors to the shell.
  The shell broadcasts `host-state` (`theme`, `wide`, `session`, `sessions`) on
  ready and on session/list/theme changes; component props gain `useTheme()`,
  `useSession()`, `useSessions(selector?)`, `useHostState(selector?)` and `wide`
  (currently always true — recorded limitation). `ctx.get('sessions')` returns a
  partial reference service (`currentProvideInfo.getSnapshot`/`subscribe`,
  `get(id)`, `list()`); unknown names are `undefined`. The shared runtime
  (148,140 bytes: React + ReactDOM UMD + `bootstrap.js` + new `primitives.js`)
  publishes an independent PARTIAL `@deepseek-ai/dsh-client-ui-primitives` shim
  (`Button`, `StateDot`, `Tooltip`, `Icon`, `Spinner`; not the upstream package)
  and the `HOST_MODULES` allowlist accepts that id. Conformance: the real MIT
  plugin `@kt11/dsh-deepseek-balance@0.2.1` is vendored as a fixture (client half
  verbatim, sha256 in `package.json`, host half stubbed) and renders its React
  card in `sidebar.footer.action`, injects its CSS and opens the popup (the data
  branch fails because dshr has no DSH host routes — recorded limitation).
  Evidence: smoke 60/60 (new `hooks`/`ui-kit` fixtures plus the real plugin),
  bridge harness 73 checks, cordis lib 26, server `plugins_live` 12, workspace
  361 passed + 3 ignored (desktop 21, total 382), web-e2e 16/16.

Known gaps: Windows AppContainer is experimental/opt-in; desktop Tauri and full i18n are deferred; full React compatibility is not a host contract (iframe bridge, ADR-013 §5): C-lite supports React slot components with host hooks/services (`ctx.get('sessions')`) and a partial design-system shim, but the full `@deepseek-ai/dsh-client-ui-*` design-system, host React context/state, SlotMap semantics, DSH plugin host HTTP routes (data plugins render, but their data branch fails), a selectable React version and a wider conformance set (n=1) await the ADR-040 C-full decision (D not recommended).
