# Architecture Decision Records

Решения, влияющие больше чем на один крейт или публичный контракт. Новые ADR добавляются сюда и в таблицу ниже.

| ADR | Решение | Статус |
|---|---|---|
| [001](001-session-format.md) | Свой формат сессий | Accepted |
| [002](002-plugin-abi.md) | ABI плагинов: WASM + sidecar | Accepted, уточнён [013](013-cordis-compat.md) |
| [003](003-ui-stack.md) | TUI сейчас, Web на Leptos, Tauri позже | Accepted |
| [004](004-storage.md) | JSONL — истина, SQLite — индекс | Accepted |
| [005](005-concurrency.md) | tokio + mpsc-актор журнала | Accepted |
| [006](006-schemas.md) | schemars + jsonschema | Accepted |
| [007](007-naming-license.md) | Имя и лицензия | Accepted |
| [008](008-token-meter.md) | Счётчик токенов | Accepted |
| [009](009-sandbox-scope.md) | Сеть и exec в политике песочницы | Accepted |
| [010](010-deepseek-wire.md) | DeepSeek official — Anthropic Messages | Accepted |
| [011](011-desktop.md) | Desktop — Tauri над `dshr serve` | Proposed |
| [012](012-yaml-parser.md) | YAML-парсер: serde-saphyr | Accepted |
| [013](013-cordis-compat.md) | Совместимость с npm/Cordis-плагинами рядом с WASM | Accepted |
| [014](014-tool-approvals.md) | Инструменты, approvals и агентный цикл | Accepted |
| [015](015-context-management.md) | Промпт, AGENTS.md, skills и compaction | Accepted |
| [016](016-search-backend.md) | Поиск: скан сейчас, индекс по гейту | Accepted |
| [017](017-external-tools.md) | MCP, web, jobs и субагенты: границы M4 | Accepted |
| [018](018-sandbox-plugins.md) | Песочница и плагины: границы M5 | Accepted |
| [019](019-server-ui.md) | Сервер, Web UI и SDK: границы M6 | Accepted |
| [020](020-workflows-worktrees.md) | Workflows и worktrees: первый срез M7 | Accepted |
| [021](021-agent-teams.md) | Agent teams: доска, владение, зависимости | Accepted |
| [022](022-schedules.md) | Расписания: durable cron и автономный запуск | Accepted |
| [023](023-lsp.md) | LSP: минимальный клиент и инструменты | Accepted |
| [024](024-linux-landlock.md) | Linux Landlock: helper и границы изоляции | Accepted |
| [025](025-guardrails.md) | Бюджет дерева, allowlist инструментов, scheduler lock | Accepted |
| [026](026-windows-appcontainer.md) | Windows AppContainer через helper | Accepted |
| [027](027-linux-seccomp-network.md) | Linux seccomp: сетевая изоляция | Accepted |
| [028](028-mcp-http-resources.md) | MCP: HTTP-транспорт и resources | Accepted |
| [029](029-durable-teams-retries.md) | Durable teams и retries workflow | Accepted |
| [030](030-office.md) | Office: docx, xlsx, pptx | Accepted |
| [031](031-browser-cua.md) | Browser/CUA: headless Chrome | Accepted |
| [032](032-wasm-plugins.md) | WASM-плагины: wasmi и JSON ABI | Accepted |
| [033](033-opentelemetry.md) | OpenTelemetry: OTLP/HTTP traces | Accepted |
| [034](034-acp.md) | ACP v1 server | Accepted |
| [035](035-tui-i18n.md) | TUI i18n ru/en | Accepted |

Шаблон: [000-template.md](000-template.md).
