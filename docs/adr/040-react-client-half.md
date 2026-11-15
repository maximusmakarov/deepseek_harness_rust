# 040. React-совместимость клиентской половины: plugin-shipped runtime и путь B → C

- **Статус:** Proposed (тиры A, B-PoC, C-lite, C1, C1.1, C2 реализованы; решение по C-full — [ADR-041](041-c-full-decision.md): отложен, точечные инкременты)
- **Дата:** 2026-11-14
- **Уточняет:** [ADR-013](013-cordis-compat.md) §5 (React-компоненты и SlotMap-семантика там остаются вне scope; этот ADR предлагает путь, как их закрывать)
- **Влияет на:** PLUGINS.md (клиентская половина), LIMITATIONS §2.4, COMPATIBILITY, TECH-DEBT, ROADMAP M5b

## Контекст

Клиентская половина плагина исполняется в sandboxed iframe за `postMessage`-мостом. Шелл — Leptos (ADR-003), поэтому React-компоненты DSH-плагинов не рендерятся, а `ownerProps`/framework-hooks/design-system пакеты (`@deepseek-ai/dsh-client-ui-*`) недоступны. В ADR-013 §5 это записано как сознательное вне-scope, но вопрос «что использовать вместо React» остаётся: существующие React-плагины DSH «как есть» не работают, а авторам новых плагинов нужен внятный контракт.

Раунд 76 закрыл региональный монтаж слотов: полный словарь имён DSH (116 ключей) разложен по 17 регионам, на каждый слот — отдельный iframe. Не хватало только UI-стека внутри слота.

Ключевое различие:

- **React как библиотека рендеринга** — может быть вшит в бандл плагина; мосту всё равно, чем плагин строит DOM.
- **React как контракт хоста** — общий module table, React-контекст, design system, SlotMap-семантика (`ownerProps`, `useSession`/`useChat`, cardinality/scope) — требует отдельного решения и стоит на порядок дороже.

## Решение

### 1. Enabler (реализован в раунде 77): `dsh.client.runtime`

`package.json` плагина может объявить один package-relative classic-скрипт:

```json
{ "dsh": { "client": {
  "platform": "web",
  "entry": "client.js",
  "runtime": "runtime.js"
} } }
```

Правила:

- путь package-relative, канонизируется и обязан оставаться внутри пакета (те же проверки, что у бандла);
- размер ≤ 8 МиБ; содержимое, содержащее `</script>`, отклоняется (400) — runtime вставляется в host-страницу как обычный `<script>` и не должен уметь закрыть тег;
- выполняется **до** бандла в том же sandboxed-документе; CSP остаётся сетевым (`default-src 'none'`), никаких новых сетевых разрешений;
- половина сама публикует `window.__DSHR_MODULES__`; `require(specifier)` в module-table бандлах (`__ModuleLoader__.load({id, factory})`) резолвит id из этой таблицы, неизвестный id падает с явной ошибкой (подсказка про `dsh.client.runtime`);
- runtime не проверяется по подписи отдельно: он часть пакета плагина и доверяется по тем же правилам, что и бандл.

Это делает возможным **React внутри слота**: плагин кладёт в `runtime.js` собранные React + ReactDOM и присваивает их в `__DSHR_MODULES__`; его бандл вызывает `require("react")` / `require("react-dom/client")` и рендерит компонент в контейнер слота. Проверено вручную (раунд 77) на React 18.3.1 UMD: рендер, `useState`-интерактивность и `ctx.onTheme` работают в slot-iframe.

### 2. Что enabler **не** даёт

- нет общей модульной таблицы шелла: каждый плагин приносит свою копию React (дублирование в каждом фрейме, свой размер и версия);
- нет design-system пакетов `@deepseek-ai/dsh-client-ui-*`, React-контекста шелла и композиции между плагинами;
- нет SlotMap-семантики: `ownerProps`, framework-hooks, cardinality/scope/chain/hoc/priority, `children`-render-prop, `t()` — компонент получает только `{theme, slot, id, plugin, context}`;
- ESM-runtime (`react/jsx-runtime` как модуль) не грузится: runtime — classic script, `jsx-runtime` приходится шимить на `React.createElement` (в PoC сделан шим).

### 3. Следующие тиры (на решение)

