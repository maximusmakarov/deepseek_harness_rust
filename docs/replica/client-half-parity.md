# Клиентская половина: паритет с DSH 0.2.2-rc.2

Оценка по методике Replica (`replica-diff`): матрица `client-half-features.csv`
против нашего моста iframe. Скоринг считает must=3, should=2, could=1, partial —
половина, `skip` и наши собственные фичи не входят.

Матрица: [client-half-features.csv](client-half-features.csv)

## Итог

- **Parity: 98.1 / 100** (53 из 54 баллов)
- must-haves: **13 из 13** — по правилам Replica это «shippable» (все must сделаны,
  score 80+, открытых S1/S2 нет)
- should: 6 done, 1 partial; could: 1 done в скоринге + 4 dshr-дополнения вне
  скоринга (deep link, plugin-shipped runtime, shared host module table и
  C-lite React-компоненты слотов); раунд 84 добавил C1 host HTTP-маршруты плагинов,
  раунд 85 — C2 (design-system шим с 30+ экспортами и conformance n=11),
  раунд 86 — хардненинг C1 (prefix-маршруты, `routes` в статусе, фильтр
  MCP-псевдо-инструментов, loopback-only LAN-политика), а раунд 87 — C1.1
  (мост `sessions`/`sessionProjections` из снимка сервера) в ту же
  строку — новых скоринговых строк нет, additions по-прежнему `original=no`
- сознательно вне скоупа (`skip`): полный React-компонентный контракт, инъекция в
  DOM шелла, внешние модули (`dsh.client.external`) — следствие ADR-013 §5, а не
  недоделка. C-lite (раунды 79–85; conformance n=11 — цель ADR-013 §6 8–12
  реальных плагинов достигнута с запасом) даёт частичную React-совместимость для простых
  UI-плагинов вне скоринга; путь к полной — [ADR-040](../adr/040-react-client-half.md)
  (Proposed: C-full после C-lite, решение владельца)

Арифметика по документированным весам (13 must, 7 should, 1 could; один should —
partial, то есть половина):

| Компонент | Баллы | Максимум |
|---|---|---|
| must done: 13 × 3 | 39 | 39 |
| should done: 6 × 2 | 12 | 12 |
| should partial: 1 × 2 × ½ | 1 | 2 |
| could done: 1 × 1 | 1 | 1 |
| **Итого** | **53** | **54** |

53 / 54 = 98.15 → **98.1**. Ровно 100 нет из-за единственного partial should:
module-table бандл исполняется, React от хоста теперь доступен opt-in
(`dsh.client.host_modules`, раунд 78), а в раунде 79 добавлен частичный
design-system шим, но полной таблицы модулей DSH (полный design-system,
React-контекст шелла) по-прежнему нет.

Строка раунда 77 `Plugin-shipped runtime/module table (bring your own React)` —
**dshr-дополнение** (`original=no`, как и deep link): enabler `dsh.client.runtime`
позволяет половине принести свою модульную таблицу и, например, собранный React.
В скоринг такие строки не входят, поэтому parity не меняется — те же 53 из 54.
Если бы строка считалась фичей оригинала, было бы 54/55 = 98.18 → 98.2; в матрице
она намеренно помечена `original=no`.

Строка раунда 78 `Shared host module table (React served by the host)` — тоже
**dshr-дополнение** (`original=no`): половина опционально берёт React и таблицу
модулей от хоста, host-страница грузит один версионированный immutable-артефакт
`/v1/runtime/react-18.3.1.js` вместо встраивания ~140 КБ в каждую страницу.
Скоринг от неё не меняется: must 13/13 (39 из 39), should 6 done (12 из 12) и
1 partial (1 из 2), could 1 done (1 из 1) — **53 из 54 = 98.15 → 98.1**.

