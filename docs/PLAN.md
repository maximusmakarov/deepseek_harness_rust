# Генеральный план разработки `dshr` — улучшенной Rust-реализации DeepSeek Harness

Версия 1.0 · 12.11.2026 · референс DSH 0.2.2-rc.2

---

## 1. Цель и контекст

### 1.1. Цель

Создать самостоятельную **улучшенную** Rust-реализацию концепции DeepSeek Harness в слое «агентная обвязка»: цикл turn/step, инструменты, журнал сессий, права и песочница, MCP, субагенты, TUI/Web-интерфейсы, плагины.

Это не порт `dsh-desktop`: сохраняем продуктовые идеи (тонкий host, «всё есть плагин», журнал как истина), но устраняем Electron/Node/pnpm, делаем один бинарь, TUI/headless первого класса, WASM-плагины и сохраняем совместимость с плагинами DSH. Полное обоснование и метрики — [IMPROVEMENTS.md](IMPROVEMENTS.md).

### 1.2. Почему это осмысленно

- **Rust даёт** предсказуемую производительность, малый бинарь, безопасную работу с конкурентностью, нативные биндинги к ОС-песочницам (seatbelt/landlock/seccomp/Job Objects), удобную упаковку.
- **DSH доказывает архитектуру**: всё разложено на швы (seams), ядро отсутствует как привилегированный модуль. Такую архитектуру можно воспроизвести на любом языке.
- **Ниша**: harness на Rust нужен там, где важны один статический бинарь, отсутствие Node-рантайма, жёсткая песочница и встраивание в другие продукты.
- **Улучшение, а не копия**: тот же класс продукта при меньшем размере, памяти и времени старта; TUI и headless без GUI; WASM-тир для недоверенного кода при сохранённой совместимости Cordis/npm (см. [IMPROVEMENTS.md](IMPROVEMENTS.md)).

### 1.3. Что НЕ является целью

- Обучение/инференс модели. `dshr` — оркестратор; модель приходит по API или из локального рантайма (llama.cpp/candle) как отдельный провайдер.
- Побайтовый порт кода DSH Desktop, его промптов, текстов интерфейса и бренда.
- Electron/Chromium и обязательный GUI: desktop-оболочка — опциональная Tauri-надстройка над `dshr serve`.
- Полная совместимость клиентских UI-плагинов DSH (React-компоненты как контракт шелла) — вне scope; host-плагины Cordis получают отдельный рантайм, UI — через региональный мост iframe ([ADR-013](adr/013-cordis-compat.md)). Enabler `dsh.client.runtime` (раунд 77) позволяет половине принести свой UI-стек, включая React, PoC тира B `dsh.client.host_modules` (раунд 78) — опционально взять React 18.3.1 и таблицу модулей от хоста одним immutable-артефактом, C-lite (раунд 79) — React-компоненты слотов с host-hooks/`ctx.get('sessions')` и частичным design-system шимом для простых UI-плагинов, раунд 80 — сервисы моста (`ctx.get('slots')`/`locale`, `ctx.inject`, диагностика `__DSHR_LAST_ERROR__`) и conformance n=2 (`dsh-remote@1.1.2`), раунд 81 — доработки React-контракта (типы `React.memo`/`forwardRef`, keyed-ids, `ctx.locale.register` + `t()`, `ctx.interval`/`ctx.timeout`), primitives шим из 7 компонентов и conformance n=4 (`@ruoyang/dsh-plugin-deepseek-balance@1.0.2`, `dsh-genui@0.3.3`), раунд 82 — conformance n=6 (`dsh-session-superagent@0.10.12` — composer dock, `dsh-mnemon@0.5.46` — крупный multi-region и референсная поверхность слотов `subscribe`/`getVersion`/`entriesOfSlot`), раунд 83 — conformance **n=9**, то есть целевой диапазон ADR-013 §6 (8–12 реальных плагинов) закрыт (`dsh-plugin-market@1.2.2` — сайдбар, `dsh-plugin-codegraph-panel@0.6.5` — панель настроек, `@goodandready/dsh-plugin-notify@0.4.9` — строки plugins-страницы), раунд 84 — **C1**: host HTTP-маршруты плагинов работают end-to-end (loopback-сервер sidecar, allowlist-прокси dshr-сервера, `connect-src` на origin сервера для кадров с `host_modules`) с записанными остатками (сервисы `sessions`/`sessionProjections`, SSE/WebSocket/стриминг, отсутствие аутентификации на plugin-маршрутах), раунд 85 — **C2**: design-system шим расширен с 7 до 30+ экспортов по фактическим импортам реальных плагинов (компоненты `Input`/`Switch`/`Checkbox`/`Tag`/`Pill`/`Menu`/`MenuSurface`/`Modal`/`Toast`/`SegmentedControl`/`SegmentedTabs`/`DisclosureRow`/`PathLabel`/`TextShimmer`, именованные `Icon*` с Proxy-fallback, `writeClipboard`, хуки `useAnchoredPosition`/`useDismissOnOutsidePointer`) и conformance n=11 (`dsh-plugin-debugging@0.8.1`, `dsh-plugin-studio@0.3.1` активируются без ошибок, их settings-UI оставался инертным без host-сервисов `pluginManager`/settings), раунд 86 — хардненинг C1: prefix-маршруты (exact выигрывает), живая таблица `routes` в `GET /v1/plugins/status`, фильтр MCP-псевдо-инструментов и loopback-only политика plugin-маршрутов с opt-in `--allow-lan-plugin-routes`; раунд 87 — **C1.1**: сервисы `sessions`/`sessionProjections` в sidecar из внутреннего снимка сервера (`/v1/internal/session-snapshot`, обновление раз в 5 с), поэтому usage-маршруты реальных плагинов считают статистику по сессиям dshr (проекции упрощены, содержимое сессий не мостится), раунд 89 — **C1.2**: seat-`inject` в регистрациях слотов, Cordis-генераторы в `slots.inject`/`ctx.inject`/`ctx.effect` и host-сервис `configForms` (in-memory writable-форма в кадре, файловая — у host-половин), поэтому settings-UI `dsh-plugin-studio`/`dsh-plugin-debugging` реально рендерится (studio — вкладки в Settings, debugging — карточка в Plugins), раунд 91 — **C1.3**: слот-контракт по сгенерированному каталогу DSH (122 слота с `kind`/`scope`/standard-kit; правила `SlotCore` — undeclared/keyed/chain/duplicate, shadowing-winners, `children`-декларации, deferred `ctx.slots.inject`, standard-kit + `ownerProps`, `ctx.slots.spec(key)`); раунд 92 (**C1.4**) выровнял host-сервисы `webServer`/`ctx.baseUrl`/`configForms` по reference-контракту, раунд 93 (**C2.1**) портировал атомы design-system и все 204 иконки из reference-исходников, раунд 94 (**C2.2**) — полный `Tooltip` (placement-engine, `ShortcutKeys`, `input-modality`), а раунд 95 (**C2.3**) — ядро `Menu`/`MenuItemButton` (ряды/selection/aria, reserve-icon, `ShortcutKeys`, footer/`children`-ряды, one-level submenu, клавиатурный walk, dismissal, `usePointerGrace`/`focusWithoutRing`, portal-позиционирование; общий рантайм-артефакт 405 323 байта), раунд 96 (**C2.4**) — reference submenu page-layout (side-карточка или single-page с non-selecting back-row `data-menu-back` и Escape/ArrowLeft на родителя), 200-мс hover grace между карточками и `MenuGroup`/`observeStickyMenuGroups` (общий рантайм-артефакт 415 972 байта), а раунд 97 (**C2.5**) — reference `JsonBlock` и API-совместимое dependency-free GFM-подмножество `MarkdownText` (общий рантайм-артефакт 424 801 байт), а раунды 98–101 — C2.6 submenu-механику (side-card/single-page по измеренной ширине, `fixed`-позиционирование с flip/clamp, `activeList`-маршрутизация клавиатуры, Escape/ArrowLeft на родителя, 200-мс hover grace, `data-menu-back`), C3.1 store-seats (`@deepseek-ai/dsh-client-store` в module-таблице: `createSnapshotStore`/`defineStore`/`shallowEqual`; `store: handle|factory`, framework-owned instance на scope-key, `useStore(selector, eq)` + baked `actions`, inject-аргументы `(sessionId, actions)`), C1.5 fallback-маршруты (`webServer.registerFallback`, одно место) и ownerProps-plumbing (`?ownerProps={...}` → `frame.ownerProps` → props после store/seat share; общий рантайм-артефакт — 430 139 байт), а раунд 102 (C4.1) добавил chat/turn-модель шелла (turn ordinal в `data-turn` и owner share), owner shares трёх реальных render-сайтов (`sidebar.session.action`, `message.action` для user, `composer.actions`) и chain-элекцию в мосте (`select(ownerProps)`; `false` → пустой DOM + `chain-unselected`), поэтому остаются upstream markdown-стек (mdast/shiki/KaTeX/footnotes/streaming), renderer-семантика, `hoc`-вложение, `registerUpgrade` (WebSocket), owner props с функциями без callback-моста, per-frame фильтрация цепочек и полный chat-view для `conversation.chat.node`); полная React-совместимость (полный design-system, React-контекст, SlotMap-остатки: `hoc`-вложение, renderer-семантика; chain-элекция и owner shares трёх сайтов поставлены в C4.1; store seats закрыты в C3.1) — [ADR-040](adr/040-react-client-half.md) (Proposed, C-full — решение владельца, не остаток C-lite; по [ADR-041](adr/041-c-full-decision.md) отложен, дальше точечные инкременты).
- Паритет с DSH 1:1 по каждой экспериментальной функции (voice, translation, liveshare, CUA) — только по явному запросу.

