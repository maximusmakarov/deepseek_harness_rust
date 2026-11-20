# Changelog

## 0.1.0 — current

- The Models first-run key field spans the welcome card with its hint below,
  and retry jitter is stored as f64 so the API and UI show `0.1` instead of an
  f32 artifact.
- API keys entered in Models or the first-run onboarding are stored
  write-only in `<home>/credentials.json` (0600, revision-fenced, environment
  wins); the Models card gains a masked key field with show/remove actions.
- The Models section mounts the upstream child slots
  `settings.models.provider-card` (per provider card, keyed by route with the
  row in owner props) and `settings.models.footer`, each in its own shell region
  instead of the flat Settings region.
- Models deletions are confirmed in a modal that names the provider or model,
  and a versioned first-run notice (durably acknowledged in `models.yaml`)
  explains where profiles and environment keys live.
- Models can ask an OpenAI-compatible or Anthropic-shaped endpoint for its
  model list (`POST /v1/config/providers/discover`) and adopt the result through
  a filterable picker; the key comes from the named environment variable and
  never reaches the page.
- Models providers carry a retry policy (`normal`/`always`, max retries,
  retryable codes, exponential backoff with jitter). It is edited in Settings,
  persisted in `models.yaml`, resolved into `dshr-llm`'s `RetryPlan`, and
  executed by both the DeepSeek and OpenAI-compatible adapters, replacing their
  hard-coded three-attempt loops.
- Models settings gained a hand-declared provider flow (Custom model API: id,
  protocol, endpoint, key variable, at least one model) and per-model reasoning
  levels (off/on, level -> API value rows, validated and persisted in
  `models.yaml`). The editor is one card shared by catalog adoption, editing and
  creation; adapters do not send the effort yet.
- Models settings is now a real provider section, not a single route pair:
  a five-provider catalog (DeepSeek Account / DeepSeek API / OpenAI /
  OpenAI-compatible / RouterAI), per-provider profiles in `<home>/models.yaml`
  (0600, atomic writes, monotonic revision with a `409` fence), a model list
  editor (id, name, context window, max output, input types), active-route
  switching checked against the list, and profile deletion with an inline
  confirmation. `dshr serve` resolves the profile's endpoint, API-key variable
  and custom routes at start-up; `model.base_url` is finally honoured. API keys
  stay in the environment (the document stores only the variable name).
- M0–M2: workspace, config, LLM adapters, JSONL sessions, tools, agent loop, TUI.
- M3: prompt/AGENTS, skills, compaction, fork/rewind/title/search.
- M4: MCP stdio, web tools, jobs, subagents.
- M5: Seatbelt/Landlock sandbox, MCP plugin manager.
- M6: HTTP/SSE server, Web UI, JSON-RPC.
- M7: workflows, teams (durable), worktrees, schedules, LSP, Office, CUA, FTS5, WASM, OTel, ACP.
- Hardening: Linux seccomp/network, tool allowlist, agent-tree budget, scheduler lock.
- Packaging: Dockerfile, release workflow, install script.
- Desktop: attach mode is now the default. The Tauri shell opens an existing
  `dshr serve` (`desktop-server.json`, `DSHR_DESKTOP_SERVER_URL`/
  `DSHR_DESKTOP_TOKEN`, or the "Server setup… (attach)" tray dialog), does not
  spawn or stop a server, and skips provider setup. Local supervision stays
  available for dev/CI via `"mode":"local"`, `DSHR_DESKTOP_LOCAL=1` or
  `--local`.
- Desktop Settings is rendered with the official design system: the theme
  token sheets and the Models section CSS from DSH 0.2.2-rc.3 are embedded
  verbatim, so provider cards, status dots, the masked API-key field with its
  reveal button and the editor match the original visually.
- Desktop Settings is a single page again: provider, model, API key, base URL
  and the server choice (embedded/attach + URL/token) share one Save, and the
  route is passed to the embedded server immediately. The web UI sidebar merges
  Account and Server into one Connection section.
- Plugin routes stream server-sent events (and chunked bodies) instead of
  buffering them, with no total timeout for SSE; buffered responses keep the
  8 MiB guard.
- Windows: draft per-user NSIS installer (`packaging/windows/dshr.nsi`) and a
  manual workflow; unverified on Windows and kept out of the release pipeline.
- Plugin routes reject cross-site browser requests: `Origin: null` (sandboxed
  plugin frames), loopback origins and native clients pass, any other origin
  gets 403; the loopback-only policy stays.
- Desktop shell is bilingual (ru/en via `DSHR_DESKTOP_LANG` or the system
  locale) for tray items and dialogs, and the tray gains "About dshr" and
  "Developer tools".
- Cordis sidecars provision a pinned Node 24.21.0 runtime pack on first use
  (macOS/Linux, sha256-verified, cached in `<home>/runtimes`); `DSHR_RUNTIME_AUTO=0`
  keeps the old PATH-only behaviour, and a failed download falls back to `node`.
- Sign-in can be cancelled (Cancel sign-in / `cancel_sign_in`) and honours
  `DSHR_DESKTOP_LOGIN_TIMEOUT_SECS` (5–3600 s). The settings sidebar gains a
  Server tab showing the origin, `/v1/health` status and the active model.
- Settings: the web UI settings screen is now a sidebar (Account / Models /
  Web search / Plugins) instead of one long page, matching the official app;
  `#settings` opens it directly.