Строка раунда 79 `React component slots with host hooks/services + partial
design-system` — тоже **dshr-дополнение** (`original=no`, как и предыдущие
runtime-строки): при host React половина регистрирует React-компонент слот-контракта
(`ctx.slots.register(entry, Component)`), мост держит один React-root на кадр,
компонент получает host-hooks (`useTheme`/`useSession`/`useSessions`/`useHostState`)
и `wide` (пока всегда true), `ctx.get('sessions')` отдаёт подмножество референсного
сервиса, а общий рантайм публикует частичный независимый design-system шим
`@deepseek-ai/dsh-client-ui-primitives`; conformance — реальный MIT-плагин
`@kt11/dsh-deepseek-balance@0.2.1`. Скоринг не меняется, арифметика та же:
must 13/13 (13 × 3 = 39 из 39), should 6 done (6 × 2 = 12 из 12) и 1 partial
(1 × 2 × ½ = 1 из 2), could 1 done (1 × 1 = 1 из 1) — **39 + 12 + 1 + 1 = 53 из
54 = 98.15 → 98.1**. Если бы строка считалась фичей оригинала, было бы
54/55 = 98.18 → 98.2; поэтому она намеренно помечена `original=no`. Строка
`React component model` в матрице остаётся must/`skip`: C-lite — не полный
React-контракт (нет SlotMap-семантики, полного design-system, React-контекста
шелла и host-маршрутов).

Раунд 80 расширил **ту же строку** C-lite, не добавляя новых скоринговых строк:
сервисы моста, на которые рассчитаны референсные плагины — `ctx.get('slots')`
(реестр слотов с `entries(key)`), `ctx.get('locale')` (минимальный
`{active: "en", locale: "en"}`) и `ctx.inject(deps, callback)` (callback только
при всех резолвнутых зависимостях, поэтому опциональные
`betterSidebar`/`sidebarRight`/`webTerminals` не монтируются), плюс диагностика
`window.__DSHR_LAST_ERROR__` — и второй conformance-плагин `dsh-remote@1.1.2`
(MIT, клиентская половина upstream без изменений, sha256 в `package.json`).
Строка по-прежнему **dshr-дополнение** (`original=no`, could/yes), поэтому
арифметика не меняется: must 13/13 (13 × 3 = 39 из 39), should 6 done
(6 × 2 = 12 из 12) и 1 partial (1 × 2 × ½ = 1 из 2), could 1 done (1 × 1 = 1 из
1) — **39 + 12 + 1 + 1 = 53 из 54 = 98.15 → 98.1**. Если бы сервисы моста или
второй плагин считались фичами оригинала, знаменатель вырос бы; в матрице
строка намеренно помечена `original=no`.

Раунд 81 расширил **ту же строку** C-lite, не добавляя новых скоринговых строк:
React-контракт моста принял типы `React.memo`/`forwardRef` (объекты с `$$typeof`,
не только функции), keyed-регистрации без `id` получили уникальные id
`<slot>:<key>`, `ctx.get('slots').entries(key)` — референсную форму с `component` и
`options`, `ctx.locale.register(ns, dicts)` + `t()` в props компонента и
`ctx.interval`/`ctx.timeout`, снимаемые вместе с фреймом; шим primitives вырос до
7 компонентов (`JsonBlock`/`MarkdownText`), а conformance — до n=4
(`@ruoyang/dsh-plugin-deepseek-balance@1.0.2`, `dsh-genui@0.3.3`). Строка
по-прежнему **dshr-дополнение** (`original=no`, could/yes), новых строк в матрице
нет, поэтому арифметика не меняется: must 13/13 (13 × 3 = 39 из 39), should 6 done
(6 × 2 = 12 из 12) и 1 partial (1 × 2 × ½ = 1 из 2), could 1 done (1 × 1 = 1 из 1)
— **39 + 12 + 1 + 1 = 53 из 54 = 98.15 → 98.1**. Накопленные dshr-дополнения (deep
link, plugin-shipped runtime, shared host module table, C-lite) по-прежнему
`original=no`, поэтому знаменатель не растёт.

Раунд 82 расширил **ту же строку** C-lite, не добавляя новых скоринговых строк:
conformance вырос до n=6 — vendored `dsh-session-superagent@0.10.12` (MIT,
клиентская половина upstream без изменений, sha256 в `package.json`) коммитит
composer dock (`conversation.composer.dock`), а `dsh-mnemon@0.5.46` (крупный
multi-region плагин, та же схема vendored fixture) активируется, публикует слоты
и сверяет собственные shadow-регистрации через референсную поверхность слотов
(`ctx.slots.subscribe`/`getVersion`/`entriesOfSlot`), добавленную в раунде 81, —
правок моста не потребовалось. Строка по-прежнему **dshr-дополнение**
(`original=no`, could/yes), новых строк в матрице нет, поэтому арифметика не
меняется: must 13/13 (13 × 3 = 39 из 39), should 6 done (6 × 2 = 12 из 12) и
1 partial (1 × 2 × ½ = 1 из 2), could 1 done (1 × 1 = 1 из 1) —
**39 + 12 + 1 + 1 = 53 из 54 = 98.15 → 98.1**. Накопленные dshr-дополнения (deep
link, plugin-shipped runtime, shared host module table, C-lite) по-прежнему
`original=no`, поэтому знаменатель не растёт.

