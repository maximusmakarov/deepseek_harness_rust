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
| npm/Cordis-плагины | sidecar (системный Node) + `plugin npm add/remove/update`; настоящий `cordis` при наличии пакета; hot reload (watcher + `/v1/plugins/reload`), capability-гранты `fs`; клиентская половина: eager registry-кадр на половину + iframe на слот, полный словарь DSH 0.2.2-rc.2 (116 ключей) → 17 регионов, протокол v1.2; экспериментальный `dsh.client.runtime` — плагин приносит свою модульную таблицу/UI-стек, включая React; PoC тира B `dsh.client.host_modules` — половина опционально берёт React 18.3.1 и таблицу модулей от хоста одним immutable-артефактом `/v1/runtime/react-18.3.1.js`; C-lite (раунд 79) — при host React `ctx.slots.register(entry, Component)` рендерится как React-компонент (один root на кадр, error boundary), host-hooks `useTheme`/`useSession`/`useSessions`/`useHostState`/`wide`, `ctx.get('sessions')`, частичный design-system шим `@deepseek-ai/dsh-client-ui-primitives`; раунд 80 — сервисы моста `ctx.get('slots')`/`ctx.get('locale')`, `ctx.inject(deps, callback)` и диагностика `__DSHR_LAST_ERROR__`, conformance n=2 (`@kt11/dsh-deepseek-balance@0.2.1` и `dsh-remote@1.1.2`), при этом eval-based бандлы требуют `'unsafe-eval'` в CSP (сознательно не включён); полного design-system, React-контекста шелла и SlotMap-семантики по-прежнему нет — C-full ([ADR-040](adr/040-react-client-half.md)) | ⚠️ частично |
| OTel | OTLP/HTTP traces | ✅ |
| Песочница | Seatbelt, Landlock+seccomp | ✅ |
| Windows sandbox | Отложено; fail-closed по умолчанию, AppContainer — opt-in (`DSHR_WINDOWS_SANDBOX`), CI-шаг `continue-on-error` | ⏸ deferred |
| Desktop | Tauri v2-оболочка (loopback `dshr serve`) + PWA | ✅ |
| Дистрибутивы | архивы на 5 таргетов, SHA256SUMS, Ed25519-подпись, provenance, бинарная формула, bottles в GHCR, reproducible-бинарь; notarization нет | ⚠️ частично |
| i18n | TUI/CLI ru/en: справка (`plugin npm`, `sessions import`), рантайм-сообщения `sessions`/`plugin`/`doctor`, префикс ошибок; `--json` не зависит от языка | ✅ |
| MCP OAuth | client-credentials + refresh токена | ✅ |

Обозначения: ✅ есть, ⚠️ частично, ❌ нет.
