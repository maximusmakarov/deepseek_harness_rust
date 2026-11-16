# Улучшенная Rust-реализация концепции DeepSeek Harness — генеральный план

**Статус:** план v5.5 · 15.11.2026 · **M0–M7 закрыты; Leptos + Tauri; WIT; Cordis — sidecar, hot reload, клиентская половина с региональным монтажом слотов, экспериментальными `dsh.client.runtime`, общим рантаймом хоста `dsh.client.host_modules` (тир B PoC) и C-lite (React-компоненты слотов, host-hooks, сервисы моста `ctx.get('slots')`/`locale`/`ctx.inject`, React-контракт моста, design-system примитивы: с раунда 93 (C2.1) атомы и все 204 иконки портированы из reference, с раунда 94 (C2.2) — полный `Tooltip`, с раунда 95 (C2.3) — ядро `Menu`/`MenuItemButton`, с раунда 96 (C2.4) — reference submenu page-layout и `MenuGroup`, с раунда 97 (C2.5) — reference `JsonBlock` и dependency-free `MarkdownText`-подмножество, с раундов 98–101 — C2.6 submenu-механика (side-card/single-page по измеренной ширине, `fixed` flip/clamp, `activeList`, Escape/ArrowLeft, 200-мс hover grace), C3.1 store-seats (`@deepseek-ai/dsh-client-store` в module-таблице, `useStore`/`actions`, inject-аргументы `(sessionId, actions)`), C1.5 fallback-маршруты (`webServer.registerFallback`) и ownerProps-plumbing (`?ownerProps={...}` → `frame.ownerProps` → props; артефакт 430 139 байт), а раунд 102 (C4.1) — chat/turn-модель шелла (turn ordinal в `data-turn` и owner share), owner shares трёх реальных render-сайтов (`sidebar.session.action`, `message.action` для user, `composer.actions`) и chain-элекцию в мосте (`select(ownerProps)`; `false` → пустой DOM + `chain-unselected`), conformance n=11 — цель ADR-013 §6 8–12 реальных плагинов достигнута), host HTTP-маршрутами плагинов (C1: sidecar-сервер + allowlist-прокси, `connect-src` на origin сервера для opt-in кадров; раунд 86 — хардненинг: prefix-маршруты, `routes` в статусе, фильтр MCP-псевдо-инструментов, loopback-only политика с opt-in для LAN; раунд 87 — C1.1: мост `sessions`/`sessionProjections` из внутреннего снимка сервера, usage-маршруты реальных плагинов считают статистику по сессиям dshr; раунд 89 — C1.2: seat-`inject` в регистрациях слотов, Cordis-генераторы (`slots.inject`/`ctx.inject`/`ctx.effect`) и host-сервис `configForms` — in-memory writable-форма в кадре, файловая у host-половин, поэтому settings-UI `dsh-plugin-studio`/`dsh-plugin-debugging` реально рендерится; раунд 91 — C1.3: слот-контракт по референсному каталогу DSH (122 слота с kind/scope/standard-kit, правила `SlotCore`: undeclared/keyed/chain/duplicate, shadowing-winners, children-декларации, deferred inject, standard-kit + ownerProps); раунд 92 — C1.4: host-сервисы `webServer`/`ctx.baseUrl`/`configForms` по reference-контракту (правила exact/prefix-маршрутов и duplicate-throw, origin dshr-сервера из host-bridge, `file://`-baseUrl, snapshot `{status, value, base, user, revision, writable}` с числовой ревизией и `set`/`unset`; `registerFallback`/`registerUpgrade` не поддержаны, browser-`configForms` in-memory, живого RPC-зеркала настроек нет))**
**Рабочее имя проекта:** `dshr` (крейты `dshr-*`, бинарь `dshr`; имя рабочее — см. [LIMITATIONS §6](docs/LIMITATIONS.md#6-юридические-и-брендовые-ограничения))
**Референс:** DeepSeek Harness `0.2.2-rc.3` — официальный upstream
[`deepseek-harness@39163dd`](reference/REFERENCE.md) (`apps/desktop`,
`packages/client/*`) и community [`anywhere-labs/dsh-desktop@233b76c`](reference/REFERENCE.md)
(MIT)
**Проверено:** живой DeepSeek-провайдер — [docs/PROVIDER-VERIFICATION.md](docs/PROVIDER-VERIFICATION.md) (12.11.2026)
**Репозиторий:** https://github.com/maximusmakarov/deepseek_harness_rust

---

## 1. Статус

M0–M6 (ядро) завершены; M7 — workflows, agent teams, worktrees, schedules, LSP. Песочница работает на macOS, Linux и Windows.

- **Ядро:** workspace `core`/`config`/`cli`, слои конфигурации, `dshr doctor`, CI на macOS/Ubuntu/Windows, ADR-001…041.
- **LLM:** DeepSeek Anthropic Messages и OpenAI-совместимый адаптеры, SSE-декодер, retry/backoff, отмена, кассеты и replay, живой smoke.
- **Сессии:** append-only JSONL, восстановление после оборванного хвоста, `derive_messages()`, resume, fork/rewind/title/search; опциональный FTS5-индекс.
- **Контекст:** детерминированный промпт, `AGENTS.md`, skills, compaction с событием журнала.
- **Инструменты:** реестр, схемы, `read`/`write`/`edit`/`apply_patch`/`glob`/`grep`/`bash` (persistent shell)/`todo`/`ask_user`/`present`, web, worktrees, LSP, Office, browser/CUA.
- **Агент:** цикл turn/step, approvals, отмена, лимит шагов, субагенты, общий бюджет токенов, allowlist инструментов.
- **Внешние:** MCP stdio/HTTP с tools, resources, prompts и stdio-sampling; `web_fetch`/`web_search`, фоновые jobs.
- **Автономность:** YAML-workflows, agent teams с доской задач, durable cron-расписания с lock.
- **Песочница:** macOS Seatbelt, Linux Landlock + seccomp; Windows — fail-closed (AppContainer experimental через `DSHR_WINDOWS_SANDBOX`).
- **Интерфейсы:** TUI, HTTP/SSE + Web UI, ACP v1 stdio.

366 тестов (+3 ignored), desktop — 38, web — 6 unit (итого 410); живой tool-loop DeepSeek проверен.
Desktop подтянут к официальной оболочке `apps/desktop` (раунд 105): single-instance
lock с форвардом папки, окружение логин-шелла для GUI-запуска и CLI-launcher
`<home>/bin/dshr`; NSIS/подпись/обновления официальной оболочки отложены владельцем. M7 продолжается: Windows sandbox, notarization, conformance-набор Cordis (сейчас n=11 — цель ADR-013 §6 8–12 реальных плагинов достигнута с запасом; остаются nightly-прогон и отчёт матрицы), C2.1–C2.6-остатки (upstream markdown-стек, renderer-семантика; submenu-механика закрыта в C2.6, store seats — в C3.1; host-сервисы `pluginManager`/settings), C1-остатки host-маршрутов (стриминг, отсутствие аутентификации на plugin-маршрутах; LAN-экспозиция закрыта loopback-only политикой с opt-in `--allow-lan-plugin-routes`; сервисы `sessions`/`sessionProjections` с раунда 87 (C1.1) работают из внутреннего снимка сервера с задержкой ≤ 5 с и упрощёнными проекциями; раунд 89 (C1.2) добавил seat-`inject`, Cordis-генераторы и `configForms`, поэтому settings-UI `dsh-plugin-studio`/`dsh-plugin-debugging` теперь рендерится, но browser-форма in-memory, а семантика конфигурационного движка DSH не воспроизведена), а раунд 91 (C1.3) перевёл слот-контракт на референсный каталог DSH (122 слота с kind/scope/standard-kit, правила `SlotCore`: undeclared/keyed/chain/duplicate, shadowing-winners, children-декларации, deferred inject, standard-kit + ownerProps), оставив как долг chain-элекцию, hoc-вложение, store seats и ownerProps от реальных shell-сайтов (chain-элекция по `select(ownerProps)` и owner shares трёх реальных сайтов закрыты позже — в C4.1, раунд 102; store seats — в C3.1), а раунд 92 (C1.4) выровнял host-сервисы `webServer` (правила маршрутов, origin-аксессоры), `ctx.baseUrl` и `configForms` (reference-shape snapshot с числовой ревизией и `set`/`unset`) по reference-контракту, а раунд 93 (C2.1) портировал атомы design-system и все 204 иконки из reference-исходников (`deepseek-ai/deepseek-harness` @ `39163dd`, `packages/client/ui-primitives/src`; артефакт на конец раунда 375 894 байта), а раунд 94 (C2.2) довёл `Tooltip` до полного reference-порта (placement-engine, `ShortcutKeys`, `input-modality`; артефакт 387 613 байт), а раунд 95 (C2.3) портировал ядро `Menu`/`MenuItemButton` по reference (ряды/selection/aria, клавиатурный walk, one-level submenu, dismissal; артефакт 405 323 байта), а раунд 96 (C2.4) — reference submenu page-layout (single-page back-row, Escape/ArrowLeft на родителя), 200-мс hover grace и `MenuGroup`/`observeStickyMenuGroups` (артефакт 415 972 байта), а раунд 97 (C2.5) — reference `JsonBlock` и API-совместимое dependency-free `MarkdownText`-подмножество, а с раундов 98–101 — C2.6 submenu-механика, C3.1 store-seats (`@deepseek-ai/dsh-client-store` в module-таблице, `useStore`/`actions`, inject-аргументы), C1.5 fallback-маршруты (`webServer.registerFallback`) и ownerProps-plumbing (артефакт 430 139 байт), а раунд 102 (C4.1) — chat/turn-модель шелла (turn ordinal в `data-turn` и owner share), owner shares трёх реальных render-сайтов и chain-элекцию в мосте (`select(ownerProps)`; `false` → пустой DOM + `chain-unselected`), оставив как частичности upstream markdown-стек, renderer-семантику, `hoc`-вложение, owner props с функциями без callback-моста, per-frame фильтрацию цепочек и полный chat-view для `conversation.chat.node`, а также `registerUpgrade` (WebSocket); bridge harness 107 (+2 chain-select в C4.1), web-e2e 18/18 (+2: turn ordinal и смонтированные `composer.actions`/`message.action` регионы)). План — [docs/PLAN.md](docs/PLAN.md), задачи — [docs/ROADMAP.md](docs/ROADMAP.md), границы — [docs/LIMITATIONS.md](docs/LIMITATIONS.md).

---

## 2. Что именно мы строим

DSH — это **не модель**, а обвязка вокруг модели: цикл «запрос → инструменты → ответ», журнал сессии как источник контекста, система плагинов, песочница, approvals, субагенты, MCP, TUI/Web/Desktop-интерфейсы. Модель приходит извне (DeepSeek API, OpenAI-совместимый endpoint, локальный инференс).

`dshr` — самостоятельная **улучшенная** реализация того же класса продукта: не порт DSH Desktop, а Rust-native переосмысление его концепции (подробно — [docs/IMPROVEMENTS.md](docs/IMPROVEMENTS.md)).

Ключевые отличия от референса:

- **Один статический бинарь** вместо Electron + Node + pnpm; нет обязательного GUI.
- **TUI и headless — равноправные поверхности**, Web/Desktop — опция.
- **WASM-плагины** с capability-моделью для недоверенного и переносимого кода.
- **Совместимость с npm/Cordis-плагинами DSH** ([ADR-013](docs/adr/013-cordis-compat.md)): настоящий `cordis` в sidecar-рантайме, его `fs`/`net`/`exec` — через capability-шину, hot reload; клиентская половина — eager registry-кадр + отдельный iframe на слот, каталог DSH 0.2.2-rc.2 (с раунда 91 — 122 слота с kind/scope/standard-kit; плюс legacy `panel`/`message.action`, всего 124 ключа) разложен по 17 регионам шелла, незнакомое имя — fallback в панель. Экспериментальный `dsh.client.runtime` позволяет половине принести свою модульную таблицу/UI-стек, включая React; PoC тира B `dsh.client.host_modules` — опционально взять React 18.3.1 и таблицу модулей от хоста одним immutable-артефактом на страницу; C-lite (раунды 79–85, включая C2) — при host React `ctx.slots.register(entry, Component)` рендерится как React-компонент с host-hooks (`useTheme`/`useSession`/`useSessions`/`useHostState`/`wide`) и `ctx.get('sessions')`, сервисы моста `ctx.get('slots')`/`ctx.get('locale')`/`ctx.inject` и диагностика `__DSHR_LAST_ERROR__`, React-контракт моста (типы `React.memo`/`forwardRef`, keyed-ids `<slot>:<key>`, `ctx.locale.register`/`t()`, `ctx.interval`/`ctx.timeout`), design-system примитивы `@deepseek-ai/dsh-client-ui-primitives`, с раунда 93 (C2.1) портированные из reference-исходников (`deepseek-ai/deepseek-harness` @ `39163dd`, `packages/client/ui-primitives/src`): атомы `Button`/`Input`/`Switch`/`Checkbox`/`Tag`/`Pill`/`SegmentedControl`/`SegmentedTabs`/`DisclosureRow`/`TextShimmer`/`StateDot`/`PathLabel`/`Toast`/`Modal`/`Menu`/`MenuSurface`/`Tooltip` и все **204** `Icon*` с точными SVG-путями (инлайн вместо спрайта), артефакт на конец раунда 375 894 байта; с раунда 94 (C2.2) `Tooltip` — полный reference-порт (placement-engine, `ShortcutKeys`, `input-modality`), артефакт 387 613 байт; с раунда 95 (C2.3) `Menu`/`MenuItemButton` — ядро по reference (ряды/selection/aria, reserve-icon, клавиатурный walk, one-level submenu, dismissal, `usePointerGrace`), артефакт 405 323 байта; с раунда 96 (C2.4) — reference submenu page-layout (side-карточка или single-page с non-selecting back-row, Escape/ArrowLeft на родителя), 200-мс hover grace между карточками и `MenuGroup`/`observeStickyMenuGroups`, а с раунда 97 (C2.5) — reference `JsonBlock` и dependency-free `MarkdownText`-подмножество, а с раундов 98–101 — C2.6 submenu-механика (side-card/single-page по измеренной ширине, `fixed` flip/clamp, `activeList`, Escape/ArrowLeft на родителя, 200-мс hover grace, `data-menu-back`), C3.1 store-seats (`@deepseek-ai/dsh-client-store` в module-таблице: `createSnapshotStore`/`defineStore`/`shallowEqual`; `store: handle|factory`, framework-owned instance на scope-key, `useStore`/`actions`, inject-аргументы `(sessionId, actions)`), C1.5 fallback-маршруты (`webServer.registerFallback`, одно место) и ownerProps-plumbing (`?ownerProps={...}` → `frame.ownerProps` → props), артефакт 430 139 байт, а с раунда 102 (C4.1) — chat/turn-модель шелла (turn ordinal в `data-turn` и owner share), owner shares трёх реальных render-сайтов и chain-элекцию в мосте (`select(ownerProps)`; `false` → пустой DOM + `chain-unselected`); при этом остаются upstream markdown-стек (mdast/shiki/KaTeX/footnotes/streaming), renderer-семантика, `hoc`-вложение, `registerUpgrade` (WebSocket), owner props с функциями без callback-моста, per-frame фильтрация цепочек и полный chat-view, и conformance **n=11** — цель ADR-013 §6 (8–12 реальных плагинов) достигнута с запасом (`@kt11/dsh-deepseek-balance@0.2.1`, `dsh-remote@1.1.2`, `@ruoyang/dsh-plugin-deepseek-balance@1.0.2`, `dsh-genui@0.3.3`, `dsh-session-superagent@0.10.12` — composer dock, `dsh-mnemon@0.5.46` — крупный multi-region и референсная поверхность слотов, `dsh-plugin-market@1.2.2` — сайдбар, `dsh-plugin-codegraph-panel@0.6.5` — панель настроек, `@goodandready/dsh-plugin-notify@0.4.9` — строки plugins-страницы, `dsh-plugin-debugging@0.8.1` и `dsh-plugin-studio@0.3.1` — примитивы-heavy половины: активируются без ошибок, их settings-UI был инертен без host-сервисов `pluginManager`/settings); с раунда 84 — **C1**: host HTTP-маршруты плагинов работают end-to-end (sidecar loopback-сервер, allowlist-прокси dshr-сервера, `connect-src` на origin сервера для кадров с `host_modules`; реальная host-половина `@kt11/dsh-deepseek-balance` отвечает своим JSON), а раунд 86 (хардненинг C1) добавил prefix-маршруты (exact выигрывает, среди prefix — самый длинный), живую таблицу `routes` в `GET /v1/plugins/status`, фильтр MCP-псевдо-инструментов в реестре инструментов плагина и loopback-only политику (403 при bind вне loopback без `--allow-lan-plugin-routes`), а раунд 87 — **C1.1**: sidecar получает DSH-shaped `sessions`/`sessionProjections` из внутреннего снимка сервера (`/v1/internal/session-snapshot`, токен в `<home>/cordis/host.json`, обновление раз в 5 с; проекции упрощены, содержимое сессий не мостится), поэтому usage-маршруты реальных плагинов считают статистику по сессиям dshr, а раунд 89 — **C1.2**: seat-`inject` в регистрациях слотов, Cordis-генераторы в `slots.inject`/`ctx.inject`/`ctx.effect` и host-сервис `configForms` (in-memory writable-форма в кадре, файловая — у host-половин), поэтому settings-UI `dsh-plugin-studio` (реальные вкладки в Settings) и `dsh-plugin-debugging` (карточка в Plugins с переключателем и Save) рендерится; остатки — стриминг, отсутствие аутентификации на plugin-маршрутах; eval-based бандлы требуют ослабления CSP — записанное ограничение (полная React-совместимость — полный design-system, React-контекст шелла и SlotMap-остатки (`hoc`-вложение, renderer-семантика, owner props с функциями, per-frame фильтрация цепочек и полный chat-view; chain-элекция и owner shares трёх реальных сайтов — в C4.1, раунд 102; store seats закрыты в C3.1, раунды 98–101; каталог/регистрация/kinds/winners/children/deferred inject закрыты в C1.3, раунд 91; host-сервисы `webServer`/`ctx.baseUrl`/`configForms` выровнены по reference-контракту в C1.4, раунд 92; `registerFallback` закрыт в C1.5, `registerUpgrade` — нет) — остаётся решением владельца по C-full ([ADR-041](docs/adr/041-c-full-decision.md): отложен, точечные инкременты) [ADR-040](docs/adr/040-react-client-half.md), Proposed).
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
| [docs/PLUGINS.md](docs/PLUGINS.md) | WASM ABI, манифесты, sidecar, клиентская половина (слоты/регионы, мост) | Для расширений |
| [docs/TELEMETRY.md](docs/TELEMETRY.md) | OTLP/HTTP, env, спаны | Для наблюдаемости |
| [docs/ACP.md](docs/ACP.md) | ACP v1: сессии, prompt, cancel, permissions | Для редакторов |
| [docs/COMPATIBILITY.md](docs/COMPATIBILITY.md) | Матрица возможностей и пробелов | Для оценки паритета |
| [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) | Команды, CI, тесты | Для разработки |
| [docs/SERVER.md](docs/SERVER.md) | Auth, rate-limit, API | Для сервера |
| [docs/SDK.md](docs/SDK.md) | Rust SDK: сессии, turn, SSE | Для интеграций |
| [docs/INSTALL.md](docs/INSTALL.md) | Готовые бинарники, Homebrew, systemd/launchd, Docker | Для установки |
| [docs/RELEASE.md](docs/RELEASE.md) | Архивы, SHA256SUMS, Ed25519-подпись, provenance | Для релизов |
| [docs/CONFIG.md](docs/CONFIG.md) | Слои и `ui.language` | Для настройки |
| [docs/DESKTOP.md](docs/DESKTOP.md) | Tauri, трей, профили, PWA, диагностика | Для desktop |
| [docs/PARITY.md](docs/PARITY.md) | Матрица паритета с DSH Desktop | Для планирования паритета |
| [docs/LSP.md](docs/LSP.md) | Конфиг LSP, инструменты, ограничения | Для навигации по коду |
| [docs/TECH-DEBT.md](docs/TECH-DEBT.md) | Реестр техдолга и приоритеты | Для планирования |
| [docs/LIMITATIONS.md](docs/LIMITATIONS.md) | Честные ограничения: что могу, чего не могу, что нужно от пользователя | Перед принятием решения |
| [docs/adr/](docs/adr/README.md) | Architecture Decision Records 001–040 | При изменении контрактов |

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
crates/sdk            типизированный HTTP/SSE клиент
crates/office         docx/xlsx/pptx
crates/browser        headless Chrome CUA
crates/lsp            LSP-клиент и инструменты навигации
crates/tui            ratatui terminal UI
crates/web            Leptos CSR web UI (trunk)
crates/desktop        Tauri v2 desktop shell over dshr serve
crates/cordis         JS/npm plugin sidecar (Cordis prototype)
crates/cli            run, tui, prompt, sessions, doctor, config show
.github/workflows     fmt, clippy, nextest, deny, coverage
docs/adr              001–041
```

## 9. Что дальше

**Осталось:**
1. Windows sandbox (persistent helper), сеть Windows.
2. Tauri: трей, автообновление, notarization (бандл `.app` уже собирается).
3. Полный Cordis: runtime pack и шим `@dshr/cordis-compat@1`, кэш грантов; conformance-набор реальных плагинов — n=11, цель ADR-013 §6 (8–12) достигнута с запасом (HMR, аудит, региональный монтаж слотов, экспериментальный `dsh.client.runtime`, PoC тира B `dsh.client.host_modules` и C-lite/C2 — React-компоненты слотов с host-hooks, сервисами `ctx.get('slots')`/`locale`/`ctx.inject`, React-контрактом моста, design-system примитивами, портированными из reference в C2.1–C2.6 (атомы и все 204 иконки, с C2.2 — полный `Tooltip`, с C2.3 — ядро `Menu`/`MenuItemButton`, с C2.4 — reference submenu page-layout, 200-мс hover grace и `MenuGroup`/`observeStickyMenuGroups`, с C2.5 — reference `JsonBlock` и dependency-free `MarkdownText`-подмножество, с C2.6 (раунды 98–101) — полная submenu-механика, C3.1 store-seats и C1.5 fallback-маршруты; остаются upstream markdown-стек и renderer-семантика), одиннадцатью реальными плагинами, C1 host HTTP-маршрутами плагинов и C1.3 слот-контрактом по референсному каталогу (122 слота, правила `SlotCore`) — уже есть); остаются host-сервисы `pluginManager`/settings для UI `dsh-plugin-debugging`/`dsh-plugin-studio`; остаются nightly-прогон и отчёт матрицы, а также C1-остатки (стриминг, отсутствие аутентификации на plugin-маршрутах; сервисы `sessions`/`sessionProjections` с раунда 87 (C1.1) работают из снимка с задержкой ≤ 5 с, prefix-маршруты, таблица `routes` в статусе, фильтр MCP-псевдо-инструментов и loopback-only политика — с раунда 86). Полная React-совместимость (полный design-system, React-контекст шелла, SlotMap-остатки: `hoc`-вложение, renderer-семантика, owner props с функциями, per-frame фильтрация цепочек и полный chat-view; chain-элекция и owner shares трёх реальных сайтов закрыты в C4.1, раунд 102, store seats — в C3.1; каталог/регистрация/kinds/winners/children/deferred inject закрыты в C1.3, раунд 91; host-сервисы `webServer`/`ctx.baseUrl`/`configForms` выровнены по reference-контракту в C1.4, раунд 92, а `registerFallback`/`registerUpgrade` остаются неподдержанными) — уже решение владельца по C-full [ADR-040](docs/adr/040-react-client-half.md) (D не рекомендуется).
4. ghcr-бутылки Homebrew (сборка уже детерминирована и воспроизводима).

Критерии выхода — [docs/PLAN.md §5](docs/PLAN.md#5-фазы), задачи — [docs/ROADMAP.md](docs/ROADMAP.md).
