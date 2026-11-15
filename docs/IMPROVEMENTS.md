# Позиционирование: улучшенная Rust-реализация, а не порт

Версия 1.0 · 12.11.2026

Этот документ фиксирует главное решение проекта: `dshr` **не является** построчным портом `anywhere-labs/dsh-desktop` (Electron + TypeScript + Cordis) и **не является** копией его UI. Мы берём продуктовую концепцию — «тонкая оболочка + мощный агентный host + всё есть плагин» — и реализуем её заново на Rust, устраняя архитектурные компромиссы Electron/Node-стека.

---

## 1. Что сохраняем (концепция)

| Идея DSH Desktop | Как живёт в `dshr` |
|---|---|
| Тонкая оболочка, host отдельно | `dshr serve` (host) + TUI/Web/Tauri как тонкие клиенты |
| Web carrier на loopback (HTTP/WS) | Тот же принцип: единственный протокол между UI и ядром |
| «Всё есть плагин» | Компилируемые крейты + WASM-плагины + Cordis-совместимость + MCP |
| Профили как композиция | Декларативные YAML-профили и бандлы |
| Сессия как источник истины | Event-sourced журнал, resume/fork/replay |
| Песочница и approvals | Нативный seam с политиками и аудитом |
| Desktop-возможности (трей, окно, обновления) | Tauri-оболочка, опционально |
| Экосистема плагинов | Свой реестр артефактов (WASM/OCI) + npm-совместимая установка Cordis-плагинов, MCP-совместимость |

---

## 2. Что улучшаем

### 2.1. Один бинарь вместо Electron + Node + pnpm

**Было:** приложение тащит Chromium, Node 24, pnpm, нативные аддоны; установка — сотни мегабайт, обновления — сложные, ABI-проблемы при смене Node.
**Стало:** один статический бинарь `dshr`; TUI и headless работают без GUI; Web UI отдаётся тем же бинарём.
**Зачем:** меньше размер, память, время старта; проще CI, упаковка и поддержка; нет цепочки поставок npm.

Совместимость с npm/Cordis-плагинами это не ломает: JS-рантайм — отдельный runtime pack, который ставится только при наличии JS-плагинов в профиле ([ADR-013](adr/013-cordis-compat.md), п. 2).

### 2.2. TUI и headless — первого класса

**Было:** ставка на Web/Desktop UI; headless-режим есть, отдельной TUI-поверхности нет.
**Стало:** `dshr` (TUI), `dshr run` (one-shot), `dshr serve` (HTTP/WS) — три равноправные поверхности на одном движке.
**Зачем:** работа по SSH, в контейнерах, в CI и на слабых машинах; автоматизация без браузера.

### 2.3. WASM-тир по умолчанию + сохранённая совместимость npm/Cordis

**Было:** единственный путь расширения — TS-модуль Cordis в общем процессе: HMR и экосистема npm, но полный доступ к рантайму и никакой изоляции.
**Стало:** два тира под одним швом. WASM-компоненты (wasmtime, capability-grants, лимиты) — для недоверенного и переносимого кода; Cordis-рантайм (настоящий `cordis` + реальные npm-пакеты) — для совместимости, HMR и экосистемы; плюс sidecar для языков вне WASM и MCP для внешних серверов. Решение и границы — [ADR-013](adr/013-cordis-compat.md).
**Зачем:** не ставить пользователя перед выбором «безопасно» или «как в DSH»; сильные стороны Cordis остаются там, где они реально живут (JS, UI, экосистема), а WASM закрывает изоляцию.

**Что именно сохраняется у Cordis-тира:** установка из npm с резолвингом зависимостей, HMR, объектный API `ctx.*` (сервисы, события, хуки), клиентские UI-расширения, TS-типы. Единственное изменение — системные вызовы плагина идут через capability-шину (`fs`, `net`, `exec` → политики и аудит): код плагина не меняется, а права становятся наблюдаемыми и ограничиваемыми.

### 2.4. Песочница — часть ядра, а не обёртка

**Было:** sandbox-провайдеры вокруг subprocess в Node-мире, с честным `partial` enforcement.
**Стало:** `dshr-sandbox` — seam с нативными бэкендами (Seatbelt, bwrap+Landlock, ACL/Job Objects), fail-closed, escape suite в CI.
**Зачем:** меньше слоёв, точнее диагностика, единая политика для всех инструментов.