Раунд 83 расширил **ту же строку** C-lite, не добавляя новых скоринговых строк:
conformance вырос до **n=9** — цель ADR-013 §6 (8–12 реальных плагинов)
достигнута. Vendored MIT-плагины с upstream-клиентской половиной без изменений
(sha256 в `package.json`, host-половины — заглушки):
`@99galaxy/dsh-plugin-market@1.2.2` (`sidebar.panellist` → регион
`sidebar.footer`, только `react`), `dsh-plugin-codegraph-panel@0.6.5` (панель
`settings.section`, upstream `immediately: true`, только `react`) и
`@goodandready/dsh-plugin-notify@0.4.9` (`plugins.item`/`plugins.row.config` →
регион `plugins.detail`, primitives + react). Все три активируются, публикуют
слоты и не оставляют ошибок; smoke монтирует sidebar-кадр market. Строка
по-прежнему **dshr-дополнение** (`original=no`, could/yes), новых строк в
матрице нет, поэтому арифметика не меняется: must 13/13 (13 × 3 = 39 из 39),
should 6 done (6 × 2 = 12 из 12) и 1 partial (1 × 2 × ½ = 1 из 2), could 1 done
(1 × 1 = 1 из 1) — **39 + 12 + 1 + 1 = 53 из 54 = 98.15 → 98.1**. Если бы три
новых плагина или достижение целевого диапазона считались фичами оригинала,
знаменатель вырос бы; в матрице строка намеренно помечена `original=no`.
Накопленные dshr-дополнения (deep link, plugin-shipped runtime, shared host
module table, C-lite) по-прежнему `original=no`, поэтому знаменатель не растёт.

Раунд 84 расширил **ту же строку** C-lite, не добавляя новых скоринговых строк:
host HTTP-маршруты плагинов стали работать end-to-end (**C1**). Sidecar поднимает
loopback-only HTTP-сервер (`127.0.0.1`, случайный порт, per-process токен), половина
регистрирует маршруты через `ctx.inject(['webServer'])`/`webServer.register({name,
kind: 'exact'/'prefix', path, handler})`, менеджер читает внутренний `__dshr_routes`
(`__dshr_*`-инструменты отфильтрованы из реестра) и отдаёт `http_route(path)`, а
dshr-сервер проксирует только allowlist-пути (method/query/body, request ≤ 1 МиБ,
response ≤ 8 МиБ, таймаут 15 с, permissive CORS для opaque-origin кадров, reserved
и неизвестные пути → 404). Кадры с `host_modules` получили `connect-src <origin
сервера>` — без этого браузерный fetch плагина к своему маршруту не проходил;
кадры без `host_modules` остаются network-free. Реальная host-половина
`@kt11/dsh-deepseek-balance` больше не застаблена (upstream вендорен дословно,
host sha256 `8771ae90…`, client sha `f0a4080…` не менялся), и smoke через кадр
плагина проверяет её JSON: `/dsh-balance/deepseek` → `500 {ok:false,
code:"not-configured"}` без ключа, `/dsh-usage/sessions?tz=UTC` → `200` с JSON и
массивом `sessions`, неизвестный путь → dshr 404. Строка по-прежнему
**dshr-дополнение** (`original=no`, could/yes), новых строк в матрице нет, поэтому
арифметика не меняется: must 13/13 (13 × 3 = 39 из 39), should 6 done
(6 × 2 = 12 из 12) и 1 partial (1 × 2 × ½ = 1 из 2), could 1 done (1 × 1 = 1 из 1)
— **39 + 12 + 1 + 1 = 53 из 54 = 98.15 → 98.1**. Если бы C1 считался фичей
оригинала, знаменатель вырос бы; в матрице строка намеренно помечена `original=no`.
Накопленные dshr-дополнения (deep link, plugin-shipped runtime, shared host module
table, C-lite) по-прежнему `original=no`, поэтому знаменатель не растёт.