| Тир | Что | Плюсы | Цена |
|---|---|---|---|
| **A. Enabler (готово, раунд 77)** | плагин приносит свой runtime/React | дешево, изоляция, Leptos не трогаем; покрывает «свои» плагины | нет host-композиции и design-system |
| **B. React-таблица от хоста (PoC готов, раунд 78)** | host-страница предоставляет общий module table + React (одна версия, immutable-артефакт), плагинам не нужно вшивать React | одна загрузка React на страницу, ближе к DSH-контракту модулей | пока только React-рантайм: нет design-system/контекста/SlotMap-семантики (это C); версия React закреплена сервером; CSP для opt-in кадров открывает origin для скриптов |
| **C-lite (PoC готов, раунд 79)** | React-компоненты слотов поверх host React, host-state/hooks в props, `ctx.get('sessions')`, частичный шим `@deepseek-ai/dsh-client-ui-primitives`, 1 реальный плагин | работает для простых UI-плагинов без переписывания шелла | частичный design-system (5 компонентов), нет React-контекста шелла и SlotMap-семантики |
| **C. React-остров в Leptos** | в шелл добавляется React-runtime, регионы рендерят React-корни, мост реализует SlotMap-подмножество | настоящая композиция, design-system, хуки | крупная фаза: семантика SlotMap дороже самого React; пересмотр границы ADR-003 |
| **D. Полный React-шелл** | переписать UI на React | буквальный паритет | против ADR-003, переписывание всего UI |

Рекомендуемый порядок: A (сделано) → B как PoC с реальными npm-плагинами → решение по C только при запросе на экосистему DSH UI. D не рекомендуется.

## Последствия

**Плюсы**

- у авторов плагинов появляется легальный путь «принеси свой UI-стек», включая React, без нарушения изоляции;
- не переписывается шелл, CSP остаётся network-free;
- появляется основа для тира B (общий module table) без ломки протокола.

**Минусы и цена**

- дублирование runtime в каждом фрейме (в PoC host-страница выросла на ~143 КБ; это плата за изоляцию; для B можно кэшировать общий модуль на origin);
- classic-script ограничение, нет ESM и подписи отдельного runtime;
- контракт `dsh.client.runtime` экспериментальный (может поменяться до тира B).

## Альтернативы

- **Оставить только `render(element, props)`** — отклонено как единственный путь: авторы React-плагинов вынуждены переписывать UI.
- **Вшить React в ядро и раздавать всем фреймам** — это тир B целиком; отдельное решение, не enabler.
- **React-остров (тир C) сразу** — отклонено на этом шаге: дорого и не проверено на реальных плагинах; сначала B-PoC.

## Открытые вопросы

1. Кто предоставляет module table в тире B: сервер (route с общими модулями) или host-страница с вендоренным React (CSP и размер)?
2. Как совмещать версии React между шеллом и плагинами в тире C (один React на страницу)?
3. Нужна ли отдельная подпись/хеш для `runtime` в CAS-модели дистрибуции (ADR-013 §6)?

## Реализация тира B (15.11.2026, раунд 78)

PoC общего рантайма хоста:

- **Манифест.** `dsh.client.host_modules: ["react", "react-dom/client", ...]` —
  массив id из allowlist (`react`, `react/jsx-runtime`, `react/jsx-dev-runtime`,
  `react-dom`, `react-dom/client`); незнакомый id отклоняется при discovery.
- **Маршрут.** `GET /v1/runtime/react-18.3.1.js` — один версионированный
  immutable-артефакт (React 18.3.1 UMD + ReactDOM UMD + наш `bootstrap.js`,
  ~143 КБ), публичный (секретов нет, sandboxed-кадр всё равно не может
  аутентифицироваться), `Cache-Control: public, max-age=31536000, immutable`.
  Исходники, лицензия и контрольные суммы — `crates/server/assets/runtime/`,
  проверка — `scripts/check-runtime-assets.sh`.
- **Host-страница.** Для кадра с `host_modules` добавляется
  `<script src="/v1/runtime/react-18.3.1.js" data-dshr-shared-runtime="1">` перед
  plugin-shipped runtime и бандлом; рантайм публикует те же id в
  `window.__DSHR_MODULES__`, что и `dsh.client.runtime`, поэтому бандлы не
  меняются. Plugin-shipped runtime (если объявлен) идёт после и может
  переопределять записи.
