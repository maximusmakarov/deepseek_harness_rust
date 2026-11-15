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
  (мост `sessions`/`sessionProjections` из снимка сервера), а раунд 89 — C1.2
  (seat-`inject`, Cordis-генераторы и `configForms`, оживившие settings-UI
  studio/debugging), а раунд 91 — C1.3 (сгенерированный каталог слотов DSH +
  SlotMap-регистрация), а раунд 92 — C1.4 (host-сервисы `webServer`/`ctx.baseUrl`/
  `configForms` по reference-контракту), а раунд 93 — C2.1 (атомы design-system и все 204 иконки портированы
  из reference-исходников), а раунд 94 — C2.2 (полный reference-порт `Tooltip`:
  placement-engine, `ShortcutKeys`, `input-modality`), а раунд 95 — C2.3 (ядро
  `Menu`/`MenuItemButton` по reference), а раунд 96 — C2.4 (reference submenu
  page-layout, 200-мс hover grace и `MenuGroup`/`observeStickyMenuGroups`), а
  раунд 97 — C2.5 (reference `JsonBlock` и API-совместимое dependency-free
  `MarkdownText`-подмножество) — всё
  в ту же строку;
  новых скоринговых строк нет, additions по-прежнему `original=no`
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
React-контракт (каталог слотов и правила `SlotCore` — `kind`/`scope`/standard-kit,
регистрация, shadowing-winners, `children`/deferred inject — закрыты в C1.3,
раунд 91, но chain select-диспетчеризация, hoc-вложение, store seats и ownerProps
от реальных shell-сайтов остаются; нет полного design-system и React-контекста
шелла).

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
их settings-UI гейтился host-сервисами `pluginManager`/settings и оставался
несмонтированным до C1.2 (штатная деградация, как у `dsh-remote` с опциональными
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

Раунд 89 расширил **ту же строку** C1/C-lite, не добавляя новых скоринговых
строк: **C1.2** — seat-`inject`, Cordis-генераторы и `configForms`.
Регистрация слота может отдать компоненту props фабрикой
`inject: () => ({...})` (ошибка фабрики деградирует компонент, но не плагин);
`ctx.slots.inject`/`ctx.inject`/`ctx.effect` принимают `function*` — тело
выполняется, `yield`-disposer'ы снимаются в обратном порядке (это
разблокировало `dsh-plugin-studio`, который регистрирует вкладки генератором);
`ctx.get('configForms')`/`ctx.configForms.get(entryId)` отдаёт форму
`{getSnapshot, subscribe, mutate(ops, revision)}` со стабильным snapshot
(требование `useSyncExternalStore`) и revision-контрактом, причём форма в кадре
in-memory и writable (правки не переживают reload кадра), а host-половинам
bootstrap пишет файловый вариант `<home>/cordis/config-forms/<entry>.json`.
В результате settings-UI `dsh-plugin-studio@0.3.1` (реальные вкладки в Settings,
`settings.plugins.tab`, свой CSS, честные error-состояния при заглушках
host-маршрутов) и `dsh-plugin-debugging@0.8.1` (карточка в Plugins,
`plugins.bundle.config`, переключатель + Save через `configForms`) реально
рендерится, а не остаётся инертным; smoke 82/82 (+2), bridge harness 90 (+3).
Строка по-прежнему **dshr-дополнение** (`original=no`, could/yes), новых строк в
матрице нет, поэтому арифметика не меняется: must 13/13 (13 × 3 = 39 из 39),
should 6 done (6 × 2 = 12 из 12) и 1 partial (1 × 2 × ½ = 1 из 2), could 1 done
(1 × 1 = 1 из 1) — **39 + 12 + 1 + 1 = 53 из 54 = 98.15 → 98.1**. Если бы C1.2
считался фичей оригинала, знаменатель вырос бы; в матрице строка намеренно
помечена `original=no`. Накопленные dshr-дополнения (deep link, plugin-shipped
runtime, shared host module table, C-lite) по-прежнему `original=no`, поэтому
знаменатель не растёт.

Раунд 91 расширил **ту же строку** C1/C-lite, не добавляя новых скоринговых
строк: **C1.3** — слот-контракт по референсному каталогу. Каталог слотов
сгенерирован из `deepseek-ai/deepseek-harness` (`packages/extensions/
cordis-client-runner/src/client/slot-catalog.ts`, коммит `39163dd`) — 122 слота
с `kind` (single 45 / list 48 / keyed 22 / chain 4 / hoc 3), `scope`
(root 57 / session 60 / session-maybe 5), framework standard-kit и
`inject`-флагом (`crates/cordis/src/slot-catalog.json`, host-страница публикует
как `window.__DSHR_SLOT_META__`). Регистрация повторяет reference `SlotCore`:
undeclared бросает (кроме dshr extension-пространств), `keyed`/`list`/`hoc`/
`chain` требуют `key`/`id`/`id`/`select`, повторная регистрация в ячейку на том
же `priority` бросает, другой `priority` затеняет (lowest renders),
`children`-декларации живут и каскадно снимаются вместе с записью (это
разблокировало собственные слоты `dsh-mnemon` — `mnemon.component.*`),
`ctx.slots.inject` ждёт объявления слота, shell получает только
shadowing-winners, а props собираются как standard-kit → seat `inject` →
`ownerProps` из `window.__DSHR_FRAME__.ownerProps` → `t`; добавлен
`ctx.slots.spec(key)`. Проверки: bridge harness 98 (+8: undeclared/keyed/chain/
duplicate throws, single shadowing, catalog standard kit, `ownerProps`, plain
values), smoke 82/82 (новых проверок нет — реальные плагины и так идут по
референсным правилам; synthetic-фикстуры получили обязательный `key`, словарь —
124 ключа включая 2 legacy), workspace 365 + 3 ignored. Границы: `chain` не
диспатчится по `select` (монтируется слоями), `hoc` не вкладывает `children`
между кадрами, store seats (`PropsStore`/`BoundActions`) и mounted-model сессий
отсутствуют, ownerProps поставляет только frame-seam (harness/расширения
shell). Строка по-прежнему **dshr-дополнение** (`original=no`, could/yes),
новых строк в матрице нет, поэтому арифметика не меняется: must 13/13
(13 × 3 = 39 из 39), should 6 done (6 × 2 = 12 из 12) и 1 partial
(1 × 2 × ½ = 1 из 2), could 1 done (1 × 1 = 1 из 1) —
**39 + 12 + 1 + 1 = 53 из 54 = 98.15 → 98.1**. Если бы C1.3 считался фичей
оригинала, знаменатель вырос бы; в матрице строка намеренно помечена
`original=no`. Накопленные dshr-дополнения (deep link, plugin-shipped runtime,
shared host module table, C-lite со всеми расширениями C1/C2/C1.1/C1.2) по-прежнему
`original=no`, поэтому знаменатель не растёт.

