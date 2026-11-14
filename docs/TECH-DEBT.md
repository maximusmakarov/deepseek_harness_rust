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
| Не было LLM-заголовков | `generate_title` (DSH-промпт), `session.auto_title` — `dshr-agent` |
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
| Deferred | Windows sandbox и сеть | Отложено по решению пользователя: нужен реальный Windows-хост. По умолчанию fail-closed; `DSHR_WINDOWS_SANDBOX` — opt-in, его CI-шаг помечен `continue-on-error` (AppContainer setup на hosted runner'ах зависает; прогон виден в summary, но не блокирует trunk). Enforcement сети на Windows — `Partial`. |
| Out of scope | Полный набор React-слотов клиентских плагинов | Решение ADR-013 §5: React-совместимости нет, есть iframe-мост. Имена слотов подмножества (`panel`, `settings.section`, `message.action`, `settings.action`) проходят через мост (smoke 29/29); региональный монтаж слотов требует eager-монтажа и iframe на слот — отдельная фаза. Replica 98.0/100. |
| P2 | Notarization | Тулинг есть (`scripts/notarize-app.sh`), нужны Apple Developer ID, сертификат и app-specific password; без них распространяется ad-hoc подпись. |
| P2 | GHCR-бутылки и brew-прогон | OCI-publisher и формула есть, push/pull подтверждены против mock и в Actions; нет tap-репозитория и учётных данных пакетов, brew-прогон на реальном GHCR не делался. |
| P2 | Центральная база отзывов plugin market | Отзывы серверные и общие для всех клиентов одного инстанса (`<home>/plugin-reviews.json`), но центральной/федеративной базы между инстансами нет. |
| P3 | Экран настроек провайдера в Web UI | Маршрут/модель/ключ настраиваются через CLI и `desktop.env`; в Web UI экрана нет. |
| Done | Визуальный diff в Web UI | Карточка `tool_start` показывает `-old`/`+new` для `edit`, добавленные строки для `write` и тело патча для `apply_patch` (сервер передаёт input инструмента); полный файловый diff с контекстом остаётся в TUI. |
| Done | i18n остатка CLI | Закрыто: TUI/CLI, справка (`plugin npm`, `sessions import`), рантайм-сообщения `sessions`/`plugin`/`doctor`/`run` и префикс ошибок локализованы; тексты `DshrError`/`ToolError` намеренно остаются en, `--json` не зависит от языка. |
