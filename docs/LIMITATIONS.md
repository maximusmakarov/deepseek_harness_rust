# Ограничения и границы проекта

Документ фиксирует границы применимости `dshr`, технические риски и внешние зависимости. Он дополняет [PLAN.md](PLAN.md): почти каждое ограничение имеет обход или явно принятую цену.

---

## 1. Назначение документа

- Отделить свойства harness от свойств модели.
- Зафиксировать, что проект сознательно не делает.
- Дать критерии, при которых план пересматривается.

---

## 2. Технические ограничения

### 2.1. Модели в проекте нет

`dshr` — harness, а не модель. Без провайдера (DeepSeek API, OpenAI-совместимый endpoint) или локального инференса (llama.cpp/candle/Metal) система не отвечает на запросы.

**Обход:** mock/replay для разработки; для реальной работы — ключ провайдера или локальная модель.

**Следствие для оценок:** качество агента зависит от модели. Слабая модель → больше шагов, хуже tool-calling, выше расход. Это не дефект harness, но влияет на демонстрации и evals.

### 2.2. Провайдеры и ключи

- Реальные вызовы требуют ключей и бюджета; без них тесты идут на кассетах.
- Поддерживаются два режима аутентификации DeepSeek: API-key и account-токен совместимого клиента (`x-dsh-auth-token`). Второй привязан к конкретному продукту и может истечь; для CI и релиза нужен собственный API-key или OpenAI-совместимый шлюз.
- Anthropic/Gemini имеют свои форматы tool-call и reasoning; паритет по ним — постоянная работа (R12).

**Обход:** кассеты, mock-сервер, контрактные тесты на записанных потоках. Ключи читаются из переменных окружения, в кассеты и логи не попадают.

### 2.3. Песочница — самый рискованный участок

- macOS: `sandbox-exec`/Seatbelt работает, но Apple не документирует формат профилей; возможны изменения в новых версиях ОС.
- Linux: Landlock зависит от ядра (≥ 5.13; полный набор — новее); в контейнерах/CI может быть недоступен.
- Windows: изоляция процесса принципиально сложнее; Job Objects не дают файловой изоляции уровня Seatbelt.
- Гарантировать «ноль побегов» на всех ОС нельзя; можно гарантировать fail-closed и набор тестов.

**Обход:** macOS Seatbelt и Linux Landlock + seccomp включены; Windows по умолчанию fail-closed, AppContainer — экспериментальный opt-in (`DSHR_WINDOWS_SANDBOX`). Fallback — контейнер/VM-провайдер. Уровень гарантий документируется по ОС.

### 2.4. Совместимость Cordis: что работает, а что нет