- **CSP.** Для таких кадров `script-src 'unsafe-inline' blob: <origin>` (ровно
  origin сервера), `connect-src 'none'` и остальные директивы прежние; кадры без
  `host_modules` остаются полностью network-free. Host-header-инъекция в CSP
  закрыта тем же `request_origin` с валидацией, что и для assets.
- **Что это даёт.** Браузер скачивает React один раз на страницу и кэширует;
  каждая половина больше не вшивает ~140 КБ в свою host-страницу. JS-реалм у
  каждого iframe свой (изоляция не ломается), поэтому «одна версия» означает один
  артефакт/одну загрузку, а не общий JS-контекст.
- **Проверка.** `plugins_live`: маршрут и заголовки, host-страница ссылается на
  рантайм и не встраивает React, враждебный Host не попадает в CSP; smoke:
  фикстура `hosted` не содержит React в бандле, рендерит интерактивный
  React-компонент (`useState`) из общего рантайма; cordis: манифест/allowlist и
  порядок «shared → plugin runtime → bundle».

Оставшаяся цена тира B: по-прежнему нет design-system, React-контекста шелла и
SlotMap-семантики; CSP для opt-in кадров открывает origin для скриптов (не для
connect) — это записанный компромисс; версия React закреплена за сервером, у
плагинов нет выбора версии. Эти ограничения — предмет тира C.

## Реализация C-lite (15.11.2026, раунд 79)

Цель шага — «экосистема работает для простых UI-плагинов без переписывания
шелла». Сделано поверх тира B:

- **React-режим моста.** Если в кадре есть host React (`host_modules` содержит
  `react`), `ctx.slots.register(entry, Component)` трактуется как
  React-компонент (референсный контракт), а не imperative-колбэк. Мост держит
  один React-root на кадр, перерисовывает его при смене темы/host-state/context,
  оборачивает в error boundary и сообщает ошибку рендера в шелл.
- **Host-state и сервисы.** Шелл рассылает `host-state` (`theme`, `wide`,
  `session`, `sessions`) при `ready`, смене сессии/списка/темы. В props
  компонента появляются `useTheme()`, `useSession()`, `useSessions(selector?)`,
  `useHostState(selector?)` (+ `wide`); `ctx.get('sessions')` отдаёт подмножество
  референсного сервиса (`currentProvideInfo`, `get`, `list`, `subscribe`).
- **Дизайн-система.** Общий рантайм публикует
  `@deepseek-ai/dsh-client-ui-primitives` — независимый частичный шим
  (`Button`, `StateDot`, `Tooltip`, `Icon`, `Spinner`) на host React и CSS-
  переменных; allowlist `HOST_MODULES` расширен этим id.
- **Conformance.** Реальный MIT-плагин `@kt11/dsh-deepseek-balance@0.2.1`
  добавлен fixture-ом с upstream-клиентской половиной без изменений (sha256 в
  `package.json`); манифест только включает `host_modules`. В smoke он рендерит
  React-карточку в `sidebar.footer.action`, вставляет свой CSS и открывает popup
  по клику. Host-маршруты DSH у плагина отсутствуют в dshr, поэтому данные
  показывают ветку «查询失败» — рендер и интерактивность это не ломает.
- **Проверка.** Bridge harness 73 checks (host-state/`ctx.get`), cordis lib 26,
  server plugins_live 12, smoke 60/60 (включая hooks-фикстуру,
  design-system шим и реальный плагин), workspace 361 passed + 3 ignored,
  web-e2e 16/16.

Что осталось до полного тира C: полный design-system
`@deepseek-ai/dsh-client-ui-*` и SlotMap-семантика (`ownerProps`, framework-hooks,
cardinality/scope/chain/hoc), React-контекст/состояние шелла и композиция между
плагинами; host-маршруты DSH-плагинов; версии React; публикация настоящего
`@dshr/cordis-compat`-шима. Это отдельное решение владельца.

## Расширение conformance и сервисов моста (15.11.2026, раунд 80)

Второй реальный плагин и недостающие точки деградации:

- **`ctx.get('slots')`** — референсные плагины получают сервис слотов через
  `ctx.get('slots')`, а не `ctx.slots`; мост отдаёт тот же реестр
  (`inject`/`register`/`remove`/`list`) плюс `entries(key)`.
