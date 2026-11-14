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
  C-lite React-компоненты слотов)
- сознательно вне скоупа (`skip`): полный React-компонентный контракт, инъекция в
  DOM шелла, внешние модули (`dsh.client.external`) — следствие ADR-013 §5, а не
  недоделка. C-lite (раунды 79–82) даёт частичную React-совместимость для простых
  UI-плагинов вне скоринга; путь к полной — [ADR-040](../adr/040-react-client-half.md)
  (Proposed: C-full после C-lite)

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

Что осталось из should (не блокирует):

| Фича | Состояние | Чем отличается |
|---|---|---|
| Bundles built for the reference module table | partial | шим `__ModuleLoader__.load({id, factory})` исполняет `factory` и `apply(ctx)`; `require` резолвит модули из `dsh.client.runtime`, если половина их опубликовала (в PoC — React 18.3.1), или из общего рантайма хоста, если половина опционально объявила `dsh.client.host_modules` (React 18.3.1 + jsx-шимы + частичный design-system шим `@deepseek-ai/dsh-client-ui-primitives`). Полной таблицы модулей DSH (полный design-system, React-контекст шелла) нет — это единственная незакрытая should-фича, поэтому score 98.1, а не 100 |

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
| react slots / host state | `crates/cordis/src/bridge.js`: React-режим `ctx.slots.register(entry, Component)`, один React-root на кадр, `host-state` (`theme`/`wide`/`session`/`sessions`), hooks `useTheme`/`useSession`/`useSessions`/`useHostState`, `ctx.get('sessions')`, error boundary; `crates/web/src/main.rs`: `push_host_state_all`; `crates/cordis/src/client.rs`: `HOST_MODULES` с id примитивов; тесты: bridge harness 87 checks (host-state/`ctx.get`/`ctx.get('slots')`/`locale`/`ctx.inject`/`__DSHR_LAST_ERROR__` + раунд-81 `reactScenario`: memo-тип, host-props/hooks, `t()`-место, keyed-ids, форма `slots.entries`), smoke 69/69 (фикстуры `hooks`, `hosted`, `dsh-remote`, `ruoyang-balance`, `dsh-genui`, `dsh-session-superagent`, `dsh-mnemon`) |
| design-system primitives | `crates/server/assets/runtime/primitives.js` — независимый **частичный** шим `@deepseek-ai/dsh-client-ui-primitives` (`Button`, `StateDot`, `Tooltip`, `Icon`, `Spinner`; с раунда 81 — ещё `JsonBlock`/`MarkdownText`, всего 7) на host React и CSS-переменных темы, в `checksums.txt`/`README.md`; smoke: фикстура `ui-kit` резолвит его из общего рантайма, а genui использует `JsonBlock`/`MarkdownText` |
| conformance | Шесть реальных MIT-плагинов в `crates/cordis/tests/fixtures/` (n=6; клиентские половины upstream без изменений, sha256 в `package.json`, host-половины — заглушки): `@kt11/dsh-deepseek-balance@0.2.1` — smoke: React-карточка в `sidebar.footer.action`, инжект собственного CSS, popup по клику (data-ветка «查询失败» — host-маршрутов DSH в dshr нет); `dsh-remote@1.1.2` (раунд 80) — smoke: discovery с примитивами в `host_modules`, registry-кадр без ошибок при отсутствии опциональных сервисов, React-слот `conversation.session.header.actions` монтируется в регион `conversation.header`; `@ruoyang/dsh-plugin-deepseek-balance@1.0.2` (раунд 81, sha256 `ca700695…`) — плавающий бейдж в `shell.overlay` через `ctx.effect`/`ctx.interval`/`ctx.locale`, рендерится без ошибок; `dsh-genui@0.3.3` (раунд 81, sha256 `8f0500ad…`) — тумблер GenUI в `conversation.input.left` и keyed-регистрация `conversation.chat.node` через `React.memo`, рендерится без ошибок; `dsh-session-superagent@0.10.12` (раунд 82, sha256 `d58f0fa2…`) — composer dock `conversation.composer.dock` монтируется в регион `composer.actions` без ошибок; `dsh-mnemon@0.5.46` (раунд 82, sha256 `d967402b…`) — крупный multi-region плагин: активируется, публикует слоты и сверяет собственные shadow-регистрации через `ctx.slots.subscribe`/`getVersion`/`entriesOfSlot`. Не входят в набор: `@michengai/dsh-archive-manager` (нужен `'unsafe-eval'` в CSP) и `@michengai/dsh-skills-manager` (падение на инициализации бандла) |
| layout | smoke: `resize` выставляет высоту конкретного кадра (кламп 24–640 px) |

Команды:

```sh
cargo test -p dshr-cordis          # bridge harness (каталог слотов, регионы, slot-режим, React-режим/host-state, сервисы моста, React-контракт) + assets + runtime + shared host runtime + manager
cargo test -p dshr-server --test plugins_live
python3 scripts/plugin-smoke.py    # 69 проверок в headless Chrome (включая hooks, ui-kit и шесть реальных плагинов: deepseek-balance, dsh-remote, ruoyang-balance, dsh-genui, dsh-session-superagent, dsh-mnemon)
```

Скоринг пересчитывается по весам выше вручную по матрице
`client-half-features.csv` (отдельного скрипта в репозитории нет).

## Что бы сделал следующий раунд

1. C-full по [ADR-040](../adr/040-react-client-half.md): полный design-system
   `@deepseek-ai/dsh-client-ui-*`, SlotMap-семантика (`ownerProps`,
   framework-hooks, cardinality/scope/chain/hoc), React-контекст/состояние шелла
   и композиция между плагинами — только если появится запрос на экосистему DSH
   UI; C-lite (React-компоненты слотов с host-hooks/`ctx.get('sessions')` и
   частичным design-system шимом) сделан в раунде 79.
2. Host-маршруты DSH-плагинов: без них реальные data-плагины (например,
   `@kt11/dsh-deepseek-balance`) рендерятся, но показывают ветку ошибки данных;
   `wide` в host-state должен отражать фактическую ширину сайдбара.
3. Conformance-набор из реальных npm-пакетов: сейчас n=6
   (`@kt11/dsh-deepseek-balance@0.2.1`, `dsh-remote@1.1.2`,
   `@ruoyang/dsh-plugin-deepseek-balance@1.0.2`, `dsh-genui@0.3.3`,
   `dsh-session-superagent@0.10.12`, `dsh-mnemon@0.5.46`), цель M5b — 8–12
   плагинов в nightly CI; для eval-based бандлов нужен opt-in `'unsafe-eval'`
   CSP, а `@michengai/dsh-skills-manager` требует триажа инициализации.
