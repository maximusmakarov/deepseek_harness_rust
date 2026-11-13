# Совместимость с DSH

Матрица возможностей текущего среза `dshr` относительно DSH Desktop/CLI.

| Область | dshr | Статус |
|---|---|---|
| TUI | ratatui, стрим, tool-карточки, approvals | ✅ |
| Headless | `dshr run`, JSON output, cassettes | ✅ |
| HTTP/SSE + Web UI | axum, bearer, approvals, JSON-RPC, rate-limit, Leptos CSR | ✅ |
| Rust SDK | typed HTTP/SSE client | ✅ |
| ACP v1 | initialize/new/load/resume/close/prompt/cancel, permissions, fs, terminal, image/audio/resource, HTTP/stdio MCP | ✅ |
| Сессии | JSONL, resume, fork/rewind/title/search, FTS5 | ✅ |
| Промпт/AGENTS/skills/compaction | deterministic, discovery, compaction | ✅ |
| Инструменты | read/write/edit/glob/grep/bash/todo/present | ✅ |
| MCP | stdio/HTTP tools/resources/prompts/sampling (GET-SSE) | ✅ |
| Web | `web_fetch` SSRF, Brave search | ✅ |
| Jobs | background, output, kill | ✅ |
| Субагенты/workflows/teams | spawn, DAG, durable board | ✅ |
| Расписания | cron, run-due, daemon, lock | ✅ |
| LSP | hover/definition/references/diagnostics | ✅ |
| Office | docx/xlsx/pptx read/create | ✅ |
| CUA/browser | Chrome navigate/screenshot/click/type/eval | ✅ |
| WASM-плагины | wasmi core + wasmtime component/WIT, fuel, 64 MiB, sha256, Ed25519 | ✅ |
| npm/Cordis-плагины | sidecar-прототип: JS-плагины как MCP-инструменты, настоящий `cordis` при наличии пакета; без HMR/клиентской половины | ⚠️ частично |
| OTel | OTLP/HTTP traces | ✅ |
| Песочница | Seatbelt, Landlock+seccomp | ✅ |
| Windows sandbox | Отложено; default fail-closed, opt-in не работает | ⏸ deferred |
| Desktop | Tauri v2-оболочка (loopback `dshr serve`) + PWA | ✅ |
| Дистрибутивы | архивы на 5 таргетов, SHA256SUMS, Ed25519-подпись, provenance, бинарная формула | ⚠️ без ghcr-бутылок и reproducible builds |
| i18n | TUI ru/en; CLI `--lang`/`DSHR_LANG` (основные help/выводы) | ⚠️ частично |
| MCP OAuth | client-credentials | ⚠️ без refresh/elicitation |

Обозначения: ✅ есть, ⚠️ частично, ❌ нет.