---

## 2. Принципы

| № | Принцип | Следствие для реализации |
|---|---|---|
| P1 | **Model-visible means logged** | Любой вход модели — событие журнала. `derive_messages()` — единственный путь построения истории. |
| P2 | **Швы вместо ядра** | LLM, FS, shell, sandbox, MCP, tools — трейты + провайдеры. Ядро не знает о конкретных реализациях. |
| P3 | **Fail-closed** | Неизвестная политика, ошибка песочницы, отсутствие approval → отказ, а не «разрешим на всякий случай». |
| P4 | **Детерминизм тестов** | Cassette replay + mock-сервер. Ни один тест не требует живой сети. |
| P5 | **Один писатель журнала** | Запись событий сериализуется; читатели — broadcast-подписка. |
| P6 | **Маленькие крейты, явные зависимости** | Зависимости направлены только вниз: `ui/server → agent → tools/session/llm → core` (стрелка — «зависит от»). |
| P7 | **Расширяемость без патча ядра** | Новый инструмент/провайдер/команда — регистрация, а не правка цикла. |
| P8 | **Безопасность по умолчанию** | Read-only старт, запись только в workspace, сеть/секреты — по явной политике. |
| P9 | **Наблюдаемость** | `tracing` + OTLP, идентификаторы сессий/шагов/вызовов во всех логах. |
| P10 | **Совместимость по возможностям, не по байтам** | Импорт/экспорт сессий DSH — опционально, через адаптер, а не как основной формат. |

---

## 3. Архитектурная стратегия (кратко)

Полная схема — в [ARCHITECTURE.md](ARCHITECTURE.md). Здесь — три решения, определяющие всё остальное.

### 3.1. Event sourcing вместо «массива сообщений»

DSH хранит сессию как типизированный append-only лог (`SessionEvent`), из которого выводится история модели. Мы делаем то же:

- события: `turn/start`, `user/message`, `assistant/message`, `tool/call`, `tool/result`, `turn/end`, `compaction/*`, `permission/*`;
- проекции: `messages`, `timeline`, `usage`, `todo`, `jobs`;
- resume/fork/replay — бесплатно, из журнала.

### 3.2. Четыре уровня расширяемости

| Уровень | Механизм | Пример |
|---|---|---|
| Компилируемый | Крейт + регистрация в композиции | Встроенные инструменты, провайдеры |
| Динамический | WASM-компонент (wasmtime), capability-grants | Сторонние инструменты/хуки без пересборки |
| Совместимый | Cordis-рантайм (JS): настоящий `cordis` + npm-пакеты, builtins через capability-шину | Существующие плагины DSH, TS-разработка, HMR |
| Внешний | MCP (stdio/HTTP), sidecar JSON-RPC | Чужие серверы инструментов, плагины на любом языке |

Все четыре тира живут под одним швом `PluginRuntime`: единый манифест, capability-модель, жизненный цикл (health-check, горячая замена, откат), один реестр вкладов и один аудит. Инструмент или команда любого тира видны остальным через host-посредник. Решение по совместимости — [ADR-013](adr/013-cordis-compat.md).

### 3.3. Один движок — несколько поверхностей

TUI, headless и HTTP API используют один и тот же `AgentHandle` и подписку на события. UI не влияет на логику; сервер — тонкий адаптер.

---

## 4. Scope: матрица возможностей

Легенда: **T1** — MVP (4–6 мес), **T2** — v1 (7–11 мес, включая M5b), **T3** — паритет + улучшения (13–21 мес). `—` — не планируется. Колонка DSH: ✓ — подтверждено документацией 0.2.2-rc.2, «частично» — поддержка ограничена или не подтверждена.

Колонки T1/T2 — исходный план; **T3 — фактическое состояние на 15.11.2026**: ✅ закрыто и проверено,
⚠️ закрыто частично (ограничение указано в строке), ⏸ сознательно отложено/заблокировано внешним условием.