Совместимость с npm/Cordis-плагинами реализована частично по [ADR-013](adr/013-cordis-compat.md): Cordis-рантайм работает как sidecar-процесс (системный Node, настоящий `cordis`, если пакет установлен), есть hot reload и capability-гранты, клиентская половина — sandboxed iframe с eager registry-кадрами и региональным монтажом слотов; с раунда 77 половина может принести свою модульную таблицу classic-скриптом `dsh.client.runtime` (включая собранный React), с раунда 78 может вместо этого опционально взять общий рантайм хоста `dsh.client.host_modules` (React 18.3.1 + таблица модулей одним immutable-артефактом на страницу), с раунда 79 (C-lite) при host React `ctx.slots.register(entry, Component)` рендерится как React-компонент с host-hooks и `ctx.get('sessions')`, а общий рантайм публикует design-system примитивы (с раунда 93, C2.1 — портированы из reference: атомы и все 204 иконки); раунд 80 добавил сервисы, которых ждут референсные плагины (`ctx.get('slots')`, `ctx.get('locale')`, `ctx.inject`) и диагностику `__DSHR_LAST_ERROR__`, а conformance-набор вырос до двух реальных плагинов; раунд 81 закрыл расхождения React-контракта (типы `React.memo`/`forwardRef`, keyed-ids `<slot>:<key>`, `ctx.locale.register` + `t()`, `ctx.interval`/`ctx.timeout`), расширил primitives шим до семи компонентов и довёл conformance до n=4, раунд 82 — до n=6 (`dsh-session-superagent@0.10.12` — composer dock, `dsh-mnemon@0.5.46` — крупный multi-region и референсная поверхность слотов), а раунд 83 довёл набор до **n=9** — цель [ADR-013](adr/013-cordis-compat.md) §6 (8–12 реальных плагинов) достигнута (`dsh-plugin-market@1.2.2`, `dsh-plugin-codegraph-panel@0.6.5`, `@goodandready/dsh-plugin-notify@0.4.9`); раунд 84 — **C1**: host HTTP-маршруты плагинов работают end-to-end (sidecar-сервер + allowlist-прокси, `connect-src` на origin сервера для opt-in кадров), реальная host-половина `@kt11/dsh-deepseek-balance` больше не застаблена и отвечает своим JSON через кадр плагина, остатки — сервисы `sessions`/`sessionProjections` (закрыты в раунде 87, C1.1) и стриминг (аутентификация на plugin-маршрутах не появилась, но LAN-экспозиция закрыта в раунде 86); раунд 85 — **C2**: design-system шим вырос с 7 до 30+ экспортов по фактическим импортам реальных плагинов (компоненты `Input`/`Switch`/`Checkbox`/`Tag`/`Pill`/`Menu`/`MenuSurface`/`Modal`/`Toast`/`SegmentedControl`/`SegmentedTabs`/`DisclosureRow`/`PathLabel`/`TextShimmer`, именованные `Icon*` с Proxy-fallback, `writeClipboard`, хуки `useAnchoredPosition`/`useDismissOnOutsidePointer`), а conformance доведён до n=11 (`dsh-plugin-debugging@0.8.1`, `dsh-plugin-studio@0.3.1` активируются без ошибок, их settings-UI оставался несмонтированным без host-сервисов `pluginManager`/settings до C1.2); раунд 86 — **хардненинг C1**: prefix-маршруты (exact выигрывает), живая таблица `routes` в `GET /v1/plugins/status`, фильтр MCP-псевдо-инструментов и loopback-only политика plugin-маршрутов (403 при bind вне loopback без `--allow-lan-plugin-routes`), что закрыло записанную ранее LAN-экспозицию и ограничение exact-only; раунд 87 — **C1.1**: sidecar получает DSH-shaped `sessions`/`sessionProjections` из внутреннего снимка сервера (`<home>/cordis/host.json` + `GET /v1/internal/session-snapshot`, обновление раз в 5 с), поэтому usage-маршруты реальных плагинов считают статистику по сессиям dshr (проекции упрощены, содержимое сессий намеренно не мостится); раунд 89 — **C1.2**: seat-`inject` в регистрациях слотов, Cordis-генераторы в `slots.inject`/`ctx.inject`/`ctx.effect` и host-сервис `configForms` (стабильный snapshot, revision-контракт `mutate`, in-memory форма в кадре; файловая персистентность — только у host-половин в `<home>/cordis/config-forms/`), поэтому settings-UI `dsh-plugin-studio`/`dsh-plugin-debugging` реально рендерится (studio — вкладки в Settings, debugging — карточка в Plugins с переключателем и Save), а при заглушках host-маршрутов показывает честные error-состояния (провалы seat-inject только логируются в лог кадра); раунд 91 — **C1.3**: слот-контракт переведён на сгенерированный каталог DSH (122 слота с `kind`/`scope`/framework standard-kit, `window.__DSHR_SLOT_META__`) и правила reference `SlotCore` — undeclared/keyed/chain/duplicate-регистрации, shadowing-winners, `children`-декларации, deferred `ctx.slots.inject`, standard-kit + `ownerProps`, `ctx.slots.spec(key)` (chain-элекция, а позже — в C4.1, раунд 102, — и owner shares реальных shell-сайтов были ещё впереди; store seats закрыты в C3.1); раунд 92 — **C1.4**: host-сервисы `webServer`/`ctx.baseUrl`/`configForms` по reference-контракту (exact/prefix-таблицы независимы, дубликат `(kind, path)` бросает, path-валидация — абсолютный pathname без trailing slash/query/hash, `getOrigin()`/`port`/`protocol` отдают origin dshr-сервера из host-bridge, `file://`-baseUrl, snapshot `{status, value, base, user, revision: number, writable}` с `mutate`/`set`/`unset` и числовым revision-fence; `registerFallback`/`registerUpgrade` в sidecar не поддержаны); раунд 93 — **C2.1**: атомы design-system (`Button`/`Input`/`Switch`/`Checkbox`/`Tag`/`Pill`/`SegmentedControl`/`SegmentedTabs`/`DisclosureRow`/`TextShimmer`/`StateDot`/`PathLabel`/`Toast`/`Modal`/`Menu`/`MenuSurface`/`Tooltip`) и все **204** `Icon*` портированы из reference-исходников (`deepseek-ai/deepseek-harness` @ `39163dd`, `packages/client/ui-primitives/src`, SVG-пути и stroke regular/medium, инлайн вместо спрайта), поэтому шим примитивов — reference-порт на `--dsw-alias-*` токенах (общий рантайм-артефакт на конец C2.4 — 415 972 байта, после C2.5 — 424 801 байт, после раундов 98–101 — 430 139 байт), при этом полный `Tooltip` доведён до reference-порта в C2.2 (раунд 94: placement-engine, `ShortcutKeys`, `input-modality`), а в C2.3 (раунд 95) `Menu`/`MenuItemButton` — reference-ядро (ряды/selection/aria, reserve-icon, клавиатурный walk, one-level submenu, dismissal, `usePointerGrace`/`focusWithoutRing`), а в C2.4 (раунд 96) — reference submenu page-layout (single-page с non-selecting back-row, Escape/ArrowLeft на родителя), 200-мс hover grace и `MenuGroup`/`observeStickyMenuGroups`; раунд 97 (C2.5) портировал `JsonBlock` из reference и заменил markdown-шим API-совместимым dependency-free GFM-подмножеством `MarkdownText` (upstream micromark/mdast, shiki, KaTeX, footnotes и инкрементальный streaming не входят), а раунды 98–101 закрыли submenu-механику (C2.6: side-card/single-page по измеренной ширине, `fixed`-позиционирование с flip/clamp, `activeList`-маршрутизация клавиатуры, Escape/ArrowLeft на родителя, 200-мс hover grace между уровнями, `data-menu-back`), store-seats (C3.1: `@deepseek-ai/dsh-client-store` в module-таблице — `createSnapshotStore`/`defineStore`/`shallowEqual`; `store: handle|factory` в регистрации слота, framework-owned instance на scope-key, `useStore(selector, eq)` + baked `actions`, inject-аргументы `(sessionId, actions)`; draft — plain deep-copy без `immer`, `persist` не персистится), fallback-маршруты (C1.5: `webServer.registerFallback`, одно fallback-место, проксирование не-зарезервированных несовпавших путей) и ownerProps-plumbing (`?ownerProps={...}` → `frame.ownerProps` → props после store/seat share), а раунд 102 (C4.1) добавил chat/turn-модель шелла (user-сообщение открывает turn, ordinal в `data-turn` и owner share), owner shares трёх реальных render-сайтов (`sidebar.session.action`, `message.action` для user, `composer.actions`) и chain-элекцию в мосте (`select(ownerProps)`; `false` → пустой DOM + `chain-unselected`); `registerUpgrade` (WebSocket) остаётся явно не поддержанным, но полной React-совместимости это не даёт — путь к ней описан в [ADR-040](adr/040-react-client-half.md) (Proposed, C-full после C-lite — решение владельца; по [ADR-041](adr/041-c-full-decision.md) отложен, дальше точечные инкременты). Границы текущего среза:

| Работает | Не работает или работает иначе |
|---|---|
| Host-половина: реальный `cordis` + npm-пакеты в sidecar-процессе | Runtime pack с вендоренным движком не сделан: нужен `node` в системе (`DSHR_NODE` переопределяет бинарь) |
| Установка из npm с транзитивными зависимостями | Собственного npm-резолвера/CAS и lock-файла нет: `plugin npm add` вызывает системный `npm install`, lifecycle-скрипты пакетов не глушатся |
| HMR и быстрый цикл правки плагина (watcher + `/v1/plugins/reload`) | Нативные аддоны (`.node`, N-API, `node-gyp`): `dlopen` закрыт — пакет с нативной частью не загрузится |
| Host-`ctx`: `ctx.tool`, `ctx.log`, capability-шина `fs` с confinement и грантами; `exec`/`net` выключены | Полный шим `@dshr/cordis-compat@1` (сервисы `tools`/`llm`/`settings`/`commands`/…), `worker_threads`/`cluster`, `process.binding` |
| Клиентская половина: каталог слотов DSH (122 слота, с раунда 91 — с `kind`/`scope`/standard-kit; плюс legacy `panel`/`message.action`, всего 124 ключа) → 17 регионов, iframe на слот, `context`, тема, resize, host-действия; `dsh.client.runtime` — плагин приносит свой UI-стек, включая React; `dsh.client.host_modules` — PoC тира B: React 18.3.1 и таблица модулей от хоста, одна immutable-загрузка на страницу; C-lite (раунды 79–85) — React-компоненты слотов с host-hooks (`useTheme`/`useSession`/`useSessions`/`useHostState`/`wide`) и `ctx.get('sessions')`, сервисы `ctx.get('slots')` (реестр + `entries(key)`), `ctx.get('locale')` (с раунда 81 — `register`/`t()`) и `ctx.inject(deps, callback)`, диагностика `window.__DSHR_LAST_ERROR__`, React-контракт моста (типы `React.memo`/`forwardRef`, keyed-ids `<slot>:<key>`, `ctx.interval`/`ctx.timeout`), design-system примитивы `@deepseek-ai/dsh-client-ui-primitives` (с раунда 93, C2.1 — портированы из reference-исходников `deepseek-ai/deepseek-harness` @ `39163dd`: атомы `Button`/`Input`/`Switch`/`Checkbox`/`Tag`/`Pill`/`SegmentedControl`/`SegmentedTabs`/`DisclosureRow`/`TextShimmer`/`StateDot`/`PathLabel`/`Toast`/`Modal`/`Menu`/`MenuSurface`/`Tooltip` и все **204** `Icon*` с точными SVG-путями, инлайн вместо спрайта; с раунда 94 (C2.2) — полный `Tooltip`, с раунда 95 (C2.3) — ядро `Menu`/`MenuItemButton`, с раунда 96 (C2.4) — reference submenu page-layout, 200-мс hover grace и `MenuGroup`/`observeStickyMenuGroups`, с раунда 97 (C2.5) — reference `JsonBlock` и dependency-free GFM-подмножество `MarkdownText`, а раунды 98–101 — C2.6 submenu-механика (side-card/single-page по измеренной ширине, `fixed` flip/clamp, `activeList`, Escape/ArrowLeft, 200-мс hover grace), C3.1 store-seats (`@deepseek-ai/dsh-client-store`, `store: handle\|factory`, `useStore`/`actions`, inject-аргументы `(sessionId, actions)`) и C1.5 fallback-маршруты (`webServer.registerFallback`); остаются upstream markdown-стек (mdast/shiki/KaTeX/footnotes/streaming) и renderer-семантика; `Spinner`/`useAnchoredPosition` — dshr-расширения, общий рантайм-артефакт 430 139 байт), conformance из **одиннадцати** реальных плагинов (`@kt11/dsh-deepseek-balance@0.2.1`, `dsh-remote@1.1.2`, `@ruoyang/dsh-plugin-deepseek-balance@1.0.2`, `dsh-genui@0.3.3`, `dsh-session-superagent@0.10.12`, `dsh-mnemon@0.5.46`, `@99galaxy/dsh-plugin-market@1.2.2`, `dsh-plugin-codegraph-panel@0.6.5`, `@goodandready/dsh-plugin-notify@0.4.9`, `dsh-plugin-debugging@0.8.1`, `dsh-plugin-studio@0.3.1`); C1 (раунд 84) — host HTTP-маршруты плагинов: exact/prefix-маршруты проксируются loopback-сервером sidecar, реальная host-половина `@kt11/dsh-deepseek-balance` отвечает своим JSON; раунд 86 (хардненинг C1) добавил prefix-маршруты (exact выигрывает, среди prefix — самый длинный), живую таблицу `routes` в `GET /v1/plugins/status`, фильтр MCP-псевдо-инструментов и loopback-only политику (403 при bind вне loopback без `--allow-lan-plugin-routes`); раунд 87 (C1.1) — сервисы `sessions`/`sessionProjections` из внутреннего снимка сервера (`/v1/internal/session-snapshot`, токен в `<home>/cordis/host.json`, обновление раз в 5 с; usage-маршруты реальных плагинов считают статистику по сессиям dshr); раунд 89 (C1.2) — seat-`inject`, Cordis-генераторы и `configForms` (in-memory writable-форма в кадре, файловая — у host-половин), поэтому settings-UI `dsh-plugin-studio`/`dsh-plugin-debugging` рендерится, а не молча инертен; раунды 98–101 — **C2.6** submenu-механика (side-card/single-page по измеренной ширине, `fixed` flip/clamp, `activeList`, Escape/ArrowLeft, 200-мс hover grace), **C3.1** store-seats (`@deepseek-ai/dsh-client-store` в module-таблице, `useStore`/`actions`, inject-аргументы `(sessionId, actions)`), **C1.5** fallback-маршруты (`webServer.registerFallback`) и ownerProps-plumbing (`?ownerProps={...}` → `frame.ownerProps` → props) | Полный design-system `@deepseek-ai/dsh-client-ui-*`, SlotMap-остатки (`hoc`-вложение, renderer-семантика, owner props с функциями без callback-моста и полный chat-view; chain-элекция по `select(ownerProps)` и owner shares трёх реальных сайтов поставлены в C4.1, раунд 102; несколько `select === true` рендерят несколько кадров — per-frame фильтрация; store seats закрыты в C3.1, раунды 98–101; каталог слотов, регистрация, kinds, winners, children/deferred inject закрыты в C1.3, раунд 91), React-контекст и состояние шелла, композиция между плагинами, произвольная инъекция в DOM шелла — вне scope §5 или C-full; мост `sessions`/`sessionProjections` — снимок с задержкой ≤ 5 с и упрощённой проекцией (`llmMs`=0, без истории тредов), содержимое сессий намеренно не мостится; browser-`configForms` in-memory — правки не переживают reload кадра, семантика конфигурационного движка DSH не воспроизведена; провалы seat-inject только логируются (компонент деградирует, плагин остаётся живым); нет SSE/WebSocket/стриминга, аутентификации на plugin-маршрутах нет (риск снижен loopback-only политикой и opt-in флагом); `wide` в host-state сейчас всегда `true`; conformance-набор n=11 (цель 8–12 ADR-013 §6 достигнута; остальные smoke-половины синтетические), референсных host-сервисов `pluginManager`/`settings` нет (их отсутствие частично компенсируют seat-`inject`/`configForms`, UI честно деградирует); eval-based бандлы (`@michengai/dsh-archive-manager` 1.3.6) требуют `'unsafe-eval'` в CSP — сознательно не включён, `@michengai/dsh-skills-manager` 1.4.8 падает на инициализации бандла и ждёт триажа; мостовые сервисы — подмножество референсных (проекции `sessionStats` упрощены, `llmMs`=0); версия React закреплена за сервером, CSP opt-in-кадров открывает origin для скриптов и `connect-src` (C1 host-маршруты); `webServer.registerFallback` поддержан с C1.5 (раунды 98–101), а `registerUpgrade` (WebSocket) по-прежнему не поддержан и бросает явную ошибку, живого RPC-зеркала настроек нет (формы in-memory/файловые); ownerProps с C4.1 (раунд 102) поставляются тремя реальными сайтами шелла, но props с функциями не передаются (нет callback-моста), а полный chat-view для `conversation.chat.node` ещё нужен; полная React-совместимость ждёт [ADR-041](adr/041-c-full-decision.md) (C-full отложен, точечные инкременты; путь — [ADR-040](adr/040-react-client-half.md)) |
| Инструменты и команды любого тира в общем реестре | Поведенческий паритет промптов/эвристик — не обещан |

