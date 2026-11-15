# Совместимость с DSH

Матрица возможностей текущего среза `dshr` относительно DSH Desktop/CLI.

| Область | dshr | Статус |
|---|---|---|
| TUI | ratatui, стрим, tool-карточки, approvals | ✅ |
| Headless | `dshr run`, JSON output, cassettes | ✅ |
| HTTP/SSE + Web UI | axum, bearer, approvals, JSON-RPC, rate-limit, Leptos CSR | ✅ |
| Rust SDK | typed HTTP/SSE client | ✅ |
| ACP v1 | initialize/new/load/resume/close/prompt/cancel, permissions, fs, terminal, image/audio/resource, HTTP/stdio MCP | ✅ |
| Сессии | JSONL, resume, fork/rewind/title/search, LLM-заголовки, FTS5, импорт DSH v4 | ✅ |
| Промпт/AGENTS/skills/compaction | deterministic, discovery, compaction | ✅ |
| Инструменты | read/write/edit/apply_patch/glob/grep/bash/todo/ask_user/present | ✅ |
| MCP | stdio/HTTP tools/resources/prompts/sampling (GET-SSE), elicitation в stdio и HTTP | ✅ |
| Web | `web_fetch` SSRF, Brave search | ✅ |
| Jobs | background, output, kill | ✅ |
| Субагенты/workflows/teams | spawn, DAG, durable board | ✅ |
| Расписания | cron, run-due, daemon, lock | ✅ |
| LSP | hover/definition/references/diagnostics | ✅ |
| Office | docx/xlsx/pptx read/create | ✅ |
| CUA/browser | Chrome navigate/screenshot/click/type/eval | ✅ |
| WASM-плагины | wasmi core + wasmtime component/WIT, fuel, 64 MiB, sha256, Ed25519 | ✅ |
| npm/Cordis-плагины | sidecar (системный Node) + `plugin npm add/remove/update`; настоящий `cordis` при наличии пакета; hot reload (watcher + `/v1/plugins/reload`), capability-гранты `fs`; клиентская половина: eager registry-кадр на половину + iframe на слот, полный словарь DSH 0.2.2-rc.2 (116 ключей) → 17 регионов, протокол v1.2; экспериментальный `dsh.client.runtime` — плагин приносит свою модульную таблицу/UI-стек, включая React; PoC тира B `dsh.client.host_modules` — половина опционально берёт React 18.3.1 и таблицу модулей от хоста одним immutable-артефактом `/v1/runtime/react-18.3.1.js`; C-lite (раунд 79) — при host React `ctx.slots.register(entry, Component)` рендерится как React-компонент (один root на кадр, error boundary), host-hooks `useTheme`/`useSession`/`useSessions`/`useHostState`/`wide`, `ctx.get('sessions')`, частичный design-system шим `@deepseek-ai/dsh-client-ui-primitives`; раунд 80 — сервисы моста `ctx.get('slots')`/`ctx.get('locale')`, `ctx.inject(deps, callback)` и диагностика `__DSHR_LAST_ERROR__`, conformance n=2 (`@kt11/dsh-deepseek-balance@0.2.1` и `dsh-remote@1.1.2`); раунд 81 — React-контракт моста (типы `React.memo`/`forwardRef`, keyed-ids `<slot>:<key>`, `ctx.get('slots').entries` с `component`/`options`, `ctx.locale.register` + `t()`, `ctx.interval`/`ctx.timeout`), primitives шим из 7 компонентов (`JsonBlock`/`MarkdownText`) и conformance n=4 (`@ruoyang/dsh-plugin-deepseek-balance@1.0.2`, `dsh-genui@0.3.3`); раунд 82 — conformance n=6 (`dsh-session-superagent@0.10.12` — composer dock `conversation.composer.dock`, `dsh-mnemon@0.5.46` — крупный multi-region, shadow-регистрации сверяются через `ctx.slots.subscribe`/`getVersion`/`entriesOfSlot`), раунд 83 — conformance **n=9**: цель ADR-013 §6 (8–12 реальных плагинов) достигнута (`@99galaxy/dsh-plugin-market@1.2.2` — запись в сайдбаре, `dsh-plugin-codegraph-panel@0.6.5` — панель `settings.section`, `@goodandready/dsh-plugin-notify@0.4.9` — строки `plugins.item`/`plugins.row.config`), при этом eval-based бандлы требуют `'unsafe-eval'` в CSP (сознательно не включён), а `@michengai/dsh-skills-manager` падает на инициализации — оба вне набора; раунд 84 — **C1**: host HTTP-маршруты плагинов работают (sidecar loopback-сервер + allowlist-прокси dshr-сервера; exact-маршруты, с раунда 86 — и prefix; `connect-src` на origin сервера для кадров с `host_modules`, реальная host-половина `@kt11/dsh-deepseek-balance` отвечает своим JSON), остатки — SSE/WebSocket/стриминг и отсутствие аутентификации на plugin-маршрутах (LAN-экспозиция закрыта в раунде 86); раунд 85 — **C2**: design-system шим вырос с 7 до 30+ экспортов по фактическим импортам (компоненты `Input`/`Switch`/`Checkbox`/`Tag`/`Pill`/`Menu`/`MenuSurface`/`Modal`/`Toast`/`SegmentedControl`/`SegmentedTabs`/`DisclosureRow`/`PathLabel`/`TextShimmer`, именованные `Icon*` с Proxy-fallback, `writeClipboard`, хуки `useAnchoredPosition`/`useDismissOnOutsidePointer`), conformance **n=11** (`dsh-plugin-debugging@0.8.1`, `dsh-plugin-studio@0.3.1` активируются без ошибок, их settings-UI оставался инертным без host-сервисов `pluginManager`/settings до C1.2); раунд 86 — **хардненинг C1**: prefix-маршруты (`kind: "exact"`/`"prefix"`, exact выигрывает, среди prefix — самый длинный), живая таблица `routes` в `GET /v1/plugins/status`, фильтр MCP-псевдо-инструментов из реестра инструментов плагина, loopback-only политика plugin-маршрутов (403 при bind вне loopback без `--allow-lan-plugin-routes`), раунд 87 — **C1.1**: мост `sessions`/`sessionProjections` (DSH-shaped сервисы из снимка сервера, обновление раз в 5 с); раунд 89 — **C1.2**: seat-`inject` в регистрациях слотов, Cordis-генераторы в `slots.inject`/`ctx.inject`/`ctx.effect` и host-сервис `configForms` (in-memory writable-форма в кадре, файловая — у host-половин), поэтому settings-UI `dsh-plugin-studio` (вкладки в Settings) и `dsh-plugin-debugging` (карточка в Plugins с переключателем и Save) реально рендерится; полного design-system, React-контекста шелла и SlotMap-семантики по-прежнему нет — C-full ([ADR-040](adr/040-react-client-half.md)) остаётся решением владельца — [ADR-041](adr/041-c-full-decision.md): отложен, точечные инкременты | ⚠️ частично |
| OTel | OTLP/HTTP traces | ✅ |
| Песочница | Seatbelt, Landlock+seccomp | ✅ |
| Windows sandbox | Отложено; fail-closed по умолчанию, AppContainer — opt-in (`DSHR_WINDOWS_SANDBOX`), CI-шаг `continue-on-error` | ⏸ deferred |
| Desktop | Tauri v2-оболочка (loopback `dshr serve`) + PWA | ✅ |
| Дистрибутивы | архивы на 5 таргетов, SHA256SUMS, Ed25519-подпись, provenance, бинарная формула, bottles в GHCR, reproducible-бинарь; notarization нет | ⚠️ частично |
| i18n | TUI/CLI ru/en: справка (`plugin npm`, `sessions import`), рантайм-сообщения `sessions`/`plugin`/`doctor`, префикс ошибок; `--json` не зависит от языка | ✅ |
| MCP OAuth | client-credentials + refresh токена | ✅ |

Обозначения: ✅ есть, ⚠️ частично, ❌ нет.