- **`ctx.get('locale')`** — минимальный сервис (`{active: "en"}`, subscribe),
  чтобы не падать на `Object.assign`/чтении локали.
- **`ctx.inject(deps, callback)`** — fiber-хелпер: callback вызывается только
  когда все зависимости доступны. Опциональные интеграции
  (`betterSidebar`/`sidebarRight`/`webTerminals`) в dshr отсутствуют, поэтому
  не монтируются — штатная деградация, на которую рассчитаны DSH-плагины.
- **Диагностика** — последняя ошибка кадра доступна как
  `window.__DSHR_LAST_ERROR__` (`{message, stack}`) без раунд-трипа к шеллу.
- **Conformance n=2.** К `@kt11/dsh-deepseek-balance@0.2.1` добавлен
  `dsh-remote@1.1.2` (MIT; клиентская половина upstream без изменений, sha256 в
  `package.json`): React-слоты в `conversation.session.header.*` и
  `sidebar.workspaces.directoryFlow` монтируются без ошибок при отсутствии
  опциональных сервисов.
- **Найденные ограничения conformance-набора.** `@michengai/dsh-archive-manager`
  использует `new Function`/eval → нужен `'unsafe-eval'` в CSP (сознательно не
  включён); `@michengai/dsh-skills-manager` падает на инициализации бандла
  (`Object.assign` над undefined) — требует триажа исходников плагина. Оба
  записаны в TECH-DEBT; набор остаётся n=2.
- **Проверка:** bridge harness 79 checks, smoke 63/63 (+3: второй плагин,
  его регионы и отсутствие ошибок), cordis lib 26, server plugins_live 12,
  workspace 361 passed + 3 ignored, web-e2e 16/16.

## Conformance n=4 и доработки React-контракта (15.11.2026, раунд 81)

Ещё два реальных плагина выявили конкретные пробелы моста — все закрыты:

- **Тип компонента.** `React.memo`/`forwardRef` возвращают объект с `$$typeof`,
  а не функцию; `normalize` теперь принимает такие типы в host-React-режиме
  (раньше падало `slot: render is required`).
- **Keyed-регистрации.** Регистрация с `key` без `id` получает уникальный id
  `<slot>:<key>`; иначе второй key того же слота молча заменял первый.
  `ctx.get('slots').entries(key)` отдаёт референсную форму с `component` и
  `options` для кода, который ищет свои entries.
- **Локали.** `ctx.locale.register(ns, dicts)` + `t()` в props компонента
  (английский словарь; `{name}`-подстановки).
- **Таймеры.** `ctx.interval`/`ctx.timeout` снимаются вместе с фреймом.
- **Референсная поверхность слотов.** `ctx.slots.subscribe(region, listener)`,
  `getVersion(region)`, `entries(key)`/`entriesOfSlot(region)` (entries несут
  `component` и `options`) — нужны плагинам, которые держат собственные
  shadow-регистрации; `ctx.locale.bind(ns)` отдаёт `t(key, params)`.
- **Primitives.** Шим расширен `JsonBlock`/`MarkdownText` (нужны genui).
- **Conformance n=4:** к `@kt11/dsh-deepseek-balance` и `dsh-remote` добавлены
  `@ruoyang/dsh-plugin-deepseek-balance@1.0.2` (плавающий бейдж в
  `shell.overlay`, `ctx.interval`/`ctx.locale`) и `dsh-genui@0.3.3` (тумблер в
  `conversation.input.left`, `React.memo`, keyed `conversation.chat.node`).
  Все — MIT, клиентские половины upstream без изменений, sha256 в `package.json`.
- **Проверка:** bridge harness 87 checks (+8: React-режим с memo/props/t и reference slots surface — subscribe/getVersion/entriesOfSlot, locale.bind),
  smoke 66/66 (+3), cordis 26, server plugins_live 12, workspace 361 passed +
  3 ignored, web-e2e 16/16.

## Conformance n=6 (15.11.2026, раунд 82)

Ещё два реальных MIT-плагина, без изменений клиентской половины:

- `dsh-session-superagent@0.10.12` — коммитит composer dock
  (`conversation.composer.dock`), активируется без ошибок;
