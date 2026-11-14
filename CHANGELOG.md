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

Known gaps: Windows AppContainer is experimental/opt-in; desktop Tauri and full i18n are deferred; full React compatibility is not a host contract (iframe bridge, ADR-013 §5): C-lite supports React slot components with host hooks/services (`ctx.get('sessions')`, plus the round-80 `ctx.get('slots')`/`ctx.get('locale')`/`ctx.inject` and `__DSHR_LAST_ERROR__` diagnostics, the round-81 React contract — `React.memo`/`forwardRef` types, keyed ids, `ctx.locale.register`/`t()`, `ctx.interval`/`ctx.timeout` — the round-82 reference slots surface check via the real `dsh-mnemon@0.5.46` and the round-83 conformance n=9 set with `dsh-plugin-market@1.2.2`, `dsh-plugin-codegraph-panel@0.6.5` and `@goodandready/dsh-plugin-notify@0.4.9`) and a partial 7-component design-system shim, but the full `@deepseek-ai/dsh-client-ui-*` design-system, host React context/state, SlotMap semantics, DSH plugin host HTTP routes (data plugins render, but their data branch fails), a selectable React version and the conformance report/nightly run await the ADR-040 C-full decision, which is now an owner decision rather than unfinished C-lite work (D not recommended). The bridge services are a subset (no `sessions` projections/`sessionStats`), eval-based bundles need an opt-in `'unsafe-eval'` CSP (not enabled) and `@michengai/dsh-skills-manager` still fails at bundle init — all recorded in TECH-DEBT/LIMITATIONS.