Раунд 92 расширил **ту же строку** C1/C-lite, не добавляя новых скоринговых
строк: **C1.4** — host-сервисы по reference-контракту. `webServer` приведён к
`packages/host/webserver/src/index.ts`: exact- и prefix-таблицы независимы,
дубликат `(kind, path)` бросает `webserver: duplicate <kind> route`, путь обязан
быть абсолютным pathname без завершающего `/`, `?` и `#` и не пересекаться с
reserved-префиксами dshr API, `getOrigin()`/`port`/`protocol` отдают origin
dshr-сервера из `<home>/cordis/host.json` (host-bridge), а не loopback-порт
sidecar-прокси, а `registerFallback`/`registerUpgrade` в sidecar не поддержаны и
бросают явную ошибку. `ctx.baseUrl` — loader-context свойство reference Cordis:
`file://` URL корня пакета плагина, нужный host-половинам (например,
`dsh-plugin-studio`) для резолва путей. `configForms` приведён к
`config-form-types.ts`: snapshot ровно
`{status, value, base, user, revision: number, writable}` со стабильной
идентичностью между чтениями, `mutate(ops, expectedRevision?)` с атомарными
`set`/`unset` и числовым revision-fence, хелперы `set(field, value)`/`unset(field)`;
форма в кадре in-memory (правки не переживают reload), у host-половин — файловая
`<home>/cordis/config-forms/<entry>.json`. Проверки: bridge harness 102 (+4:
форма-контракт, стабильный snapshot, set с числовой ревизией, unset), server
`plugins_live` 16 (proxy-тест расширен duplicate-route и trailing-slash
правилами), smoke 82/82, workspace 365 + 3 ignored. Границы: `registerFallback`/
`registerUpgrade` не поддержаны, browser-`configForms` не персистится через
reload, живого RPC-зеркала настроек нет; остаток reference-работы — реальный
порт `ui-primitives` из `packages/client/ui-primitives` (следующий кандидат),
store seats/renderer, chain dispatch/hoc-nesting и ownerProps от реальных
shell-сайтов. Строка по-прежнему **dshr-дополнение** (`original=no`, could/yes),
новых строк в матрице нет, поэтому арифметика не меняется: must 13/13
(13 × 3 = 39 из 39), should 6 done (6 × 2 = 12 из 12) и 1 partial
(1 × 2 × ½ = 1 из 2), could 1 done (1 × 1 = 1 из 1) —
**39 + 12 + 1 + 1 = 53 из 54 = 98.15 → 98.1**. Если бы C1.4 считался фичей
оригинала, знаменатель вырос бы; в матрице строка намеренно помечена
`original=no`. Накопленные dshr-дополнения (deep link, plugin-shipped runtime,
shared host module table, C-lite со всеми расширениями C1/C2/C1.1/C1.2/C1.3)
по-прежнему `original=no`, поэтому знаменатель не растёт.

Раунд 93 расширил **ту же строку** C1/C-lite, не добавляя новых скоринговых
строк: **C2.1** — атомы design-system и все 204 иконки портированы из
reference-исходников. `crates/server/assets/runtime/primitives.js` переписан по
`deepseek-ai/deepseek-harness` @ `39163dd`,
`packages/client/ui-primitives/src`: `Button` (variants
primary/ghost/outline/toolbar, sizes md/sm, leading icon, forwardRef), `Input`,
`Switch` (role switch, `aria-label`), `Checkbox` (native input + label,
`data-modal-autofocus`), `Tag` (`data-tone`), `Pill`, `SegmentedControl`
(tablist + indicator, roving tabindex, walk-keys, `<id>-<value>`/`-panel`),
`SegmentedTabs`, `DisclosureRow` (`expandOnRowClick`/`previewChevron`/
`collapsedContent`/`headerAccessory`/running shimmer), `TextShimmer`, `StateDot`
(done/warning/ongoing/error/idle, SVG spinner, `appearance: "step"`), `PathLabel`,
`Toast` (portal, `role=alert`, `holdMs`+fade), `Modal` (portal, mask, Escape,
focus), `Menu`, `MenuSurface` (forwardRef, `data-menu-material`/`phase`),
`Tooltip` (cloneElement-якорь); все **204** `Icon*` сгенерированы из upstream
`icons/artwork.ts` (точные SVG-пути, stroke regular 1 / medium 1.3, инлайн вместо
document-спрайта, Proxy-fallback для будущих имён); стили — reference
`--dsw-alias-*` токены с fallback'ами; synthetic-фикстуры приведены к
reference-контрактам (`StateDot state:"done"`, `Tooltip` element-child), а
C2-probe проверяет структурные признаки (`role=switch`/`aria-label`,
`data-tone`, tablist-табы, `svg`-счётчик, `label.dshr-checkbox`, path-label
`textContent`). Расширения dshr, не входящие в reference-порт: `Spinner`,
упрощённые `MarkdownText`/`JsonBlock` (upstream требует shiki/katex/mdast) и
`useAnchoredPosition`; `writeClipboard`/`useDismissOnOutsidePointer` портированы
по reference-алгоритмам. Проверки: smoke 82/82 (два прогона подряд), bridge
harness 102, cordis lib 26, server `plugins_live` 16, workspace 365 + 3 ignored
(desktop 21 — итого 386), web-e2e 16/16; общий рантайм-артефакт вырос с 176 725
до **375 894 байт**. Границы: полные `Menu` (submenu/focus) и `Tooltip`
(placement) не портированы, `MarkdownText`/`JsonBlock` упрощены; store
seats/renderer, chain dispatch/hoc-nesting и ownerProps от реальных shell-сайтов
остаются C-full. Строка по-прежнему **dshr-дополнение** (`original=no`,
could/yes), новых строк в матрице нет, поэтому арифметика не меняется:
must 13/13 (13 × 3 = 39 из 39), should 6 done (6 × 2 = 12 из 12) и 1 partial
(1 × 2 × ½ = 1 из 2), could 1 done (1 × 1 = 1 из 1) —
**39 + 12 + 1 + 1 = 53 из 54 = 98.15 → 98.1**. Если бы C2.1 считался фичей
оригинала, знаменатель вырос бы; в матрице строка намеренно помечена
`original=no`. Накопленные dshr-дополнения (deep link, plugin-shipped runtime,
shared host module table, C-lite со всеми расширениями C1/C2/C1.1/C1.2/C1.3/
C1.4/C2.1) по-прежнему `original=no`, поэтому знаменатель не растёт.