**Следствие:** большинство «инструментальных» плагинов DSH переносится как есть, простые UI-плагины на React работают через C-lite, host-маршруты DSH-плагинов реализованы частично (C1, раунды 84–86: exact/prefix-маршруты проксируются, маршруты видны в статусе и закрыты loopback-only политикой по умолчанию, данные, зависящие от сервисов `sessions`/`sessionProjections`, доступны из снимка с задержкой до 5 с и упрощённой проекцией; с раунда 92 (C1.4) host-сервисы `webServer` (правила exact/prefix-маршрутов, origin-аксессоры), `ctx.baseUrl` и `configForms` (reference-shape snapshot, числовая ревизия, `set`/`unset`) выровнены по reference-контракту; `registerUpgrade` в sidecar не поддержан, а `registerFallback` закрыт в C1.5; с раунда 93 (C2.1) design-system примитивы — reference-порт: атомы и все 204 иконки, с раунда 94 (C2.2) — полный `Tooltip`, с раунда 95 (C2.3) — ядро `Menu`/`MenuItemButton`, с раунда 96 (C2.4) — reference submenu page-layout, 200-мс hover grace и `MenuGroup`/`observeStickyMenuGroups`, а с раунда 97 (C2.5) — reference `JsonBlock` и dependency-free GFM-подмножество `MarkdownText`, а раунды 98–101 закрыли ещё submenu-механику (C2.6), store-seats (C3.1) и fallback-маршруты (C1.5) плюс ownerProps-plumbing, так что упрощёнными/незакрытыми остаются upstream markdown-стек, renderer-семантика, `hoc`-вложение, owner props с функциями и полный chat-view — chain-элекция и owner shares трёх реальных сайтов закрыты в C4.1, раунд 102), сложные UI-плагины требуют адаптации или ждут C-full, плагины с нативными зависимостями — нет.