Раунд 85 расширил **ту же строку** C-lite, не добавляя новых скоринговых строк:
design-system шим вырос с 7 до 30+ экспортов по фактическим импортам реальных
плагинов — компоненты `Input`, `Switch`, `Checkbox`, `Tag`, `Pill`, `Menu`,
`MenuSurface` (forwardRef-контейнер), `Modal`, `Toast`, `SegmentedControl`,
`SegmentedTabs`, `DisclosureRow`, `PathLabel`, `TextShimmer` (прежние
`Button`/`StateDot`/`Tooltip`/`Icon`/`Spinner`/`JsonBlock`/`MarkdownText`
сохранены, `Tooltip` стал `forwardRef`), именованные `Icon*`-компоненты с
SVG-глифами и Proxy-fallback для неизвестных имён, `writeClipboard`
(navigator.clipboard + execCommand fallback) и хуки `useAnchoredPosition` /
`useDismissOnOutsidePointer`. Conformance вырос до **n=11**: vendored
`dsh-plugin-debugging@0.8.1` (MIT, client sha256 `9b828160…`) и
`dsh-plugin-studio@0.3.1` (MIT, client sha256 `1194c18b…`) — клиентские половины
upstream без изменений, host-половины — заглушки; обе активируются без ошибок, но
их UI гейтится host-сервисами `pluginManager`/settings и остаётся
несмонтированным (штатная деградация, как у `dsh-remote` с опциональными
сервисами). Smoke (79/79) добавляет две проверки: дерево всех смоделированных
примитивов рендерится в host-React кадре (12 текстовых маркеров, портальные
Menu/Modal/Toast), а реальные вклады `dsh-plugin-notify`/`dsh-mnemon` монтируются
в диалоге Plugins без `__DSHR_LAST_ERROR__`. Строка по-прежнему
**dshr-дополнение** (`original=no`, could/yes), новых строк в матрице нет,
поэтому арифметика не меняется: must 13/13 (13 × 3 = 39 из 39), should 6 done
(6 × 2 = 12 из 12) и 1 partial (1 × 2 × ½ = 1 из 2), could 1 done (1 × 1 = 1 из 1)
— **39 + 12 + 1 + 1 = 53 из 54 = 98.15 → 98.1**. Если бы C2 считался фичей
оригинала, знаменатель вырос бы; в матрице строка намеренно помечена `original=no`.
Накопленные dshr-дополнения (deep link, plugin-shipped runtime, shared host module
table, C-lite) по-прежнему `original=no`, поэтому знаменатель не растёт.

Раунд 86 расширил **ту же строку** C1/C-lite, не добавляя новых скоринговых
строк: хардненинг C1. `webServer.register` принимает `kind: "exact"` и
`kind: "prefix"` (exact выигрывает, среди prefix — самый длинный; так же в
`CordisManager::http_route` и fallback-прокси), `GET /v1/plugins/status` отдаёт
per-plugin `routes` — живую таблицу host-маршрутов рядом с `tools`,
MCP-псевдо-инструменты (`list_resources`/`read_resource`/`list_prompts`/
`get_prompt`) отфильтрованы из реестра инструментов плагина, а plugin-маршруты
при bind вне loopback отвечают 403, пока явно не передан
`--allow-lan-plugin-routes` (`ServerConfig.allow_plugin_routes_lan`) — это
закрывает ранее записанный долг «routes public as DSH / LAN exposure». Строка
по-прежнему **dshr-дополнение** (`original=no`, could/yes), новых строк в
матрице нет, поэтому арифметика не меняется: must 13/13 (13 × 3 = 39 из 39),
should 6 done (6 × 2 = 12 из 12) и 1 partial (1 × 2 × ½ = 1 из 2), could 1 done
(1 × 1 = 1 из 1) — **39 + 12 + 1 + 1 = 53 из 54 = 98.15 → 98.1**. Если бы
хардненинг C1 считался фичей оригинала, знаменатель вырос бы; в матрице строка
намеренно помечена `original=no`. Накопленные dshr-дополнения (deep link,
plugin-shipped runtime, shared host module table, C-lite) по-прежнему
`original=no`, поэтому знаменатель не растёт.

