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
| Windows shell не работал (fail-closed) | AppContainer через `sandboxrs-windows` — [ADR-026](adr/026-windows-appcontainer.md) |
| Linux network не ограничивался | seccomp-bpf deny сетевых syscalls — [ADR-027](adr/027-linux-seccomp-network.md) |
| MCP HTTP/resources отсутствовали | Streamable HTTP + resource tools — [ADR-028](adr/028-mcp-http-resources.md) |
| Team-доска терялась, workflow не повторялся | durable state + `retries` — [ADR-029](adr/029-durable-teams-retries.md) |
| Office-документы не поддерживались | docx/xlsx/pptx tools — [ADR-030](adr/030-office.md) |

## Открыто

| Приоритет | Долг | Комментарий |
|---|---|---|
| P1 | Сеть Windows | AppContainer не гарантирует сеть; enforcement `Partial`. |
| P1 | CUA/browser | Навигация, скриншоты, действия. |
| P2 | MCP prompts/sampling | Tools/resources есть; server-initiated протоколы нет. |
| P2 | WASM/sidecar плагины | MCP — единственный плагинный путь. |
| P2 | Поиск | Скан; FTS5/Tantivy только после бенчмарк-гейта [ADR-016](adr/016-search-backend.md). |
| P2 | SDK/Leptos/rate-limit | JSON-RPC-подмножество и vanilla UI. |
| P2 | OTel, desktop, ACP, i18n | Не начаты. |
| P3 | cargo-dist/brew, подпись | Только Dockerfile. |