**Обход:** WASM/sidecar/MCP для того, что не переносится; для нативных зависимостей — sidecar-процесс, запускающий их в настоящем Node.

**Цена:** runtime pack (десятки МБ по требованию), дрейф Cordis/Node API, отслеживание CVE движка.

### 2.5. Web/Desktop UI — большой отдельный объём

TUI и headless закрывают большинство сценариев разработчика. Полноценный Web UI (чат, diff, approvals, настройки, реконнект) — сопоставимый по трудозатратам фронтенд-проект; Desktop (Tauri) добавляет упаковку и автообновление.

**Обход:** Web UI — после T1; минимальный набор экранов; без pixel-perfect паритета.

### 2.6. Поведенческий паритет не гарантирован

Промпты, эвристики compaction, формулировки инструментов, порядок секций — продуктовая работа. Промпты DSH не копируются; следовательно, поведение будет отличаться.

**Обход:** собственные evals (набор задач + метрики успеха), итеративная настройка.

### 2.7. DSH быстро меняется

Референс — 0.2.2-rc.2; архитектура и API могут меняться. Гнаться за каждым релизом нецелесообразно.

**Обход:** референс фиксируется, маппинг — по возможностям, а не по именам пакетов; обновление референса — раз в квартал.

### 2.8. Проверка UI