- `dsh-mnemon@0.5.46` — крупный multi-region плагин (шапка чата, composer,
  панель, настройки, plugins.*); активируется, публикует слоты и сверяет свои
  shadow-регистрации через `ctx.slots.subscribe/getVersion/entriesOfSlot` —
  референсная поверхность слотов, добавленная в раунде 81, выдержала реальный
  плагин.

Conformance-набор: 6 MIT-плагинов (4 + 2), все fixture с sha256 в
`package.json`; `@michengai/dsh-archive-manager` (нужен `unsafe-eval`) и
`@michengai/dsh-skills-manager` (падение на инициализации бандла) остаются вне
набора и записаны в TECH-DEBT.

Проверка: smoke 69/69 (+3), bridge harness 87, cordis 26, server plugins_live 12,
workspace 361 passed + 3 ignored, web-e2e 16/16.

## Conformance n=9 (15.11.2026, раунд 83)

Ещё три реальных MIT-плагина, все — клиентские половины upstream без изменений:

- `@99galaxy/dsh-plugin-market@1.2.2` — запись в сайдбаре
  (`sidebar.panellist` → регион `sidebar.footer`), требуется только `react`;
- `dsh-plugin-codegraph-panel@0.6.5` — панель в `settings.section`
  (`immediately: true`), требуется только `react`;
- `@goodandready/dsh-plugin-notify@0.4.9` — строки в `plugins.item` /
  `plugins.row.config` (регион `plugins.detail`), требует primitives + react.

Все три активируются, публикуют слоты и не оставляют ошибок; market-кадр
монтируется в сайдбар. Conformance-набор достиг **n=9** (цель ADR-013 §6 —
8–12 реальных плагинов); остались вне набора `@michengai/dsh-archive-manager`
(`unsafe-eval`) и `@michengai/dsh-skills-manager` (падение инициализации).

Проверка: smoke 72/72 (+3), bridge harness 87, cordis 26, server plugins_live 12,
workspace 361 passed + 3 ignored, web-e2e 16/16.

## C1: host HTTP-маршруты плагинов (15.11.2026, раунд 84)

Реальные host-половины DSH регистрируют маршруты через
`ctx.inject(['webServer'], scope => scope.webServer.register({ name, kind, path, handler }))`
с Node `(req, res)`. dshr теперь это обслуживает:

- **Sidecar.** Loopback-only HTTP-сервер (`127.0.0.1`, случайный порт,
  per-process токен), `webServer.register` с валидацией пути (package/reserved
  prefixes), `credentials.resolve(name)` из окружения (ключи не покидают
  sidecar), маршруты снимаются вместе с процессом.
- **Host.** После старта sidecar manager читает внутренний инструмент
  `__dshr_routes` (порт, токен, таблица путей), хранит её в `Running` и отдаёт
  `http_route(path)`; внутренние `__dshr_*`-инструменты не попадают в реестр
  инструментов агента.
- **Proxy.** Fallback dshr-сервера проксирует только exact-пути из allowlist
  (`127.0.0.1:port`, заголовок с токеном), ограничивает request 1 МиБ /
  response 8 МиБ, таймаут 15 с; reserved-префиксы (`/v1`, `/dshr`, `/openapi`,
  UI-ассеты) и неизвестные пути → обычный 404; CORS/permissive для
  opaque-origin кадров; preflight отвечает глобальный CorsLayer до прокси.
- **CSP.** Кадры с `host_modules` теперь получают `connect-src <origin
  сервера>` — иначе браузерный fetch плагина к своему маршруту блокировался бы
  (реальная причина «Failed to fetch» в C-lite). Внешняя сеть по-прежнему
  запрещена, кадры без `host_modules` остаются network-free.
- **Проверка.** Реальная host-половина `@kt11/dsh-deepseek-balance` больше не
  застаблена: в smoke её маршрут `/dsh-balance/deepseek` отвечает своим JSON
  (`500`, `{ok:false, code:"not-configured"}` без ключа), `/dsh-usage/sessions`
  отдаёт `200` c JSON и query-строкой, неизвестный путь — 404 dshr; серверный
  integration test проверяет GET/POST/OPTIONS/404 и снятие маршрутов при
  остановке плагина.