Раунд 94 расширил **ту же строку** C1/C-lite, не добавляя новых скоринговых
строк: **C2.2** — последний упрощённый атом C2.1 доведён до полного
reference-порта. `Tooltip` повторяет `packages/client/ui-primitives/src/
Tooltip.tsx`: label-резолвер вычисляется только при показе; независимые
hover/focus-триггеры с `delayMs`/`focusDelayMs`; `openOnClick`-pinning с
outside-pointer/Escape/Tab-dismiss; `onlyWhenClipped`, `maxWidth`, `portal`,
`data-side`/`data-pinned`, nested-suppression через контекст, `aria-describedby`
при pinning, pointer-modality (фокус после указателя не поднимает бабл),
touch-hover исключён, forward ref сквозь клон якоря. `tooltip-placement.ts`
портирован как есть: requested side → opposite → perpendicular, viewport-clamp,
overflow-fallback; `input-modality.ts` (`data-input-modality`,
`pointerModality`) и `ShortcutKeys` (keycaps с `+`-разделителями) портированы и
экспортированы, а C2-probe дополнен узлом `Tooltip` в synthetic-фикстуре
(`ShortcutKeys` отрисовывается внутри Tooltip). Проверки: smoke 82/82, bridge harness 102, cordis lib 26,
server `plugins_live` 16, workspace 365 + 3 ignored (desktop 21 — итого 386),
web-e2e 16/16; общий рантайм-артефакт вырос с 375 894 до **387 613 байт**.
Границы: полный `Menu` (submenu/focus) по-прежнему не портирован,
`MarkdownText`/`JsonBlock` упрощены (upstream требует shiki/katex/mdast); store
seats/renderer, chain dispatch/hoc-nesting и ownerProps от реальных shell-сайтов
остаются C-full. Строка по-прежнему **dshr-дополнение** (`original=no`,
could/yes), новых строк в матрице нет, поэтому арифметика не меняется:
must 13/13 (13 × 3 = 39 из 39), should 6 done (6 × 2 = 12 из 12) и 1 partial
(1 × 2 × ½ = 1 из 2), could 1 done (1 × 1 = 1 из 1) —
**39 + 12 + 1 + 1 = 53 из 54 = 98.15 → 98.1**. Если бы C2.2 считался фичей
оригинала, знаменатель вырос бы; в матрице строка намеренно помечена
`original=no`. Накопленные dshr-дополнения (deep link, plugin-shipped runtime,
shared host module table, C-lite со всеми расширениями C1/C2/C1.1/C1.2/C1.3/
C1.4/C2.1/C2.2) по-прежнему `original=no`, поэтому знаменатель не растёт.

Раунд 95 расширил **ту же строку** C1/C-lite, не добавляя новых скоринговых
строк: **C2.3** — `Menu`/`MenuItemButton` переписаны по reference-ядру
`packages/client/ui-primitives/src/Menu.tsx`/`MenuRow.tsx`: data-`items` и
компонентные `MenuItemButton`, separators/labels, `reserveIcon`-колонки
(contextual), selection `check`/`fill`, `danger`/`current`, disabled-reason
через reference `Tooltip`, `shortcut` + `ShortcutKeys`, `children`-ряды и
`footer`-группа, aria (`menuitem`/`haspopup`/`expanded`/`disabled`/`describedby`/
`keyshortcuts`/`current`), `dense`/`compact`, portal-позиционирование по
anchor-rect (`minWidthFromAnchor`/`anchorPoint`/`align`/`side`) и
contextual-атрибут; клавиатурный walk (Arrow/Home/End с roving focus, Escape
закрывает и возвращает фокус через `focusWithoutRing`, Tab «settles» ряд /
Shift+Tab уходит, `autoFocus`), one-level submenu на hover/ArrowRight с
закрытием по ArrowLeft/Escape/выбору, outside-pointer dismissal + закрытие при
blur в iframe, `closeOnPointerLeave` с 200-мс grace (`usePointerGrace`);
экспортированы `MenuItemButton`, `MENU_TRANSITION_MS`, `usePointerGrace`,
`focusWithoutRing`, `isMobileDevice`. Проверки: smoke 82/82, bridge harness 102,
cordis lib 26, server `plugins_live` 16, workspace 365 + 3 ignored (desktop 21 —
итого 386), web-e2e 16/16; общий рантайм-артефакт вырос с 387 613 до
**405 323 байт**. Границы C2.3: полная upstream submenu-машинерия
(`useMenuSubmenu`: single-panel back-page, меж-уровневый hover grace) и
`MenuGroup`/`observeStickyMenuGroups` не портированы (закрыто в C2.4 — следующий
абзац); `MarkdownText`/`JsonBlock`
упрощены (upstream требует shiki/katex/mdast), store seats/renderer, chain
dispatch/hoc-nesting и ownerProps от реальных shell-сайтов остаются C-full.
Строка по-прежнему **dshr-дополнение** (`original=no`, could/yes), новых строк
в матрице нет, поэтому арифметика не меняется: must 13/13 (13 × 3 = 39 из 39),
should 6 done (6 × 2 = 12 из 12) и 1 partial (1 × 2 × ½ = 1 из 2), could 1 done
(1 × 1 = 1 из 1) — **39 + 12 + 1 + 1 = 53 из 54 = 98.15 → 98.1**. Если бы C2.3
считался фичей оригинала, знаменатель вырос бы; в матрице строка намеренно
помечена `original=no`. Накопленные dshr-дополнения (deep link, plugin-shipped
runtime, shared host module table, C-lite со всеми расширениями C1/C2/C1.1/
C1.2/C1.3/C1.4/C2.1/C2.2/C2.3) по-прежнему `original=no`, поэтому знаменатель
не растёт.