| Возможность | DSH | T1 | T2 | T3 (факт) |
|---|---|---|---|---|
| LLM: DeepSeek Messages, OpenAI-совместимые | ✓ | ✓ | ✓ | ✅ DeepSeek (Anthropic wire) + OpenAI-совместимые |
| LLM: Anthropic, Gemini, локальные | частично | — | ✓ | ⚠️ отдельные адаптеры Anthropic/Gemini и локальные рантаймы не заведены; Anthropic-совместимый wire используется DeepSeek |
| Стриминг, reasoning-контент, usage | ✓ | ✓ | ✓ | ✅ |
| Агентный цикл turn/step, отмена | ✓ | ✓ | ✓ | ✅ |
| Инструменты: fs (read/write/edit), glob, grep | ✓ | ✓ | ✓ | ✅ + `apply_patch` (DSH-формат) |
| Инструменты: bash (в т.ч. persistent), jobs | ✓ | bash, jobs | ✓ | ✅ persistent bash на Unix, jobs durable |
| Инструменты: web_search / web_fetch | ✓ | ✓ | ✓ | ✅ (SSRF-guard) |
| Инструменты: todo, ask_user, present | ✓ | ✓ | ✓ | ✅ `ask_user` в TUI/Web/ACP |
| Инструменты: subagent, workflow | ✓ | subagent | ✓ | ✅ + teams |
| Инструменты: schedule, goal, skill, terminal | ✓ | skill | — | ⚠️ schedule/skill/terminal-шимы есть, отдельного `goal`-инструмента нет |
| Журнал сессий, resume, list, export | ✓ | ✓ | ✓ | ✅ |
| Fork, rewind, поиск по сессиям (FTS) | ✓ | ✓ | ✓ | ✅ FTS5 |
| Compaction (сжатие контекста) | ✓ | ✓ | ✓ | ✅ live-гейт 100k |
| Approvals + политики прав | ✓ | ✓ | ✓ | ✅ |
| Песочница ОС (Seatbelt / bwrap+Landlock / ACL) | ✓ | — | macOS/Linux | ⏸ macOS Seatbelt, Linux Landlock+seccomp — ✅; Windows AppContainer — experimental, нужен реальный Windows-хост |
| MCP-клиент (tools/resources) | ✓ | ✓ | ✓ | ✅ stdio/HTTP, resources, prompts, sampling, elicitation, OAuth refresh |
| MCP-сервер | — (клиент) | — | ✓ | ✅ |
| Субагенты (spawn/fork, дерево, сообщения) | ✓ | ✓ | ✓ | ✅ |
| Фоновые задачи (jobs) | ✓ | ✓ | ✓ | ✅ durable |
| Skills (`SKILL.md`) | ✓ | ✓ | ✓ | ✅ |
| Плагины: WASM + Cordis-совместимость + sidecar | Cordis/TS | — | ✓ (host-половина) | ⚠️ WASM + Cordis-sidecar + host-половина ✅; клиентская половина — eager registry-кадр + iframe на слот, каталог DSH 0.2.2-rc.2 (с раунда 91 — 122 слота с `kind`/`scope`/standard-kit; плюс legacy `panel`/`message.action`, всего 124 ключа) разложен по 17 регионам шелла, незнакомое имя — fallback в панель (раунд 76); с раунда 77 — экспериментальный `dsh.client.runtime` (половина приносит свою модульную таблицу/React), с раунда 78 — PoC тира B `dsh.client.host_modules` (React 18.3.1 и таблица модулей от хоста); C-lite (раунды 79–85): при host React `ctx.slots.register(entry, Component)` — React-компонент с host-hooks (`useTheme`/`useSession`/`useSessions`/`useHostState`/`wide`) и `ctx.get('sessions')`, сервисы `ctx.get('slots')`/`ctx.get('locale')`/`ctx.inject`, диагностика `__DSHR_LAST_ERROR__`, design-system примитивы `@deepseek-ai/dsh-client-ui-primitives` (с раунда 93, C2.1 — reference-порт: атомы и все 204 иконки из `deepseek-ai/deepseek-harness` @ `39163dd`; с раунда 94, C2.2 — полный `Tooltip`; с раунда 95, C2.3 — ядро `Menu`/`MenuItemButton`; с раунда 96, C2.4 — reference submenu page-layout, 200-мс hover grace и `MenuGroup`/`observeStickyMenuGroups`; с раунда 97, C2.5 — reference `JsonBlock` и API-совместимое dependency-free `MarkdownText`-подмножество; раунды 98–101 — C2.6 submenu-механика, C3.1 store-seats, C1.5 fallback-маршруты и ownerProps-plumbing; общий рантайм-артефакт 430 139 байт), React-контракт моста (типы `React.memo`/`forwardRef`, keyed-ids `<slot>:<key>`, `locale.register`/`t()`, `ctx.interval`/`ctx.timeout`) и одиннадцать реальных DSH-плагинов в conformance (n=11, цель ADR-013 §6 достигнута с запасом); с раунда 84 — **C1**: host HTTP-маршруты плагинов работают (sidecar-сервер + allowlist-прокси, `connect-src` origin для opt-in кадров), реальная host-половина `@kt11/dsh-deepseek-balance` отвечает своим JSON, а раунд 86 (хардненинг C1) добавил prefix-маршруты (exact выигрывает), `routes` в `GET /v1/plugins/status`, фильтр MCP-псевдо-инструментов и loopback-only политику с opt-in `--allow-lan-plugin-routes`; раунд 87 — **C1.1**: сервисы `sessions`/`sessionProjections` из внутреннего снимка сервера (usage-маршруты реальных плагинов считают статистику по сессиям dshr; задержка ≤ 5 с, проекции упрощены); остатки — стриминг, отсутствие аутентификации на plugin-маршрутах; с раунда 85 — **C2**: design-system шим вырос с 7 до 30+ экспортов (компоненты, именованные `Icon*` с Proxy-fallback, `writeClipboard`, хуки), conformance n=11 — `dsh-plugin-debugging`/`dsh-plugin-studio` активируются без ошибок, их settings-UI был инертен без host-сервисов `pluginManager`/settings, но с раунда 89 (C1.2) рендерится через seat-`inject` и `configForms` и честно деградирует; раунд 91 (C1.3) — слот-контракт по референсному каталогу DSH (122 слота, правила `SlotCore`, children/deferred inject, winners, standard-kit + ownerProps), раунд 92 (C1.4) — host-сервисы `webServer`/`ctx.baseUrl`/`configForms` по reference-контракту (правила маршрутов и origin-аксессоры, `file://`-baseUrl, snapshot с числовой ревизией и `set`/`unset`; `registerFallback`/`registerUpgrade` не поддержаны), раунд 93 (C2.1) — атомы design-system и все 204 иконки портированы из reference-исходников (`deepseek-ai/deepseek-harness` @ `39163dd`, `packages/client/ui-primitives/src`), общий рантайм-артефакт на конец раунда 375 894 байта, а раунд 94 (C2.2) — полный reference-порт `Tooltip` (placement-engine, `ShortcutKeys`, `input-modality`; артефакт 387 613 байт), а раунд 95 (C2.3) — ядро `Menu`/`MenuItemButton` (артефакт 405 323 байта), а раунд 96 (C2.4) — reference submenu page-layout, 200-мс hover grace и `MenuGroup`/`observeStickyMenuGroups` (артефакт 415 972 байта), а раунд 97 (C2.5) — reference `JsonBlock` и API-совместимое dependency-free GFM-подмножество `MarkdownText` (артефакт 424 801 байт), а раунды 98–101 (C2.6 submenu-механика, C3.1 store-seats, C1.5 fallback-маршруты и ownerProps-plumbing) — 430 139 байт, а раунд 102 (C4.1) добавил chat/turn-модель шелла (turn ordinal в `data-turn` и owner share), owner shares трёх реальных render-сайтов (`sidebar.session.action`, `message.action` для user, `composer.actions`) и chain-элекцию в мосте (`select(ownerProps)`; `false` → пустой DOM + `chain-unselected`); остаются upstream markdown-стек (mdast/shiki/KaTeX/footnotes/streaming), renderer-семантика, `hoc`-вложение, `registerUpgrade` (WebSocket), owner props с функциями (нет callback-моста), per-frame фильтрация цепочек (несколько `select === true` рендерят несколько кадров) и полный chat-view для `conversation.chat.node`; полная React-совместимость (полный design-system, React-контекст, SlotMap-остатки: `hoc`-вложение, renderer-семантика; chain-элекция и owner shares трёх реальных сайтов поставлены в C4.1 (раунд 102); store seats закрыты в C3.1) — C-full ADR-040 |
| Профили/бандлы/патчи конфигурации | ✓ | базово | ✓ | ✅ |
| TUI | ✓ | ✓ | ✓ | ✅ |
| Headless / one-shot | ✓ | ✓ | ✓ | ✅ |
| HTTP/WS API | ✓ | — | ✓ | ✅ |
| Web UI (чат, diff, approvals) | ✓ | — | ✓ | ✅ чат/стрим/карточки/approvals (e2e), превью diff (`-old`/`+new`) и диалог настроек провайдера (маршрут/модель/base_url, без ключей) |
| Desktop (Tauri) | Electron | — | — | ⚠️ macOS `.app`+DMG ✅; Linux deb/tar.gz и Windows zip собираются, NSIS-инсталлятора нет |
| SDK (JSON-RPC, Python/TS) | ✓ | — | ✓ | ✅ |
| ACP | ✓ | — | — | ✅ v1 (сессии, permissions, fs, terminal, MCP, elicitation) |
| Workflows (скриптовые мультиагентные сценарии) | ✓ | — | — | ✅ |
| Расписания (cron) | ✓ | — | — | ✅ durable-очередь |
| Agent Teams, worktrees | ✓ | — | — | ✅ |
| LSP, Office, CUA/browser | ✓ | — | — | ✅ |
| OTel/телеметрия | ✓ | — | — | ✅ OTLP/HTTP |
| i18n (ru/en) | ✓ | ru | ru/en | ✅ TUI/CLI, справка и рантайм-сообщения ru/en; тексты ошибок намеренно en, `--json` не зависит от языка |
| Plugin market | ✓ | — | — | ⚠️ каталог с категориями/поиском/карточкой, установка, отзывы в home; общей серверной базы отзывов нет |
| Eval-набор и perf-гейты (DoD M7) | — | — | — | ✅ 10 задач + бюджет токенов, perf-бюджеты §20 в CI |
| Воспроизводимые сборки/дистрибутивы | ✓ | — | — | ✅ архивы и бинарь воспроизводимы, brew-формула/бутылки есть, GHCR-индекс `0.1.0` со всеми платформами и tap `homebrew-dshr` проверены на релизе; ⚠️ только notarization (Developer ID) |