API и TUI проверяются автоматически; визуальное качество Web/Desktop-интерфейса требует ручного ревью и обратной связи. Браузерный e2e на headless Chrome/CDP покрывает сценарии, но не оценивает удобство.

---

## 3. Организационные ограничения

### 3.1. Итеративная поставка

Проект движется этапами M0–M7; каждый этап заканчивается демонстрацией и проверкой критериев выхода. Большие изменения не сливаются без ревью.

### 3.2. Оценки приблизительны

До завершения M1 погрешность ±50 %: неизвестны реальные сложности парсинга SSE, поведения модели, песочницы. После M1 — ±25 %.

### 3.3. Bus factor

Проект ведётся небольшой командой. Снижение риска: ADR, документация, CI, воспроизводимые тесты, отсутствие «магии» в коде.

### 3.4. Ревью и ответственность

- Опасные зоны (sandbox, approvals, секреты, сеть) требуют отдельного ревью.
- Публикация, релизы и юридические решения принимаются владельцем проекта.

---

## 4. Экономика

| Статья | Оценка | Комментарий |
|---|---|---|
| API-токены на разработку | $200–$2000 за M0–M4 | Mock-first снижает до нижней границы |
| API-токены на эксплуатацию | зависит от модели и задач | Обычно основная статья расходов |
| CI | $0–$50/мес | macOS-раннеры дороже; ночные прогоны |
| Время | 4–6 мес до T1 | При 60–75 % загрузки |