Раунд 87 расширил **ту же строку** C1/C-lite, не добавляя новых скоринговых
строк: мост `sessions`/`sessionProjections` (**C1.1**). Сервер пишет
`<home>/cordis/host.json` (`{url, token}`) и публикует
`GET /v1/internal/session-snapshot` — recent ≤50 сессий (id/title/created_at/
model/turns/steps и usage-события assistant/turn/step/tokens; содержимое
сообщений и промпты не покидают сервер; 404 без настроенного токена, 401 при
неверном `x-dshr-internal-token`). Sidecar получает путь через
`DSHR_HOST_BRIDGE`, раз в 5 с (unref-таймер) обновляет снимок и транслирует его
в DSH-shaped сервисы: `ctx.get('sessions')` (`list()`/`get(id)`,
`header.createdAt` в epoch ms, DSH-shaped `log` с `request/context`,
`session/title`, `assistant/message` c `usage`) и
`ctx.get('sessionProjections')` (`snapshot(session).values`: `sessionStats`
turns/steps/`llmMs`=0 и `tokenUsage`). В результате usage-маршруты реального
`@kt11/dsh-deepseek-balance` (`/dsh-usage/sessions`) перечисляют фактические
сессии dshr и считают totals — smoke опрашивает маршрут из кадра плагина и
требует `sessions` ≥ 1 и totals (`plugins_live` 16, smoke 80/80). Строка
по-прежнему **dshr-дополнение** (`original=no`, could/yes), новых строк в
матрице нет, поэтому арифметика не меняется: must 13/13 (13 × 3 = 39 из 39),
should 6 done (6 × 2 = 12 из 12) и 1 partial (1 × 2 × ½ = 1 из 2), could 1 done
(1 × 1 = 1 из 1) — **39 + 12 + 1 + 1 = 53 из 54 = 98.15 → 98.1**. Если бы C1.1
считался фичей оригинала, знаменатель вырос бы; в матрице строка намеренно
помечена `original=no`. Накопленные dshr-дополнения (deep link, plugin-shipped
runtime, shared host module table, C-lite) по-прежнему `original=no`, поэтому
знаменатель не растёт.

Что осталось из should (не блокирует):

| Фича | Состояние | Чем отличается |
|---|---|---|
| Bundles built for the reference module table | partial | шим `__ModuleLoader__.load({id, factory})` исполняет `factory` и `apply(ctx)`; `require` резолвит модули из `dsh.client.runtime`, если половина их опубликовала (в PoC — React 18.3.1), или из общего рантайма хоста, если половина опционально объявила `dsh.client.host_modules` (React 18.3.1 + jsx-шимы + частичный design-system шим `@deepseek-ai/dsh-client-ui-primitives` — с раунда 85 (C2) 30+ экспортов, включая компоненты, именованные `Icon*` с Proxy-fallback и хуки). Полной таблицы модулей DSH (полный design-system, React-контекст шелла) нет — это единственная незакрытая should-фича, поэтому score 98.1, а не 100 |

## Чем это подтверждено