Раунд 96 расширил **ту же строку** C1/C-lite, не добавляя новых скоринговых
строк: **C2.4** — reference submenu page-layout и `MenuGroup`. Размещение
submenu повторяет `packages/client/ui-primitives/src/menu-submenu-position.ts`:
если дочерняя карточка помещается сбоку родительской (справа или слева) — это
side-карточка, иначе включается single-page layout: дочерние ряды заменяют
родительскую страницу, сверху non-selecting back-row (`data-menu-back`,
chevron-left, `submenuBackLabel` или label родителя) + separator, а
Escape/ArrowLeft возвращают на родительскую страницу с фокусом на родительском
ряду. Focus-контракт: hover открывает submenu без перехвата фокуса, а
ArrowRight/клик фокусируют первый дочерний ряд. Между родительской и дочерней
карточками действует 200-мс hover grace — уход курсора в дочернюю отменяет
запланированное закрытие (`cancelSubmenuClose`). `MenuGroup` +
`observeStickyMenuGroups` портированы по `MenuGroup.tsx`: `section[role=group]`
с `aria-labelledby`, атрибутами `data-menu-group`/`data-menu-group-start`/
`data-menu-group-heading` и sticky-заголовком (`data-stuck`) через
IntersectionObserver/ResizeObserver. C2-probe теперь рендерит `MenuGroup` и
submenu-родителя и требует `menuRows >= 3`. Проверки: smoke 82/82, bridge
harness 102, cordis lib 26, server `plugins_live` 16, workspace 365 + 3 ignored
(desktop 21 — итого 386), web-e2e 16/16; общий рантайм-артефакт вырос с
405 323 до **415 972 байт**. Границы: детальная `useMenuSubmenu`-механика
focus-transition/page-anchor остаётся упрощённой (у нас CSS/JS-позиции);
`MarkdownText`/`JsonBlock` на конец C2.4 упрощены (upstream требует
shiki/katex/mdast; закрыто в C2.5 — ниже),
store seats/renderer, chain dispatch/hoc-nesting и ownerProps от реальных
shell-сайтов остаются C-full. Строка по-прежнему **dshr-дополнение**
(`original=no`, could/yes), новых строк в матрице нет, поэтому арифметика не
меняется: must 13/13 (13 × 3 = 39 из 39), should 6 done (6 × 2 = 12 из 12) и
1 partial (1 × 2 × ½ = 1 из 2), could 1 done (1 × 1 = 1 из 1) —
**39 + 12 + 1 + 1 = 53 из 54 = 98.15 → 98.1**. Если бы C2.4 считался фичей
оригинала, знаменатель вырос бы; в матрице строка намеренно помечена
`original=no`. Накопленные dshr-дополнения (deep link, plugin-shipped
runtime, shared host module table, C-lite со всеми расширениями C1/C2/C1.1/
C1.2/C1.3/C1.4/C2.1/C2.2/C2.3/C2.4) по-прежнему `original=no`, поэтому
знаменатель не растёт.

Раунд 97 расширил **ту же строку** C1/C-lite, не добавляя новых скоринговых
строк: **C2.5** — reference `JsonBlock` и API-совместимое `MarkdownText`-
подмножество. `JsonBlock` портирован из
`packages/client/ui-primitives/src/markdown/JsonBlock.tsx`: collapsible (▾/▸),
props `label`/`payload`/`defaultOpen`/`truncatedLabel`, cap 20 000 символов с
хвостом «truncated» и безопасный `JSON.stringify` с fallback на `String`.
`MarkdownText` получил reference-поверхность props
(`text`/`streaming`/`labels`/`fileMentions`/`pathImages`/`variant`/`omitHtml`/
`textInteractionSource`) и dependency-free рендерер GFM-подмножества:
заголовки 1–6, абзацы, ul/ol, fenced code, blockquote, hr, inline
`code`/`**bold**`/`*italic*`/`~~strike~~`, ссылки и изображения только
`http(s)`/`mailto` (небезопасные протоколы `javascript:`/`data:` отбрасываются;
относительные изображения не рендерятся), raw HTML не рендерится, `variant="compact"` даёт
`data-markdown-variant`/`data-text-interaction-source`. C2-probe рендерит
`MarkdownText` (заголовок, bold, fenced code, ссылка) и `JsonBlock` (expanded
JSON) и проверяет DOM/текст. Upstream-стек (micromark/mdast, shiki, KaTeX,
footnotes, инкрементальный streaming) **не** портирован — `MarkdownText`
остаётся подмножеством. Проверки: smoke 82/82, bridge harness 102, cordis lib
26, server `plugins_live` 16, workspace 365 + 3 ignored (desktop 21 — итого
386), web-e2e 16/16; общий рантайм-артефакт вырос с 415 972 до
**424 801 байта**. Строка по-прежнему **dshr-дополнение**
(`original=no`, could/yes), новых строк в матрице нет, поэтому арифметика не
меняется: must 13/13 (13 × 3 = 39 из 39), should 6 done (6 × 2 = 12 из 12) и
1 partial (1 × 2 × ½ = 1 из 2), could 1 done (1 × 1 = 1 из 1) —
**39 + 12 + 1 + 1 = 53 из 54 = 98.15 → 98.1**. Если бы C2.5 считался фичей
оригинала, знаменатель вырос бы; в матрице строка намеренно помечена
`original=no`. Накопленные dshr-дополнения (deep link, plugin-shipped
runtime, shared host module table, C-lite со всеми расширениями C1/C2/C1.1/
C1.2/C1.3/C1.4/C2.1/C2.2/C2.3/C2.4/C2.5) по-прежнему `original=no`, поэтому
знаменатель не растёт.