Стоимость агентных прогонов может быть значительной (десятки тысяч токенов на задачу). Harness должен показывать расход и лимиты — это часть T1.

---

## 5. Ограничения окружения

- macOS (Apple Silicon) — основная платформа разработки; Linux/Windows проверяются в CI.
- Наличие Docker/Podman для fallback-песочницы не гарантировано.
- headless Chrome может отсутствовать — браузерный e2e (`scripts/web-e2e.py`, `scripts/plugin-smoke.py`) тогда пропускается.
- Локальный инференс (llama.cpp/Metal) возможен, но не гарантирован по производительности и качеству.

---

## 6. Юридические и брендовые ограничения

- DSH распространяется под MIT: идеи, архитектура и совместимые интерфейсы — можно; копирование кода — только с сохранением уведомления. Код DSH в проект не копируется.
- Название «DeepSeek Harness», логотипы и тексты не используются в публичном продукте; рабочее имя `dshr` подлежит замене.
- Промпты DSH не копируются; используются собственные.
- Лицензия проекта: MIT/Apache-2.0 dual — совместима с зависимостями и удобна для встраивания.

---

## 7. Вне scope

| Направление | Причина | Альтернатива |
|---|---|---|
| Полный аналог DSH в одном релизе | Объём — человеко-месяцы | Поэтапный план и рабочие инкременты |
| Копирование плагинов и промптов DSH | Промпты и UI не копируются; host-плагины запускаются через Cordis-рантайм, а не портируются | Свой ABI + runtime pack, свои промпты |
| Гарантия безопасности на всех ОС | Разные механизмы изоляции | Fail-closed + тесты + документированные гарантии |
| Улучшение модели | Не входит в harness | Выбор модели, промпт, инструменты |
| Все функции DSH одновременно | Расползание scope | Тиры T1 → T2 → T3 |

---

## 8. Предпосылки для продолжения

1. **Тир:** T1 (рекомендуется), T2 или исследовательский прототип.
2. **Доступ к модели:** ключ DeepSeek (или совместимый endpoint) либо mock/replay до M4.
3. **Имя проекта:** `dshr` как рабочее или замена до публикации.
4. **Приоритет поверхности:** TUI, headless, Web.
5. **Ревью опасных зон:** approvals, sandbox, работа с секретами.

---

## 9. Критерии остановки

Проект стоит пересмотреть, если:
- после M2 агент не выполняет типовой сценарий «прочитал → изменил → прогнал тесты» без ручных правок;
- sandbox не удаётся сделать fail-closed на macOS за 10 ИД;
- стоимость эксплуатации (токены) превышает ценность по сравнению с готовым DSH;
- поддержка DSH-совместимости требует больше ресурсов, чем собственная экосистема.

---

## 10. Итог