| Область | Доказательство |
|---|---|
| discovery | `crates/cordis/src/client.rs`: `discover_client_half`, тесты на entry/exports/escape; серверный тест `/v1/plugins/clients` |
| security | канонизация пути, confinement пакета, тест traversal; capability-ассеты проверяются на чужой/просроченный cap (серверный тест) |
| slots | `crates/cordis/src/bridge.js`: таблица `SLOT_REGIONS` — 116 ключей DSH 0.2.2-rc.2 плюс legacy `panel`/`message.action`, 17 регионов, fallback `panel`; `tests/bridge_harness.mjs` (порядок, группы `inject`, `list()`, slot-режим); smoke: вкладки и переключение слотов |
| regions | eager registry-кадр на половину (`mode=registry`) и iframe `mode=slot` на регистрацию (`crates/web/src/main.rs`: `SlotRegion`/`SlotFrame`/`RegistryFrames`/`PanelRegion`); smoke: монтаж в каждый регион, fallback незнакомого имени, `context` повторяющихся регионов, закрытие диалога снимает его фреймы |
| lifecycle | harness: `activate`/`deactivate`/`dispose`, `effect` cleanup; smoke: eager-кадр переживает закрытие, горячая перезагрузка панели |
| theme | harness: `onTheme`; smoke: переключение темы доходит до всех фреймов |
| actions | harness: round-trip `action-result` с адресацией отправителю; smoke: `close`, `reload`, `compose`, `notify`, неизвестное действие |
| loading | серверные тесты inline и capability-ассетов; smoke: многофайловый бандл и module-table бандл рендерятся |
| runtime | `crates/cordis/src/client.rs`: `ClientHalf.runtime`/`runtime_path`, `host_page`/`host_page_assets` встраивают runtime до бандла, `requireShim` резолвит `window.__DSHR_MODULES__`; `crates/server/src/lib.rs`: `/host` читает и валидирует runtime (confined к пакету, ≤ 8 МиБ, `</script>` → 400); тесты: cordis +4, `plugins_live` +3, smoke +3 (discovery, runtime-before-bundle, require render); React 18.3.1 PoC (UMD в `runtime.js`, 143 197 байт) — рендер, `useState`, `ctx.onTheme` проверены вручную |
| shared runtime | `crates/cordis/src/client.rs`: `HOST_MODULES`, `HOST_RUNTIME_VERSION`, `host_runtime_url`, `PageRuntime` (порядок shared → plugin runtime → bundle); `crates/server/src/lib.rs`: `GET /v1/runtime/{file}` (immutable-кэш, 404 на неизвестную версию), `client_host_csp` (origin сервера только для opt-in кадров); `crates/server/assets/runtime/` + `scripts/check-runtime-assets.sh`; тесты: cordis +4, `plugins_live` +3, smoke +5 (discovery `host_modules`, immutable-артефакт, host-страница без встроенного React, React из общего рантайма, `useState`) |
| react slots / host state | `crates/cordis/src/bridge.js`: React-режим `ctx.slots.register(entry, Component)`, один React-root на кадр, `host-state` (`theme`/`wide`/`session`/`sessions`), hooks `useTheme`/`useSession`/`useSessions`/`useHostState`, `ctx.get('sessions')`, error boundary; `crates/web/src/main.rs`: `push_host_state_all`; `crates/cordis/src/client.rs`: `HOST_MODULES` с id примитивов; тесты: bridge harness 87 checks (host-state/`ctx.get`/`ctx.get('slots')`/`locale`/`ctx.inject`/`__DSHR_LAST_ERROR__` + раунд-81 `reactScenario`: memo-тип, host-props/hooks, `t()`-место, keyed-ids, форма `slots.entries`), smoke 80/80 (фикстуры `hooks`, `hosted`, `ui-kit`, `dsh-remote`, `ruoyang-balance`, `dsh-genui`, `dsh-session-superagent`, `dsh-mnemon`, `dsh-plugin-market`, `dsh-plugin-codegraph-panel`, `dsh-plugin-notify`, `dsh-plugin-debugging`, `dsh-plugin-studio`; C1 host-маршруты — через реальную host-половину `@kt11/dsh-deepseek-balance`) |
| design-system primitives | `crates/server/assets/runtime/primitives.js` — независимый **частичный** шим `@deepseek-ai/dsh-client-ui-primitives` (`Button`, `StateDot`, `Tooltip`, `Icon`, `Spinner`; с раунда 81 — ещё `JsonBlock`/`MarkdownText`; с раунда 85 (C2) — 30+ экспортов: `Input`/`Switch`/`Checkbox`/`Tag`/`Pill`/`Menu`/`MenuSurface`/`Modal`/`Toast`/`SegmentedControl`/`SegmentedTabs`/`DisclosureRow`/`PathLabel`/`TextShimmer`, `writeClipboard`, `useAnchoredPosition`/`useDismissOnOutsidePointer` и 18 именованных `Icon*` с Proxy-fallback) на host React и CSS-переменных темы, в `checksums.txt`/`README.md`; smoke: фикстура `ui-kit` резолвит его из общего рантайма, genui использует `JsonBlock`/`MarkdownText`, а C2-проба рендерит дерево всех смоделированных примитивов (12 маркеров, портальные Menu/Modal/Toast) |
| conformance | **Одиннадцать** реальных MIT-плагинов в `crates/cordis/tests/fixtures/` (n=11 — цель ADR-013 §6 8–12 достигнута с запасом; клиентские половины upstream без изменений, sha256 в `package.json`, host-половины — заглушки, кроме `@kt11/dsh-deepseek-balance` с раунда 84): `@kt11/dsh-deepseek-balance@0.2.1` — smoke: React-карточка в `sidebar.footer.action`, инжект собственного CSS, popup по клику; с раунда 84 (C1) host-половина реальная (upstream без изменений, host sha256 `8771ae90…`), и её маршруты отвечают через прокси: 500 `not-configured` без ключа и 200 JSON для usage-маршрута; `dsh-remote@1.1.2` (раунд 80) — smoke: discovery с примитивами в `host_modules`, registry-кадр без ошибок при отсутствии опциональных сервисов, React-слот `conversation.session.header.actions` монтируется в регион `conversation.header`; `@ruoyang/dsh-plugin-deepseek-balance@1.0.2` (раунд 81, sha256 `ca700695…`) — плавающий бейдж в `shell.overlay` через `ctx.effect`/`ctx.interval`/`ctx.locale`, рендерится без ошибок; `dsh-genui@0.3.3` (раунд 81, sha256 `8f0500ad…`) — тумблер GenUI в `conversation.input.left` и keyed-регистрация `conversation.chat.node` через `React.memo`, рендерится без ошибок; `dsh-session-superagent@0.10.12` (раунд 82, sha256 `d58f0fa2…`) — composer dock `conversation.composer.dock` монтируется в регион `composer.actions` без ошибок; `dsh-mnemon@0.5.46` (раунд 82, sha256 `d967402b…`) — крупный multi-region плагин: активируется, публикует слоты и сверяет собственные shadow-регистрации через `ctx.slots.subscribe`/`getVersion`/`entriesOfSlot`; `@99galaxy/dsh-plugin-market@1.2.2` (раунд 83, sha256 `27abb2db…`) — запись в сайдбаре (`sidebar.panellist` → регион `sidebar.footer`), smoke: кадр монтируется; `dsh-plugin-codegraph-panel@0.6.5` (раунд 83, sha256 `f834fadb…`) — панель `settings.section` (upstream-флаг `immediately: true`), активируется и публикует слот; `@goodandready/dsh-plugin-notify@0.4.9` (раунд 83, sha256 `7a2843fa…`) — строки `plugins.item`/`plugins.row.config` (регион `plugins.detail`), primitives + react, активируется и публикует слоты; `dsh-plugin-debugging@0.8.1` (раунд 85, C2, client sha256 `9b828160…`) и `dsh-plugin-studio@0.3.1` (раунд 85, C2, client sha256 `1194c18b…`) — примитивы-heavy половины (30+ экспортов шима): активируются без ошибок, но их UI гейтится host-сервисами `pluginManager`/settings и не монтируется. Не входят в набор: `@michengai/dsh-archive-manager` (нужен `'unsafe-eval'` в CSP) и `@michengai/dsh-skills-manager` (падение на инициализации бандла) |
| host routes (C1) | `crates/cordis/src/bootstrap.mjs`: loopback-only `node:http`-сервер (`127.0.0.1`, случайный порт), per-process токен, `webServer.register({name, kind, path, handler})` с валидацией пути (package/reserved-префиксы) и `kind: "exact"`/`kind: "prefix"` (раунд 86: exact выигрывает, среди prefix — самый длинный, так же в менеджере и прокси), `ctx.inject(['webServer'])`, `credentials.resolve` из env, внутренний `__dshr_routes`; `crates/cordis/src/lib.rs`: `SidecarHttp`/`sidecar_http` (порт, токен, таблица маршрутов), фильтр `__dshr_*`-инструментов и (раунд 86) MCP-псевдо-инструментов (`list_resources`/`read_resource`/`list_prompts`/`get_prompt`) из реестра; `crates/cordis/src/manager.rs`: `http_route(path)` (exact→prefix, longest-prefix) и `http_routes()`, `PluginStatus.routes` — live-таблица маршрутов в `GET /v1/plugins/status` (раунд 86); `crates/server/src/lib.rs`: fallback `plugin_http_proxy` (allowlist, method/query/body, 1 МиБ request / 8 МиБ response / 15 с, reserved → 404, permissive CORS), loopback-only LAN-политика (раунд 86: 403 при bind вне loopback без `allow_plugin_routes_lan`/`--allow-lan-plugin-routes`, `crates/cli/src/serve_cmd.rs`) и `client_host_csp(allow_connect)` — `connect-src <origin>` только для кадров с `host_modules`; тесты: `plugins_live` 16 (+1: `internal_session_snapshot_feeds_the_host_bridge` — снимок с usage-полями, 401/404; `plugin_routes_are_loopback_only_unless_opted_in`; `proxies_plugin_host_routes` — GET/query/POST/OPTIONS/prefix/404/reserved/снятие маршрутов/status `routes`/meta-tools, `plain_frames_stay_network_free`), smoke 80/80 (реальная host-половина `@kt11/dsh-deepseek-balance` через кадр плагина) |
| sessions bridge (C1.1) | `crates/server/src/lib.rs`: `ServerConfig.internal_token`, `serve()` генерирует per-run токен и пишет `<home>/cordis/host.json` (`write_host_bridge`), `GET /v1/internal/session-snapshot` (`internal_session_snapshot`: recent ≤50 сессий, id/title/created_at/model/turns/steps + usage-события, 401 при неверном `x-dshr-internal-token`, 404 без настроенного токена); `crates/cordis/src/lib.rs`: `DSHR_HOST_BRIDGE` в env sidecar; `crates/cordis/src/bootstrap.mjs`: `refreshHostSessions` (раз в 5 с, unref), `dshSessionFromSnapshot`, сервисы `sessions`/`sessionProjections` в `serviceMap`; тесты: `plugins_live` 16 (+1: `internal_session_snapshot_feeds_the_host_bridge`) и smoke 80/80 (+1: C1.1 — usage-маршрут реального `@kt11/dsh-deepseek-balance` возвращает `sessions` ≥ 1 и totals); ограничения — снимок с задержкой ≤ 5 с, `llmMs`=0 и без истории тредов, содержимое сессий не мостится |
| layout | smoke: `resize` выставляет высоту конкретного кадра (кламп 24–640 px) |