Осталось от C1: сервисы `sessions`/`sessionProjections` (статистика по сессиям
в usage-маршрутах), SSE/WebSocket/стриминг, LAN-политика для plugin-маршрутов,
аутентификация (сейчас как в DSH — public loopback). Полный C-full (design-system,
SlotMap-семантика, React-контекст) остаётся решением владельца.

## C2: расширение design-system шима (15.11.2026, раунд 85)

Реальные плагины набора conformance используют гораздо больше, чем прежние 7
компонентов. Шим расширен по фактическим импортам (не по догадкам):

- **Компоненты:** `Input`, `Switch`, `Checkbox`, `Tag`, `Pill`, `Menu`,
  `MenuSurface`, `Modal`, `Toast`, `SegmentedControl`, `SegmentedTabs`,
  `DisclosureRow`, `PathLabel`, `TextShimmer` (плюс прежние `Button`,
  `StateDot`, `Tooltip`, `Icon`, `Spinner`, `JsonBlock`, `MarkdownText`) —
  семантика и props взяты из вызовов `dsh-mnemon`/`dsh-session-superagent`
  (`Menu` c `items/onSelect/anchor`, `Modal` c `title/description/footer`,
  `Segmented*` c `options/items/value/onChange`, `DisclosureRow` c
  `title/open/expandable/collapsedContent/headerAccessory`, и т.д.);
- **Иконки:** именованные `Icon*`-компоненты с SVG-глифами для реально
  используемых имён; для неизвестных `Icon*`-имён — Proxy-fallback на генерик,
  чтобы импорт не давал `undefined` (частая причина краха бандла);
- **Хуки:** `useAnchoredPosition` (fixed-позиция по anchor/panel refs с
  clamp в viewport) и `useDismissOnOutsidePointer` — их супер-агент пробует
  через `typeof === "function"` перед использованием host-попапа;
- **`writeClipboard`** (navigator.clipboard + execCommand fallback).

Проверка: smoke рендерит в host-React кадре дерево из всех новых примитивов
(включая портальные Menu/Modal/Toast) и проверяет 12 текстовых маркеров;
реальные `dsh-plugin-notify`/`dsh-mnemon` монтируют свои вклады в диалоге
Plugins без ошибок (`__DSHR_LAST_ERROR__` пуст).

Conformance **n=11**: добавлены `dsh-plugin-debugging@0.8.1` и
`dsh-plugin-studio@0.3.1` (MIT, клиентские половины без изменений). На момент
C2 они активируются без ошибок, но их UI остаётся несмонтированным (нужны
seat-`inject`/генераторы/`configForms`); раунд 89 (C1.2) научил их рендериться —
см. раздел C1.2 ниже.

Осталось от C2: полный design-system (тир C-full), больше компонентов по мере
роста набора, точные визуальные токены upstream.

## Хардненинг C1 (15.11.2026, раунд 86)

- **Prefix-маршруты:** `webServer.register` принимает `kind: "exact"` и
  `kind: "prefix"`; sidecar матчит exact раньше prefix, среди prefix — самый
  длинный; менеджер и прокси ведут себя так же.
- **Диагностика:** `GET /v1/plugins/status` теперь отдаёт per-plugin `routes`
  (живая таблица host-маршрутов), рядом с `tools`.
- **Шум MCP:** псевдо-инструменты моста (`list_resources`, `read_resource`,
  `list_prompts`, `get_prompt`) больше не регистрируются для Cordis-плагинов;
  в реестре остаются только реальные инструменты плагина (и `__dshr_*`
  по-прежнему скрыт).
- **LAN-политика:** plugin-маршруты неаутентифицированы (у iframe нет токена),
  поэтому при bind вне loopback прокси отвечает 403; явный opt-in —
  `--allow-lan-plugin-routes` (ServerConfig.allow_plugin_routes_lan). Это
  закрывает записанный в TECH-DEBT пункт «routes public as DSH / LAN».

Проверка: server integration tests 15 (proxy: exact/prefix/query/POST/preflight/
404/reserved/route-removal/status/meta-tools; loopback-only policy), smoke 79/79
без изменений, workspace tests 364 + 3 ignored.

## C1.1: сервисы sessions/sessionProjections в sidecar (15.11.2026, раунд 87)

Host-половины реальных плагинов читают статистику через
`ctx.get('sessions')` и `ctx.get('sessionProjections')`. dshr закрывает это
снимком, а не живым RPC:

- **Сервер.** `serve()` генерирует per-run внутренний токен, пишет
  `<home>/cordis/host.json` (`{url, token}`) и публикует
  `GET /v1/internal/session-snapshot` — recent ≤50 сессий (id/title/created_at/
  model/turns/steps и usage-события assistant/turn/step/tokens; **содержимое
  сообщений и промпты не отправляются**). Без настроенного токена эндпоинт
  отвечает 404, при неверном — 401.
- **Sidecar.** Bootstrap читает bridge-файл, раз в 5 с (unref-таймер) обновляет
  снимок и транслирует его в DSH-shaped данные: `sessions.list()/get(id)` с
  `header.createdAt` (epoch ms) и `log` (`request/context`,
  `session/title`, `assistant/message` c `usage`), а также
  `sessionProjections.snapshot(session).values` с `sessionStats` и
  `tokenUsage`. Методы синхронные (снимок), поэтому data может отставать до
  5 с — записано как ограничение.
- **Эффект.** Usage-маршруты реального `@kt11/dsh-deepseek-balance`
  (`/dsh-usage/sessions`) теперь перечисляют сессии dshr и считают totals;
  smoke подтверждает это из браузера (sessions ≥ 1, totals на месте).

Осталось: проекции упрощены (`llmMs`=0, без истории тредов), содержимое сессий
намеренно не покидает сервер, streaming/SSE — по-прежнему нет. Полный C-full
(design-system, SlotMap, React-контекст) остаётся решением владельца.

## C1.2: seat inject, генераторы и configForms (15.11.2026, раунд 89)

Ещё два контракта DSH-слотов, без которых settings-плагины оставались инертными:

- **seat `inject`** в регистрации слота: `inject: () => ({...})` даёт
  компоненту дополнительные props (framework seat). Ошибка фабрики (нет
  host-сервиса) деградирует компонент, но не помечает плагин упавшим;
- **Cordis-генераторы**: `slots.inject`, `ctx.inject` и `ctx.effect` принимают
  `function*`; тело исполняется, `yield`-disposer'ы собираются и вызываются на
  teardown в обратном порядке. Это то, что ждёт `dsh-plugin-studio`
  (регистрирует вкладки генератором);
- **`configForms`**: browser-bridge отдаёт `get(entryId)` с формой
  (`getSnapshot`/`subscribe`/`mutate`), стабильный snapshot-объект для
  `useSyncExternalStore`, revision-контракт; host-side bootstrap — файловый
  вариант под `<home>/cordis/config-forms/`.

Проверка: smoke 82/82 (+2) рендерит реальные вкладки studio в Settings и
проводит переключатель+Save карточки debugging; harness 90 (+3) покрывает
seat-props, генераторный `slots.inject` и генераторный `effect`.

Границы: browser-`configForms` in-memory (правки не переживают reload кадра),
семантика DSH-конфигурации не воспроизведена; seat-inject-провалы только
логируются. Полный C-full — по [ADR-041](041-c-full-decision.md) отложен.

## C1.3: слот-контракт по референсному каталогу (15.11.2026, раунд 91)

SlotMap-семантика больше не «на глаз»: каталог слотов сгенерирован из
`deepseek-ai/deepseek-harness` (тот же generated `slot-catalog.ts`, что отдаёт
модели `cordis_inspect what:"client"`), и `bridge.js` проверяет регистрации по
reference `SlotCore`:

- **каталог** (`crates/cordis/src/slot-catalog.json`): 122 слота,
  `kind`/`scope`/framework standard-kit/`inject`-флаг; host-страница отдаёт
  его как `window.__DSHR_SLOT_META__`;
- **валидация**: undeclared-слот бросает, `keyed`/`list`/`hoc`/`chain` требуют
  `key`/`id`/`id`/`select`, конфликт в ячейке на том же `priority` бросает,
  другой priority затеняет (lowest renders);
- **children-декларации**: запись может объявить дочерние слоты
  (`children: {key: {kind, scope}}`), объявление каскадно живёт/умирает вместе
  с записью — это то, что использует `dsh-mnemon` для своих
  `mnemon.component.*`;
- **deferred inject**: `ctx.slots.inject(key, factory)` ждёт объявления слота и
  перезапускается при появлении, как reference registry;