Что осталось из should (не блокирует):

| Фича | Состояние | Чем отличается |
|---|---|---|
| Bundles built for the reference module table | partial | шим `__ModuleLoader__.load({id, factory})` исполняет `factory` и `apply(ctx)`; `require` резолвит модули из `dsh.client.runtime`, если половина их опубликовала (в PoC — React 18.3.1), или из общего рантайма хоста, если половина опционально объявила `dsh.client.host_modules` (React 18.3.1 + jsx-шимы + design-system примитивы `@deepseek-ai/dsh-client-ui-primitives`, с раунда 93 (C2.1) портированные из reference: атомы и все 204 иконки; с раунда 94 (C2.2) — полный `Tooltip`; с раунда 95 (C2.3) — ядро `Menu`/`MenuItemButton`; с раунда 96 (C2.4) — reference submenu page-layout, 200-мс hover grace и `MenuGroup`/`observeStickyMenuGroups`; с раунда 97 (C2.5) — reference `JsonBlock` и API-совместимое dependency-free `MarkdownText`-подмножество, при этом upstream markdown-стек (micromark/mdast, shiki, KaTeX, footnotes, streaming) не входит, а детальная `useMenuSubmenu`-механика focus-transition/page-anchor остаётся упрощённой). Полной таблицы модулей DSH (полный design-system, React-контекст шелла) нет — это единственная незакрытая should-фича, поэтому score 98.1, а не 100 |

## Чем это подтверждено