- Settings: the web UI owns Models, Web search and Plugins (a `#settings` deep
  link opens them); a new `search` config section and `/v1/config/search`
  endpoint store the Brave key and re-register `web_search` live. The desktop
  window keeps Account + Server and links to the app settings; provider/model
  now come from `config.yaml` instead of `--provider/--model` flags.
- Desktop: "Sign in with DeepSeek" uses the public platform OAuth the official
  client uses (PKCE S256 against `/auth-api/v0/dsh/auth_init` +
  `/auth_exchange`, five `x-client-*` headers, loopback callback); the grant is
  stored as `DSH_DEEPSEEK_TOKEN` in `desktop.env`. Settings gained the Models,
  Web search (Brave key) and Plugins sections, the plugin list driving the same
  `/v1/plugins/*` endpoints as the web UI.
- Desktop: self-contained by default again, like the official app — the shell
  runs its embedded `dshr serve` and opens the UI; attach to an external server
  is an option. The native provider dialogs are gone: the in-app Settings window
  covers Account (provider, model, DeepSeek account token / API key, base URL)
  and Server (embedded or attach), validates the fields, probes `/v1/health`
  for attach, writes `desktop.env`/`desktop-server.json` (0600) and restarts.
  There is no OAuth sign-in: the token is entered in Settings.