Проект реализуем. Основные ограничения: (а) объём — месяцы, не дни; (б) внешняя модель и ключи; (в) кросс-платформенная песочница; (г) частичная совместимость плагинной экосистемы: host — да, client — мостом; C-lite даёт React-компоненты слотов с host-hooks, `ctx.get('sessions')`/`ctx.get('slots')`/`ctx.get('locale')`/`ctx.inject`, диагностику `__DSHR_LAST_ERROR__`, conformance-набор n=11 (цель ADR-013 §6 достигнута; C2, раунд 85 — 30+ экспортов design-system шима; раунд 89 (C1.2) — seat-`inject`, Cordis-генераторы и `configForms`, поэтому UI `dsh-plugin-debugging`/`dsh-plugin-studio` рендерится, хотя референсных host-сервисов `pluginManager`/settings по-прежнему нет и browser-форма in-memory), раунд 91 (C1.3) — слот-контракт по референсному каталогу DSH (122 слота с `kind`/`scope`/standard-kit; правила `SlotCore`: undeclared/keyed/chain/duplicate, shadowing-winners, `children`-декларации, deferred `ctx.slots.inject`, standard-kit + `ownerProps`, `ctx.slots.spec(key)`), а раунд 92 (C1.4) выровнял host-сервисы `webServer`/`ctx.baseUrl`/`configForms` по reference-контракту (правила маршрутов и origin-аксессоры, `file://`-baseUrl, snapshot с числовой ревизией и `set`/`unset`; `registerFallback` закрыт в C1.5, `registerUpgrade` — нет), а раунды 98–101 — C2.6 submenu-механику, C3.1 store-seats (`@deepseek-ai/dsh-client-store`, framework-owned instance на scope-key, `useStore`/`actions`), C1.5 fallback-маршруты и ownerProps-plumbing, хотя `hoc`-вложение, renderer-семантика, owner props с функциями (нет callback-моста) и полный chat-view остаются долгом — chain-элекция и owner shares трёх реальных сайтов закрыты в C4.1, раунд 102), design-system примитивы, портированные из reference в C2.1–C2.5 (раунд 93: атомы и все 204 иконки; раунд 94: полный `Tooltip` — placement-engine, `ShortcutKeys`, `input-modality`; раунд 95: ядро `Menu`/`MenuItemButton`; раунд 96: reference submenu page-layout, 200-мс hover grace и `MenuGroup`/`observeStickyMenuGroups`; раунд 97 (C2.5): reference `JsonBlock` и dependency-free GFM-подмножество `MarkdownText`, раунды 98–101: C2.6 submenu-механика (side-card/single-page, `activeList`, hover grace), C3.1 store-seats, C1.5 fallback и ownerProps-plumbing; остаются upstream markdown-стек (mdast/shiki/KaTeX/footnotes/streaming) и renderer-семантика), и частичные host-маршруты DSH-плагинов (C1, раунды 84–86: exact/prefix-маршруты проксируются, реальная host-половина balance-плагина отвечает, маршруты видны в статусе, LAN-публикация только через явный opt-in; раунд 87 (C1.1) закрыл сервисы `sessions`/`sessionProjections` через снимок сервера (задержка ≤ 5 с, упрощённые проекции), но нет стриминга и аутентификации), но полного design-system (C2.1 закрыл атомы и иконки, C2.2 — полный Tooltip, C2.3 — ядро Menu, C2.4 — submenu page-layout/hover grace/MenuGroup, C2.5 — reference JsonBlock и GFM-подмножество MarkdownText, раунды 98–101 — C2.6 submenu-механика, C3.1 store-seats, C1.5 fallback и ownerProps-plumbing; остаются upstream markdown-стек и renderer-семантика), React-контекста шелла и SlotMap-остатков (`hoc`-вложение, renderer-семантика, owner props с функциями без callback-моста и полный chat-view; chain-элекция по `select(ownerProps)` и owner shares трёх реальных сайтов закрыты в C4.1, раунд 102; store seats — в C3.1) нет (C-full — решение владельца, [ADR-041](adr/041-c-full-decision.md): отложен, точечные инкременты); (д) необходимость ревью опасных зон.

При тире T1 и mock-first разработке первый рабочий результат достигается за 4–6 месяцев итераций; каждый этап даёт самостоятельную ценность.
