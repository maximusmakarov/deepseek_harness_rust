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

Шаблон: [000-template.md](000-template.md).
