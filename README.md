# Улучшенная Rust-реализация концепции DeepSeek Harness — генеральный план

**Статус:** план v4.5 · 13.11.2026 · **M0–M7/P2 закрыты; desktop = PWA**
**Рабочее имя проекта:** `dshr` (крейты `dshr-*`, бинарь `dshr`; имя рабочее — см. [LIMITATIONS §6](docs/LIMITATIONS.md#6-юридические-и-брендовые-ограничения))
**Референс:** DeepSeek Harness `0.2.2-rc.2` (Desktop/macOS) + исходники DSH Desktop [`anywhere-labs/dsh-desktop@233b76c`](reference/REFERENCE.md) (MIT)
**Проверено:** живой DeepSeek-провайдер — [docs/PROVIDER-VERIFICATION.md](docs/PROVIDER-VERIFICATION.md) (12.11.2026)
**Репозиторий:** https://github.com/maximusmakarov/deepseek_harness_rust

---

## 1. Статус

M0–M6 (ядро) завершены; M7 — workflows, agent teams, worktrees, schedules, LSP. Песочница работает на macOS, Linux и Windows.

- **Ядро:** workspace `core`/`config`/`cli`, слои конфигурации, `dshr doctor`, CI на macOS/Ubuntu/Windows, ADR-001…024.
- **LLM:** DeepSeek Anthropic Messages и OpenAI-совместимый адаптеры, SSE-декодер, retry/backoff, отмена, кассеты и replay, живой smoke.
- **Сессии:** append-only JSONL, восстановление после оборванного хвоста, `derive_messages()`, resume, fork/rewind/title/search; опциональный FTS5-индекс.
- **Контекст:** детерминированный промпт, `AGENTS.md`, skills, compaction с событием журнала.
- **Инструменты:** реестр, схемы, `read`/`write`/`edit`/`glob`/`grep`/`bash`/`todo`/`present`, web, worktrees, LSP, Office, browser/CUA.
- **Агент:** цикл turn/step, approvals, отмена, лимит шагов, субагенты, общий бюджет токенов, allowlist инструментов.
- **Внешние:** MCP stdio/HTTP с tools, resources, prompts и stdio-sampling; `web_fetch`/`web_search`, фоновые jobs.
- **Автономность:** YAML-workflows, agent teams с доской задач, durable cron-расписания с lock.
- **Песочница:** macOS Seatbelt, Linux Landlock + seccomp; Windows — fail-closed (AppContainer experimental через `DSHR_WINDOWS_SANDBOX`).
- **Интерфейсы:** TUI, HTTP/SSE + Web UI, ACP v1 stdio.

199 тестов; живой tool-loop DeepSeek проверен. M7 продолжается: Office/CUA, OTel, desktop, ACP, i18n. План — [docs/PLAN.md](docs/PLAN.md), задачи — [docs/ROADMAP.md](docs/ROADMAP.md), границы — [docs/LIMITATIONS.md](docs/LIMITATIONS.md).

---

## 2. Что именно мы строим

DSH — это **не модель**, а обвязка вокруг модели: цикл «запрос → инструменты → ответ», журнал сессии как источник контекста, система плагинов, песочница, approvals, субагенты, MCP, TUI/Web/Desktop-интерфейсы. Модель приходит извне (DeepSeek API, OpenAI-совместимый endpoint, локальный инференс).

`dshr` — самостоятельная **улучшенная** реализация того же класса продукта: не порт DSH Desktop, а Rust-native переосмысление его концепции (подробно — [docs/IMPROVEMENTS.md](docs/IMPROVEMENTS.md)).

Ключевые отличия от референса:

- **Один статический бинарь** вместо Electron + Node + pnpm; нет обязательного GUI.
- **TUI и headless — равноправные поверхности**, Web/Desktop — опция.
- **WASM-плагины** с capability-моделью для недоверенного и переносимого кода.
- **Совместимость с npm/Cordis-плагинами DSH сохранена**: настоящий `cordis` в JS-рантайме, а его `fs`/`net`/`exec` проходят через capability-шину ([ADR-013](docs/adr/013-cordis-compat.md)).
- **Нативная песочница** и event-sourced журнал — часть ядра, а не обёртки.

```text
┌──────────────────────────────────────────────────────────────┐
│  CLI / TUI (ratatui)        HTTP+WS API        Web UI (T2)   │
├──────────────────────────────────────────────────────────────┤
│  Agent loop: turn → step → tool calls → turn end             │
│  ├─ System prompt assembly      ├─ Compaction / skills       │
│  ├─ Tool registry + approvals   ├─ Subagents / jobs          │
├──────────────────────────────────────────────────────────────┤
│  Session log (append-only, event-sourced, JSONL + SQLite)    │
├──────────────────────────────────────────────────────────────┤
│  Seams: LLM · FS · Shell · Sandbox · MCP · Plugins (WASM/Cordis) │
└──────────────────────────────────────────────────────────────┘
```

Ключевой принцип DSH, который мы наследуем: **«model-visible means logged»** — всё, что видит модель, должно восстанавливаться из журнала сессии. Это даёт resume, fork, replay, аудит и тестируемость.

---

## 3. Три уровня объёма (тиры)

| Тир | Что входит | Результат | Оценка |
|---|---|---|---|
| **T1 — MVP** | LLM-адаптеры (DeepSeek + OpenAI-совместимые), агентный цикл, инструменты fs/shell/search, журнал сессий, resume, compaction, skills, субагенты, jobs, web-инструменты, approvals, TUI, headless-режим, MCP-клиент | Полноценный локальный coding-агент, пригодный для ежедневной работы | 4–6 мес |
| **T2 — v1** | Песочница (macOS/Linux), плагины (WASM + Cordis-совместимость), MCP-сервер, LLM Anthropic/Gemini, HTTP/WS API, Web UI, SDK | Продукт, сравнимый с DSH по основным сценариям | 7–11 мес (всего) |
| **T3 — паритет + улучшения** | Workflows, расписания, agent teams, worktrees, LSP, Office, CUA/browser, телеметрия, desktop-оболочка (Tauri), ACP, мост UI-плагинов | Паритет по большинству подсистем DSH 0.2.x плюс целевые улучшения (размер, старт, TUI, WASM) | 13–21 мес (всего) |

Рекомендация: **идти по T1 → T2**, T3 — по мере необходимости. Полный паритет «всё сразу» — главный риск проекта (см. R1).

---

## 4. Документы плана

| Документ | Что внутри | Когда читать |
|---|---|---|
| [docs/PLAN.md](docs/PLAN.md) | Стратегия: принципы, scope, фазы M0–M7 (включая M5b — совместимость Cordis), критерии выхода, план спринтов, метрики, риски | Перед стартом и на каждом этапе |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Целевая архитектура: workspace крейтов, ключевые трейты, поток turn/step, хранение, плагины, маппинг DSH → Rust, ADR | Перед написанием кода |
| [docs/ROADMAP.md](docs/ROADMAP.md) | Декомпозиция по задачам с DoD и оценками | Для планирования спринтов |
| [docs/IMPROVEMENTS.md](docs/IMPROVEMENTS.md) | Чем улучшенная реализация отличается от DSH Desktop: 10 улучшений, целевые метрики, компромиссы | Перед выбором scope |
| [docs/PROVIDER-VERIFICATION.md](docs/PROVIDER-VERIFICATION.md) | Живая проверка DeepSeek: Anthropic Messages wire, auth, streaming, tool round-trip | Перед M1 |
| [docs/API.md](docs/API.md) | HTTP API, SSE, JSON-RPC, Web UI | Для интеграций и SDK |
| [docs/WORKFLOWS.md](docs/WORKFLOWS.md) | YAML-DAG шагов, шаблоны, запуск | Для мультиагентных сценариев |
| [docs/TEAMS.md](docs/TEAMS.md) | Доска задач, claim, зависимости, воркеры | Для параллельных команд |
| [docs/SCHEDULES.md](docs/SCHEDULES.md) | Cron, run-due, daemon, автономные сессии | Для регулярных задач |
| [docs/MCP.md](docs/MCP.md) | stdio/HTTP MCP, resources, plugin add | Для внешних инструментов |
| [docs/OFFICE.md](docs/OFFICE.md) | docx/xlsx/pptx чтение и создание | Для документов |
| [docs/BROWSER.md](docs/BROWSER.md) | Chrome/CUA: навигация, скриншоты, действия | Для браузера |
| [docs/SEARCH.md](docs/SEARCH.md) | Гейт, benchmark, FTS5, reindex | Для поиска |
| [docs/PLUGINS.md](docs/PLUGINS.md) | WASM ABI, манифесты, sidecar | Для расширений |
| [docs/TELEMETRY.md](docs/TELEMETRY.md) | OTLP/HTTP, env, спаны | Для наблюдаемости |
| [docs/ACP.md](docs/ACP.md) | ACP v1: сессии, prompt, cancel, permissions | Для редакторов |
| [docs/COMPATIBILITY.md](docs/COMPATIBILITY.md) | Матрица возможностей и пробелов | Для оценки паритета |
| [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) | Команды, CI, тесты | Для разработки |
| [docs/CONFIG.md](docs/CONFIG.md) | Слои и `ui.language` | Для настройки |
| [docs/DESKTOP.md](docs/DESKTOP.md) | PWA-установка, статус Tauri | Для desktop |
| [docs/LSP.md](docs/LSP.md) | Конфиг LSP, инструменты, ограничения | Для навигации по коду |
| [docs/TECH-DEBT.md](docs/TECH-DEBT.md) | Реестр техдолга и приоритеты | Для планирования |
| [docs/LIMITATIONS.md](docs/LIMITATIONS.md) | Честные ограничения: что могу, чего не могу, что нужно от пользователя | Перед принятием решения |
| [docs/adr/](docs/adr/README.md) | Architecture Decision Records 001–012 | При изменении контрактов |

---

## 5. Оценки верхнего уровня

Оценки в **идеальных инженерных днях** (1 день = 6–8 часов чистой работы), календарь = ИД / 3,5 в неделю + 15 % буфер (≈60–75 % загрузки с агентной поддержкой).

| Этап | Содержание | Идеальные дни | Календарь |
|---|---|---|---|
| M0 | Каркас: workspace, CI, конфиг, CLI, логи | 3–5 | 1–1,5 нед |
| M1 | LLM-слой, headless-чат, журнал сессий, replay | 10–16 | 3–4 нед |
| M2 | Инструменты, агентный цикл, approvals, TUI | 15–20 | 4–6 нед |
| M3 | Промпт, resume/fork, поиск, compaction, skills | 10–15 | 3–4 нед |
| M4 | MCP, web, jobs, субагенты | 10–15 | 3–4 нед |
| **T1 итого** | | **48–70** | **4–6 мес** |
| M5 | Песочница, плагины, композиция конфигов | 15–25 | 4–7 нед |
| M5b | Совместимость npm/Cordis (runtime pack, шим, conformance) | 12–18 | 3–5 нед |
| M6 | Server API, Web UI, SDK | 15–25 | 4–7 нед |
| **T2 итого** | | **90–138** | **7–11 мес** |
| M7 | Паритет: workflows, расписания, teams, LSP, CUA, desktop | 84–145 | 7–11 мес |

Погрешность оценок: **±50 %** до завершения M1, дальше ±25 %.

---

## 6. Критерии успеха

1. **Ежедневная пригодность:** агент проходит сценарий «прочитай задачу → найди код → измени → прогони тесты → покажи diff» без ручного вмешательства, с approvals на записи.
2. **Детерминизм:** любой прогон воспроизводится из кассеты (записанного потока LLM) байт-в-байт на уровне журнала сессии.
3. **Безопасность:** ни одна запись/команда не выполняется без политики; sandbox escape suite зелёный на документированном уровне enforcement; секреты не попадают в логи.
4. **Расширяемость:** новый инструмент — один крейт + регистрация, без правки ядра; новый провайдер — один адаптер.
5. **Производительность:** время до первого токена < 1 с поверх провайдера; оверхед harness на turn < 50 мс (p95), кроме вызовов модели.

---

## 7. Глоссарий

| Термин | Значение |
|---|---|
| **Turn** | Один пользовательский ход: от входа до момента, когда агенту больше нечего делать. |
| **Step** | Один запрос к модели + все инструменты, вызванные в нём. Turn = 0..N шагов. |
| **Seam (шов)** | Заменяемая способность: объявление интерфейса + провайдер + потребитель. |
| **Session log** | Append-only журнал событий сессии — источник контекста для модели. |
| **Approval** | Одноразовое подтверждение пользователем опасного действия. |
| **Profile / bundle** | Композиция плагинов: профиль — именованный набор, бандл — поставляемый набор строк конфигурации. |
| **Skill** | Каталог с `SKILL.md`, подгружаемый в контекст по требованию. |

---

## 8. Быстрый старт

```sh
cargo build
./target/debug/dshr --version
./target/debug/dshr doctor
./target/debug/dshr config show --schema

# живой прогон: нужен DSH_DEEPSEEK_TOKEN или DEEPSEEK_API_KEY
./target/debug/dshr run "Explain the repository layout"

# с инструментами и авто-approval (осторожно: разрешает bash)
./target/debug/dshr run --danger "Create hello.txt and read it back"

# интерактивный TUI
./target/debug/dshr tui
./target/debug/dshr prompt

# workflow и team из .dshr/workflows / .dshr/teams
./target/debug/dshr run --danger "use workflow review with input '.'"
./target/debug/dshr run --danger "use team release"

# расписания
./target/debug/dshr schedule add --name nightly --cron "0 0 2 * * *" --prompt "Run nightly checks"
./target/debug/dshr schedule run-due

# сервер + Web UI
./target/debug/dshr serve --port 55678
# открыть http://127.0.0.1:55678 и вставить токен из вывода

# плагины (MCP)
./target/debug/dshr plugin add fs --command npx --arg -y --arg @modelcontextprotocol/server-filesystem --arg .
./target/debug/dshr plugin list

# сессии
./target/debug/dshr sessions fork <id> --at 5
./target/debug/dshr sessions rewind <id> --turn 1
./target/debug/dshr sessions search "hello" --json

# офлайн-прогон по кассете
./target/debug/dshr run --cassette cassette.json "hi"

# сессии
./target/debug/dshr sessions list
./target/debug/dshr sessions show <session-id>
./target/debug/dshr sessions export <session-id> --format markdown
```

Дерево:

```text
crates/core           ids, DshrError, redaction
crates/config         слои defaults < profile < user < patch
crates/llm            Message/StreamChunk/SSE/assembler/provider trait
crates/llm-deepseek   Anthropic Messages (DeepSeek)
crates/llm-openai     OpenAI-compatible (RouterAI и др.)
crates/llm-mock       cassette replay + mock SSE server
crates/session        события, derive_messages, проекции
crates/session-jsonl  append-only JSONL, repair, export
crates/tools          registry + read/write/edit/glob/grep/bash/todo/present
crates/agent          turn/step loop, approvals, observers
crates/prompt         секции промпта, AGENTS.md, детерминизм
crates/skills         discovery, каталог, skill tool
crates/compaction     оценка токенов и суммаризация
crates/mcp            MCP stdio-клиент и адаптеры инструментов
crates/jobs           фоновые процессы и job_* инструменты
crates/sandbox        политика, Seatbelt и Landlock
crates/server         HTTP/SSE API, JSON-RPC и встроенный UI
crates/workflow       YAML-DAG шагов через субагентов
crates/team           общая доска задач и конкурентные воркеры
crates/schedule       durable cron-расписания
crates/search         FTS5-индекс сессий
crates/wasm           WASM-плагины
crates/acp            ACP v1 stdio-сервер
crates/i18n           TUI ru/en
crates/office         docx/xlsx/pptx
crates/browser        headless Chrome CUA
crates/lsp            LSP-клиент и инструменты навигации
crates/tui            ratatui terminal UI
crates/cli            run, tui, prompt, sessions, doctor, config show
.github/workflows     fmt, clippy, nextest, deny, coverage
docs/adr              001–036
```

## 9. Что дальше

**Осталось:**
1. Windows sandbox (persistent helper), сеть Windows.
2. ACP fs/terminal, WASM component model/WIT, MCP GET-SSE/OAuth.
3. Tauri-оболочка (отложена).

Критерии выхода — [docs/PLAN.md §5](docs/PLAN.md#5-фазы), задачи — [docs/ROADMAP.md](docs/ROADMAP.md).