### 2.5. Детерминизм и воспроизводимость

**Стало:** кассеты LLM на уровне адаптера, golden-журналы, property-тесты событий, sandbox escape suite.
**Зачем:** тесты без сети и без токенов; воспроизведение багов пользователя; регрессия поведения при смене модели.

### 2.6. Журнал и индекс вместо «папки с JSON»

**Стало:** append-only JSONL + SQLite FTS5, миграции формата, восстановление после `kill -9`, поиск по всем сессиям.
**Зачем:** быстрый поиск, аудит, безопасное resume, независимость от UI.

### 2.7. Учёт токенов и стоимости как встроенная функция

**Стало:** usage по каждому шагу, агрегация по turn/сессии/дереву субагентов, лимиты и предупреждения.
**Зачем:** агентные прогоны дороги; расход должен быть виден до счёта.

### 2.8. Дистрибуция плагинов без pnpm-цепочки

**Стало:** content-addressed store, подписанные артефакты (WASM/OCI), npm-тарболы через собственный резолвер с lock-файлом, откат версий.
**Зачем:** целостность, воспроизводимость, отсутствие pnpm и транзитивных postinstall-скриптов при сохранении доступа к экосистеме npm.

### 2.9. Remote-first

**Стало:** `dshr serve` можно поднять на удалённом хосте, TUI/Web подключаются по HTTP/WS; SSH-режим — штатный сценарий.
**Зачем:** разработка на удалённых стендах, единый агент для локальных и серверных задач.

### 2.10. Ноль `unsafe` вне FFI и явные границы

**Стало:** `#![forbid(unsafe_code)]` в крейтах, кроме узких FFI-модулей; `cargo-deny`, аудит зависимостей.
**Зачем:** безопасность памяти — главный аргумент Rust; его нельзя терять в обёртках.

---

## 3. Целевые метрики (проверяются в M6)

| Метрика | DSH Desktop (Electron) | Цель `dshr` | Как измеряем |
|---|---|---|---|
| Установочный размер | ~300–700 МБ | ≤ 80 МБ (headless/TUI), ≤ 120 МБ (Web UI) | `du` релизного артефакта |
| Холодный старт | 1,5–4 с | < 400 мс (headless/TUI), < 1,5 с (desktop) | гиперфайн-замер, 20 прогонов |
| RAM в простое | 300–800 МБ | < 120 МБ (headless), < 250 МБ (desktop) | RSS после 5 мин |
| Внешние рантаймы | Node 24, pnpm, Electron | нет в базовой поставке; JS-рантайм — отдельный пакет по требованию | `otool`/`ldd` + дерево процессов |
| Плагины | TS/Cordis, in-process | WASM + Cordis-совместимость + sidecar + MCP | пример плагина каждого тира, аудит capability |
| Совместимость плагинов DSH | 100 % (эталон) | conformance-набор из 8–12 реальных плагинов в nightly CI (числовая цель достигнута с запасом: n=11 — ADR-013 §6 закрыт, smoke-гейт `web-e2e` в CI; отдельного nightly-прогона и отчёта пока нет); набор: `@kt11/dsh-deepseek-balance@0.2.1`, `dsh-remote@1.1.2`, `@ruoyang/dsh-plugin-deepseek-balance@1.0.2`, `dsh-genui@0.3.3`, `dsh-session-superagent@0.10.12`, `dsh-mnemon@0.5.46`, `@99galaxy/dsh-plugin-market@1.2.2`, `dsh-plugin-codegraph-panel@0.6.5`, `@goodandready/dsh-plugin-notify@0.4.9`, `dsh-plugin-debugging@0.8.1`, `dsh-plugin-studio@0.3.1`) | отчёт матрицы «загрузился / инструменты / хуки» |
| Песочница | Node-провайдеры | нативные бэкенды, fail-closed | escape suite |
| Поверхности | Web (Electron) | TUI + Web + headless | e2e по трём поверхностям |

Цифры DSH Desktop — ориентир по Electron-классу, а не измерение конкретной сборки; цель `dshr` фиксируется после M6, до этого — проектная. Бюджеты размера и старта относятся к базовой поставке: при установленном Cordis-рантайме они пересматриваются (размер до ≤ 130 МБ, старт — при активированном JS-плагине).