| Область | Доказательство |
|---|---|
| discovery | `crates/cordis/src/client.rs`: `discover_client_half`, тесты на entry/exports/escape; серверный тест `/v1/plugins/clients` |
| security | канонизация пути, confinement пакета, тест traversal; capability-ассеты проверяются на чужой/просроченный cap (серверный тест) |
| slots | `crates/cordis/src/bridge.js`: таблица `SLOT_REGIONS` — 122 слота каталога DSH 0.2.2-rc.2 (после раунда 91 — с `kind`/`scope`/standard-kit) плюс legacy `panel`/`message.action`, всего 124 ключа, 17 регионов, fallback `panel`; `tests/bridge_harness.mjs` (порядок, группы `inject`, `list()`, slot-режим); smoke: вкладки и переключение слотов |
| slot contract (C1.3) | `crates/cordis/src/slot-catalog.json` — сгенерированный из `deepseek-ai/deepseek-harness` (@ `39163dd`, `cordis-client-runner/src/client/slot-catalog.ts`; поле `source` в JSON) каталог: 122 слота с `kind` (single 45 / list 48 / keyed 22 / chain 4 / hoc 3), `scope` (root 57 / session 60 / session-maybe 5), framework standard-kit и `inject`-флагом; `crates/cordis/src/client.rs`: `SLOT_CATALOG_JSON` (`include_str!`), host-страница публикует `window.__DSHR_SLOT_META__`; `crates/cordis/src/bridge.js`: `SLOT_META`/`declaredSlots` (каталог + runtime-декларации), `declareChildSlots`/`undeclareChildSlots` (children живут и каскадно снимаются вместе с записью), `normalize` (undeclared/keyed/list/hoc/chain-требования, same-cell same-priority throw, shadowing), `winners` (single/keyed/list/chain/hoc), `slotStandardProps` + `frameOwnerProps` (standard-kit → seat inject → ownerProps → t), `slots.inject` (deferred до объявления), `slots.spec`; тесты: bridge harness 98 (+8: undeclared/keyed/chain/duplicate throws, single shadowing, catalog standard kit, ownerProps, plain values), smoke 82/82 (реальные плагины по тем же правилам, synthetic-фикстуры с обязательным `key`, словарь 124 ключа) |
| regions | eager registry-кадр на половину (`mode=registry`) и iframe `mode=slot` на регистрацию (`crates/web/src/main.rs`: `SlotRegion`/`SlotFrame`/`RegistryFrames`/`PanelRegion`); smoke: монтаж в каждый регион, fallback незнакомого имени, `context` повторяющихся регионов, закрытие диалога снимает его фреймы |
| lifecycle | harness: `activate`/`deactivate`/`dispose`, `effect` cleanup; smoke: eager-кадр переживает закрытие, горячая перезагрузка панели |
| theme | harness: `onTheme`; smoke: переключение темы доходит до всех фреймов |
| actions | harness: round-trip `action-result` с адресацией отправителю; smoke: `close`, `reload`, `compose`, `notify`, неизвестное действие |
| loading | серверные тесты inline и capability-ассетов; smoke: многофайловый бандл и module-table бандл рендерятся |
| runtime | `crates/cordis/src/client.rs`: `ClientHalf.runtime`/`runtime_path`, `host_page`/`host_page_assets` встраивают runtime до бандла, `requireShim` резолвит `window.__DSHR_MODULES__`; `crates/server/src/lib.rs`: `/host` читает и валидирует runtime (confined к пакету, ≤ 8 МиБ, `</script>` → 400); тесты: cordis +4, `plugins_live` +3, smoke +3 (discovery, runtime-before-bundle, require render); React 18.3.1 PoC (UMD в `runtime.js`, 143 197 байт) — рендер, `useState`, `ctx.onTheme` проверены вручную |
| shared runtime | `crates/cordis/src/client.rs`: `HOST_MODULES`, `HOST_RUNTIME_VERSION`, `host_runtime_url`, `PageRuntime` (порядок shared → plugin runtime → bundle); `crates/server/src/lib.rs`: `GET /v1/runtime/{file}` (immutable-кэш, 404 на неизвестную версию), `client_host_csp` (origin сервера только для opt-in кадров); `crates/server/assets/runtime/` + `scripts/check-runtime-assets.sh`; тесты: cordis +4, `plugins_live` +3, smoke +5 (discovery `host_modules`, immutable-артефакт, host-страница без встроенного React, React из общего рантайма, `useState`) |
| react slots / host state | `crates/cordis/src/bridge.js`: React-режим `ctx.slots.register(entry, Component)`, один React-root на кадр, `host-state` (`theme`/`wide`/`session`/`sessions`), hooks `useTheme`/`useSession`/`useSessions`/`useHostState`, `ctx.get('sessions')`, error boundary; `crates/web/src/main.rs`: `push_host_state_all`; `crates/cordis/src/client.rs`: `HOST_MODULES` с id примитивов; тесты: bridge harness 102 checks (host-state/`ctx.get`/`ctx.get('slots')`/`locale`/`ctx.inject`/`__DSHR_LAST_ERROR__` + раунд-81 `reactScenario`: memo-тип, host-props/hooks, `t()`-место, keyed-ids, форма `slots.entries`, раунд-89 C1.2: seat-`inject` props, генераторный `slots.inject`, генераторный `effect`, раунд-91 C1.3: catalog standard kit, `ownerProps` spread, plain catalog values), smoke 82/82 (фикстуры `hooks`, `hosted`, `ui-kit`, `dsh-remote`, `ruoyang-balance`, `dsh-genui`, `dsh-session-superagent`, `dsh-mnemon`, `dsh-plugin-market`, `dsh-plugin-codegraph-panel`, `dsh-plugin-notify`, `dsh-plugin-debugging`, `dsh-plugin-studio`; C1 host-маршруты — через реальную host-половину `@kt11/dsh-deepseek-balance`) |
| design-system primitives | `crates/server/assets/runtime/primitives.js` — с раунда 93 (C2.1) **reference-порт** `@deepseek-ai/dsh-client-ui-primitives` (`deepseek-ai/deepseek-harness` @ `39163dd`, `packages/client/ui-primitives/src`): атомы `Button` (variants primary/ghost/outline/toolbar, sizes md/sm, leading icon, forwardRef), `Input`, `Switch` (role switch, aria-label), `Checkbox` (native input + label, data-modal-autofocus), `Tag` (`data-tone`), `Pill`, `SegmentedControl` (tablist+indicator, roving tabindex, walk-keys, id-value/-panel), `SegmentedTabs`, `DisclosureRow` (expandOnRowClick/previewChevron/collapsedContent/headerAccessory/running shimmer), `TextShimmer`, `StateDot` (done/warning/ongoing/error/idle, SVG spinner, appearance step), `PathLabel`, `Toast` (portal, role=alert, holdMs+fade), `Modal` (portal, mask, Escape, focus), `Menu` (с раунда 95, C2.3 — ядро по reference: data/компонентные ряды, reserve-icon, selection, aria, клавиатурный walk, one-level submenu, dismissal), `MenuSurface` (forwardRef, data-menu-material/phase), `Tooltip` (с раунда 94, C2.2 — полный reference-порт: placement-engine, `ShortcutKeys`, `input-modality`) и все **204** `Icon*` из upstream `icons/artwork.ts` (точные SVG-пути, stroke regular 1 / medium 1.3, инлайн вместо спрайта, Proxy-fallback); стили — reference `--dsw-alias-*` токены с fallback'ами; с раунда 95 (C2.3) `Menu`/`MenuItemButton` — reference-ядро (data/компонентные ряды, reserve-icon, selection, aria, клавиатурный walk, one-level submenu, dismissal), а раунд 96 (C2.4) закрыл reference submenu page-layout (`menu-submenu-position.ts`: side-карточка или single-page с non-selecting back-row `data-menu-back` и Escape/ArrowLeft на родителя), 200-мс hover grace (`cancelSubmenuClose`) и `MenuGroup`/`observeStickyMenuGroups` (sticky `data-stuck` через IntersectionObserver/ResizeObserver), а раунд 97 (C2.5) — reference `JsonBlock` (collapsible ▾/▸, `label`/`payload`/`defaultOpen`/`truncatedLabel`, cap 20 000 символов, безопасный `JSON.stringify`) и API-совместимое dependency-free GFM-подмножество `MarkdownText` с reference-поверхностью props (заголовки/списки/fenced code/blockquote/hr/inline-стили, sanitize URL, raw HTML не рендерится); упрощённой остаётся только детальная `useMenuSubmenu`-механика focus-transition/page-anchor, а `Tooltip` закрыт в C2.2 (раунд 94); upstream markdown-стек (micromark/mdast, shiki, KaTeX, footnotes, streaming) не входит, `Spinner`/`useAnchoredPosition` — dshr-расширения. Файл в `checksums.txt`/`README.md`; smoke: фикстура `ui-kit` резолвит его из общего рантайма, genui использует `JsonBlock`/`MarkdownText`, а C2-проба рендерит дерево всех смоделированных примитивов, включая `MenuGroup`/submenu-родителя (`menuRows >= 3`) и `MarkdownText`/`JsonBlock` (проверяются `.dshr-markdown h1`, `.dshr-markdown pre code` и `.dshr-json-block-body`), и проверяет структурные признаки (role=switch/aria-label, data-tone, tablist, svg, label.dshr-checkbox, path-label textContent, data-menu-group) |
| conformance | **Одиннадцать** реальных MIT-плагинов в `crates/cordis/tests/fixtures/` (n=11 — цель ADR-013 §6 8–12 достигнута с запасом; клиентские половины upstream без изменений, sha256 в `package.json`, host-половины — заглушки, кроме `@kt11/dsh-deepseek-balance` с раунда 84): `@kt11/dsh-deepseek-balance@0.2.1` — smoke: React-карточка в `sidebar.footer.action`, инжект собственного CSS, popup по клику; с раунда 84 (C1) host-половина реальная (upstream без изменений, host sha256 `8771ae90…`), и её маршруты отвечают через прокси: 500 `not-configured` без ключа и 200 JSON для usage-маршрута; `dsh-remote@1.1.2` (раунд 80) — smoke: discovery с примитивами в `host_modules`, registry-кадр без ошибок при отсутствии опциональных сервисов, React-слот `conversation.session.header.actions` монтируется в регион `conversation.header`; `@ruoyang/dsh-plugin-deepseek-balance@1.0.2` (раунд 81, sha256 `ca700695…`) — плавающий бейдж в `shell.overlay` через `ctx.effect`/`ctx.interval`/`ctx.locale`, рендерится без ошибок; `dsh-genui@0.3.3` (раунд 81, sha256 `8f0500ad…`) — тумблер GenUI в `conversation.input.left` и keyed-регистрация `conversation.chat.node` через `React.memo`, рендерится без ошибок; `dsh-session-superagent@0.10.12` (раунд 82, sha256 `d58f0fa2…`) — composer dock `conversation.composer.dock` монтируется в регион `composer.actions` без ошибок; `dsh-mnemon@0.5.46` (раунд 82, sha256 `d967402b…`) — крупный multi-region плагин: активируется, публикует слоты и сверяет собственные shadow-регистрации через `ctx.slots.subscribe`/`getVersion`/`entriesOfSlot`; `@99galaxy/dsh-plugin-market@1.2.2` (раунд 83, sha256 `27abb2db…`) — запись в сайдбаре (`sidebar.panellist` → регион `sidebar.footer`), smoke: кадр монтируется; `dsh-plugin-codegraph-panel@0.6.5` (раунд 83, sha256 `f834fadb…`) — панель `settings.section` (upstream-флаг `immediately: true`), активируется и публикует слот; `@goodandready/dsh-plugin-notify@0.4.9` (раунд 83, sha256 `7a2843fa…`) — строки `plugins.item`/`plugins.row.config` (регион `plugins.detail`), primitives + react, активируется и публикует слоты; `dsh-plugin-debugging@0.8.1` (раунд 85, C2, client sha256 `9b828160…`) и `dsh-plugin-studio@0.3.1` (раунд 85, C2, client sha256 `1194c18b…`) — примитивы-heavy половины (30+ экспортов шима): активируются без ошибок; их settings-UI рендерится с раунда 89 (C1.2) — studio показывает вкладки в Settings, debugging — карточку в Plugins через seat-`inject`/`configForms` (browser-форма in-memory). Не входят в набор: `@michengai/dsh-archive-manager` (нужен `'unsafe-eval'` в CSP) и `@michengai/dsh-skills-manager` (падение на инициализации бандла) |
| host routes (C1) | `crates/cordis/src/bootstrap.mjs`: loopback-only `node:http`-сервер (`127.0.0.1`, случайный порт), per-process токен, `webServer.register({name, kind, path, handler})` с валидацией пути (package/reserved-префиксы) и `kind: "exact"`/`kind: "prefix"` (раунд 86: exact выигрывает, среди prefix — самый длинный, так же в менеджере и прокси), `ctx.inject(['webServer'])`, `credentials.resolve` из env, внутренний `__dshr_routes`; `crates/cordis/src/lib.rs`: `SidecarHttp`/`sidecar_http` (порт, токен, таблица маршрутов), фильтр `__dshr_*`-инструментов и (раунд 86) MCP-псевдо-инструментов (`list_resources`/`read_resource`/`list_prompts`/`get_prompt`) из реестра; `crates/cordis/src/manager.rs`: `http_route(path)` (exact→prefix, longest-prefix) и `http_routes()`, `PluginStatus.routes` — live-таблица маршрутов в `GET /v1/plugins/status` (раунд 86); `crates/server/src/lib.rs`: fallback `plugin_http_proxy` (allowlist, method/query/body, 1 МиБ request / 8 МиБ response / 15 с, reserved → 404, permissive CORS), loopback-only LAN-политика (раунд 86: 403 при bind вне loopback без `allow_plugin_routes_lan`/`--allow-lan-plugin-routes`, `crates/cli/src/serve_cmd.rs`) и `client_host_csp(allow_connect)` — `connect-src <origin>` только для кадров с `host_modules`; тесты: `plugins_live` 16 (+1: `internal_session_snapshot_feeds_the_host_bridge` — снимок с usage-полями, 401/404; `plugin_routes_are_loopback_only_unless_opted_in`; `proxies_plugin_host_routes` — GET/query/POST/OPTIONS/prefix/404/reserved/снятие маршрутов/status `routes`/meta-tools, а с раунда 92 (C1.4) — duplicate-route (бросает) и отклонение trailing-slash; `plain_frames_stay_network_free`), smoke 82/82 (реальная host-половина `@kt11/dsh-deepseek-balance` через кадр плагина) |
| host services (C1.4) | `crates/cordis/src/bootstrap.mjs`: `validRoutePath` (абсолютный pathname без trailing slash/query/hash, вне reserved-префиксов dshr API), `webRoutes` (независимые `exact`/`prefix`-таблицы, дубликат `(kind, path)` бросает `webserver: duplicate <kind> route`), `getOrigin()`/`port`/`protocol` из host-bridge файла `<home>/cordis/host.json`, `unsupportedWebServerFeature` (`registerFallback`/`registerUpgrade` бросают), `ctx.baseUrl` = `pathToFileURL(root + sep).href`, reference-shape `configForms` (`base`/`user`, числовая `revision` со стабильным snapshot, `mutate` с `set`/`unset` и expectedRevision-fence, хелперы `set`/`unset`); `crates/cordis/src/bridge.js`: `browserConfigForm`/`configFormsService` с той же формой snapshot и in-memory формой (не переживает reload); тесты: bridge harness 102 (+4: форма-контракт, стабильный snapshot, set с числовой ревизией, unset), `plugins_live` 16 (duplicate-route/trailing-slash в proxy-тесте); ограничения — `registerFallback`/`registerUpgrade` не поддержаны, browser-форма in-memory, живого RPC-зеркала настроек нет |
| sessions bridge (C1.1) | `crates/server/src/lib.rs`: `ServerConfig.internal_token`, `serve()` генерирует per-run токен и пишет `<home>/cordis/host.json` (`write_host_bridge`), `GET /v1/internal/session-snapshot` (`internal_session_snapshot`: recent ≤50 сессий, id/title/created_at/model/turns/steps + usage-события, 401 при неверном `x-dshr-internal-token`, 404 без настроенного токена); `crates/cordis/src/lib.rs`: `DSHR_HOST_BRIDGE` в env sidecar; `crates/cordis/src/bootstrap.mjs`: `refreshHostSessions` (раз в 5 с, unref), `dshSessionFromSnapshot`, сервисы `sessions`/`sessionProjections` в `serviceMap`; тесты: `plugins_live` 16 (+1: `internal_session_snapshot_feeds_the_host_bridge`) и smoke 82/82 (C1.1-проверка входит в набор: usage-маршрут реального `@kt11/dsh-deepseek-balance` возвращает `sessions` ≥ 1 и totals); ограничения — снимок с задержкой ≤ 5 с, `llmMs`=0 и без истории тредов, содержимое сессий не мостится |
| seat inject / generators / configForms (C1.2) | `crates/cordis/src/bridge.js`: `injectedProps` (seat-`inject` `() => ({...})` мержится в props компонента; брошенная фабрика логируется в кадр и деградирует компонент, не плагин), `cordisDisposer` (генераторные `slots.inject`/`ctx.inject`/`ctx.effect`: тело исполняется, `yield`-disposer'ы снимаются в обратном порядке), `browserConfigForm`/`configFormsService` (`get(entryId)` → `{getSnapshot, subscribe, mutate(ops, revision)}`, стабильный snapshot для `useSyncExternalStore`, revision-контракт; in-memory writable-форма в кадре); `crates/cordis/src/bootstrap.mjs`: файловая форма `<home>/cordis/config-forms/<entry>.json` для host-половин; тесты: bridge harness 102 (+3 в C1.2; +8 в C1.3 — см. строку slot contract; +4 в C1.4 — см. строку host services (C1.4)), smoke 82/82 (+2: реальная settings-вкладка `dsh-plugin-studio` в `settings.plugins.tab` через seat-`inject`; карточка `dsh-plugin-debugging` в `plugins.bundle.config` рендерится, переключатель+Save проходят через `configForms`); ограничения — browser-форма in-memory (правки не переживают reload кадра), семантика конфигурационного движка DSH не воспроизведена, провалы seat-inject только логируются |
| layout | smoke: `resize` выставляет высоту конкретного кадра (кламп 24–640 px) |

Команды:

```sh
cargo test -p dshr-cordis          # bridge harness 102 (каталог слотов, регионы, slot-режим, React-режим/host-state, сервисы моста, React-контракт, раунд-89 C1.2: seat inject/generator slots.inject/generator effect, раунд-91 C1.3: reference slot contract, раунд-92 C1.4: configForms-контракт) + assets + runtime + shared host runtime + manager
cargo test -p dshr-server --test plugins_live   # 16 checks incl. the C1 host-route proxy + hardening (prefix/status/meta-tools/LAN), the C1.4 reference route rules (duplicate throws, trailing slash rejected) and the C1.1 internal session snapshot (401/404 + usage fields)
python3 scripts/plugin-smoke.py    # 82 проверки в headless Chrome (включая hooks, ui-kit, C1.1 bridge, C1.2 seat-inject/configForms-рендер settings-UI и одиннадцать реальных плагинов: deepseek-balance, dsh-remote, ruoyang-balance, dsh-genui, dsh-session-superagent, dsh-mnemon, dsh-plugin-market, dsh-plugin-codegraph-panel, dsh-plugin-notify, dsh-plugin-debugging, dsh-plugin-studio)
```

Скоринг пересчитывается по весам выше вручную по матрице
`client-half-features.csv` (отдельного скрипта в репозитории нет).

## Что дальше: предложения и решение владельца

1. C-full по [ADR-040](../adr/040-react-client-half.md) — это **решение
   владельца об объёме/приоритете**, зафиксированное [ADR-041](../adr/041-c-full-decision.md)
   как отложенное (точечные инкременты по спросу), а не остаток C-lite: полный design-system
   `@deepseek-ai/dsh-client-ui-*`, SlotMap-остатки (`chain` select-диспетчеризация,
   `hoc`-вложение, store seats `PropsStore`/`BoundActions`, ownerProps от реальных
   render-сайтов шелла), React-контекст/состояние шелла
   и композиция между плагинами — только если появится запрос на экосистему DSH
   UI; C-lite (React-компоненты слотов с host-hooks/`ctx.get('sessions')` и
   design-system примитивами) сделан в раундах 79–83, раунд 85 (C2)
   расширил шим до 30+ экспортов и довёл conformance до n=11, а раунд 89 (C1.2)
   добавил seat-`inject`, Cordis-генераторы и `configForms`, поэтому settings-UI
   `dsh-plugin-studio`/`dsh-plugin-debugging` теперь рендерится (хотя референсных
   host-сервисов `pluginManager`/settings и семантики конфигурационного движка
   DSH по-прежнему нет, а browser-форма in-memory). Раунд 91 (C1.3) закрыл
   каталог слотов и правила `SlotCore`: 122 слота с kinds/scopes/standard-kit,
   undeclared/keyed/chain/duplicate, shadowing-winners, children-декларации,
   deferred inject, standard-kit + ownerProps и `ctx.slots.spec(key)` —
   остаток SlotMap перечислен выше. Раунд 92 (C1.4) выровнял host-сервисы
   `webServer`/`ctx.baseUrl`/`configForms` по reference-контракту (правила
   маршрутов и origin-аксессоры, `file://`-baseUrl, snapshot с числовой
   ревизией и `set`/`unset`), а раунд 93 (C2.1) закрыл реальный порт `ui-primitives` из `packages/client/ui-primitives` (атомы и все 204 иконки), а раунд 94 (C2.2) довёл до полного reference-порта `Tooltip` (placement-engine, `ShortcutKeys`, `input-modality`; артефакт 387 613 байт), а раунд 95 (C2.3) — ядро `Menu`/`MenuItemButton` (ряды/selection/aria, клавиатурный walk, one-level submenu, dismissal; артефакт 405 323 байта), а раунд 96 (C2.4) — reference submenu page-layout, 200-мс hover grace и `MenuGroup`/`observeStickyMenuGroups` (артефакт 415 972 байта), а раунд 97 (C2.5) — reference `JsonBlock` и API-совместимое dependency-free `MarkdownText`-подмножество (артефакт 424 801 байт), поэтому упрощёнными остаются детальная `useMenuSubmenu`-механика focus-transition/page-anchor и upstream markdown-стек (micromark/mdast, shiki, KaTeX, footnotes, streaming), а следующий кандидат reference-работы — store seats/renderer, chain dispatch/hoc-nesting, ownerProps от shell-сайтов.
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