- Desktop: server settings moved in-app (the official app's Settings pattern): a
  "dshr server" window edits the attach URL/token or switches to the supervised
  local server, validates the URL, probes `/v1/health`, saves
  `desktop-server.json` and restarts. A first run without a configured URL opens
  that window instead of a blocking native dialog.- Tests: the desktop crate gets its missing `tempfile` dev-dependency and unique
  temp-home names; the npm registry stub now switches accepted sockets to
  blocking mode (macOS inherits non-blocking) and installs routes before
  accepting, removing intermittent 404s.- Cordis compat: the sidecar answers the service probes real plugins make —
  `loader`, `profileContext`, `workspaceRegistry`, `agentPresets`,
  `credentials`, `jobs`, `sessionPersistence`, `sessionQuery`, `pluginPackages`,
  `agentDefaultModel` and a fail-closed `approval` — with honest empty or
  denial results instead of `undefined`. docs/PLUGINS.md now carries the
  service-by-service support matrix.
- TUI: `/help` lists built-in and plugin commands and `/name args` runs a
  `cmd__<plugin>__<name>` tool in the current session; plugin commands are now
  reachable from the agent, over HTTP and from the terminal. A nightly workflow
  runs the conformance matrix and uploads its report.
- Cordis compat: `scripts/conformance.py` and docs/CONFORMANCE.md run the set of
  real DSH plugins and record which tools and commands work. The pass fixed two
  real bugs: plugins that `console.log` no longer corrupt the MCP channel
  (stdout carries JSON only, console goes to stderr), and duplicate tar entries
  or half-unpacked store entries no longer fail an install. A plugin that needs
  a newer Node now gets a hint instead of a bare stack trace.
- Cordis compat: plugin commands are published to the agent as
  `cmd__<plugin>__<name>` tools (with the host session descriptor injected) and
  over HTTP via `GET /v1/commands` and `POST /v1/commands/{name}`, instead of
  living only in the sidecar registry.
- Cordis compat: the sidecar exposes a `settings` service implementing the
  reference layering (schema defaults → composition base → persisted user
  layer) with `register`/`describe`/`get`/`update`/`replace`/`mutate`, watchers,
  revisions and a JSON document at `<home>/cordis/settings.json`.
  `dsh-mnemon` now mounts without any warning next to `dsh-remote`.
- Cordis compat: `dshr-core` gained an `AgentRegistry` of live agents, the
  server publishes it on `/v1/internal/agents` (bridge token only, metadata
  only), and the sidecar exposes an `agents` service (`list`/`roots`/`get`/
  `isOwnedBy`/`rootOf`/`resolveName`) hydrated from that endpoint with
  `agent/created`/`agent/disposed` events. A `commands` registry service was
  added as well. `dsh-mnemon@0.5.46` now mounts with three tools next to
  `dsh-remote@1.2.0`.
- Cordis compat: the sidecar exposes a `subagents` service (provider registry
  with `list`/`getProvider`/`registerProvider` and `subagent/provider-*`
  events). It is a registry, not a runtime: the agent tree lives in the host, so
  plugins that need to start children degrade through their own feature probes.
- Cordis compat: the sidecar exposes the reference `ctx` surface (`effect`,
  `on`/`emit`, `provide`, `tools.register`, `systemPrompt.section`,
  `sandbox.confine`, `config`), resolves a named `export const Config` for
  `apply(ctx, config)`, and accepts host tools (`parameters`/`execute` from
  `defineTool`). `plugin npm add` installs peer dependencies as host-level
  singletons with cycle guards, so real DSH plugins resolve
  `@deepseek-ai/dsh-tools`; `dsh-remote@1.2.0` now loads and executes its 21
  tools through the sidecar.
- npm resolver: `dshr-npm` resolves registry specs (ranges, dist-tags, npm
  spacing/hyphen/`||` syntax), walks transitive dependencies with hoisting and
  nested conflicts, verifies tarballs against `dist.integrity`, stores them once
  in `<home>/npm/cas`, hard-links the tree and writes `dshr-npm-lock.json`; no
  lifecycle scripts run. `dshr plugin npm add|update` use it instead of the npm
  binary (file:/git specs still fall back to npm).
- Runtime packs: `dshr runtime install|list|use|remove` installs a pinned engine
  (Node) for Cordis sidecars from a local `.tar.gz` or an https URL. sha256 is
  mandatory, an optional Ed25519 signature is verified over the archive before
  unpacking, extraction rejects absolute paths, `..`, escaping symlinks and
  special files, and the pack is staged then renamed into place. The sidecar now
  resolves `DSHR_NODE` → installed `node` pack → PATH, and its failure message
  names the install command.
- Distribution: first tagged release `v0.1.0` (run 43832256936) shipped all 17
  artifacts, including the Linux `.deb` and a multi-platform GHCR bottle index;
  `maximusmakarov/homebrew-dshr` carries the GHCR formula, and
  `brew fetch maximusmakarov/dshr/dshr` pulls the `arm64_sequoia` bottle from
  GHCR anonymously with a matching sha256, and `brew install` poured that bottle
  (`poured_from_bottle: true`, `brew test dshr` passes). Homebrew 7 needs
  `brew trust maximusmakarov/dshr` before installing from the tap.
- Release path: the GHCR bottle publisher now writes one OCI index per version
  containing every platform; per-bottle pushes replaced the index and left only
  the last platform installable. `bottles-ghcr` pushes all bottles in one call.
- Release path: the Linux desktop job uploaded only `dist/*.tar.gz*`, so the
  promised `.deb` never reached the release or `SHA256SUMS`; it is uploaded now.
- Release path: `publish` waits for `bottles-ghcr` but still ships the binaries
  when the package push fails, and only renders `dshr-ghcr.rb` after a successful
  push.
- Release path: `workflow_dispatch` with a `version` input runs build, desktop
  and bottle jobs without publishing (no tag, no release), so the desktop-release
  path can be verified before the first tag.
- Release path: the rendered Homebrew formulas pass `brew audit --strict` (brew
  7.0.13): bottles are ordered ARM-macOS/Intel-macOS/ARM-Linux/Intel-Linux,
  digests are aligned, and the redundant `version` line is gone (brew scans it
  from the URL).
- Release path: `scripts/release-version.sh` normalises the version, and
  `scripts/sync-homebrew-tap.sh` installs the GHCR formula into a tap checkout
  with `root_url`/bottle-`sha256`/`ruby -c` validation.
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
- Plugins: C2.3 — `Menu`/`MenuItemButton` are now a reference-core port of
  `packages/client/ui-primitives/src/Menu.tsx`/`MenuRow.tsx` (the C2.3 section
  of ADR-040): data and component rows, separators/labels, `reserveIcon`
  columns (contextual), selection `check`/`fill`, `danger`/`current`,
  disabled-reason through the reference `Tooltip`, `shortcut` + `ShortcutKeys`,
  `children` rows and a `footer` group, aria (`menuitem`/`haspopup`/`expanded`/
  `disabled`/`describedby`/`keyshortcuts`/`current`), `dense`/`compact`, portal
  placement from the anchor rect (`minWidthFromAnchor`/`anchorPoint`/`align`/
  `side`) and the contextual attribute; keyboard walk (Arrow/Home/End with
  roving focus, Escape closes and returns focus via `focusWithoutRing`, Tab
  settles the row / Shift+Tab leaves, `autoFocus`), a one-level submenu on
  hover/ArrowRight closing on ArrowLeft/Escape/selection, outside-pointer
  dismissal plus close-on-iframe-blur, and `closeOnPointerLeave` with a 200 ms
  pointer grace (`usePointerGrace`); `MenuItemButton`, `MENU_TRANSITION_MS`,
  `usePointerGrace`, `focusWithoutRing` and `isMobileDevice` are exported.
  Evidence: smoke 82/82, bridge harness 102, cordis lib 26, server
  `plugins_live` 16, workspace 365 passed + 3 ignored (desktop 21, total 386),
  web-e2e 16/16; the shared runtime artifact grew from 387,613 to
  **405,323 bytes**. Remaining partials: the full upstream submenu machinery
  (`useMenuSubmenu` single-panel back-page, cross-level hover grace) and
  `MenuGroup`/`observeStickyMenuGroups` are not ported; `MarkdownText`/
  `JsonBlock` (upstream needs shiki/katex/mdast), store seats/renderer
  semantics, chain dispatch/hoc nesting and ownerProps from real shell sites
  remain.

- Plugins: C2.4 — reference submenu page-layout and `MenuGroup` (the C2.4
  section of ADR-040). Submenu placement follows `menu-submenu-position.ts`: a
  side child card when either side of the parent card fits, otherwise the
  single-page layout where child rows replace the parent page behind a
  non-selecting back row (`data-menu-back`, chevron-left, `submenuBackLabel` or
  the parent label) plus a separator, with Escape/ArrowLeft returning to the
  parent page and focus on the parent row. Focus contract: hovering opens the
  submenu without stealing focus, ArrowRight/click focus the first child row.
  200 ms hover grace between the parent and child cards (entering the child
  cancels the close via `cancelSubmenuClose`). `MenuGroup` +
  `observeStickyMenuGroups` ported from `MenuGroup.tsx`: `section[role=group]`
  with `aria-labelledby`, `data-menu-group`/`data-menu-group-start`/
  `data-menu-group-heading` and sticky headings (`data-stuck`) driven by
  IntersectionObserver/ResizeObserver. The C2 smoke probe now renders
  `MenuGroup` and a submenu parent and asserts `menuRows >= 3`. Evidence:
  smoke 82/82, bridge harness 102, cordis lib 26, server `plugins_live` 16,
  workspace 365 passed + 3 ignored (desktop 21, total 386), web-e2e 16/16; the
  shared runtime artifact grew from 405,323 to **415,972 bytes**. Remaining
  partials: the detailed `useMenuSubmenu` focus-transition/page-anchor
  mechanics stay simplified (our own CSS/JS placement); `MarkdownText`/
  `JsonBlock` (upstream needs shiki/katex/mdast), store seats/renderer
  semantics, chain dispatch/hoc nesting and ownerProps from real shell sites
  remain.

- Plugins: C2.5 — reference `JsonBlock` and an API-compatible `MarkdownText`
  subset (the C2.5 section of ADR-040). `JsonBlock` is ported from
  `packages/client/ui-primitives/src/markdown/JsonBlock.tsx`: collapsible
  (▾/▸), `label`/`payload`/`defaultOpen`/`truncatedLabel` props, a
  20,000-character cap with a truncated footer and a safe `JSON.stringify`
  fallback to `String`. `MarkdownText` now has the reference prop surface
  (`text`/`streaming`/`labels`/`fileMentions`/`pathImages`/`variant`/`omitHtml`/
  `textInteractionSource`) and a dependency-free GFM-subset renderer: headings
  1–6, paragraphs, ul/ol, fenced code, blockquotes, hr, inline
  code/bold/italic/strike, links and images restricted to `http(s)`/`mailto`
  (unsafe protocols — `javascript:`/`data:` — are dropped and relative images are not rendered), raw HTML rendered
  as nothing and `variant="compact"` data attributes. The upstream
  micromark/mdast pipeline, shiki highlighting, KaTeX, footnotes and
  incremental streaming are **not** bundled at this round (closed by the
  round-103 C2.7 entry below). The C2 smoke probe now renders
  `MarkdownText` (heading, bold, fenced code, link) and `JsonBlock` (expanded
  JSON) and asserts the DOM/text. Evidence: smoke 82/82, bridge harness 102,
  cordis lib 26, server `plugins_live` 16, workspace 365 passed + 3 ignored
  (desktop 21, total 386), web-e2e 16/16; the shared runtime artifact grew from
  415,972 to **424,801 bytes**. Remaining partials: the upstream markdown stack
  (micromark/mdast, shiki, KaTeX, footnotes, streaming), the detailed
  `useMenuSubmenu` focus-transition/page-anchor mechanics, store seats/renderer
  semantics, chain dispatch/hoc nesting and ownerProps from real shell sites.

- Plugins: C2.6/C3.1/C1.5 — submenu mechanics, store seats, fallback routes
  and ownerProps plumbing (rounds 98–101). **C2.6** completes the reference
  submenu mechanics in `primitives.js`: the side-card vs single-page decision
  measures the natural child width before choosing (`menu-submenu-position.ts`),
  the side card is positioned `fixed` and flipped/clamped against the parent
  card, the keyboard is routed through the `activeList` (parent list vs child
  page), Escape/ArrowLeft return focus to the parent row and close the menu,
  the 200 ms hover grace between levels is honoured, hovering another row
  collapses an open submenu, and `data-menu-back`/`submenuBackLabel` render the
  non-selecting back row; hover is gated by `menuHoverAllowed` (non-touch,
  `any-hover: hover`). **C3.1** publishes `@deepseek-ai/dsh-client-store` in
  the shared module table (`createSnapshotStore`, `defineStore`,
  `shallowEqual`; a plain deep-copied draft stands in for immer and the
  `persist` key is accepted but not persisted): a slot registration may declare
  `store: handle|factory`, the frame owns one instance per scope key (`session`
  / `session-maybe` key by session id), hands the component
  `useStore(selector, eq)` plus baked `actions`, and calls the inject factory
  with the reference `(sessionId, actions)` arguments. **C1.5** implements
  `webServer.registerFallback` (one fallback seat, a second owner throws
  `webserver: fallback already registered`) and dshr proxies only
  non-reserved unmatched paths to it; `registerUpgrade` stays explicitly
  unsupported. **ownerProps plumbing**: the shell can pass render-site owner
  props via `/v1/plugins/{name}/host?ownerProps={...}`; the server validates
  the JSON object and embeds it as `window.__DSHR_FRAME__.ownerProps`, and the
  bridge merges them into component props after the store/seat share (reference
  `PropsRuntime` order). Real shell render sites still need the chat/turn data
  model and supply nothing yet. Evidence: smoke 82/82 (new store-engine check:
  `defineStore` → `create` → `actions.inc` → snapshot `n === 1`), bridge
  harness 105 (+3 store-seat), server `plugins_live` 17 (+fallback test plus
  ownerProps validation), cordis lib 26, workspace 366 passed + 3 ignored
  (desktop 21, total 387), web-e2e 16/16; the shared runtime artifact grew from
  424,801 to **430,139 bytes**. Remaining partials: the upstream markdown stack
  (micromark/mdast, shiki, KaTeX, footnotes, streaming), renderer semantics,
  chain dispatch/hoc nesting in the frame-per-slot architecture, ownerProps
  from real shell sites and `registerUpgrade` (WebSocket).
- Plugins: C4.1 (round 102) — chat/turn model in the Leptos shell and chain
  election by owner props. The shell opens a new turn per user message and
  writes the ordinal to every message `data-turn` and to the owner share;
  `SlotFrame`/`SlotRegion` gained `owner_props` (the frame URL carries
  `&ownerProps=...`). Real render sites now supply owner props:
  `sidebar.session.action` → `{wide:true}`, `message.action` for user messages
  → `{source:{role,index}, kind:"user"}` (reference `UserBeforeOwnerProps`) and
  `composer.actions` → `ComposerChainProps`
  `{sessionId, session:{id,model,turns}, pendingInteraction:null}`. The bridge
  elects chain slots: a frame calls its registration `select(ownerProps)`; a
  `false` election clears the frame, leaves an empty DOM and reports
  `chain-unselected`, while `true` renders the selected entry — owner-specific
  dispatch without server-side JS. Evidence: bridge harness **107** (+2
  chain-select false/true), web-e2e **18** (+2: the turn ordinal on messages
  and the mounted `composer.actions`/`message.action` owner-props regions),
  server `plugins_live` 17 (ownerProps validation), smoke 82/82, cordis 26,
  workspace 366 passed + 3 ignored (desktop 21, total 387); the shared runtime
  artifact is unchanged at 430,139 bytes. Limits: owner props that carry
  functions (e.g. `settings.section.close`) are not delivered (no callback
  bridge), several `select === true` results render several frames (per-frame
  filtering, not one owner renderer), and `conversation.chat.node` plus the
  other conversation slots still need a full chat view.

- Plugins: C2.7 (round 103) — the upstream markdown stack ships in the shared
  runtime. `markdown.js` (**814,222 bytes**) is built by
  `scripts/build-markdown-runtime.mjs` from pinned npm packages
  (`scripts/markdown-runtime/pins.json` pins every one of the 135 installed
  packages through `overrides`; the install tree lives in the gitignored
  `target/markdown-runtime`) and concatenated after `primitives.js`, so client
  halves get the real `MarkdownText` instead of the dependency-free GFM subset:
  the reference component over micromark/mdast + GFM (tables, task lists,
  strikethrough, autolinks, footnotes) + math/KaTeX 0.16.47, shiki 4.3.1 core
  with the JavaScript regex engine and the TypeScript/shellscript/JSON boot
  grammars, and the reference incremental streaming parser (frozen blocks plus
  trailing-tail reparse). The build is a classic-script IIFE (no ESM import, no
  `eval`) with React, ReactDOM and `react/jsx-runtime` aliased to small shims
  over `window.React`/`window.ReactDOM`/`window.__DSHR_MODULES__`, so no second
  React is bundled. `markdown.js` overrides `MarkdownText` in the
  `@deepseek-ai/dsh-client-ui-primitives` module entry (plus the additive
  `extractMarkdownPlainText`) and keeps every other port export; the port's
  `.dshr-markdown` styling hooks are restored on an outer div and `JsonBlock`
  stays the reference port (`primitives.js`) because it matches the upstream
  DOM and keeps the `.dshr-json-block-*` classes. KaTeX's `@font-face` is
  stripped (no font files ride in the artifact) and the reference's lazy
  read-card shiki grammars are stubbed empty so ~1.6 MB of never-used grammars
  stay out (such fences render as plain code). Evidence: smoke **84/84** (+2
  C2.7 probes: `$x^2$` and a display block → `.katex`/MathML; a TypeScript
  fence → `pre.shiki` with `--shiki-*` spans and a computed color; `[^1]` +
  definition → `.footnotes` with `#user-content-fn-1` and a backref; an
  unfinished streaming tail renders with no error), `cargo test -p dshr-server`
  47 passed, `scripts/check-runtime-assets.sh` OK (5 files),
  `node scripts/build-markdown-runtime.mjs --check` OK; the shared runtime
  artifact grew from 430,139 to **1,244,361 bytes**.

- Plugins: C4.2 (round 104) — the SlotMap remainder is closed: owner props carry
  functions, a chain slot renders through one owner-renderer, `hoc` children nest
  across frames, the shell has a full chat view, and `webServer.registerUpgrade`
  (WebSocket) works end-to-end. Owner props with callbacks: the shell encodes a
  callback as the JSON marker `{"$callback": "<name>"}` (the server still
  validates `ownerProps` as a JSON object), and `crates/cordis/src/bridge.js`
  materializes markers into functions that post
  `ctx.action("callback", {name, args})` to the shell and resolve with its
  `action-result`; `settings.section.close` closes the settings dialog,
  `openFile` opens the new `/v1/workspace/file` inspector (workspace-confined
  after canonicalization, 64 KiB cap, truncation flag), `inspectCall` opens the
  tool-call inspector. Materialized callbacks keep their marker, so a parent
  component forwarding owner props into a child slot re-serializes them, and big
  owner shares (a whole chat node) travel out of band as an `owner-props` bridge
  message instead of a URL. Chain: `select(ownerProps)` follows the reference
  election (the first registration whose selector does not decline wins and the
  component receives `{...ownerProps, matched}`); `SlotRegion` serves every chain
  slot through a single `ChainHost` owner-renderer that mounts one candidate
  frame at a time and advances on `chain-unselected` (or a candidate `error`), so
  several `select === true` registrations no longer render several frames. `hoc`:
  an entry that declares `children` gets `renderSlot`/`renderSlotChain`; the
  frame reports each placeholder box (`anchor`), the shell mounts the declared
  child slot frames absolutely over it inside the parent wrapper, and reports the
  child's measured height back (`anchor-size`) so the parent reserves that space
  — children nest in the parent's layout instead of being flattened into a
  region. Shell chat view: the transcript is projected into the reference
  turn/node model (`user`/`assistant-step` nodes with text, thinking, images and
  tool calls; tool results attach to their call) and the conversation slots mount
  per node: `conversation.chat.node` (keyed by node kind; the shell's two core
  kinds stay built-in and plugin-specific kinds are replaced by their
  registration), `assistant-actions`, `message.images`/`user.images`,
  `reasoning.body`, `text-interaction`, `turnTail`, `user.before` (chain),
  `flow`, `markdown.fence` (hoc) and the `conversation.composer` chain seat.
  The shell's two core node kinds keep their built-in renderer (`user`,
  `assistant-step`) because a plugin frame cannot host the shell's rendering and
  the reference plugins fall back to a builtin component dshr does not publish —
  a documented divergence. Owner callback surfaces are wired at
  `settings.section` (`close`), `conversation.chat.node`
  (`openFile`/`inspectCall`/`forkAt`/`loadImage`) and `tool.call.toolview`
  (`openFile`/`inspectCall`). `webServer.registerUpgrade`: the sidecar keeps an
  exact-path upgrade table (duplicate path throws, the same path rules as HTTP
  routes), publishes it in `__dshr_routes` and the per-plugin `upgrades` array of
  `GET /v1/plugins/status`; dshr tunnels the upgraded connection (request-head
  replay, upstream handshake replay, bidirectional copy) under the same
  loopback/exact-route policy as HTTP routes. Evidence: bridge harness **119**
  (+12: callback materialization and resolution, deferred owner props, chain
  `matched`, hoc anchors/`anchor-size`/undeclared-child rejection, wire
  metadata), `crates/server/tests/plugins_live.rs` **18**
  (+`proxies_plugin_websocket_upgrade`: real-socket 101 handshake, byte
  tunneling, duplicate/trailing rejection, status surface), web-e2e **18/18**,
  smoke **88/88** (+owner callback closes the Settings dialog; +3 hoc-nesting probes: child frame mounts inside the parent wrapper, gets its own measured height, and renders the declared child slot), cordis 26,
  workspace 367 passed + 3 ignored (desktop 21, total 388); the shared runtime
  artifact is unchanged at 1,244,361 bytes.