Команды:

```sh
cargo test -p dshr-cordis          # bridge harness (каталог слотов, регионы, slot-режим, React-режим/host-state, сервисы моста, React-контракт) + assets + runtime + shared host runtime + manager
cargo test -p dshr-server --test plugins_live   # 16 checks incl. the C1 host-route proxy + hardening (prefix/status/meta-tools/LAN) and the C1.1 internal session snapshot (401/404 + usage fields)
python3 scripts/plugin-smoke.py    # 80 проверок в headless Chrome (включая hooks, ui-kit, C1.1 bridge и одиннадцать реальных плагинов: deepseek-balance, dsh-remote, ruoyang-balance, dsh-genui, dsh-session-superagent, dsh-mnemon, dsh-plugin-market, dsh-plugin-codegraph-panel, dsh-plugin-notify, dsh-plugin-debugging, dsh-plugin-studio)
```

Скоринг пересчитывается по весам выше вручную по матрице
`client-half-features.csv` (отдельного скрипта в репозитории нет).

## Что дальше: предложения и решение владельца

1. C-full по [ADR-040](../adr/040-react-client-half.md) — это **решение
   владельца об объёме/приоритете**, а не остаток C-lite: полный design-system
   `@deepseek-ai/dsh-client-ui-*`, SlotMap-семантика (`ownerProps`,
   framework-hooks, cardinality/scope/chain/hoc), React-контекст/состояние шелла
   и композиция между плагинами — только если появится запрос на экосистему DSH
   UI; C-lite (React-компоненты слотов с host-hooks/`ctx.get('sessions')` и
   частичным design-system шимом) сделан в раундах 79–83, а раунд 85 (C2)
   расширил шим до 30+ экспортов и довёл conformance до n=11.
