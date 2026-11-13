# Совместимость с DSH

Матрица возможностей текущего среза `dshr` относительно DSH Desktop/CLI.

| Область | dshr | Статус |
|---|---|---|
| TUI | ratatui, стрим, tool-карточки, approvals | ✅ |
| Headless | `dshr run`, JSON output, cassettes | ✅ |
| HTTP/SSE + Web UI | axum, bearer, approvals, JSON-RPC, rate-limit | ✅ |
| ACP v1 | initialize/new/load/resume/close/prompt/cancel, permissions, fs, terminal | ⚠️ без image |
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
| WASM-плагины | wasmi JSON ABI, fuel | ✅ |
| OTel | OTLP/HTTP traces | ✅ |
| Песочница | Seatbelt, Landlock+seccomp | ✅ |
| Windows sandbox | Отложено; default fail-closed, opt-in не работает | ⏸ deferred |
| Desktop | PWA (standalone), Tauri нет | ⚠️ частично |
| i18n | TUI ru/en; CLI `--lang`/`DSHR_LANG` (основные help/выводы) | ⚠️ частично |
| MCP OAuth | client-credentials | ⚠️ без refresh/elicitation |

Обозначения: ✅ есть, ⚠️ частично, ❌ нет.