- Desktop: hardening against the official `apps/desktop` shell
  (`deepseek-harness` `39163dd`, 0.2.2-rc.3). `crates/desktop/src/instance.rs`
  holds an exclusive `<home>/desktop.lock` for the process lifetime, waits up to
  3 s so a tray restart can hand the lock over, and a second launch forwards its
  folder argument through `<home>/desktop-pending-folder` before exiting (the
  running shell restarts its server on that folder). `crates/desktop/src/login_env.rs`
  runs the login shell once (`-ilc`, `env -0` between delimiters), clamps
  `DSHR_DESKTOP_LOGIN_SHELL_TIMEOUT_MS` to 1–2,147,483,647 ms (default 10,000),
  and merges the result into the child `dshr serve` environment ahead of
  `desktop.env` while filtering shell bookkeeping and shell-owned names
  (`DSHR_*`, `TAURI_*`, `DSH_HOME`) — provider keys exported from `~/.zshrc`
  now reach a GUI launch. `crates/desktop/src/cli.rs` writes
  `<home>/bin/dshr` (`dshr.cmd` on Windows), a launcher for the bundled binary
  bound to the home and active profile that is refreshed on every start and
  works while the application is closed; the tray's "Install CLI command…"
  links it into `~/.local/bin` without overwriting a foreign command. Evidence:
  desktop tests 38 (35 unit + 3 integration; was 21, +17), `dshr-desktop --check` logs
  `login shell environment: 54 variables from /bin/zsh`, fmt/clippy clean.
  Deferred by the owner: NSIS installer, Windows signing and the mandatory /
  differential update machinery of the official shell.