---

## 5. Фазы

Общее правило: **фаза не начинается, пока не выполнены критерии выхода предыдущей**, кроме исследований, идущих параллельно. Оценки — в идеальных днях (ИД); календарь = ИД / 3,5 в неделю + 15 % буфер (см. [README §5](../README.md#5-оценки-верхнего-уровня)).

### M0 — Фундамент (3–5 дней)

**Статус:** выполнен 12.11.2026.

**Содержание**
- Cargo workspace, `rust-toolchain.toml` (stable, edition 2024), линтеры: `clippy -D warnings`, `rustfmt`, `cargo-deny`.
- CI: GitHub Actions (macOS/Ubuntu/Windows), кэш, `cargo nextest`, покрытие.
- Каркас CLI (`clap`): `dshr --version`, `dshr doctor`, `dshr config show`.
- Конфиг: YAML + JSON Schema (`schemars`), слои `defaults < profile < user < --patch`, пути `~/.dshr/`.
- Логи `tracing` (JSON), коды ошибок (`thiserror`), `Result`-алиас.
- `docs/adr/` + шаблон ADR, `CONTRIBUTING.md`, `LICENSE` (MIT/Apache-2.0 dual).

**Критерии выхода**
- `cargo test --workspace` и `cargo clippy --workspace -- -D warnings` зелёные в CI на трёх ОС.
- `dshr doctor` печатает версию, пути, доступность провайдера и backend песочницы (`n/a` до M5).
- `dshr config show` собирает конфиг из четырёх слоёв; golden-тест на приоритет.

### M1 — LLM + журнал + headless (10–16 дней)

**Статус:** выполнен 12.11.2026.

**Содержание**
- `dshr-llm`: `Message`, `ContentBlock` (text/thinking/tool_use/tool_result/image), `StreamChunk`, `Request`, `Usage`, `Capabilities`.
- Адаптеры: DeepSeek **Anthropic Messages** (`/anthropic/v1/messages`, auth `x-dsh-auth-token` для account-режима и API-key), OpenAI-совместимый (RouterAI/шлюзы); retry/backoff, таймауты, отмена, классификация ошибок.
- Каталог моделей `GET /models` с тем же auth, TTL-кэш и fallback; SSE-парсер с `ping`, `content_block_*`, `message_delta/stop`.
- Mock-сервер + cassette record/replay (запись SSE в JSON, вырезание auth-заголовков, воспроизведение без сети).
- `dshr-session`: события, `derive_messages()`, JSONL-хранилище (append, fsync, repair хвоста), `SessionHeader`.
- CLI: `dshr run "…"` (one-shot, стрим в stdout), `--session`, `--model`, `--json`.
- `dshr sessions list|show|export`.

**Критерии выхода**
- 20+ тестов: парсер SSE, ретраи, отмена, round-trip событий (proptest), repair повреждённого хвоста.
- `dshr run` на кассете даёт идентичный журнал при повторном прогоне.
- Незавершённая сессия после `kill -9` открывается и продолжается.
- Живой smoke (опционально, `DSH_DEEPSEEK_TOKEN`): `/models` + Messages + streaming + tool round-trip — см. [PROVIDER-VERIFICATION.md](PROVIDER-VERIFICATION.md).

### M2 — Инструменты + цикл + TUI (15–20 дней)

**Статус:** закрыто. Ядро — 12.11.2026; `apply_patch`, `ask_user`, persistent bash, diff-view и eval-набор (10 задач + бюджеты токенов, CI-джоба `evals`) перенесены.

**Содержание**
- `dshr-tools`: реестр со scoped-регистрацией, JSON Schema (`schemars` + `jsonschema`), пайплайн `pre → execute → post`, лимиты вывода, spill в файл, параллелизм с семафором.
- Инструменты: `read`, `write`, `edit` (уникальная замена), `apply_patch`, `glob`, `grep` (`ignore`), `bash`, `todo`, `ask_user`, `present`.
- `dshr-agent`: состояние turn/step, батчи tool-call, повторные запросы, max steps, отмена, ошибки инструментов как результаты.
- Approvals: режимы `read-only | workspace-write | danger`, правила по инструменту/шаблону команды, аудит-события, «запомнить выбор».
- TUI (`ratatui`): стрим, карточки вызовов, diff, модалка approval, выбор сессии, отмена по `Esc`, панель jobs.
- Демо-сценарий: «найди баг → исправь → прогони тесты → покажи diff».

**Критерии выхода**
- Approval-матрица: 12+ тестов, ни одна запись вне workspace без подтверждения.
- TUI-сценарий проходит в `expectrl`-тесте.
- Отмена во время стрима и во время инструмента оставляет журнал консистентным (`turn/end` с причиной).

### M3 — Промпт, сессии, compaction, skills (10–15 дней)

**Статус:** ядро выполнено 12.11.2026; SQLite FTS5, авто-заголовки через LLM, импорт DSH и allowlist схем перенесены.

**Содержание**
- `dshr-prompt`: секции системного промпта, окружение (cwd, git, OS), discovery `AGENTS.md`, допуск схем инструментов, кэш-стабильные границы.
- Сессии: `resume`, `fork` на границе turn, `rewind`, заголовки, `SQLite`-индекс + FTS5, поиск.
- Compaction: подсчёт токенов, пороги, суммаризация через LLM, pruning tool-результатов, события `compaction/*`.
- Skills: discovery (`~/.dshr/skills`, `.dshr/skills`, плагины), каталог в промпте, инструмент `skill`, загрузка ресурсов.
- Экспорт: Markdown/JSON; импорт DSH-сессий — за фичефлагом.

**Критерии выхода**
- Сессия > 100k токенов продолжается после compaction без потери последних N сообщений.
- `fork` и `rewind` проходят round-trip: журнал валиден, модель видит ожидаемое.
- Skills-каталог не раздувает промпт: только имя+описание до вызова.

### M4 — MCP, web, jobs, субагенты (10–15 дней)

**Статус:** ядро выполнено 12.11.2026; MCP HTTP/resources, бюджеты субагентов и durable jobs перенесены.

**Содержание**
- MCP-клиент: stdio + Streamable HTTP, discovery tools/resources/prompts, namespacing, таймауты, reconnect, scoped-доступ.
- `web_search`/`web_fetch`: провайдеры, readability-извлечение, SSRF-guard, лимиты размера.
- Jobs: фоновые процессы, кольцевые логи, `job_output/job_kill/job_list`, персистентность между запусками.
- Субагенты: spawn (новый контекст) и fork (наследование), дерево, сообщения, ожидание, отмена, агрегация результата.
- Ограничение параллелизма и бюджета токенов на дерево.

**Критерии выхода**
- Интеграция с тестовым MCP-сервером (echo + fs) в CI.
- Субагент выполняет подзадачу и возвращает результат; родитель видит дерево и стоимость.
- Job переживает перезапуск TUI и корректно убивается.

### M5 — Песочница + плагины + композиция (15–25 дней)

**Статус:** ядро выполнено 12.11.2026; Linux/Windows бэкенды, WASM/sidecar и hot reload перенесены.

**Содержание**
- `dshr-sandbox`: модель политики (fs-режимы, сеть, exec), провайдеры: macOS seatbelt (`sandbox-exec` / профиль), Linux landlock+seccomp (через `landlock`/`seccompiler`), Windows — Job Objects + restricted token (best effort).
- Обёртка argv (`ConfinedArgv`), fail-closed при недоступном backend, диагностика.
- `dshr-plugin`: единый шов `PluginRuntime` (native / wasm / external), WASM-компоненты (wasmtime, component model, WIT, один диспетчерский импорт `host.call`), манифест с `api_version`, capability grants и кэш грантов, лимиты (fuel/epoch/память), integrity-check, горячая замена; sidecar-плагины по JSON-RPC.
- Композиция: профили/бандлы/патчи, `dshr --dump-config`, `dshr plugin add|remove|list`, lock-файл.
- Тесты побега: попытки записи вне workspace, чтения `~/.ssh`, сети, fork-бомбы.

**Критерии выхода**
- Sandbox escape suite: все попытки блокируются на документированном уровне enforcement; partial-кейсы перечислены в отчёте, ошибки типизированы.
- Пример плагина собирается по документации за ≤ 15 минут.
- Инструмент из каждого тира (native/wasm/external) виден в общем реестре и вызывается агентом.
- Golden-тесты композиции: 4 слоя, патч заменяет строку по id.

### M5b — Совместимость npm/Cordis (12–18 дней)

Решение — [ADR-013](adr/013-cordis-compat.md). Отдельная фаза, чтобы WASM-тир и песочница не ждали JS-рантайма.

**Статус:** частично. Сделано: Cordis-рантайм как sidecar-процесс (системный Node, настоящий `cordis` при наличии пакета), hot reload (watcher + endpoint), capability-гранты (`ctx.fs`, `ctx.exec`/`ctx.net` выключены), клиентская половина — eager registry + iframe на слот (17 регионов, полный словарь имён DSH), экспериментальный `dsh.client.runtime` (половина приносит свою модульную таблицу/UI-стек, включая React), PoC тира B `dsh.client.host_modules` (React 18.3.1 и таблица модулей от хоста) и C-lite (раунды 79–85): React-компоненты слотов с host-hooks/`ctx.get('sessions')`, сервисы `ctx.get('slots')`/`locale`/`ctx.inject` и диагностика `__DSHR_LAST_ERROR__`, design-system примитивы `@deepseek-ai/dsh-client-ui-primitives` (с раунда 93, C2.1 — reference-порт: атомы и все 204 иконки из `deepseek-ai/deepseek-harness` @ `39163dd`; с раунда 94, C2.2 — полный `Tooltip`; с раунда 95, C2.3 — ядро `Menu`/`MenuItemButton`; с раунда 96, C2.4 — reference submenu page-layout, 200-мс hover grace и `MenuGroup`/`observeStickyMenuGroups`; с раунда 97, C2.5 — reference `JsonBlock` и API-совместимое dependency-free `MarkdownText`-подмножество; раунды 98–101 — C2.6 submenu-механика, C3.1 store-seats, C1.5 fallback-маршруты и ownerProps-plumbing; общий рантайм-артефакт 430 139 байт), React-контракт моста (типы `React.memo`/`forwardRef`, keyed-ids, `locale.register`/`t()`, `ctx.interval`/`ctx.timeout`), одиннадцать реальных DSH-плагинов в conformance (n=11: `@kt11/dsh-deepseek-balance`, `dsh-remote`, `@ruoyang/dsh-plugin-deepseek-balance`, `dsh-genui`, `dsh-session-superagent`, `dsh-mnemon`, `dsh-plugin-market`, `dsh-plugin-codegraph-panel`, `dsh-plugin-notify`, `dsh-plugin-debugging`, `dsh-plugin-studio`) — цель 8–12 из ADR-013 §6 по числу плагинов достигнута с запасом; вне набора остаются `@michengai/dsh-archive-manager` (`'unsafe-eval'` в CSP) и `@michengai/dsh-skills-manager` (падение на инициализации). Runtime pack сделано в раунде 117 (`dshr runtime install|list|use|remove`: sha256 + опциональная Ed25519-подпись, офлайн-установка, атомарная распаковка `.tar.gz`, sidecar берёт `node` из пака после `DSHR_NODE` и перед PATH). npm-резолвер/CAS сделано в раунде 118, `peerDependencies` и первая половина шима — в 119: sidecar отдаёт `effect`/`on`/`provide`/`tools.register`/`systemPrompt.section`/`sandbox.confine`/`config` и резолвит именованный `Config`, поэтому `dsh-remote@1.2.0` загружается и исполняет свои 21 инструмент. Сервисы `subagents` (реестр, раунд 120), `agents` (мост к дереву агентов хоста) и `commands` (реестр) добавлены в раунде 121 — под sidecar работают `dsh-remote` (21 инструмент) и `dsh-mnemon` (3). Сервис `settings` (послойное разрешение + персист) добавлен в раунде 122. Шим доведён в раундах 123–126: команды плагинов доступны агенту (`cmd__*`), по HTTP (`/v1/commands`) и в TUI (`/name`), сервисы `settings`/`agents`/`subagents`/`commands` реализованы, матрица сервисов и conformance-отчёт записаны ([CONFORMANCE](CONFORMANCE.md), [PLUGINS §Матрица сервисов](PLUGINS.md)). Осталось: клиентские половины (C-full, [ADR-040](adr/040-react-client-half.md)) и сервисы, требующие хостового рантайма (`llm`, `conversation`, `attachments`, `inputTriggers`, `spillStore`, `sandboxPolicy`). Host HTTP-маршруты DSH-плагинов с раунда 84 — отдельный C1-трек ([ADR-040](adr/040-react-client-half.md) §C1): sidecar-сервер + allowlist-прокси + `connect-src` origin для opt-in кадров работают end-to-end (реальная host-половина `@kt11/dsh-deepseek-balance`), раунд 86 (хардненинг C1) добавил prefix-маршруты, таблицу `routes` в `GET /v1/plugins/status`, фильтр MCP-псевдо-инструментов и loopback-only политику с opt-in `--allow-lan-plugin-routes`; раунд 87 (C1.1) добавил сервисы `sessions`/`sessionProjections` из внутреннего снимка сервера (usage-маршруты реальных плагинов считают статистику по сессиям dshr; задержка ≤ 5 с, проекции упрощены, содержимое сессий не мостится); остатки — SSE/WebSocket/стриминг, отсутствие аутентификации на plugin-маршрутах (риск снижен loopback-only политикой). Раунд 85 (C2) расширил design-system шим с 7 до 30+ экспортов по фактическим импортам (компоненты, именованные `Icon*` с Proxy-fallback, `writeClipboard`, хуки `useAnchoredPosition`/`useDismissOnOutsidePointer`) и довёл conformance до **n=11** (`dsh-plugin-debugging@0.8.1`, `dsh-plugin-studio@0.3.1` активируются без ошибок, их settings-UI был инертен без host-сервисов `pluginManager`/settings, но с раунда 89 (C1.2) рендерится через seat-`inject` и `configForms`; smoke раунда 85 — 79/79, раунд 87 — 80/80 с проверкой C1.1 bridge, раунд 89 — 82/82 с рендером реальных settings-вкладки и карточки; раунд 91 (C1.3) перевёл слот-контракт на сгенерированный каталог DSH (122 слота с kind/scope/standard-kit) и референсные правила `SlotCore` — undeclared/keyed/chain/duplicate, winners, children/deferred inject, standard-kit + ownerProps, `ctx.slots.spec(key)`; bridge harness 98 (+8); раунд 92 (C1.4) выровнял host-сервисы `webServer`/`ctx.baseUrl`/`configForms` по reference-контракту (правила маршрутов и origin-аксессоры, `file://`-baseUrl, snapshot с числовой ревизией и `set`/`unset`; `registerFallback`/`registerUpgrade` не поддержаны) — bridge harness 102 (+4), а раунд 93 (C2.1) портировал атомы design-system и все 204 иконки из reference-исходников (`deepseek-ai/deepseek-harness` @ `39163dd`, `packages/client/ui-primitives/src`) — артефакт на конец раунда 375 894 байта; раунд 94 (C2.2) — полный reference-порт `Tooltip` (placement-engine, `ShortcutKeys`, `input-modality`; артефакт 387 613 байт), а раунд 95 (C2.3) — ядро `Menu`/`MenuItemButton` (артефакт 405 323 байта), а раунд 96 (C2.4) — reference submenu page-layout, 200-мс hover grace и `MenuGroup`/`observeStickyMenuGroups` (артефакт 415 972 байта), а раунд 97 (C2.5) — reference `JsonBlock` и dependency-free GFM-подмножество `MarkdownText` (артефакт 424 801 байт), а раунды 98–101 (C2.6 submenu-механика, C3.1 store-seats, C1.5 fallback-маршруты и ownerProps-plumbing) — 430 139 байт, а раунд 102 (C4.1) добавил chat/turn-модель шелла (turn ordinal в `data-turn` и owner share), owner shares трёх реальных render-сайтов (`sidebar.session.action`, `message.action` для user, `composer.actions`) и chain-элекцию в мосте (`select(ownerProps)`; `false` → пустой DOM + `chain-unselected`); остаются upstream markdown-стек (mdast/shiki/KaTeX/footnotes/streaming), renderer-семантика, `hoc`-вложение, `registerUpgrade` (WebSocket), owner props с функциями (нет callback-моста), per-frame фильтрация цепочек (несколько `select === true` рендерят несколько кадров) и полный chat-view для `conversation.chat.node`). C-full (полный design-system/React-контекст/SlotMap-остатки/триаж отклонённых плагинов) — отдельное решение владельца по [ADR-040](adr/040-react-client-half.md) (Proposed), а не остаток C-lite; [ADR-041](adr/041-c-full-decision.md) зафиксировал отложение и точечные инкременты.

**Содержание**
- Runtime pack: вендоренный Node-совместимый движок, хеш и подпись, `dshr runtime install|list|remove`, офлайн-режим; режимы `cordis-sidecar` (по умолчанию) и `cordis-embedded` (опция).
- Регистрация Cordis-рантайма под тем же `PluginRuntime`: единый манифест, capability-модель, health-check, откат.
- Шим `@dshr/cordis-compat@1`: сервисы `ctx.*` (`tools`, `fs`, `shell`, `llm`, `session`, `settings`, `commands`, `approval`, `jobs`, `subagents`) поверх host-RPC; матрица поддержки.
- Capability-шина builtins: `path`/`url`/`events`/`util`/`buffer` — чистый JS; `fs` → `ctx.fs` (sandbox, read-before-edit); `child_process` → exec + approvals; `net` → deny by default; `process` → фильтр env; `crypto` → host.
- npm-резолвер: registry v2, тарболы в content-addressed store, lock, транзитивные зависимости; без pnpm и без `postinstall` по умолчанию.
- HMR для JS-тира: watch → перезагрузка модулей → dispose/rebuild fiber.
- Conformance-набор: 8–12 реальных плагинов DSH в CI + отчёт матрицы. Числовая часть закрыта с запасом (n=11, раунды 83–85, smoke-гейт `web-e2e` в CI); отчёт матрицы и отдельный nightly-прогон — нет.

**Критерии выхода**
- 8+ плагинов DSH загружаются без правок кода; для каждого есть отчёт «инструменты/хуки работают».
- Плагин, который лезет в `~/.ssh` или в сеть без capability, получает типизированный отказ, и это видно в аудите.
- Плагин с нативным аддоном падает с внятной диагностикой и подсказкой про sidecar-режим.
- `dshr plugin add <npm-name>` ставит плагин с зависимостями; lock воспроизводим на чистой машине.
- Базовая поставка без JS-плагинов не качает runtime pack и держит прежние бюджеты размера и старта.

### M6 — Server + Web UI + SDK (15–25 дней)

**Статус:** ядро выполнено 12.11.2026; Leptos, полный SDK, rate-limit и дистрибутивы перенесены.

**Содержание**
- `dshr-server` (`axum`): REST + WS/SSE, аутентификация bearer-токеном, CORS, статика, OpenAPI.
- Web UI: чат со стримом, карточки инструментов, diff, approvals, список сессий, настройки. Стек: Leptos (ADR-003).
- SDK: JSON-RPC поверх stdio + HTTP-клиент; примеры на Python/TS.
- E2E: Playwright на поднятом сервере.

**Критерии выхода**
- Браузерный e2e: чат → approval → выполнение → diff.
- SDK-пример запускает сессию и стримит события.
- API-контракт зафиксирован OpenAPI и проверяется в CI.

### M7 — Паритет (84–145 дней, итеративно)

**Статус:** срез 14.11.2026 — workflows, agent teams, worktrees, schedules, LSP, guardrails; DoD-гейты (eval-набор и perf-бюджеты §20 как CI-гейт) закрыты, см. ROADMAP «DoD-гейты M7».

Workflows (скриптовый движок + мультиагентные сценарии), расписания, agent teams, worktrees/forge, LSP, Office, CUA/browser, OTel, i18n, упаковка (cargo-dist, brew, Docker), desktop (Tauri), ACP, plugin market.

**Критерий выхода фазы** — у каждого эпика есть DoD: eval-набор (задачи + метрики успеха) и perf-бюджеты §20 ARCHITECTURE как CI-гейт (оба гейта в CI: `evals`, `perf`); матрица §4 заполнена для T3.

---

## 6. План спринтов (2 недели)

| Спринт | Фаза | Цель | Демо |
|---|---|---|---|
| S0 | M0 | Каркас, CI, конфиг | `dshr doctor`, CI зелёный |
| S1 | M1 | LLM-адаптер + стрим | `dshr run` отвечает с кассеты |
| S2 | M1 | Журнал + resume | `kill -9` и продолжение |
| S3 | M2 | Реестр + fs-инструменты | агент правит файл |
| S4 | M2 | bash + approvals | агент гоняет тесты с подтверждением |
| S5 | M2 | TUI + демо-сценарий | полноценная сессия в терминале |
| S6 | M3 | Промпт + skills | агент читает `AGENTS.md` и skill |
| S7 | M3 | Compaction + поиск + fork | длинная сессия, FTS, ветвление |
| S8 | M4 | MCP + web | внешний MCP-сервер в сессии |
| S9 | M4 | Субагенты + jobs → **T1 demo** | параллельная подзадача |
| S10 | M5 | Sandbox macOS + fail-closed | escape suite на macOS |
| S11 | M5 | Sandbox Linux + плагины WASM | сторонний WASM-инструмент |
| S12 | M5 | Композиция конфигов + sidecar | профили/бандлы, плагин на Python |
| S12b | M5b | Cordis-рантайм + шим `ctx.*` | плагин DSH из npm ставится и отдаёт инструмент |
| S12c | M5b | Capability-шина builtins + conformance | попытка записи вне workspace отклонена и зааудирована |
| S13 | M6 | Server API | OpenAPI + curl-сценарий |
| S14 | M6 | Web UI | чат + approval в браузере |
| S15 | M6 | SDK + упаковка → **T2 demo** | SDK-пример, установка с нуля |
| S16+ | M7 | Паритет по эпикам | по эпикам ROADMAP |

Спринт = 2 недели ≈ 7 идеальных дней. В таблице — базовый план без буфера; буфер 15 % на спринт (баги, ревью, неожиданности) учтён в календарных оценках README.

---

## 7. Процесс разработки

- **Ветки:** trunk-based, короткоживущие ветки, squash-merge.
- **Коммит:** conventional commits, обязательное описание риска и способа проверки.
- **Ревью:** каждый PR — self-review чеклист + прогон CI; для опасных зон (sandbox, approvals, секреты) — отдельный ревью-проход.
- **ADR:** решение фиксируется до кода, если затрагивает > 1 крейт или публичный контракт.
- **Definition of Done:** код + тесты + документация + observability (логи/метрики) + отсутствие `TODO` без issue.
- **Демо каждые 2 недели** по сценарию из таблицы спринтов.

---

## 8. Качество и тестирование

| Уровень | Инструмент | Что покрывает |
|---|---|---|
| Unit | `cargo test`, `rstest` | Парсеры, сериализация, политики |
| Property | `proptest` | Round-trip событий, границы compaction |
| Contract | кассеты + mock | LLM-адаптеры, MCP |
| Integration | `testcontainers`/локальные бинари | Хранилище, sandbox, сервер |
| Golden | `insta` | Промпт-сборка, журналы, конфиг |
| E2E TUI | `expectrl` | Сценарии терминала |
| E2E Web | Playwright | Браузерные сценарии |
| Fuzz | `cargo-fuzz` | JSONL/SSE/парсеры |
| Bench | `criterion` | Сборка промпта, стрим, поиск |

Целевое покрытие: **70 %** для `core/session/agent/tools`, **50 %** для UI; покрытие не является самоцелью — обязательны сценарии.

CI-гейты: fmt, clippy, test, deny, docs (rustdoc без warnings), coverage threshold, license check.

---

## 9. Безопасность

- Секреты — только в OS keyring; в конфиге — ссылки (`CredentialRef`), не значения.
- Редакция логов: ключи, токены, приватные пути.
- Песочница: fail-closed; политика на сессию; запись только в workspace; сеть — по правилу.
- Approvals: аудит-события, «запомнить» — с ограничением области (сессия/проект/команда).
- Web: SSRF-guard, лимиты размера, таймауты, запрет file://.
- Плагины: capabilities, лимиты ресурсов, без ambient authority; для Cordis-тира тот же принцип — builtins опосредованы capability-шиной, а не отданы как в Node ([ADR-013](adr/013-cordis-compat.md)).
- Supply chain: `cargo-deny`, lock-файл, минимальные фичи, аудит зависимостей.

---

## 10. Риски

| ID | Риск | Вероятность | Влияние | Мера |
|---|---|---|---|---|
| R1 | Расползание scope | Высокая | Высокое | Тиры, критерии выхода, M7 отдельно |
| R2 | Погоня за меняющимся DSH (0.2.x) | Высокая | Среднее | Фиксировать референс, маппинг по возможностям, не по API |
| R3 | Дрейф Cordis/Node API и неполная совместимость плагинов DSH | Высокая | Среднее | Привязка к API 0.2.x, conformance-набор в nightly CI, явная матрица; несовместимое — через WASM/sidecar/MCP |
| R13 | JS-рантайм подрывает бюджеты размера/старта | Средняя | Среднее | Runtime pack по требованию, lazy init, бюджеты §3 IMPROVEMENTS и CI-гейт |
| R4 | Побег из песочницы / различия ОС | Средняя | Критическое | Fail-closed, escape suite, Windows позже |
| R5 | Недетерминизм LLM ломает тесты | Высокая | Среднее | Кассеты, mock, golden |
| R6 | Плохое качество compaction | Средняя | Среднее | Evals на длинных сессиях, настраиваемые пороги |
| R7 | Производительность сборки промпта | Низкая | Среднее | Бенчи, кэш секций |
| R8 | Стоимость токенов на разработку | Средняя | Среднее | Mock-first, кассеты, малые модели для тестов |
| R9 | Юридические/брендовые претензии | Низкая | Высокое | Своё имя, свои тексты, MIT-совместимость |
| R10 | Bus factor = 1 | Средняя | Высокое | ADR, документация, CI, воспроизводимость |
| R11 | Расхождение Unix/Windows (пути, shell) | Высокая | Среднее | Абстракция `Shell`, тест-матрица, Windows — T2+ |
| R12 | Дрейф API провайдеров | Средняя | Среднее | Версионирование адаптеров, контрактные тесты |

---

## 11. Метрики

**Продуктовые**
- Time-to-first-token (p50/p95) поверх провайдера.
- Оверхед harness на turn (без LLM), p95.
- Доля сессий без ручных исправлений журнала.
- Средний расход токенов на типовую задачу; коэффициент compaction.
- Число approval-событий на задачу (UX-метрика трения).

**Инженерные**
- Покрытие core-крейтов; mutation score (опционально).
- Время CI < 15 мин.
- Sandbox escape suite: 0 побегов в документированном наборе; partial-кейсы перечислены.
- Количество `unsafe` блоков (цель: 0 вне FFI).

---

## 12. Бюджет

- **Время:** см. §5 и README.
- **Токены на разработку:** при mock-first — десятки миллионов токенов на цикл M0–M4; деньги — порядок $200–$2000 в зависимости от модели и интенсивности. Уточняется после M1 по факту.
- **Инфраструктура:** GitHub Actions (macOS runner дороже), 3 ОС в матрице; локальные прогоны на macOS.

---

## 13. Вопросы, закрываемые ADR в M0

1. **ADR-001:** собственный формат сессий или совместимость с DSH JSONL? *Рекомендация: свой, версионированный; импорт DSH — адаптер.*
2. **ADR-002 + ADR-013:** ABI плагинов — WASM, proc-macro, sidecar, совместимость Cordis? *Решение: WASM + sidecar + Cordis-рантайм; нативного `libloading` нет. Совместимость — [ADR-013](adr/013-cordis-compat.md).*
3. **ADR-003:** Web UI — Leptos или React? *Решение: Leptos ([ARCHITECTURE §19](ARCHITECTURE.md#19-adr-решения-по-умолчанию)); подтвердить в M0, что экосистемы хватает.*
4. **ADR-004:** хранение — JSONL + SQLite или встраиваемая БД? *Рекомендация: JSONL как истина, SQLite как индекс.*
5. **ADR-005:** модель конкурентности — tokio tasks + mpsc-актор журнала. *Рекомендация: да.*
6. **ADR-006:** схема инструментов — `schemars` + `jsonschema`. *Рекомендация: да, с ручной валидацией критичных полей.*
7. **ADR-007:** имя проекта и лицензия. *Рекомендация: MIT/Apache-2.0 dual, имя вне бренда DeepSeek.*
8. **ADR-008:** источник счётчика токенов — `usage` провайдера и/или локальный tokenizer. *Рекомендация: usage после ответа + локальная оценка до запроса.*
9. **ADR-009:** `network`/`exec` в политике песочницы — расширение dshr сверх файлового `SandboxMode` DSH. *Рекомендация: да, с явной документацией.*
10. **ADR-010:** DeepSeek official — Anthropic Messages wire, два режима auth. *Решение подтверждено живой проверкой 12.11.2026.*
11. **ADR-011:** desktop-оболочка — Tauri над `dshr serve` (loopback HTTP/WS), без Electron. *Рекомендация: да, T3.*
12. **ADR-012:** YAML-парсер — `serde-saphyr` (safe Rust) вместо `serde_yaml_ng`/`unsafe-libyaml`. *Решение принято по RUSTSEC-2024-0449.*

---

## 14. Критерии пересмотра плана

План пересматривается, если:
- M1 занял > 25 дней (оценки неверны — резать scope);
- sandbox на macOS не удаётся сделать fail-closed за 10 дней (менять модель изоляции: контейнер/VM);
- качество агента на реальных задачах < 60 % успеха (проблема в промпте/модели, не в коде);
- DSH меняет ключевую архитектуру (пересмотр маппинга, не кода).