2. Host-маршруты DSH-плагинов: C1 (раунд 84) сделал их работающими
   (sidecar-сервер + allowlist-прокси + `connect-src`, реальная host-половина
   `@kt11/dsh-deepseek-balance` отвечает своим JSON), а раунд 86 (хардненинг C1)
   добавил prefix-маршруты, `routes` в `GET /v1/plugins/status`, фильтр
   MCP-псевдо-инструментов и loopback-only LAN-политику с opt-in
   `--allow-lan-plugin-routes`; раунд 87 (C1.1) закрыл сервисы
   `sessions`/`sessionProjections` (usage-маршруты реальных плагинов считают
   статистику по сессиям dshr из снимка с задержкой ≤ 5 с и упрощённой проекцией);
   остатки — SSE/WebSocket/стриминг и отсутствие аутентификации на
   plugin-маршрутах. `wide` в host-state должен отражать фактическую ширину сайдбара.
3. Conformance-набор из реальных npm-пакетов: цель M5b 8–12 плагинов
   **достигнута с запасом** — n=11 (`@kt11/dsh-deepseek-balance@0.2.1`,
   `dsh-remote@1.1.2`, `@ruoyang/dsh-plugin-deepseek-balance@1.0.2`,
   `dsh-genui@0.3.3`, `dsh-session-superagent@0.10.12`, `dsh-mnemon@0.5.46`,
   `@99galaxy/dsh-plugin-market@1.2.2`, `dsh-plugin-codegraph-panel@0.6.5`,
   `@goodandready/dsh-plugin-notify@0.4.9`, `dsh-plugin-debugging@0.8.1`,
   `dsh-plugin-studio@0.3.1`); остаётся nightly-прогон и отчёт
   матрицы. Для eval-based бандлов нужен opt-in `'unsafe-eval'` CSP, а
   `@michengai/dsh-skills-manager` требует триажа инициализации.