- C-full phase (ADR-042) opened on the owner's decision; slice C5.1 rebased the
  core chat-node renderer onto the reference SlotMap contract. The built-in
  `conversation.chat.node` view is no longer hard-coded for `user`/
  `assistant-step`: `crates/web/src/main.rs` resolves the keyed cell by priority
  (`CORE_RENDERER_PRIORITY = 0`, `chat_node_replaced_by_plugin`), so a plugin
  shadows the built-in view only with `priority < 0` (the reference
  `dsh-genui@0.3.3` registers `assistant-step` at `-1`), while the default
  priority keeps it. Only keyed cells can be shadowed; shell-owned `single`
  cells stay reserved, and built-in capture is impossible for a Leptos view in a
  sandboxed frame (recorded in PLUGINS.md; the refusing-frame fallback is C5.2).
  Evidence: 6 host-target unit tests in the web crate (`cargo test --bins`),
  `trunk build --release`, rebuilt `dshr` (embedded dist), client-half smoke
  88/88; browser e2e 17/18 locally only because the agent sandbox cannot nest
  `sandbox-exec` (CI is unaffected), and `web-e2e` gained an opt-in
  `DSHR_E2E_DANGER=1` for such environments.

- C5.2: a crashed renderer no longer leaves an empty chat node. Frame `error`
  messages are recorded by slot token (`UiState.renderer_errors`), and
  `chat_node_renderer` resolves the keyed cell to the built-in view when the
  winning plugin frame failed; the message row exposes
  `data-renderer-state="builtin|plugin"` and `data-renderer-token` for
  diagnostics. A null render stays a renderer decision (reference parity) — only
  errors fall back. Evidence: web unit tests 8 (was 6), rebuilt dist and `dshr`,
  client-half smoke 88/88; the browser probe with a crashing chat-node renderer
  ships together with C5.3 (chat and plugin in one scenario).

