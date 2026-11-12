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

## Открыто

| Приоритет | Долг | Комментарий |
|---|---|---|
| P0 | Windows sandbox | Сейчас fail-closed: shell/jobs на Windows не выполняются. Нужны AppContainer/ACL/Job Objects. |
| P0 | Linux seccomp и сеть | Landlock ABI V1 покрывает только FS; enforcement `Partial`. |
| P1 | MCP HTTP и resources | Реализован только stdio-tools; нет resources/prompts/sampling. |
| P1 | Durable teams и workflow retries | Доска в памяти, шаги последовательны, нет повторных попыток. |
| P1 | Office и CUA | docx/xlsx/pptx, браузер/скриншоты. |
| P2 | WASM/sidecar плагины | MCP — единственный плагинный путь. |
| P2 | Поиск | Скан; FTS5/Tantivy только после бенчмарк-гейта [ADR-016](adr/016-search-backend.md). |
| P2 | SDK/Leptos/rate-limit | JSON-RPC-подмножество и vanilla UI. |
| P2 | OTel, desktop, ACP, i18n | Не начаты. |
| P3 | cargo-dist/brew, подпись | Только Dockerfile. |