- **победители**: shell монтирует только shadowing-winners (`single` — один,
  `keyed` — по ключу, `list`/`chain`/`hoc` — все слои);
- **props**: standard-kit (по каталогу; нереализованные hooks остаются
  вызываемыми) → seat `inject` → ownerProps из frame config → `t`.

Проверка: bridge harness **98** (+8: undeclared/keyed/chain/duplicate throws,
single shadowing, standard kit, ownerProps, plain values), smoke 82/82
(реальные плагины регистрируются по тем же правилам; synthetic fixtures
дополнены `key`), workspace 365 + 3 ignored.

Границы: `chain` select-диспетчеризация, `hoc`-вложение, store-seats и
mounted session model остаются долгом; ownerProps от shell пока только seam.
Полный C-full (design-system, store/renderer) — по [ADR-041](041-c-full-decision.md).

## C1.4: webServer, baseUrl и configForms по reference-контракту (15.11.2026, раунд 92)

Сверка с исходниками upstream (клон `deepseek-ai/deepseek-harness`) выявила
расхождения в host-сервисах, которые закрыты:

- **webServer** (`packages/host/webserver/src/index.ts`): exact/prefix —
  независимые таблицы; duplicate `(kind, path)` бросает; путь — абсолютный
  pathname без trailing slash/query/hash; `getOrigin()/port/protocol` отдают
  origin dshr-сервера (из host-bridge), а не loopback-порт sidecar-прокси;
  `registerFallback`/`registerUpgrade` не поддержаны в sidecar и явно бросают;
- **ctx.baseUrl** — loader-context свойство (`file://` URL корня пакета), как в
  reference Cordis; нужно host-половинам, которые резолвят пути от `baseUrl`;
- **configForms** (`config-form-types.ts`): snapshot
  `{status, value, base, user, revision: number, writable}` со стабильной
  идентичностью, `mutate` с `set`/`unset`-операциями и числовым
  revision-fence, хелперы `set`/`unset`; обе реализации (browser in-memory и
  host-side file-backed) приведены к контракту.

Проверка: bridge harness **102** (+4: форма-контракт, стабильный snapshot,
set/unset и числовая ревизия), server proxy test расширен проверками duplicate
route / trailing-slash; smoke 82/82, workspace 365+3, web-e2e 16/16.

Осталось: `registerFallback`/`registerUpgrade` (нужны для чужих SPA/WS-плагинов),
persistence browser-configForms, затем полный design-system — по
[ADR-041](041-c-full-decision.md).

## C2.1: атомы design-system по reference-исходникам (15.11.2026, раунд 93)

Раньше `primitives.js` был независимым «похожим» шимом. Теперь он портирован из
`deepseek-ai/deepseek-harness @ 39163dd`,
`packages/client/ui-primitives/src`:

- **атомы** — `Button`, `Input`, `Switch`, `Checkbox`, `Tag`, `Pill`,
  `SegmentedControl`, `SegmentedTabs`, `DisclosureRow`, `TextShimmer`,
  `StateDot`, `PathLabel`, `Toast`, `Modal`, `Menu`, `MenuSurface`,
  `Tooltip` повторяют reference DOM/ARIA/props/поведение (см. PLUGINS C2.1);
- **иконки** — все 204 export'а `Icon*` сгенерированы из upstream
  `icons/artwork.ts` (точные SVG-пути, regular/medium stroke), инлайн вместо
  спрайта; Proxy-fallback оставлен для будущих имён;
- **стили** — те же `--dsw-alias-*` токены, что использует reference
  (с fallback'ами), поэтому тема dshr управляет видом;
- **совместимость** — synthetic fixtures приведены к reference-контрактам
  (`StateDot state:"done"`, `Tooltip` с element-child), C2-probe проверяет
  структурные признаки (role switch, data-tone, tablist, svg, path-label).

Проверка: smoke 82/82 (два прогона подряд), harness 102, workspace 365+3,
web-e2e 16/16; общий рантайм-артефакт вырос до 375 894 байт (cached immutable
один раз на страницу).

Осталось: полный `Menu` (submenu/focus-движок) и `Tooltip` (placement-engine),
`MarkdownText`/`JsonBlock` (тяжёлые markdown-зависимости), `store`-seats/
renderer-семантика, `chain`-диспетчеризация/`hoc`-вложение.