- C5.3: live session feed for plugin host halves. The server broadcasts the same
  session entry the internal snapshot serves (`AppState.feed` +
  `publish_session` after create, title generation and turn completion) through
  `GET /v1/internal/session-events` (SSE, `x-dshr-internal-token`, 404 when the
  bridge is disabled, `resync` on lag). The sidecar hydrates from the snapshot,
  then merges `session` events (`mergeHostSession`) and re-hydrates/reconnects
  with backoff; the fixed 5 s poll is gone. Evidence: server tests +3
  (401/404/200, creation event, usage after a finished turn), cordis
  26+1+8+2, client-half smoke 88/88; workspace 369 passed + 3 ignored.

- C5.4a: the shell's session list is event-driven. `GET /v1/sessions/events`
  (bearer token, same feed as the internal bridge) streams `session` frames, and
  `connect_session_feed` merges them into the sidebar: a known id is replaced in
  place, a new one is inserted newest-first by `created_at`, and `resync`
  re-reads `/v1/sessions`. No page action or reload is needed to see a created,
  renamed or finished session. Evidence: web unit tests 11 (+3), server 20 (+1
  public-feed auth/event test), client-half smoke 88/88; workspace 370 passed +
  3 ignored.

- C5.4b: `sessionStats.llmMs` is real instead of a zero placeholder.
  `SessionEvent::AssistantMessage` gained an optional `latency_ms` (serde
  default + skip-when-none, so pre-existing journals replay), the agent measures
  the provider call, and the session entry now carries `llm_ms`, `turns_detail`
  (per turn: steps, llm time, tokens) and per-step latency; the sidecar maps
  `llm_ms` into `sessionStats.llmMs`. There is no DSH thread model in dshr —
  subagent sessions are standalone sessions — recorded as an absence, not a
  debt. Evidence: session tests 6 (+1 backward-compatible round-trip), server 20
  (feed fields), smoke 88/88; workspace 371 passed + 3 ignored.