---

## 4. Чего НЕ делаем

- Не копируем UI, тексты, иконки и бренд DSH Desktop; дизайн свой.
- Не обещаем React-компонентов клиентских UI-плагинов DSH как контракта шелла: мост слотов с eager registry-кадрами и iframe на регистрацию, полный словарь имён DSH разложен по 17 регионам шелла, а не React-совместимость; `ownerProps`/hooks-семантика — вне scope ([ADR-013](adr/013-cordis-compat.md), п. 5). При этом с раунда 77 половина может принести свой UI-стек classic-скриптом `dsh.client.runtime` (включая React), с раунда 78 — опционально взять React 18.3.1 и таблицу модулей от хоста через `dsh.client.host_modules` (один immutable-артефакт на страницу), а с раунда 79 (C-lite) — писать слоты как React-компоненты с host-hooks (`useTheme`/`useSession`/`useSessions`/`useHostState`/`wide`), `ctx.get('sessions')` и частичным design-system шимом `@deepseek-ai/dsh-client-ui-primitives`; раунд 80 добавил сервисы `ctx.get('slots')`/`ctx.get('locale')`/`ctx.inject`, диагностику `__DSHR_LAST_ERROR__` и довёл conformance до n=2 (`dsh-remote@1.1.2`), а раунд 81 — React-контракт моста (типы `React.memo`/`forwardRef`, keyed-ids `<slot>:<key>`, `ctx.locale.register` + `t()`, `ctx.interval`/`ctx.timeout`), primitives шим из 7 компонентов и conformance n=4, а раунд 82 — до n=6 (`dsh-session-superagent@0.10.12` — composer dock, `dsh-mnemon@0.5.46` — крупный multi-region и референсная поверхность слотов), а раунд 83 — до n=9, то есть до цели ADR-013 §6 (8–12 реальных плагинов): `dsh-plugin-market@1.2.2` (сайдбар), `dsh-plugin-codegraph-panel@0.6.5` (панель настроек), `@goodandready/dsh-plugin-notify@0.4.9` (строки plugins-страницы). Раунд 84 добавил **C1** — host HTTP-маршруты плагинов работают (sidecar-сервер + allowlist-прокси, `connect-src` на origin сервера для opt-in кадров; реальная host-половина `@kt11/dsh-deepseek-balance` отвечает своим JSON), остатки — стриминг, отсутствие аутентификации на plugin-маршрутах. Раунд 85 (C2) расширил design-system шим с 7 до 30+ экспортов по фактическим импортам реальных плагинов (компоненты `Input`/`Switch`/`Checkbox`/`Tag`/`Pill`/`Menu`/`MenuSurface`/`Modal`/`Toast`/`SegmentedControl`/`SegmentedTabs`/`DisclosureRow`/`PathLabel`/`TextShimmer`, именованные `Icon*` с Proxy-fallback, `writeClipboard`, хуки `useAnchoredPosition`/`useDismissOnOutsidePointer`) и довёл conformance до n=11 (`dsh-plugin-debugging@0.8.1`, `dsh-plugin-studio@0.3.1` активируются без ошибок, их settings-UI оставался инертным без host-сервисов `pluginManager`/settings), а раунд 86 (хардненинг C1) добавил prefix-маршруты, `routes` в `GET /v1/plugins/status`, фильтр MCP-псевдо-инструментов и loopback-only политику с opt-in `--allow-lan-plugin-routes`, а раунд 87 (C1.1) — сервисы `sessions`/`sessionProjections` из внутреннего снимка сервера (usage-маршруты реальных плагинов считают статистику по сессиям dshr; задержка ≤ 5 с, проекции упрощены, содержимое сессий намеренно не мостится), а раунд 89 (C1.2) — seat-`inject` в регистрациях слотов, Cordis-генераторы в `slots.inject`/`ctx.inject`/`ctx.effect` и host-сервис `configForms` (in-memory writable-форма в кадре, файловая — у host-половин), поэтому settings-UI `dsh-plugin-studio` (вкладки в Settings) и `dsh-plugin-debugging` (карточка в Plugins с рабочим переключателем и Save) реально рендерится, а не остаётся инертным, а раунд 91 (C1.3) перевёл слот-контракт на сгенерированный каталог DSH (122 слота с `kind`/`scope`/standard-kit; правила `SlotCore` — undeclared/keyed/chain/duplicate, shadowing-winners, `children`-декларации, deferred `ctx.slots.inject`, standard-kit + `ownerProps`, `ctx.slots.spec(key)`), а раунд 92 (C1.4) выровнял host-сервисы `webServer` (правила exact/prefix-маршрутов, duplicate-throw, origin-аксессоры), `ctx.baseUrl` (`file://` корня пакета) и `configForms` (reference-shape snapshot `{status, value, base, user, revision: number, writable}`, числовая ревизия, `set`/`unset`) по reference-контракту, причём `registerFallback`/`registerUpgrade` в sidecar не поддержаны, а раунд 93 (C2.1) портировал атомы design-system и все 204 иконки из reference-исходников (`deepseek-ai/deepseek-harness` @ `39163dd`, `packages/client/ui-primitives/src`: `Button`/`Input`/`Switch`/`Checkbox`/`Tag`/`Pill`/`SegmentedControl`/`SegmentedTabs`/`DisclosureRow`/`TextShimmer`/`StateDot`/`PathLabel`/`Toast`/`Modal`/`Menu`/`MenuSurface`/`Tooltip`, точные SVG-пути иконок, `--dsw-alias-*` токены, артефакт на конец раунда 375 894 байта), а раунд 94 (C2.2) довёл `Tooltip` до полного reference-порта (placement-engine, `ShortcutKeys`, `input-modality`; артефакт 387 613 байт), а раунд 95 (C2.3) портировал ядро `Menu`/`MenuItemButton` по reference (ряды/selection/aria, reserve-icon, клавиатурный walk, one-level submenu, dismissal; артефакт 405 323 байта), а раунд 96 (C2.4) — reference submenu page-layout (single-page с non-selecting back-row, Escape/ArrowLeft на родителя), 200-мс hover grace и `MenuGroup`/`observeStickyMenuGroups` (артефакт 415 972 байта), а раунд 97 (C2.5) — reference `JsonBlock` и API-совместимое dependency-free GFM-подмножество `MarkdownText` (артефакт 424 801 байт), а раунды 98–101 — C2.6 submenu-механику (side-card/single-page по измеренной ширине, `fixed` flip/clamp, `activeList`, Escape/ArrowLeft, 200-мс hover grace), C3.1 store-seats (`@deepseek-ai/dsh-client-store` в module-таблице, `useStore`/`actions`, inject-аргументы), C1.5 fallback-маршруты (`webServer.registerFallback`) и ownerProps-plumbing (артефакт 430 139 байт), так что остаются upstream markdown-стек (mdast/shiki/KaTeX/footnotes/streaming), renderer-семантика, `registerUpgrade` (WebSocket) и ownerProps от реальных shell-сайтов. Полный design-system, React-контекст/состояние шелла и SlotMap-остатки — путь C-full в [ADR-040](adr/040-react-client-half.md), то есть уже решение владельца, а не остаток C-lite; [ADR-041](adr/041-c-full-decision.md) зафиксировал отложение до появления спроса.
- Не тащим Node как обязательную зависимость: JS-рантайм — runtime pack по требованию.
- Не тащим Electron и Chromium; desktop — Tauri (системный webview) и только как опция.
- Не обещаем побайтовый паритет поведения промптов и эвристик.
- Не поддерживаем «все фичи DSH» сразу: тиры T1→T2→T3.

