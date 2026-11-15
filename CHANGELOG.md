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
  ADR-040. Evidence: smoke 63/63, workspace tests 361 passed + 3 ignored (desktop 21,
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
  Evidence: smoke 60/60 at the C-lite cut (new `hooks`/`ui-kit` fixtures plus
  the real plugin), extended to 63/63 with the second conformance plugin in the
  follow-up below; bridge harness 73 checks at the cut, 79 after the bridge
  services below; cordis lib 26, server `plugins_live` 12, workspace 361 passed
  + 3 ignored (desktop 21, total 382), web-e2e 16/16.
- Plugins: C-lite conformance extended to n=2 and the bridge services reference
  DSH plugins expect (`ctx.get('slots')`, `ctx.get('locale')`, `ctx.inject`,
  diagnostics `window.__DSHR_LAST_ERROR__`). `ctx.get('slots')` is the same
  registry as `ctx.slots` (`inject`/`register`/`remove`/`list`) plus
  `entries(key)`; `ctx.get('locale')` is a minimal `{active: "en", locale: "en"}`
  service with `subscribe`; `ctx.inject(deps, callback)` runs `callback(ctx)`
  only when every dependency resolves (optional `betterSidebar`/`sidebarRight`/
  `webTerminals` integrations stay unmounted — the graceful degradation DSH
  plugins are written for); the last frame error is available inside the frame
  as `window.__DSHR_LAST_ERROR__` (`{message, stack}`). Conformance n=2: the
  vendored `dsh-remote@1.1.2` (MIT, client half verbatim, sha256 in
  `package.json`, host half stubbed, `host_modules` added) registers React slots
  in `conversation.session.header.actions`/`utilities` and
  `sidebar.workspaces.directoryFlow` and activates without errors when the
  optional services are absent (the existing `@kt11/dsh-deepseek-balance@0.2.1`
  fixture stays). Rejected from the set for now (recorded in TECH-DEBT and
  LIMITATIONS, not part of n=2): `@michengai/dsh-archive-manager` 1.3.6 needs
  `'unsafe-eval'` in the CSP (deliberately not enabled) and
  `@michengai/dsh-skills-manager` 1.4.8 throws during bundle init (`Object.assign`
  over `undefined`) — needs source triage. Evidence: smoke 63/63 (+3: second real
  plugin discovery, its error-free registry frame, its header-region mount),
  bridge harness 79 checks (+6: slots alias/register/entries, locale,
  `ctx.inject`, diagnostics); workspace 361 passed + 3 ignored (desktop 21,
  total 382), cordis lib 26, server `plugins_live` 12, web-e2e 16/16; the shared
  runtime artifact is unchanged (148,140 bytes).

- Plugins: C-lite conformance n=4 and the React-contract gaps the two new real
  plugins exposed (ADR-040). The bridge now accepts React component *types*
  (`React.memo`/`forwardRef` objects carrying `$$typeof`, not only functions),
  gives keyed registrations without an `id` a unique `<slot>:<key>` id (and
  `ctx.get('slots').entries(key)` returns the reference shape with `component`
  and `options`), supports `ctx.locale.register(ns, dicts)` plus a `t()` seat in
  component props (English dictionary, `{name}` substitution), and tears down
  `ctx.interval`/`ctx.timeout` with the frame. The primitives shim gained
  `JsonBlock`/`MarkdownText` (`Button`, `StateDot`, `Tooltip`, `Icon`, `Spinner`,
  `JsonBlock`, `MarkdownText` — 7 components), so the shared runtime artifact is
  now 149,442 bytes. Conformance n=4: the vendored
  `@ruoyang/dsh-plugin-deepseek-balance@1.0.2` (MIT, client half verbatim, sha256
  `ca700695…`, host half stubbed) renders a floating badge in `shell.overlay`
  via `ctx.effect`/`ctx.interval`/`ctx.locale`, and `dsh-genui@0.3.3`
  (MIT, client half verbatim, sha256 `8f0500ad…`, host half stubbed) adds a GenUI
  toggle in `conversation.input.left` and a keyed `conversation.chat.node`
  registration; the existing `@kt11/dsh-deepseek-balance@0.2.1` and
  `dsh-remote@1.1.2` fixtures stay. Evidence: bridge harness 87 checks (+8: memo
  type, host props/hooks, `t()` seat, keyed ids, `slots.entries` shape), smoke
  66/66 (+3: two-plugin discovery, the overlay badge render, the GenUI toggle render
  in `conversation.input.left`), cordis lib 26, server `plugins_live` 12,
  workspace 361 passed + 3 ignored (desktop 21, total 382), web-e2e 16/16.

- Plugins: C-lite conformance n=6 — two more real MIT plugins with the upstream
  client half unchanged (only `host_modules` added to the manifest). The vendored
  `dsh-session-superagent@0.10.12` (MIT, sha256 `d58f0fa2…`, host half stubbed)
  commits a composer dock (`conversation.composer.dock`) and activates without
  errors; the vendored `dsh-mnemon@0.5.46` (MIT, sha256 `d967402b…`) is a large
  multi-region plugin (conversation header, composer, panel, settings,
  `plugins.*`) that activates, publishes slots and reconciles its own shadow
  registrations through the reference slots surface
  (`ctx.slots.subscribe`/`getVersion`/`entriesOfSlot`) shipped in round 81 — no
  bridge changes were needed, so the surface held against a real plugin. The
  existing `@kt11/dsh-deepseek-balance@0.2.1`, `dsh-remote@1.1.2`,
  `@ruoyang/dsh-plugin-deepseek-balance@1.0.2` and `dsh-genui@0.3.3` fixtures
  stay. Evidence: smoke 69/69 (+3: the two-plugin discovery, the composer-dock
  mount, the mnemon activation with published slots), bridge harness 87, cordis
  lib 26, server `plugins_live` 12, workspace 361 passed + 3 ignored (desktop
  21, total 382), web-e2e 16/16; the shared runtime artifact is unchanged
  (149,442 bytes).

- Plugins: C-lite conformance n=9 — the ADR-013 §6 target of 8–12 real plugins
  is reached, with three more real MIT plugins whose upstream client half is
  unchanged (sha256 in `package.json`, host halves stubbed, the fixture
  manifest opts into the shared runtime via `host_modules`): the vendored `@99galaxy/dsh-plugin-market@1.2.2`
  (sha256 `27abb2db…`) registers its sidebar entry (`sidebar.panellist` → region
  `sidebar.footer`, only `react`) and its frame mounts; the vendored
  `dsh-plugin-codegraph-panel@0.6.5` (sha256 `f834fadb…`) publishes a
  `settings.section` panel (`immediately: true` upstream, only `react`
  required); and the vendored
  `@goodandready/dsh-plugin-notify@0.4.9` (sha256 `7a2843fa…`) publishes rows in
  `plugins.item`/`plugins.row.config` (region `plugins.detail`, primitives +
  react). All three activate, publish slots and leave no errors; the existing
  six fixtures stay. The smoke fixture set is now 22 configured client halves
  (nine real, the rest synthetic). Rejected from the set and still outside n=9:
  `@michengai/dsh-archive-manager` (`'unsafe-eval'` in the CSP, deliberately not
  enabled) and `@michengai/dsh-skills-manager` (bundle-init failure). Evidence:
  smoke 72/72 (+3: the three-half discovery, the three-plugin activation with
  published slots, the market sidebar-frame mount), bridge harness 87, cordis
  lib 26, server `plugins_live` 12, workspace 361 passed + 3 ignored (desktop
  21, total 382), web-e2e 16/16; the shared runtime artifact is unchanged
  (149,442 bytes). Full React compatibility stays an owner decision (ADR-040
  C-full, still Proposed).

- Plugins: C1 — plugin host HTTP routes work end-to-end (ADR-040 §C1). A host
  half calls `ctx.inject(['webServer'], scope => scope.webServer.register({name,
  kind: 'exact', path, handler}))` with a plain Node `(req, res)` handler, exactly
  like DSH. The Cordis sidecar now runs a loopback-only HTTP server (`127.0.0.1`,
  random port, per-process token; reserved `/v1`/`/dshr`/`/openapi`/UI prefixes
  rejected), exposes an internal `__dshr_routes` control tool (port, token, route
  table), and reads `credentials.resolve(name)` from the environment so keys never
  reach the browser. `CordisManager` keeps the table in `Running` and answers
  `http_route(path)` for exact paths only; `__dshr_*` tools are filtered out of the
  agent tool registry. The dshr server has a fallback proxy for allowlisted exact
  paths only: it forwards method/query/body (request cap 1 MiB, response cap 8 MiB,
  15 s timeout), answers reserved and unknown paths with the normal dshr 404, and
  adds permissive CORS for opaque-origin frames (the global `CorsLayer` answers
  preflight first). Routes are live — reload/stop removes them. CSP change: frames
  that opt into `host_modules` now get `connect-src <server origin>` — the real
  cause of the earlier "Failed to fetch" — while external network stays denied and
  frames without `host_modules` stay network-free (`connect-src 'none'`). The real
  `@kt11/dsh-deepseek-balance` host half is no longer stubbed: the upstream host
  half is vendored verbatim (host sha256 `8771ae90…`, client sha unchanged
  `f0a4080…`), and smoke proves through the plugin frame that `/dsh-balance/deepseek`
  answers 500 JSON `{ok:false, code:"not-configured"}` without a key,
  `/dsh-usage/sessions?tz=UTC` answers 200 JSON with a `sessions` array, and an
  unknown plugin path keeps the dshr 404. Remaining C1 limits (recorded in
  TECH-DEBT/LIMITATIONS): no `sessions`/`sessionProjections` services yet (usage
  routes that build session statistics return empty/error data), no
  SSE/WebSocket/streaming through the proxy, exact paths only, plugin routes are
  unauthenticated/public like DSH and reachable on the LAN interface in LAN mode;
  a Cordis sidecar connects through MCP, so MCP meta-tools (`list_resources`/
  `read_resource`/`list_prompts`/`get_prompt`) still appear in the tool list of a
  host-only plugin. Evidence: smoke 75/75 (+3: proxied host route with its own
  JSON, query/status preservation, unknown-path 404), bridge harness 87, cordis lib
  26, server `plugins_live` 14 (+2: `proxies_plugin_host_routes`,
  `plain_frames_stay_network_free`), workspace 363 passed + 3 ignored (desktop 21,
  total 384), web-e2e 16/16; the shared runtime artifact is unchanged (149,442
  bytes). Full React compatibility (full design-system, host React context, SlotMap
  semantics) stays an owner decision (ADR-040 C-full, still Proposed).

- Plugins: C2 — the design-system shim grew from 7 to 30+ exports based on what
  the real conformance plugins actually import (ADR-040 §C2). New components:
  `Input`, `Switch`, `Checkbox`, `Tag`, `Pill`, `Menu` (`items`/`onSelect`/
  `anchor`/`selectedId`/`align`/`side`/`portal`), `MenuSurface` (forwardRef
  container), `Modal` (`title`/`description`/`footer`/`onClose`/`closeLabel`,
  portalled), `Toast` (`text`/`tone`/`holdMs`/`onDone`/`icon`, auto-dismiss),
  `SegmentedControl` (radiogroup + arrow keys), `SegmentedTabs` (tablist),
  `DisclosureRow` (`title`/`open`/`expandable`/`collapsedContent`/
  `headerAccessory`), `PathLabel`, `TextShimmer`; the previous `Button`,
  `StateDot`, `Tooltip`, `Icon`, `Spinner`, `JsonBlock`, `MarkdownText` are kept
  and `Tooltip` is now a `forwardRef`. Named `Icon*` components
  (close/chevron/search/settings/link/refresh/warning/skill/check/info/plus/
  trash/copy) ship SVG glyphs, and a `Proxy` fallback resolves any unmodelled
  `Icon*` name to a generic glyph so an import never yields `undefined` and
  crashes the bundle. Also new: `writeClipboard` (navigator.clipboard +
  execCommand fallback) and the hooks `useAnchoredPosition` /
  `useDismissOnOutsidePointer`. Conformance reaches **n=11**: the vendored MIT
  `dsh-plugin-debugging@0.8.1` (client sha256 `9b828160…`) and
  `dsh-plugin-studio@0.3.1` (client sha256 `1194c18b…`) have verbatim upstream
  client halves (sha256 in `package.json`) and stubbed host halves; both activate
  cleanly, but their UI is gated on the host services `pluginManager`/settings
  that dshr does not provide, so it stays unmounted — the same graceful
  degradation as `dsh-remote` with its optional services. Smoke gets two more
  checks: a tree of every modelled primitive renders inside a host-React frame
  (12 text markers, portal Menu/Modal/Toast) and the real `dsh-plugin-notify`/
  `dsh-mnemon` contributions mount in the Plugins dialog without
  `__DSHR_LAST_ERROR__`. Evidence: smoke 79/79 (+2), bridge harness 87, cordis
  lib 26, server `plugins_live` 14, workspace 363 passed + 3 ignored (desktop 21,
  total 384), web-e2e 16/16; the shared runtime artifact grew to **176,725 bytes**
  (React + ReactDOM UMD + `bootstrap.js` + `primitives.js` with 30+ exports).
  Remaining C2/C-full: the full `@deepseek-ai/dsh-client-ui-*` design-system and
  visual tokens (the shim is an independent partial implementation driven by
  real imports), more components as the set grows, the `pluginManager`/`settings`
  host services, SlotMap semantics, host React context and inter-plugin
  composition — an owner decision (ADR-040 C-full, still Proposed).

- Plugins: C1 hardening (the C1 hardening section of ADR-040). Route kinds:
  `webServer.register` now accepts `kind: "exact"` and `kind: "prefix"`; the
  sidecar matches exact before prefix and the longest prefix wins, and
  `CordisManager::http_route` plus the server fallback proxy follow the same
  rules (`crates/cordis/src/bootstrap.mjs`, `crates/cordis/src/manager.rs`,
  `crates/server/src/lib.rs`). Diagnostics: `GET /v1/plugins/status` now exposes
  a per-plugin `routes` list (the live host-route table) beside `tools` — also
  useful when the host half registers a prefix route. MCP noise: the bridge
  pseudo-tools `list_resources`/`read_resource`/`list_prompts`/`get_prompt` are
  filtered out of a Cordis plugin's tool registry, next to the internal
  `__dshr_*` tools (`crates/cordis/src/lib.rs`), so the agent sees only real
  plugin tools. LAN policy: plugin routes are unauthenticated (the browser frame
  has no token), so when the server binds a non-loopback address the proxy
  answers 403 unless the new `--allow-lan-plugin-routes` flag /
  `ServerConfig.allow_plugin_routes_lan` opts in (`crates/cli/src/serve_cmd.rs`,
  `crates/server/src/lib.rs`) — this closes the recorded "plugin routes public
  as DSH / LAN exposure" debt item instead of just documenting it. Evidence:
  server `plugins_live` 15 (+1: `plugin_routes_are_loopback_only_unless_opted_in`;
  `proxies_plugin_host_routes` gained prefix/status/meta-tool assertions), bridge
  harness 87, cordis lib 26, workspace 364 passed + 3 ignored (desktop 21,
  total 385), web-e2e 16/16, smoke 79/79 unchanged; the shared runtime artifact
  is unchanged (176,725 bytes). Remaining after C1 hardening: the
  `sessions`/`sessionProjections` bridge (C1.1), SSE/WebSocket/streaming through
  the proxy, full design-system/SlotMap/host React context (C-full, owner
  decision) and the `pluginManager`/settings host services.

- Plugins: C1.1 — the sessions/sessionProjections bridge (the C1.1 section of
  ADR-040). The server writes a per-run internal bridge file
  `<home>/cordis/host.json` (`{url, token}`; `write_host_bridge` in
  `crates/server/src/lib.rs`, the token is generated by `serve()`,
  `ServerConfig.internal_token`) and serves `GET /v1/internal/session-snapshot`:
  the recent ≤50 sessions with id/title/created_at/model/turns/steps plus
  assistant usage events (input/output/cache tokens) — message content and prompts
  never leave the server; the endpoint is 404 when no internal token is configured
  (tests/embeddings) and 401 on a wrong `x-dshr-internal-token`. The Cordis
  sidecar receives the file path through the `DSHR_HOST_BRIDGE` env var
  (`crates/cordis/src/lib.rs`), reads it in `bootstrap.mjs`, refreshes the snapshot
  every 5 s on an unref'd timer (`refreshHostSessions`) and translates it into the
  DSH-shaped services that reference plugins resolve lazily
  (`dshSessionFromSnapshot`): `ctx.get('sessions')` (`list()`/`get(id)`,
  `header.createdAt` in epoch ms, a DSH-shaped `log` with `request/context`,
  `session/title` and `assistant/message` usage) and
  `ctx.get('sessionProjections')` (`snapshot(session).values` with `sessionStats`
  turns/steps/`llmMs=0` and `tokenUsage`). The methods are synchronous over the
  cached snapshot, so data can lag up to 5 s. Effect: the real
  `@kt11/dsh-deepseek-balance` plugin's usage routes (`/dsh-usage/sessions`) now
  list actual dshr sessions and compute totals; the smoke polls that route from
  the plugin frame and requires `sessions >= 1` plus totals. Evidence: smoke
  80/80 (+1), server `plugins_live` 16 (+1:
  `internal_session_snapshot_feeds_the_host_bridge`, with the 401/404 cases),
  bridge harness 87, cordis lib 26, workspace 365 passed + 3 ignored (desktop 21,
  total 386), web-e2e 16/16; the shared runtime artifact is unchanged (176,725
  bytes). Remaining C1.1 limits: the bridge is a snapshot rather than a live RPC
  (staleness ≤ 5 s), the projections are simplified (`llmMs=0`, no thread
  history), session content is deliberately not bridged, and there is still no
  SSE/WebSocket/streaming through the proxy. C-full (design-system/SlotMap/host
  React context) remains an owner decision.

- Docs: round 88 recorded the C-full decision as ADR-041 — C-full (the full
  design-system, SlotMap semantics, host React context/state, the
  `pluginManager`/`settings` host services and streaming) is **deferred**;
  further work is targeted increments on demand, and the return criterion is
  ≥3 real plugins that need those capabilities (there are none right now).
  Candidate increments: the `pluginManager`/`settings` host services, a SlotMap
  PoC for `conversation.*` and a streaming proxy. The full `scripts/check.sh`
  gate suite passes, the reproducible build was verified and the release
  artifacts were rebuilt and re-checked.

- Plugins: C1.2 — seat inject, Cordis generator factories and the `configForms`
  host service (the C1.2 section of ADR-040), the contracts without which the real
  settings UIs stayed inert. Slot registrations may supply extra component props
  with `inject: () => ({...})` (merged into the component at mount in both the
  React and the imperative path); a throwing factory degrades the component,
  logs into the frame and does **not** mark the plugin failed
  (`injectedProps` in `crates/cordis/src/bridge.js`). `ctx.slots.inject`,
  `ctx.inject` and `ctx.effect` accept `function*` factories: the body runs,
  every yielded disposer is collected and run in reverse on teardown
  (`cordisDisposer`) — this unblocked `dsh-plugin-studio`, which registers its
  settings tabs through a generator. The browser bridge exposes
  `ctx.get('configForms')`/`ctx.configForms` with `get(entryId)` returning a form
  `{getSnapshot, subscribe, mutate(ops, revision)}`: the snapshot object is stable
  between notifications (`useSyncExternalStore` requirement), the revision
  contract is enforced (a stale revision returns `false`), and the in-frame form
  is in-memory and writable — **edits do not survive a frame reload** (recorded
  limit). Host halves additionally get a file-backed form from the bootstrap at
  `<home>/cordis/config-forms/<entry>.json`. Effect: `dsh-plugin-studio@0.3.1`
  now renders real tabs in the Settings dialog (`settings.plugins.tab`, its own
  CSS, honest error states when host routes are stubbed) and
  `dsh-plugin-debugging@0.8.1` renders its settings card in the Plugins dialog
  (`plugins.bundle.config`) with a working toggle + Save round-trip through
  `configForms`. Evidence: smoke 82/82 (+2: the real studio settings tab with
  seat inject; the debugging config card renders and saves), bridge harness 90
  (+3: seat props, generator `slots.inject`, generator `effect`), cordis lib 26,
  server `plugins_live` 16, workspace 365 passed + 3 ignored (desktop 21,
  total 386), web-e2e 16/16; the shared runtime artifact is unchanged
  (176,725 bytes). C1.2 limits: browser `configForms` is in-memory (no reload
  persistence), the DSH configuration-engine semantics are not reproduced, and
  seat-inject failures are only logged; C-full stays deferred per ADR-041.

- Plugins: C1.3 — the reference slot contract (the C1.3 section of ADR-040). The
  client slot catalog is no longer hand-made: `crates/cordis/src/slot-catalog.json`
  is generated from the reference repository (`deepseek-ai/deepseek-harness` @
  `39163dd`, `packages/extensions/cordis-client-runner/src/client/slot-catalog.ts`)
  and holds 122 slots with kind (single 45 / list 48 / keyed 22 / chain 4 / hoc 3),
  scope (root 57 / session 60 / session-maybe 5), framework standard-kit names and a
  slot-`inject` flag; every host page publishes it as `window.__DSHR_SLOT_META__`.
  Bridge registration mirrors reference `SlotCore`: an undeclared slot throws (the
  dshr extension namespaces `panel`/`message.action`/`custom.*`/`x-*`/`dshr.*` are
  exempt), `keyed` requires `key`, `list`/`hoc` require `id`, `chain` requires
  `select`; a second registration in the same cell (single = slot, keyed = key,
  otherwise id) at the same priority throws while a different priority shadows
  (lowest renders); registrations may declare child slots with
  `children: {key: {kind, scope}}` that live and cascade away with the parent entry
  (this unblocked `dsh-mnemon`'s own `mnemon.component.*` slots);
  `ctx.slots.inject(key, factory)` defers until the slot is declared and re-runs on
  declaration; the shell mounts only shadowing winners (single one, keyed one per
  key, list/chain/hoc all layers ordered priority → order → registration); component
  props compose as catalog standard kit (unimplemented hooks stay callable and return
  `undefined`, `sessionId` from host state) → seat `inject` → `ownerProps` from
  `window.__DSHR_FRAME__.ownerProps` → `t`; and `ctx.slots.spec(key)` returns
  `{key, kind, scope, standardProps, inject}` like the reference introspection mnemon
  uses. The region map gained 6 upstream slot keys (`conversation.input.model.pane`,
  `forge.*`) — 124 vocabulary keys including the 2 legacy ones — and the synthetic
  smoke fixtures supply the now-required `key` for keyed slots. Effect: `dsh-mnemon`
  registers its own child slots and deferred injections by the reference rules.
  Evidence: bridge harness 98 (+8: undeclared/keyed/chain/duplicate throws, single
  shadowing, catalog standard kit, ownerProps spread, plain values), smoke 82/82
  (three consecutive green runs; no new checks — the real plugins already followed the
  rules), cordis lib 26, server `plugins_live` 16, workspace 365 passed + 3 ignored
  (desktop 21, total 386), web-e2e 16/16; the shared runtime artifact is unchanged
  (176,725 bytes). C1.3 limits: `chain` select dispatch is not implemented (layers
  mount instead), `hoc` children nesting across frames is absent, store seats
  (`PropsStore`/`BoundActions`) and the mounted session model are missing, and
  `ownerProps` are only supplied through the frame seam/harness — real shell render
  sites (`conversation.chat.node` etc.) do not provide data yet. The C-full remainder
  (full design-system, store/renderer semantics, chain/hoc rendering) stays deferred
  per ADR-041.

- Plugins: C1.4 — host services aligned with the reference repository (the C1.4
  section of ADR-040). `webServer` follows `packages/host/webserver/src/index.ts`:
  exact and prefix routes are independent tables, a duplicate `(kind, path)` throws
  `webserver: duplicate <kind> route`, a registered path must be an absolute
  pathname without a trailing slash, query or hash and must stay out of dshr's
  reserved API prefixes; `getOrigin()`/`port`/`protocol` return the dshr server
  origin read from `<home>/cordis/host.json` (the host bridge) instead of the
  loopback sidecar proxy port; `registerFallback`/`registerUpgrade` are unsupported
  in the sidecar and throw an explicit error (recorded as remaining debt — no
  conformance plugin needs them). `ctx.baseUrl` mirrors the reference loader
  context: the `file://` URL of the plugin package root, which host halves like
  `dsh-plugin-studio` resolve paths from. `configForms` follows
  `packages/client/ui-settings/src/client/config-form-types.ts`: the snapshot is
  exactly `{status, value, base, user, revision: number, writable}` with a stable
  identity between reads, `mutate(ops, expectedRevision?)` supports atomic
  `set`/`unset` ops with a numeric revision fence, and `set(field, value)`/
  `unset(field)` helpers exist; the browser form stays in-memory (edits do not
  survive a reload, documented) and the host-side form stays file-backed under
  `<home>/cordis/config-forms/`. Evidence: bridge harness 102 (+4: reference
  snapshot shape, stable identity between reads, `set` with a numeric revision
  fence, `unset`), server `plugins_live` 16 (the proxy test now covers
  duplicate-route and trailing-slash rules), smoke 82/82, cordis lib 26,
  workspace 365 passed + 3 ignored (desktop 21, total 386), web-e2e 16/16; the
  shared runtime artifact is unchanged (176,725 bytes). C1.4 limits:
  `registerFallback`/`registerUpgrade` are unsupported in the sidecar, browser
  `configForms` are not persisted across a reload, and there is no live RPC
  settings mirror (file/memory stand-ins). Remaining reference work: a real
  `ui-primitives` port from `packages/client/ui-primitives` (next candidate),
  store seats/renderer, chain dispatch/hoc nesting and ownerProps from the real
  shell sites.

- Plugins: C2.1 — the design-system atoms and all 204 icons are ported from the
  reference repository (the C2.1 section of ADR-040) instead of the previous
  independently written shim. `primitives.js` now follows
  `deepseek-ai/deepseek-harness` @ `39163dd`,
  `packages/client/ui-primitives/src`: `Button` (variants
  primary/ghost/outline/toolbar, sizes md/sm, leading icon, forwardRef),
  `Input` (wrapper + leading icon), `Switch` (role switch, `aria-label`, thumb
  only), `Checkbox` (native input + label, `data-modal-autofocus`), `Tag`
  (`data-tone`: outline/solid/neutral/quiet/success/info/warning/danger), `Pill`
  (span/button by `onClick`, `active`), `SegmentedControl` (tablist + indicator,
  roving tabindex, walk keys, `<id>-<value>`/`-panel` wiring), `SegmentedTabs`
  (Pill tabs, grid indicator, arrow/Home/End focus), `DisclosureRow`
  (header/row, `expandOnRowClick`, `previewChevron`, `collapsedContent`,
  `headerAccessory`, running shimmer), `TextShimmer` (nested context +
  decoration/sweep), `StateDot` (done/warning/ongoing/error/idle, SVG spinner,
  `appearance: "step"`), `PathLabel` (directory/name split, clipped mask,
  `title`), `Toast` (portal, `role=alert`, success check-circle, `holdMs`+fade,
  actions, anchor), `Modal` (portal, mask, Escape, focus, warning tone,
  headless, footer), `Menu` (data rows/separators/labels, fixed list,
  outside/Escape, `selectedId`/`selectedIds`, portal; the full submenu/focus
  machinery is not ported), `MenuSurface` (forwardRef,
  `data-menu-material`/`phase`), `Tooltip` (label-based cloneElement anchor +
  hover/focus bubble; the full placement engine is not ported). All **204**
  reference `Icon*` exports are generated from upstream `icons/artwork.ts` with
  exact SVG paths and regular (1) / medium (1.3) strokes, inlined instead of the
  document sprite, and the Proxy fallback is kept for future names. Styles use
  the reference `--dsw-alias-*` token names with fallbacks. Extensions that stay
  dshr-only: `Spinner`, simplified `JsonBlock`/`MarkdownText` (upstream needs
  shiki/katex/mdast) and `useAnchoredPosition`; `writeClipboard` and
  `useDismissOnOutsidePointer` are ported from the reference algorithms.
  Synthetic fixtures follow the reference contracts (`StateDot`
  `state: "done"`, `Tooltip` with an element child) and the C2 smoke probe now
  asserts structural markers — `role=switch` with `aria-label`, `data-tone`,
  tablist tabs, inline `svg` count, `label.dshr-checkbox`, path-label
  `textContent` — instead of text only. Evidence: smoke 82/82 (two consecutive
  green runs), bridge harness 102, cordis lib 26, server `plugins_live` 16,
  workspace 365 passed + 3 ignored (desktop 21, total 386), web-e2e 16/16; the
  shared runtime artifact grew from 176,725 to **375,894 bytes** (React +
  ReactDOM UMD + `bootstrap.js` + the ported `primitives.js`; cached immutable
  once per page). C2.1 limits: the full `Menu` submenu/focus machinery and the
  full `Tooltip` placement engine are not ported, and `MarkdownText`/`JsonBlock`
  stay simplified; store seats/renderer semantics, chain dispatch/hoc nesting
  and ownerProps from real shell sites remain C-full.
- Plugins: C2.2 — the last simplified C2.1 atom, `Tooltip`, is now a full port
  of `packages/client/ui-primitives/src/Tooltip.tsx` (the C2.2 section of
  ADR-040): the label resolver is evaluated only on show, hover and focus
  triggers are independent (`delayMs`/`focusDelayMs`), `openOnClick` pinning is
  dismissed by an outside pointer, Escape or Tab, plus `onlyWhenClipped`,
  `maxWidth`, `portal`, `data-side`/`data-pinned`, nested suppression through
  context, `aria-describedby` while pinned, pointer modality (focus after a
  pointer never raises the bubble), touch hover excluded, and a forward ref
  through the cloned anchor. `tooltip-placement.ts` is ported as-is (requested
  side → opposite → perpendicular, viewport clamp, overflow fallback) and
  `input-modality.ts` (`data-input-modality`, `pointerModality`) plus
  `ShortcutKeys` (keycaps with `+` separators) are ported and exported.
  Evidence: smoke 82/82 (the C2 probe adds a `Tooltip` node to the synthetic fixture; `ShortcutKeys` is exported and rendered inside the Tooltip label), bridge harness 102, cordis lib 26, server
  `plugins_live` 16, workspace 365 passed + 3 ignored (desktop 21, total 386),
  web-e2e 16/16; the shared runtime artifact grew from 375,894 to
  **387,613 bytes**. Remaining partials: the full `Menu` submenu/focus
  machinery, `MarkdownText`/`JsonBlock` (upstream needs shiki/katex/mdast),
  store seats/renderer semantics, chain dispatch/hoc nesting and ownerProps
  from real shell sites.

Known gaps: Windows AppContainer is experimental/opt-in; desktop Tauri and full i18n are deferred; full React compatibility is not a host contract (iframe bridge, ADR-013 §5): C-lite supports React slot components with host hooks/services (`ctx.get('sessions')`, plus the round-80 `ctx.get('slots')`/`ctx.get('locale')`/`ctx.inject` and `__DSHR_LAST_ERROR__` diagnostics, the round-81 React contract — `React.memo`/`forwardRef` types, keyed ids, `ctx.locale.register`/`t()`, `ctx.interval`/`ctx.timeout` — the round-82 reference slots surface check via the real `dsh-mnemon@0.5.46` and the round-83 conformance n=9 set with `dsh-plugin-market@1.2.2`, `dsh-plugin-codegraph-panel@0.6.5` and `@goodandready/dsh-plugin-notify@0.4.9`, and the round-85 C2 expansion — 30+ design-system exports and conformance n=11 with `dsh-plugin-debugging@0.8.1`/`dsh-plugin-studio@0.3.1`, whose settings UI stayed inert without the `pluginManager`/settings host services until the round-89 C1.2 seat-inject/`configForms` work made it render — browser forms are in-memory and the DSH configuration engine is not reproduced), the round-91 C1.3 reference slot contract is generated from the reference catalog (122 slots with kind/scope/standard kit; undeclared/keyed/chain/duplicate registration rules, shadowing winners, children declarations, deferred inject, standard kit + ownerProps; `chain` select dispatch, `hoc` nesting, store seats and ownerProps from real shell sites remain debt), the round-92 C1.4 host services follow the reference contract (`webServer` independent exact/prefix tables, duplicate `(kind, path)` throws, absolute pathname rules, `getOrigin()`/`port`/`protocol` read from the host bridge, the `file://`-baseUrl, and the reference `configForms` snapshot `{status, value, base, user, revision, writable}` with a numeric revision fence and `set`/`unset`; `registerFallback`/`registerUpgrade` are unsupported in the sidecar) and, since the round-93 C2.1 port (with the round-94 C2.2 full Tooltip), design-system atoms, the full Tooltip and all 204 icons following the reference sources (`primitives.js` is a reference port, not an independent shim; the shared runtime artifact is 387,613 bytes; the full `Menu` submenu/focus machinery and `MarkdownText`/`JsonBlock` stay simplified), but the full `@deepseek-ai/dsh-client-ui-*` design-system, host React context/state, the SlotMap remainder (chain select dispatch, hoc nesting, store seats, ownerProps from real shell sites), a selectable React version and the conformance report/nightly run await the ADR-040 C-full decision, which is now an owner decision rather than unfinished C-lite work (D not recommended). C1 host HTTP routes work (sidecar loopback server + allowlist proxy + `connect-src` for opt-in frames, with the real `@kt11/dsh-deepseek-balance` host half answering through the plugin frame); round 86 added exact and prefix routes, a live per-plugin `routes` table in `GET /v1/plugins/status`, a filter for MCP pseudo-tools (`list_resources`/`read_resource`/`list_prompts`/`get_prompt`) in the plugin tool registry, and a loopback-only LAN policy (403 unless `--allow-lan-plugin-routes`). Remaining C1 limits are recorded too: C1.1 now bridges `sessions`/`sessionProjections` from an internal server snapshot (≤ 5 s staleness, simplified projections: `llmMs=0`, no thread history; session content deliberately not bridged), so usage routes see real dshr sessions; there is still no SSE/WebSocket/streaming, and plugin routes still lack authentication. The bridge services are a subset (snapshot-based `sessions`/`sessionProjections` with simplified projections), eval-based bundles need an opt-in `'unsafe-eval'` CSP (not enabled) and `@michengai/dsh-skills-manager` still fails at bundle init — all recorded in TECH-DEBT/LIMITATIONS.