- C5.5: the `pluginManager` host service. A plugin's host half resolves
  `ctx.get('pluginManager')` and calls `list`/`listBundles`/`status` or
  `setPluginEnabled`/`installBundle`/`removeBundle`; the sidecar forwards those
  to `/v1/plugins/*` with the per-run internal token, and the server accepts
  `x-dshr-internal-token` on its plugin endpoints next to the user bearer
  (`authorize_service`). It is a facade over dshr's CLI-backed operations, not a
  port of the upstream pnpm manager (bundle patches, registry plans,
  `allowBuilds`, version exemptions are not reproduced). Evidence: cordis tests
  26+1+9+2 (+1: probe fixture drives list/toggle against a stub server), server
  21 (+1 internal-token), smoke 88/88; workspace 373 passed + 3 ignored.

- C5.6a: a settings store behind `configForms`. `GET/POST
  /v1/config-forms/{entry}` serve the same
  `<home>/cordis/config-forms/<entry>.json` documents the host halves already
  used: `{status, value, base, user, revision, writable}`, `ops` (`set`/`unset`,
  dot-string or array paths), a numeric `expected_revision` fence (stale form →
  409), atomic temp-file write, and internal-token access next to the user
  bearer. Entry names are validated. Browser halves still keep their form in the
  frame memory until the C5.6b host relay. Evidence: server tests 23 (+2:
  round-trip with 409, unset, persistence, bad entry, internal token), smoke
  88/88; workspace 375 passed + 3 ignored.

- C5.6b: browser `configForms` is a relay to the settings store. A frame has no
  user token and no network, so `configForms` sends `config-form-read` /
  `config-form-mutate` (with `expected_revision`) to the shell; the shell calls
  `/v1/config-forms/{entry}` with the user token and answers
  `config-form-snapshot`. The frame applies the returned snapshot, so edits
  survive a frame reload, a stale revision is rejected, and
  `useSyncExternalStore` keeps a stable snapshot. A numeric revision arriving as
  f64 is narrowed to u64 before serialization (0.0 used to be rejected as a
  float). Evidence: bridge harness 121 (+2), client-half smoke 89/89 (+1: after
  Save the settings document exists with revision >= 1); workspace 375 passed +
  3 ignored.

- C-full phase closed (ADR-042), rounds 106-114. Three directions now have a
  recorded contract and evidence: (1) core renderer semantics — the built-in
  chat-node view is a keyed-cell entry at `CORE_RENDERER_PRIORITY`, a plugin
  shadows it only with `priority < 0`, a crashed winner falls back to the
  built-in view (`data-renderer-state`), shell-owned `single` cells stay
  reserved, builtin-capture is impossible for a Leptos view in a sandboxed frame
  (boundary, not debt); (2) mounted-model sessions — internal and public
  `session` feeds, sidecar without the 5 s poll (`resync` → snapshot), shell
  applies deltas to the sidebar, real `llmMs` from per-step latency and
  `turns_detail`; there is no DSH thread model (subagent sessions are
  standalone); (3) host services — `pluginManager` facade over the CLI-backed
  operations with the internal token, settings store
  `/v1/config-forms/{entry}` with a revision fence and atomic writes, browser
  forms persisted through the shell relay. Out of C5 scope and recorded in
  LIMITATIONS/TECH-DEBT: the upstream pnpm plugin manager, the DSH configuration
  engine semantics (schema/layers/migrations), builtin capture, thread model.
  Gates: workspace 376 passed + 3 ignored, desktop 38, web 11, bridge harness
  121, client-half smoke 89; fmt/clippy/deny clean.