---

## 5. Компромиссы улучшений

| Улучшение | Чем платим |
|---|---|
| Нет Electron | Tauri использует системный webview: различия рендеринга между ОС; UI-полировка дольше |
| WASM-тир | Свой SDK и своя экосистема; UI-расширения в WASM не сделать |
| Совместимость Cordis | Runtime pack (десятки МБ по требованию), дрейф Cordis/Node API, отслеживание CVE V8/Node, client-половина только через iframe-мост: свой React можно принести через `dsh.client.runtime`, React от хоста — `dsh.client.host_modules`, C-lite даёт React-компоненты слотов с host-hooks, сервисы `ctx.get('slots')`/`locale`/`ctx.inject`, диагностику `__DSHR_LAST_ERROR__`, conformance n=11 (цель ADR-013 §6 достигнута с запасом; раунд 85 добавил `dsh-plugin-debugging`/`dsh-plugin-studio` с инертным UI на host-сервисах `pluginManager`/settings) и design-system примитивы, портированные из reference в C2.1–C2.5 (раунд 93: атомы и все 204 иконки из `deepseek-ai/deepseek-harness` @ `39163dd`, артефакт на конец раунда 375 894 байта), а раунд 94 (C2.2) довёл `Tooltip` до полного reference-порта (placement-engine, `ShortcutKeys`, `input-modality`; артефакт 387 613 байт), а раунд 95 (C2.3) — ядро `Menu`/`MenuItemButton` (артефакт 405 323 байта), а раунд 96 (C2.4) — reference submenu page-layout, 200-мс hover grace и `MenuGroup`/`observeStickyMenuGroups` (артефакт 415 972 байта), а раунд 97 (C2.5) — reference `JsonBlock` и dependency-free GFM-подмножество `MarkdownText` (артефакт 424 801 байт), а раунды 98–101 — C2.6 submenu-механика, C3.1 store-seats, C1.5 fallback-маршруты и ownerProps-plumbing (артефакт 430 139 байт); остаются upstream markdown-стек (mdast/shiki/KaTeX/footnotes/streaming), renderer-семантика, `registerUpgrade` и ownerProps от реальных shell-сайтов; C1 (раунды 84–86) добавляет host HTTP-маршруты плагинов (sidecar-сервер + allowlist-прокси, `connect-src` для opt-in кадров; prefix-маршруты, `routes` в статусе, фильтр MCP-псевдо-инструментов и loopback-only политика с opt-in `--allow-lan-plugin-routes`) с остатками (стриминг, отсутствие аутентификации на plugin-маршрутах; сервисы `sessions`/`sessionProjections` с раунда 87 (C1.1) — снимок с задержкой ≤ 5 с); но без полного design-system/React-контекста шелла/SlotMap-остатков (chain select-диспетчеризация, hoc-вложение, renderer-семантика, ownerProps от shell; store seats закрыты в C3.1, раунды 98–101; каталог/регистрация/kinds/winners/children/deferred inject закрыты в C1.3, раунд 91; host-сервисы `webServer`/`ctx.baseUrl`/`configForms` выровнены по reference-контракту в C1.4, раунд 92; `registerFallback` закрыт в C1.5, `registerUpgrade` — нет; C-full ADR-040) |
| Один бинарь | Размер Rust-бинаря и время сборки выше, чем у «пустого» Node-рантайма |
| TUI-first | Часть пользователей ждёт GUI; Web UI нужно догонять |
| Нативная песочница | Разные гарантии по ОС; Windows сложнее и позже |
| Свой формат сессий | Нет прямой совместимости с DSH; импорт — адаптер |
| Ноль `unsafe` | Иногда медленнее/многословнее, чем ручные оптимизации |

---

## 6. Как доказываем, что «улучшенная»

1. **Бенчмарки** размера/старта/RAM в CI (артефакты + отчёт).
2. **Escape suite** песочницы на macOS/Linux.
3. **Evals**: 20 задач (поиск, правка, тесты, рефакторинг) с метриками успеха/стоимости.
4. **Parity-матрица** возможностей против DSH 0.2.2-rc.2 — честно, с `—` там, где не делаем.
5. **Публичный отчёт** «что лучше и чем платим» после M6.

---

## 7. Влияние на план

- M0–M2 не меняются: ядро, цикл, инструменты, TUI.
- M1: DeepSeek-адаптер — Anthropic Messages wire (проверено, см. [PROVIDER-VERIFICATION.md](PROVIDER-VERIFICATION.md)).
- M5: плагины — WASM + подписанные артефакты; песочница нативная.
- M5b: Cordis-совместимость — runtime pack, capability-шина для builtins, conformance-набор ([ADR-013](adr/013-cordis-compat.md)).
- M6: Web UI + `dshr serve`; desktop-оболочка (Tauri) переезжает в T3.
- T3: Tauri, трей, автообновление, подпись; сравнение с DSH Desktop по метрикам §3.
