# Tech debt register

Обновляется по мере закрытия; приоритеты — по влиянию на безопасность и основные сценарии.

## Закрыто

| Долг | Решение |
|---|---|
| Linux shell не работал (fail-closed) | Landlock через `__sandbox-exec` — [ADR-024](adr/024-linux-landlock.md) |
| Нет общего бюджета токенов на дерево агентов | `Budget` в `AgentConfig`, флаг `--max-total-tokens` — [ADR-025](adr/025-guardrails.md) |
| Нельзя ограничить набор инструментов | `tools.allowed` / `tools.denied` — [ADR-025](adr/025-guardrails.md) |
| Два scheduler-демона могли работать одновременно | lock-файл `schedules.lock` — [ADR-025](adr/025-guardrails.md) |
| Безымянные сессии после `run` | заголовок из первой строки prompt |
| LSP-навигация отсутствовала | `dshr-lsp` — [ADR-023](adr/023-lsp.md) |
| Linux network не ограничивался | seccomp-bpf deny сетевых syscalls — [ADR-027](adr/027-linux-seccomp-network.md) |
| MCP HTTP/resources отсутствовали | Streamable HTTP + resource tools — [ADR-028](adr/028-mcp-http-resources.md) |
| Team-доска терялась, workflow не повторялся | durable state + `retries` — [ADR-029](adr/029-durable-teams-retries.md) |
| Office-документы не поддерживались | docx/xlsx/pptx tools — [ADR-030](adr/030-office.md) |
| CUA/browser отсутствовал | headless Chrome tools — [ADR-031](adr/031-browser-cua.md) |
| Поиск только сканом | FTS5-индекс по гейту — [ADR-016](adr/016-search-backend.md) |
| Не было WASM-плагинов | wasmi + JSON ABI — [ADR-032](adr/032-wasm-plugins.md) |
| Не было телеметрии | OTLP/HTTP traces — [ADR-033](adr/033-opentelemetry.md) |
| Не было ACP | ACP v1 stdio + permissions + fs — [ADR-034](adr/034-acp.md) |
| Не было i18n | TUI ru/en — [ADR-035](adr/035-tui-i18n.md) |
| Не было desktop | PWA-манифест — [ADR-036](adr/036-pwa-desktop.md) |
| MCP prompts отсутствовали | `list_prompts`/`get_prompt` — [ADR-028](adr/028-mcp-http-resources.md) |
| MCP sampling отсутствовал | stdio + HTTP GET-SSE `sampling/createMessage` — [ADR-028](adr/028-mcp-http-resources.md) |
| MCP OAuth отсутствовал | client-credentials — [ADR-028](adr/028-mcp-http-resources.md) |

## Открыто

| Приоритет | Долг | Комментарий |
|---|---|---|
| Deferred | Windows sandbox | Отложено по решению пользователя. По умолчанию fail-closed; experimental `DSHR_WINDOWS_SANDBOX` не работает (setup зависает). Нужен Win32/low-integrity backend или persistent daemon. |
| Deferred | Сеть Windows | Отложено вместе с Windows sandbox; enforcement `Partial`. |
| P2 | SDK/Leptos | JSON-RPC-подмножество и vanilla UI; rate-limit уже есть. |
| P2 | Desktop | PWA есть; Tauri-оболочки нет. |
| P2 | WASM component model/WIT | Текущий ABI — JSON core module. |
| P2 | ACP image/MCP | Permissions, fs, terminal, load/resume/close есть; image/audio и клиентские MCP — нет. |
| P3 | i18n остатка CLI | Основные help/выводы ru/en; часть аргументов, ошибок и export — en. |
| P2 | MCP OAuth refresh/elicitation | Только client-credentials без refresh. |
| P3 | WASM signature | Memory cap и sha256 есть; криптографической подписи нет. |
| P3 | cargo-dist/brew, подпись | Release workflow + Dockerfile есть; brew/подпись нет. |