Known gaps: Windows AppContainer is experimental/opt-in; desktop Tauri and full i18n are deferred; full React compatibility is not a host contract (iframe bridge, ADR-013 §5): C-lite supports React slot components with host hooks/services (`ctx.get('sessions')`, plus the round-80 `ctx.get('slots')`/`ctx.get('locale')`/`ctx.inject` and `__DSHR_LAST_ERROR__` diagnostics, the round-81 React contract — `React.memo`/`forwardRef` types, keyed ids, `ctx.locale.register`/`t()`, `ctx.interval`/`ctx.timeout` — the round-82 reference slots surface check via the real `dsh-mnemon@0.5.46` and the round-83 conformance n=9 set with `dsh-plugin-market@1.2.2`, `dsh-plugin-codegraph-panel@0.6.5` and `@goodandready/dsh-plugin-notify@0.4.9`, and the round-85 C2 expansion — 30+ design-system exports and conformance n=11 with `dsh-plugin-debugging@0.8.1`/`dsh-plugin-studio@0.3.1`, whose settings UI stayed inert without the `pluginManager`/settings host services until the round-89 C1.2 seat-inject/`configForms` work made it render — browser forms are in-memory and the DSH configuration engine is not reproduced), the round-91 C1.3 reference slot contract is generated from the reference catalog (122 slots with kind/scope/standard kit; undeclared/keyed/chain/duplicate registration rules, shadowing winners, children declarations, deferred inject, standard kit + ownerProps; `chain` select dispatch and owner shares of three real shell sites were closed by the round-102 C4.1 work, while `hoc` nesting, owner props carrying functions and the full chat view were closed by the round-104 C4.2 work (the shell's two core node kinds keep a built-in renderer); store seats are closed by the round-98–101 C3.1 work), the round-92 C1.4 host services follow the reference contract (`webServer` independent exact/prefix tables, duplicate `(kind, path)` throws, absolute pathname rules, `getOrigin()`/`port`/`protocol` read from the host bridge, the `file://`-baseUrl, and the reference `configForms` snapshot `{status, value, base, user, revision, writable}` with a numeric revision fence and `set`/`unset`; the round-98–101 C1.5 work implemented `registerFallback` — one seat, non-reserved unmatched paths only — and the round-104 C4.2 work implemented `registerUpgrade` (exact-path socket protocol owners tunneled to the sidecar)) and, since the round-93 C2.1 port (with the round-94 C2.2 full Tooltip, the round-95 C2.3 Menu core, the round-96 C2.4 submenu page-layout/`MenuGroup`, the round-97 C2.5 reference `JsonBlock` plus the dependency-free `MarkdownText` GFM subset, the round-98–101 C2.6 submenu mechanics + C3.1 `@deepseek-ai/dsh-client-store`, and the round-103 C2.7 upstream markdown stack), design-system atoms, the full Tooltip, the reference Menu (core + submenu page layout + sticky groups), the reference `JsonBlock`, the upstream `MarkdownText` markdown stack (C2.7; `markdown.js`, 814,222 bytes), all 204 icons and the store-seat module following the reference sources (`primitives.js`/`bootstrap.js` are a reference port, not an independent shim; the shared runtime artifact is 1,244,361 bytes), but the full `@deepseek-ai/dsh-client-ui-*` design-system, host React context/state, the SlotMap remainder (hoc nesting, renderer semantics, owner props carrying functions without a callback bridge, per-frame chain filtering and a full chat view — all closed by the round-104 C4.2 work; chain election by `select(ownerProps)` and owner shares of three real shell sites were closed by the round-102 C4.1 work), a selectable React version and the conformance report/nightly run await the ADR-040 C-full decision, which is now an owner decision rather than unfinished C-lite work (D not recommended). C1 host HTTP routes work (sidecar loopback server + allowlist proxy + `connect-src` for opt-in frames, with the real `@kt11/dsh-deepseek-balance` host half answering through the plugin frame); round 86 added exact and prefix routes, a live per-plugin `routes` table in `GET /v1/plugins/status`, a filter for MCP pseudo-tools (`list_resources`/`read_resource`/`list_prompts`/`get_prompt`) in the plugin tool registry, and a loopback-only LAN policy (403 unless `--allow-lan-plugin-routes`). Remaining C1 limits are recorded too: C1.1 now bridges `sessions`/`sessionProjections` from an internal server snapshot (≤ 5 s staleness, simplified projections: `llmMs=0`, no thread history; session content deliberately not bridged), so usage routes see real dshr sessions; dshr's own API still has no SSE/WebSocket/streaming (plugin upgrades are tunneled since round 104), and plugin routes still lack authentication. The bridge services are a subset (snapshot-based `sessions`/`sessionProjections` with simplified projections), eval-based bundles need an opt-in `'unsafe-eval'` CSP (not enabled) and `@michengai/dsh-skills-manager` still fails at bundle init — all recorded in TECH-DEBT/LIMITATIONS.
