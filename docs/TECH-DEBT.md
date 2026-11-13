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
| ACP audio/stdio MCP отсутствовали | audio-блоки и stdio MCP из `session/new` — [ADR-034](adr/034-acp.md) |
| Не было криптоподписи WASM | Ed25519 над именем и sha256, `plugins.trusted_keys` — [ADR-032](adr/032-wasm-plugins.md) |
| Не было `ask_user` | observer-канал в TUI/Web, fail-closed в headless — `dshr-agent` |
| Не было `apply_patch` | multi-file `*** Begin Patch` (add/update/delete/move), тесты — `dshr-tools` |
| Jobs не переживали рестарт | состояние и логи в `<home>/jobs`, running → interrupted при загрузке |
| Не было component model/WIT | wasmtime + WIT-мир `dshr:plugin@1.0.0`, пустой WASI; legacy wasmi сохранён — [ADR-032](adr/032-wasm-plugins.md) |
| Не было desktop-оболочки | `crates/desktop`: Tauri v2 над loopback `dshr serve` — [ADR-011](adr/011-desktop.md) |
| Не было Leptos UI | `crates/web` (Leptos CSR, trunk), dist встроен в сервер — [ADR-003](adr/003-ui-stack.md) |
| ACP не принимал embedded `resource` | текст/blob → text/image/audio — [ADR-034](adr/034-acp.md) |
| ACP игнорировал `resource_link` | блок разворачивается в текст с именем/URI — [ADR-034](adr/034-acp.md) |
| Не было стабильных бинарных дистрибутивов | Архивы на 5 таргетов, SHA256SUMS, Ed25519-подпись, provenance, бинарная формула — [ADR-039](adr/039-release-artifacts.md) |

## Открыто

| Приоритет | Долг | Комментарий |
|---|---|---|
| Deferred | Windows sandbox | Отложено по решению пользователя. По умолчанию fail-closed; experimental `DSHR_WINDOWS_SANDBOX` не работает (setup зависает). Нужен Win32/low-integrity backend или persistent daemon. |
| Deferred | Сеть Windows | Отложено вместе с Windows sandbox; enforcement `Partial`. |
| P3 | i18n остатка CLI | Help, sessions/show/export, plugin/release — ru/en; сообщения ошибок и часть справки остаются en. |
| P2 | MCP elicitation | OAuth refresh (expiry + 401 retry) есть; `elicitation/create` не поддержан. |
| P2 | Cordis/npm-совместимость | Sidecar-прототип есть (`dshr-cordis`); нет HMR, кэша грантов/аудита, полного набора Node-builtins и клиентской UI-половины. |
| P3 | ghcr-бутылки и reproducible builds | Бинарная формула, суммы и подпись есть; `brew bottle` и байтовая воспроизводимость — нет — [ADR-039](adr/039-release-artifacts.md). |
