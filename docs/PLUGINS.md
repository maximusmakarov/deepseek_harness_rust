# Плагины: WASM и sidecar

## WASM

Поддерживаются два ABI: legacy core-модуль на wasmi (`dshr.wasm.v1`, JSON через
упакованный `ptr<<32|len`) и компонент на wasmtime (`dshr:plugin@1.0.0`, WIT,
строки через canonical ABI). Хост определяет тип по бинарю автоматически; подпись
и sha256 проверяются для обоих одинаково.

Плагин — `.wasm` + YAML-манифест в `~/.dshr/plugins` или `<cwd>/.dshr/plugins`.

```yaml
name: echo
description: Echo plugin
wasm: echo.wasm
# optional: sha256: <hex SHA-256>
# optional signature block, written by `dshr plugin sign`:
# signature: <hex Ed25519 signature>
# signer: <hex Ed25519 public key>
```

### Component model (WIT)

```wit
package dshr:plugin@1.0.0;
world plugin {
    export list-tools: func() -> string;
    export call-tool: func(name: string, arguments: string) -> string;
}
```

`list-tools` возвращает JSON `{"tools":[...]}`; `call-tool` получает имя инструмента и
JSON-аргументы, возвращает JSON `{"content":"...","isError":false}`. Компонент
запускается на wasmtime с пустым WASI-контекстом (без preopens, args, env и сети),
fuel 10M и лимитом памяти 64 MiB. Готовый пример — `crates/wasm/tests/guest`,
фикстура — `crates/wasm/tests/fixtures/wit_plugin.wasm`; сборка:
`cd crates/wasm/tests/guest && cargo component build --release`.

### Legacy core ABI

ABI `dshr.wasm.v1`:

| Экспорт | Сигнатура | Назначение |
|---|---|---|
| `memory` | — | память гостя |
| `list_tools` | `() -> i64` | JSON `{"tools":[{name,description,inputSchema}]}` |
| `call_tool` | `(i32, i32) -> i64` | JSON `{"content":"...","isError":false}` |
| `alloc` | `(i32) -> i32` | опционально; получить буфер под JSON-запрос |

Результат упакован как `ptr << 32 | len`; host читает UTF-8 из `memory`.

Свойства:
- нет host-импортов, WASI и сети — только вычисления;
- опциональная проверка `sha256` манифеста;
- опциональная Ed25519-подпись (`sha256` не заменяет авторизацию: подпись привязывает содержимое и имя плагина к доверенному ключу);
- fuel-лимит 10M инструкций, memory cap 64 MiB;
- инструменты получают имена `wasm__<plugin>__<tool>`;
- модуль инстанцируется на каждый вызов (изоляция, но без кэша состояния).

## Подпись WASM-плагинов

Подпись покрывает сообщение `dshr.wasm.v1\n<name>\n<sha256 wasm>`: переименование
плагина или подмена `.wasm` ломает проверку. Доверие задаётся только конфигом, не
манифестом:

```yaml
plugins:
  trusted_keys:
    - <hex Ed25519 public key>
```

```sh
dshr plugin keygen --out ~/.dshr/plugins.key   # печатает публичный ключ
dshr plugin sign ~/.dshr/plugins/echo.yaml --key ~/.dshr/plugins.key
dshr plugin verify ~/.dshr/plugins/echo.yaml   # берёт `signer` из манифеста
dshr plugin verify ~/.dshr/plugins/echo.yaml --key <hex public key>
```

Поведение загрузчика (fail-closed): непустой `trusted_keys` требует подпись у каждого
плагина; неподписанный плагин, подпись от недоверенного ключа или подпись при пустом
списке ключей отклоняются. Приватный ключ — 32-байтная hex-строка, `keygen` пишет
файл с правами `0600` на Unix.

## WASM hot reload

`WasmManager` следит за `<home>/plugins` и `.dshr/plugins` (fingerprint
манифестов/`.wasm`), перерегистрирует `wasm__*` инструменты при изменении;
`POST /v1/plugins/reload` синхронизирует WASM и Cordis, `wasm_tools` в ответе.

## Cordis grants

`plugins.cordis[].grants.read/write` расширяют доступ `ctx.fs` за пределы
workspace (пути относительно корня); `ctx.exec`/`ctx.net` остаются выключены.

## Cordis audit

Каждый вызов `cordis__<plugin>__<tool>` дописывает строку в
`<home>/cordis/audit.jsonl` (time, plugin, tool, ok, error).

## Cordis / npm (sidecar)

JS-плагины исполняются отдельным процессом `node`; bootstrap публикует их
`ctx.tool(...)` как MCP-сервер, поэтому инструменты идут через существующий конвейер.
При наличии пакета `cordis` рядом с плагином создаётся настоящий `Context` и
вызывается `ctx.plugin(plugin)`, иначе вызывается `plugin.apply(ctx)` напрямую.

```yaml
plugins:
  cordis:
    - name: demo
      entry: plugins/demo.mjs   # файл или каталог пакета, относительно workspace
      env: {}
```

```js
export default {
  apply(ctx) {
    ctx.tool({
      name: "echo",
      description: "Echo",
      inputSchema: { type: "object", properties: { text: { type: "string" } } },
      handler: (args) => `echo:${args.text}`,
    });
  },
};
```

Инструменты получают имена `cordis__<plugin>__<tool>`. Capability-шина:
`ctx.fs` ограничен workspace-корнем, `ctx.exec` и `ctx.net` бросают ошибку;
наследования env нет, переменные задаются явно. `DSHR_NODE` переопределяет бинарь
node, `DSHR_CORDIS_ROOT` — корень (ставится автоматически). Кэша грантов нет:
каждый запуск sidecar получает гранты заново из конфига.

## Cordis hot reload

Набор плагинов — это живое множество, а не снимок на старте. `dshr serve`
поднимает `dshr_cordis::CordisManager`: он владеет sidecar-процессами и держит
serving-реестр инструментов (`ToolRegistry` — общий мутируемый handle) в
соответствии с конфигом. Перезагрузка = остановить sidecar (снять его инструменты
из реестра, процесс завершается после in-flight вызовов) и запустить новый
процесс с новой генерацией.

Триггеры:

- **Файловый watcher** (по умолчанию включён в `serve`, интервал 750 мс): обходит
  каталог пакета плагина (пропускает `node_modules`, `.git`, `target`) и сравнивает
  mtime/размер; правка кода плагина перезагружает только его. Так же следятся
  `profiles/<profile>/profile.yaml` и `<home>/config.yaml`.
- **Конфиг**: `plugin npm add/remove/enable/disable` (в том числе из Web UI) меняют
  конфиг, watcher делает reconcile — новые плагины стартуют, удалённые
  останавливаются, изменённые (entry/env/grants) перезапускаются, без рестарта
  harness.
- **HTTP/UI**: `POST /v1/plugins/reload` `{ "name": "demo" }` или `{}` для всех;
  кнопка Reload в панели Plugins.

Статус: `GET /v1/plugins/status` — `name`, `enabled`, `running`, `generation`,
`tools`, `routes` (живые host-маршруты C1), `started_at`, `error`. Ошибка старта запоминается в статусе и
перепопытка происходит при следующем изменении или Reload. Отключить watcher:
`dshr serve --no-watch-plugins`.

Семантика, которую надо учитывать в плагине: перезагрузка — это новый процесс,
поэтому in-memory состояние плагина сбрасывается; инструменты, которые уже
исполняются, доигрывают на старом процессе. Инструменты сверх
`tools.allowed`/`tools.denied` не регистрируются (фильтр применяется и к
динамическим регистрациям).

## Клиентская половина (dsh.client)

Метаданные совместимы с DSH: `dsh.client` в `package.json` плюс `./client`-экспорт.

```json
{
  "name": "@acme/demo",
  "main": "index.mjs",
  "dsh": { "client": { "platform": "web", "entry": "client.js", "immediately": false, "inject": [] } },
  "exports": { "./client": "./client.js" }
}
```

`entry` (или `exports["./client"]`, ключи `browser`/`import`/`default`/`require`)
должен указывать на файл внутри пакета; `platform` поддерживается только `web`.
Клиентский бандл — ES-модуль, который при активации получает мост:

```js
export default function activate(ctx) {
  ctx.slots.register(
    { name: "sidebar.footer.action", id: "demo-footer", title: "Demo", order: 10 },
    (element, props) => { element.textContent = `theme: ${props.theme}`; },
  );
  ctx.effect(() => {
    const timer = setInterval(refresh, 5000);
    return () => clearInterval(timer);
  });
  ctx.onTheme((theme) => document.body.dataset.theme = theme);
  ctx.action("reload").then((result) => ctx.log("reload:", result.ok));
}
```

Форма регистрации повторяет референс: `{ name, id, title, order }` и
`ctx.slots.inject(ownerKey, () => ctx.slots.register(...))` — регистрации группы
снимаются вместе, а `ctx.effect(factory)` возвращает disposer и тоже принадлежит
фрейму. Устаревшая форма `{ slot, render }` тоже принимается.

### Слоты и регионы

Имя слота — ключ из словаря слотов DSH; шелл отображает его в **регион** своей
раскладки и монтирует туда **отдельный iframe на регистрацию**. React не нужен:
`render(element, props)` получает контейнер и строит DOM сам. Полный словарь DSH
(116 ключей каталога `@deepseek-ai/dsh-client-ui-*` версии 0.2.2-rc.2) разложен по
регионам; незнакомое имя монтируется в панель плагина (fallback, как раньше).

| Регион | Где в шелле | Примеры ключей DSH |
|---|---|---|
| `panel` | панель плагина (вкладки) | `panel`, `main`, `conversation.view`, `contextMenu.content`, `rightbar`, `deliverables.*` |
| `sidebar.top` | верх левой колонки | `sidebar.workspaces`, `sidebar.brand.mark`, `sidebar.brand.name`, `sidebar.toggle.badge` |
| `sidebar.footer` | низ левой колонки | `sidebar.footer.action`, `sidebar.settings`, `sidebar.panellist*`, `sidebar.workspaces.session.menu.item` |
| `sidebar.session.action` | строка сессии | `sidebar.session.action`, `sidebar.session.row.trailing/leading/hover`, `sidebar.workspaces.session.row.action` |
| `settings.section` | тело диалога Settings | `settings.section`, `settings.general.item`, `settings.header`, `settings.models.*`, `settings.plugins.tab` |
| `settings.icon` | навигация Settings | `settings.section.icon` |
| `settings.action` | футер Settings | `settings.action`, `settings.close`, `settings.models.footer` |
| `message.action` | под сообщением | `message.action`, `conversation.chat.assistant-actions`, `conversation.chat.turnTail`, `conversation.chat.user.before`, `conversation.user.text.node` |
| `tool.card` | под карточкой инструмента | `tool.call.toolview`, `tool.call.images`, `tool.view.cordis` |
| `conversation.header` | шапка чата | `conversation.header`, `conversation.session.header.actions`, `.lineage`, `.utilities`, `.corner`, `.views.actions` |
| `composer.actions` | строка ввода | `conversation.input.right/left/activity/model/...`, `conversation.composer*`, `conversation.plan-review.actions` |
| `shell.leading` | левый край шапки | `shell.leading`, `settings.launcher*`, `settings.trigger` |
| `shell.bottom` | нижняя полоса | `shell.bottom`, `shell.quota-notice`, `workspace.toast` |
| `shell.overlay` | фиксированный overlay над шеллом | `shell.overlay` |
| `plugins.add` | диалог Plugins (над списком) | `plugins.add.actions` |
| `plugins.detail` | диалог Plugins (низ) | `plugins.detail.*`, `plugins.tab`, `plugins.item`, `plugins.row.config`, `plugins.bundle.*` |
| `approval.detail` | диалог approval | `conversation.approval.detail` |

Повторяющиеся регионы (`message.action`, `tool.card`, `sidebar.session.action`)
монтируются на каждое вхождение и получают `props.context`:

```js
ctx.slots.register(
  { name: "conversation.chat.assistant-actions", id: "copy" },
  (element, props) => {
    element.textContent = `message #${props.context.index} (${props.context.role})`;
  },
);
```

- сообщение — `{index, role}`; карточка инструмента — `{index, role: "tool"}`;
- строка сессии — `{session}`; карточка панели — `{}`; регистрация без контекста — `null`.

### Как монтируются фреймы

- **Eager registry**: при старте шелл монтирует по одному **скрытому** iframe на
  каждую половину (в порядке `inject`; `immediately`-половины идут первыми среди
  корней). Эти фреймы публикуют регистрации и их регионы — поэтому панель, сайдбар
  и диалоги знают о слотах до открытия панели.
- **Slot frame**: на каждую регистрацию (и на каждое вхождение повторяющегося
  региона) шелл создаёт отдельный iframe `mode=slot` с `slot`/`id`/`context`;
  бандл в нём рендерит ровно эту регистрацию. Фреймы всегда-видимых регионов
  монтируются со стартом; фреймы панели — при открытии панели, диалогов — при
  открытии диалога, повторяющихся — вместе со строкой.
- `ctx.effect` и `inject`-группы живут вместе с фреймом: закрытие панели/диалога
  снимает его фреймы (регистрации в registry-фрейме при этом сохраняются).
- Тема: начальная приходит в URL, дальше шелл рассылает её всем фреймам
  (`onTheme`), перезагрузка не нужна.
- `resize`: после рендера (и на resize окна) мост шлёт высоту, шелл выставляет
  inline-height фрейма (24…640 px).
- `reload`: все фреймы плагина перезагружают бандл (в URL меняется `v`), панель
  остаётся открытой.

Мост (`window.dshr`) в изолированном iframe:

- `ctx.name`, `ctx.theme`, `ctx.protocol === 1`, `ctx.onTheme(listener)`;
- `ctx.slots.register(entry, render)`, `inject(ownerKey, factory)`, `remove(id)`,
  `list()`; `render(element, props)` получает контейнер слота (React не нужен);
- `ctx.effect(factory)` — ресурсы с cleanup'ом, снимаются на `dispose`/выгрузке;
- `ctx.action(action, payload) -> Promise<{ok, result?, error?}>`; шелл выполняет
  `close` (закрыть панель), `reload` (перезагрузить плагин и все его фреймы),
  `compose` (`payload.text` — вставка в поле ввода), `notify`
  (`payload.message` — статус-строка); неизвестное действие возвращает `ok: false`;
- `ctx.log(...)` — строка в консоль шелла.

Протокол `postMessage` v1.2: iframe → шелл `ready`, `slots`, `action`, `resize`,
`error`, `log`; шелл → iframe `theme`, `action-result`, а также `activate`/
`deactivate` для registry-фрейма (совместимость). Все сообщения помечены
`ns: "dshr-client"` / `ns: "dshr-shell"`, `v: 1`, именем плагина, токеном фрейма
(`frame`) и режимом (`mode: "registry"|"slot"`). В `slots` и `resize` приходит
`region`; в `ready` — готовность бандла.

Доставка бандла — два режима:

- **inline** (по умолчанию для одного файла): бандл встроен base64 и исполняется из
  blob-URL, CSP запрещает любые сетевые запросы, origin iframe непрозрачный;
- **assets** (автоматически, если в бандле есть относительные `import`/`require`,
  либо `dsh.client.bundle: "assets"`): host-страница получает одноразовый capability
  `/v1/plugins/{name}/asset/{capability}/…` (TTL 1 час), модульный граф грузится
  relative-импортами по этому пути, CSP разрешает только этот origin.

### Свой UI-стек внутри слота (`dsh.client.runtime`)

React из шелла плагину не выдаётся (шелл — Leptos, ADR-003/ADR-013 §5), поэтому
внутри слота используйте одно из двух:

- **ванильный DOM** — `render(element, props)` получает контейнер, дальше
  `innerHTML`/`createElement`/шаблоны;
- **свой UI-стек, собранный в бандл** — Preact, Lit, Solid, Vue, Svelte или
  **React**: iframe ничего не навязывает, если библиотека вшита в сам бандл.

Устаревшие бандлы, собранные под reference-таблицу модулей (`require("react")`,
`__ModuleLoader__.load({id, factory})`), могут принести свою модульную таблицу
одним classic-скриптом `dsh.client.runtime`:

```json
{
  "dsh": { "client": {
    "platform": "web",
    "entry": "client.js",
    "runtime": "runtime.js"
  } }
}
```

`runtime.js` выполняется **до** бандла в том же sandboxed-документе и может
опубликовать `window.__DSHR_MODULES__`; `require(specifier)` в бандле резолвит id
из этой таблицы, неизвестный id падает с явной ошибкой. Для React это выглядит так:

```js
// runtime.js: плагин несёт React 18 UMD + свой мини-шим jsx-runtime
window.__DSHR_MODULES__ = {
  react: window.React,
  "react-dom/client": window.ReactDOM,
  "react/jsx-runtime": { jsx: (type, props, key) => React.createElement(type, { ...props, key }) },
};
// client.js: module-table бандл
window.__ModuleLoader__.load({ id: "@acme/react", factory(require) {
  const React = require("react");
  const { createRoot } = require("react-dom/client");
  return { inject: ["slots"], apply(ctx) {
    ctx.slots.register({ name: "panel", id: "react" }, (el) => {
      createRoot(el).render(React.createElement("h1", null, "hello from React"));
    });
  } };
} });
```

Правила runtime: package-relative путь (конфайнмент как у бандла), classic-скрипт
(не ESM), ≤ 8 МиБ, содержимое с `</script>` отклоняется (400), CSP остаётся
network-free, отдельной подписи у runtime нет (часть пакета плагина). Каждый
фрейм грузит runtime заново — это цена изоляции. Проверено на React 18.3.1:
рендер, `useState`-интерактивность и `ctx.onTheme` работают (раунд 77).

### Host HTTP-маршруты плагина (C1)

Host-половина может отдавать свои HTTP-маршруты — так реальные плагины получают
данные (например, карточка баланса ходит в свой `/dsh-balance/deepseek`):

```js
export function apply(ctx) {
  ctx.inject(["webServer"], (scope) => {
    scope.webServer.register({
      name: "my-route",
      kind: "exact",
      path: "/my-plugin/data",
      handler: async (req, res) => {
        res.writeHead(200, { "content-type": "application/json" });
        res.end(JSON.stringify({ ok: true }));
      },
    });
  });
}
```

Как это устроено в dshr:

- sidecar поднимает **loopback-only** HTTP-сервер (`127.0.0.1`, случайный порт) с
  per-process токеном; bootstrap предоставляет `ctx.inject(['webServer'])`
  и `webServer.register({ name, kind, path, handler })` с обычными Node
  `(req, res)`; `credentials.resolve(name)` читает значение из окружения
  (ключи никогда не попадают в браузер);
- Rust-сервер проксирует **только пути из живого allowlist** (exact-match,
  `kind: "exact"`), не даёт зарегистрировать зарезервированные префиксы
  (`/v1`, `/dshr`, `/openapi`, UI-ассеты), ограничивает тело запроса 1 МиБ,
  ответ 8 МиБ и таймаут 15 с; неизвестный путь остаётся обычным 404 dshr;
- маршруты живые: reload/stop плагина убирает их из allowlist;
- для кадров с `host_modules` CSP открывает `connect-src` на origin сервера
  (внешняя сеть по-прежнему запрещена, `default-src 'none'`); кадры без
  `host_modules` остаются полностью network-free; CORS для opaque-origin кадров
  разрешён только на plugin-маршрутах (и preflight обрабатывается до прокси);
- `kind: "exact"` и `kind: "prefix"` (prefix матчит себя и детей, longest-prefix
  выигрывает; exact всегда важнее prefix); маршруты видны в
  `GET /v1/plugins/status` (`routes`), MCP-псевдо-инструменты
  (`list_resources`/`read_resource`/`list_prompts`/`get_prompt`) больше не
  попадают в реестр инструментов плагина;
- публичные plugin-маршруты по умолчанию **не** публикуются в LAN: если сервер
  слушает не loopback, прокси отвечает 403, пока явно не передан
  `--allow-lan-plugin-routes`.

**Сервисы `sessions`/`sessionProjections` (C1.1).** Сервер публикует в
`<home>/cordis/host.json` внутренний URL и per-run токен, а sidecar забирает
`GET /v1/internal/session-snapshot` (recent ≤50 сессий: id/title/model/turns/
steps/usage-события, без содержимого сообщений) и раз в 5 с обновляет снимок.
Из него bootstrap строит DSH-совместимые сервисы: `ctx.get('sessions')`
(`list()`/`get(id)`, объекты с `header.createdAt` и DSH-shaped `log`) и
`ctx.get('sessionProjections')` (`snapshot(session).values` с `sessionStats` и
`tokenUsage`). Реальные usage-маршруты плагинов теперь считают статистику по
сессиям dshr; данные могут отставать до интервала обновления (5 с), а проекция
упрощена (без `llmMs`/истории тредов).

Ограничения C1: нет SSE/WebSocket/стриминга и потоковых ответов через прокси;
поддерживаются `kind: "exact"` и `kind: "prefix"` (longest-prefix), других kind
нет. Plugin-маршруты не аутентифицированы (как в DSH), поэтому при bind вне
loopback сервер отвечает 403, пока явно не передан
`--allow-lan-plugin-routes`.

### webServer и configForms по reference-контракту (C1.4)

Host-сервисы выровнены по исходникам `deepseek-ai/deepseek-harness`:

- **webServer** (`packages/host/webserver`): exact и prefix — независимые
  таблицы; дубликат `(kind, path)` бросает `webserver: duplicate ... route`;
  путь обязан быть абсолютным pathname без завершающего `/`, `?` и `#`;
  `webServer.getOrigin()`/`port`/`protocol` отдают origin dshr-сервера из
  host-bridge; `registerFallback`/`registerUpgrade` в sidecar не поддержаны и
  бросают явную ошибку (записано как долг, реальным плагинам из набора не
  нужны);
- **ctx.baseUrl**: как loader-context reference — `file://` URL корня
  пакета плагина (host-половины вроде `dsh-plugin-studio` читают его для
  резолва путей);
- **configForms** (`packages/client/ui-settings/src/client/config-form-types.ts`):
  snapshot ровно `{status, value, base, user, revision: number, writable}` со
  стабильной идентичностью между чтениями, `mutate(ops, expectedRevision?)` с
  атомарными `set`/`unset`, `set`/`unset` хелперы, числовой revision-fence.
  В кадре форма in-memory (правки не переживают reload), host-половинам —
  файловая под `<home>/cordis/config-forms/`.

### Примитивы портированы из reference (C2.1)

`@deepseek-ai/dsh-client-ui-primitives` больше не «похожий шим»: компоненты
переписаны по исходникам `deepseek-ai/deepseek-harness @ 39163dd`
(`packages/client/ui-primitives/src`) — совпадают DOM-структура, props, ARIA и
поведение:

- `Button` (variant `primary`/`ghost`/`outline`/`toolbar`, size `md`/`sm`,
  leading `icon`, `forwardRef`), `Input` (wrapper + leading icon),
  `Switch` (role switch, `aria-label`, thumb без видимого текста),
  `Checkbox` (native input + label, `data-modal-autofocus`), `Tag`
  (`data-tone`: outline/solid/neutral/quiet/success/info/warning/danger),
  `Pill` (span/button по наличию `onClick`, `active`);
- `SegmentedControl` (tablist с indicator, roving tabindex, walk-клавиши,
  `<id>-<value>`/`-panel`) и `SegmentedTabs` (Pill-табы, grid, indicator,
  Left/Right/Home/End с переносом фокуса);
- `DisclosureRow` (полная структура: header/row, expand-on-row-click,
  previewChevron, `collapsedContent`, `headerAccessory`, `running`-shimmer),
  `TextShimmer` (nested-context, decoration/sweep), `StateDot`
  (done/warning/ongoing/error/idle, SVG-spinner, `appearance: "step"`),
  `PathLabel` (directory/name split, clipped-маска, `title`);
- `Toast` (portal, `role=alert`, success check-circle, `holdMs` + fade,
  actions, anchor-центрирование), `Modal` (portal, mask, Escape, focus,
  warning tone, headless, footer), `MenuSurface` (forwardRef,
  `data-menu-material`/`phase`), `Menu`/`MenuItemButton` — **порт ядра**: data-
  и компонентные ряды, separators/labels, reserve-icon, selection
  `check`/`fill`, `danger`/`current`, disabled-reason через Tooltip,
  ShortcutKeys, footer-группа, `children`-ряды, submenu side-card/single-page
  layout (menu-submenu-position: сбоку, если помещается, иначе back-row-страница;
  hover/ArrowRight), клавиатурный walk
  (Arrow/Home/End, Escape с возвратом фокуса, Tab-семантика), outside-
  dismissal + iframe-blur, autoFocus, `copyOnPointerLeave` grace 200 мс,
  portal-позиционирование с `minWidthFromAnchor`/`anchorPoint`, dense/compact;
  `submenuBackLabel`/back-row; reference-выбор layout'а
  `menu-submenu-position.ts` (side-карточка, если помещается сбоку, иначе
  single-page с non-selecting back-row); hover-grace 200 мс между родительской
  и дочерней карточками; `MenuGroup`+`observeStickyMenuGroups` (sticky
  заголовки групп через Intersection/ResizeObserver) — портированы;
  `Tooltip` — **полный порт**: label-резолвер,
  hover/focus-триггеры с независимыми задержками, `openOnClick`-pinning,
  Escape/Tab/outside-dismiss, `onlyWhenClipped`, `maxWidth`, portal,
  nested-suppression, `ShortcutKeys`, детерминированный placement-engine
  (reference `tooltip-placement.ts`: side → opposite → perpendicular, viewport
  clamp) и `pointerModality` (reference `input-modality.ts`);
- **иконки**: все **204** reference-экспорта `Icon*` сгенерированы из
  upstream `icons/artwork.ts` — те же SVG-пути/stroke (regular 1, medium 1.3),
  инлайн вместо document-спрайта; для неизвестных `Icon*` остаётся Proxy-global.

`JsonBlock` портирован из reference (collapsible, cap 20 000 символов,
`truncatedLabel`). `MarkdownText` — API-совместимый dependency-free рендерер:
принимает `text`/`labels`/`variant`/`omitHtml`/`textInteractionSource`/
`fileMentions`/`pathImages`, рендерит GFM-подмножество (заголовки, абзацы,
списки, fenced code, blockquotes, hr, inline bold/italic/strike/code/links/
images с sanitize URL; raw HTML не рендерится), но **не** upstream-стек
mdast/micromark/shiki/KaTeX/footnotes. Остаются расширения dshr (не из
reference): `Spinner`, `useAnchoredPosition`. `writeClipboard` и `useDismissOnOutsidePointer`
портированы по reference-алгоритмам.

### Слот-контракт по референсному каталогу (C1.3, SlotMap-semantics)

Регистрация слотов теперь проверяется по **реальному каталогу DSH**, а не по
рукописному списку: `crates/cordis/src/slot-catalog.json` сгенерирован из
`deepseek-ai/deepseek-harness` (`packages/extensions/cordis-client-runner/
src/client/slot-catalog.ts`, коммит `39163dd`) — 122 слота с `kind`
(`single`/`list`/`keyed`/`chain`/`hoc`), `scope`
(`root`/`session`/`session-maybe`), framework standard-kit и флагом
slot-`inject`. Host-страница публикует его как `window.__DSHR_SLOT_META__`.

Правила регистрации повторяют reference `SlotCore`:

- неизвестный слот → `slot "<name>" is not declared` (исключение — dshr
  extension-пространства `panel`/`message.action`/`custom.*`/`x-*`/`dshr.*`);
- `keyed` требует `key`, `list`/`hoc` — `id`, `chain` — `select`;
- вторая регистрация в ту же ячейку (`single` — слот, `keyed` — key, иначе id)
  с **тем же `priority`** бросает ошибку; другой `priority` затеняет
  (рендерится меньший);
- регистрация может **объявлять дочерние слоты**:
  `children: { "my.slot": { kind, scope } }` — объявление живёт вместе с
  записью и каскадно снимается (вместе с регистрациями внутрь), это
  разблокировало собственные слоты `dsh-mnemon`;
- `ctx.slots.inject(key, factory)` ждёт **объявления** слота (reference
  semantics) и перезапускается при его появлении, а не просто «когда сервисы
  готовы»;
- shell получает только **победителей** ячеек: `single` — один, `keyed` —
  по одному на ключ, `list`/`chain`/`hoc` — все слои в порядке
  (priority → order → регистрация);
- props компонента собираются как в reference:
  `standard-kit` (hooks из каталога; нереализованные остаются вызываемыми и
  возвращают `undefined`, `sessionId` — из host-state) → seat
  `inject` → **ownerProps** (из `window.__DSHR_FRAME__.ownerProps`) → `t`.

Отличия от reference, зафиксированные как долг: `chain` не диспатчится по
`select`, а монтируется слоями; `hoc` не вкладывает `children` между кадрами;
store-seats (`PropsStore`/`BoundActions`) и mounted-model сессий не
реализованы; ownerProps пока передаются только через harness/расширения shell
(реальные shell-сайты вроде `conversation.chat.node` ещё не поставляют данные).

### Slot-level `inject`, generators и `configForms` (C1.2)

Реальные settings-плагины используют контракты слотов, которых не было в
C-lite:

- **seat `inject`**: `ctx.slots.register({ name, inject: () => ({...}) }, Component)`
  — возвращённые props добавляются компоненту при монтировании; если фабрика
  бросила (зависит от отсутствующего host-сервиса), компонент рендерится
  деградированно, а плагин не помечается упавшим (событие пишется в лог кадра);
- **Cordis-генераторы**: фабрики `slots.inject`, `ctx.inject` и `ctx.effect`
  могут быть `function*`; тело выполняется, все `yield`-disposer'ы
  сохраняются, а при teardown вызываются в обратном порядке. Это разблокировало
  `dsh-plugin-studio`, который регистрирует вкладки генератором;
- **`configForms`**: `ctx.get('configForms')`/`ctx.configForms` отдаёт
  `get(entryId)` с формой (`getSnapshot`, `subscribe`, `mutate(ops, revision)`).
  В кадре это in-memory writable-форма (карточка рендерится, переключается,
  `mutate` соблюдает revision-контракт); правки **не переживают reload кадра**.
  Host-половинам bootstrap дополнительно даёт файловый вариант
  (`<home>/cordis/config-forms/<entry>.json`).

Эффект: `dsh-plugin-studio` рендерит реальные вкладки в Settings
(`settings.plugins.tab`, свой CSS, честные error-состояния при отсутствии
host-маршрутов), а `dsh-plugin-debugging` — карточку настроек в Plugins
(`plugins.bundle.config`) с рабочим переключателем и сохранением в форме.

### Общая модульная таблица хоста (`dsh.client.host_modules`)

Вместо того чтобы вшивать React в каждый бандл, половина может попросить
**общий рантайм хоста** (тир B, [ADR-040](adr/040-react-client-half.md)):

```json
{ "dsh": { "client": {
  "platform": "web",
  "entry": "client.js",
  "host_modules": ["react", "react-dom/client"]
} } }
```

Доступные id: `react`, `react/jsx-runtime`, `react/jsx-dev-runtime`, `react-dom`,
`react-dom/client` (React 18.3.1, версия закреплена
`dshr_cordis::client::HOST_RUNTIME_VERSION`) и
`@deepseek-ai/dsh-client-ui-primitives` (частичный независимый шим
design-system, см. ниже). Незнакомый id отклоняется при discovery.

Как это работает:

- host-страница такого кадра добавляет до бандла один classic-скрипт
  `<script src="/v1/runtime/react-18.3.1.js" data-dshr-shared-runtime="1"></script>`;
  это **единственный** артефакт на страницу — браузер скачивает его один раз и
  кэширует (`Cache-Control: public, max-age=31536000, immutable`), а не получает
  ~140 КБ React внутри каждой host-страницы;
- рантайм публикует те же id в `window.__DSHR_MODULES__`, что и plugin-shipped
  runtime, поэтому `require("react")` / `require("react-dom/client")` работают в
  module-table бандлах без изменений; plugin-shipped `dsh.client.runtime` (если
  объявлен) выполняется после и может переопределить записи;
- JS-реалм у каждого iframe свой (изоляция), поэтому рантайм исполняется в каждом
  кадре; «одна версия на страницу» — это один версионированный артефакт и одна
  загрузка по сети, а не один общий JS-контекст;
- CSP такого кадра остаётся жёстким: `default-src 'none'`, внешняя сеть
  запрещена; из сети разрешены только origin сервера —
  `script-src ... <origin>` для рантайма и `connect-src <origin>`, иначе
  браузерный fetch плагина к его же C1-маршруту блокировался бы. Кадры без
  `host_modules` по-прежнему полностью network-free (`connect-src 'none'`).

Ограничения тира B: это React-рантайм и модульная таблица, но **не** полный
design-system `@deepseek-ai/dsh-client-ui-*`, не React-контекст шелла, не
композиция между плагинами и не SlotMap-семантика (`ownerProps`/framework-hooks) — базовый контракт слотов
уже референсный (C1.3), но chain-диспетчеризация, hoc-вложение и store-seats —
тир C-full. PoC проверен в smoke:
фикстура `hosted` не содержит React в бандле и рендерит интерактивный
React-компонент из общего рантайма.

### C-lite: React-компоненты слотов, host-hooks и design-system шим

Если у кадра есть host React (`host_modules` содержит `react`), мост переключается
на **референсный контракт рендера**: значение, переданное в
`ctx.slots.register(entry, Component)`, трактуется как React-компонент (функция
или класс), а не как imperative-колбэк `render(element, props)`. Компонент
получает props:

| Prop | Значение |
|---|---|
| `theme`, `wide` | текущая тема и признак широкого сайдбара (обновляются) |
| `slot`, `id`, `plugin`, `context` | как в imperative-режиме; `context` — из `mode=slot` |
| `useTheme()` | React-хук текущей темы |
| `useSession()` | React-хук текущей сессии (`{id, title}` или `null`) |
| `useSessions(selector?)` | React-хук списка сессий |
| `useHostState(selector?)` | React-хук всего host-state |

Host-state приходит от шелла сообщением `host-state` (`theme`, `wide`, `session`,
`sessions`) при `ready`, смене сессии/списка и темы; компонент перерисовывается
сам, без ремоунта кадра. Ошибка рендера ловится error boundary: в регионе
появляется `Plugin render error: …`, а шелл получает `error`.

Сервисы в мосте (C-lite; неизвестное имя — `undefined`):

- `ctx.get('sessions')` — `currentProvideInfo.getSnapshot()/subscribe`, `get(id)`,
  `list()` (подмножество референсного сервиса, без проекций);
- `ctx.get('slots')` / `ctx.slots` — реестр (`inject`/`register`/`remove`/`list`,
  `entries(key)`/`entriesOfSlot(region)`) плюс референсные
  `subscribe(region, listener)`/`getVersion(region)` для плагинов, которые
  сверяют собственные shadow-регистрации;
- `ctx.get('locale')` / `ctx.locale` — минимальный сервис с `getSnapshot()` →
  `{active: "en", locale: "en"}`, `register(ns, dicts)` и `bind(ns)` (возвращает
  `t(key, params)`; шелл англоязычный для кадров плагинов);
- `ctx.inject(deps, callback)` — референсный fiber-хелпер: `callback(inner)`
  вызывается только если **все** зависимости резолвятся, иначе не вызывается
  (опциональные интеграции `betterSidebar`/`sidebarRight`/`webTerminals` и т.п.
  просто не монтируются — именно на такую деградацию рассчитаны DSH-плагины).

Последняя ошибка кадра доступна изнутри как `window.__DSHR_LAST_ERROR__`
(`{message, stack}`) — это упрощает разбор несовместимости реального плагина.

**Дизайн-система:** host-рантайм публикует
`@deepseek-ai/dsh-client-ui-primitives` — **частичный независимый шим**
(не пакет upstream; с раунда 93 атомы портированы из reference-исходников,
см. C2.1): `Button`, `Input`, `Switch`, `Checkbox`, `Tag`, `Pill`,
`Menu`, `MenuSurface`, `Modal`, `Toast`, `Tooltip`, `SegmentedControl`,
`SegmentedTabs`, `DisclosureRow`, `PathLabel`, `TextShimmer`, `StateDot`,
`Spinner`, `JsonBlock`, `MarkdownText`, `Icon` + именованные иконки
(`IconCloseOutlineRegular`, `IconChevronDownOutlineRegular`, `IconSearchOutlineRegular`,
`IconSettingsOutlineRegular`, `IconWarning*`, …; любое неизвестное `Icon*`-имя
резолвится в генерик-глиф), `writeClipboard`, хуки
`useAnchoredPosition`/`useDismissOnOutsidePointer` на host React и
CSS-переменных темы (шиму 30+ экспортов, C2). Этого хватает реальным плагинам
набора conformance; полный набор `@deepseek-ai/dsh-client-ui-*` — тир C-full.
Компоненты `React.memo`/`forwardRef` принимаются как тип слота; регистрации с
`key` без `id` получают уникальный id (`<slot>:<key>`), а
`ctx.get('slots').entries(key)` отдаёт референсную форму с `component`/`options`.
Локаль: `ctx.locale.register(ns, dicts)` + `t()` в props компонента
(английский словарь; шелл англоязычный). Таймеры: `ctx.interval`/`ctx.timeout`
снимаются вместе с фреймом.

**Conformance (n=11):** реальные MIT-плагины с upstream-клиентской половиной без
изменений (в манифесте только `host_modules`):

- `@kt11/dsh-deepseek-balance@0.2.1` — React-карточка в `sidebar.footer.action`,
  свой CSS, popup по клику; **host-половина настоящая** (не заглушка): её
  маршруты `/dsh-balance/deepseek` и `/dsh-usage/*` отдаются через C1-proxy.
  Без ключа карточка показывает собственную ветку `not-configured`, с ключом в
  окружении — реальный баланс; usage-маршруты, зависящие от сервисов сессий,
  пока отдают пустые данные (ограничение C1);
- `dsh-remote@1.1.2` — регистрирует React-слоты в `conversation.session.header.*`,
  `sidebar.workspaces.directoryFlow`; активируется без ошибок, опциональные
  сервисы (`betterSidebar`, `sidebarRight`, `webTerminals`) отсутствуют и
  интеграции не монтируются — штатная деградация;
- `@ruoyang/dsh-plugin-deepseek-balance@1.0.2` — плавающий бейдж в
  `shell.overlay` с `ctx.effect`/`ctx.interval`/`ctx.locale`;
- `dsh-genui@0.3.3` — тумблер GenUI в `conversation.input.left`, регистрации
  через `React.memo` и keyed-слот `conversation.chat.node`;
- `dsh-session-superagent@0.10.12` — composer dock (`conversation.composer.dock`);
- `dsh-mnemon@0.5.46` — крупный multi-region плагин; активируется и публикует
  слоты, сверяет собственные shadow-регистрации через
  `ctx.slots.subscribe/getVersion/entriesOfSlot`;
- `@99galaxy/dsh-plugin-market@1.2.2` — запись в sidebar; использует только
  `react`; `dsh-plugin-codegraph-panel@0.6.5` — панель в `settings.section`;
  `@goodandready/dsh-plugin-notify@0.4.9` — строки в `plugins.item` /
  `plugins.row.config`.

Все одиннадцать fixture лежат в `crates/cordis/tests/fixtures/` (sha256 в
`package.json`). Известные ограничения conformance-набора:
`@michengai/dsh-archive-manager` требует `unsafe-eval` в CSP (не включён),
`@michengai/dsh-skills-manager` падает на инициализации бандла (`Object.assign`
над undefined) — оба записаны в TECH-DEBT и не входят в n=11.

Чего не хватает и после тиров A и B: design-system пакетов
`@deepseek-ai/dsh-client-ui-*`, React-контекста и состояния шелла, композиции между
плагинами и SlotMap-семантики (`ownerProps`, framework-hooks, cardinality/scope).
Тир B даёт только React-рантайм и модульную таблицу (version pinned сервером);
полная React-совместимость — **тир C** по [ADR-040](adr/040-react-client-half.md).

Маршруты:

- `GET /v1/plugins/clients` — список половин (`name`, `package`, `platform`,
  `entry`, `bundle`, `immediately`, `inject`, `runtime`) и ошибки разбора;
- `GET /v1/plugins/{name}/host?token=…&theme=light|dark[&frame=…&mode=registry|slot&slot=…&id=…&context=…]`
  — host-страница для `<iframe sandbox="allow-scripts">`; токен нужен только для
  её выдачи. `frame` — непрозрачный токен шелла (эхо в сообщениях), `mode=slot` +
  `slot`/`id`/`context` — рендер одной регистрации; невалидные значения → 400.
  Если у половины есть `dsh.client.runtime`, его исходник встраивается в страницу
  перед бандлом;
- `GET /v1/plugins/{name}/asset/{capability}/{path}` — файлы пакета по capability
  (без bearer-токена; путь канонизируется и не выходит за пакет; ≤ 8 МиБ);
- `GET /v1/runtime/react-18.3.1.js` — общий модульный рантайм (React 18.3.1 +
  `bootstrap.js`, ~143 КБ); публичный, `immutable`-кэш; исходники и контрольные
  суммы — `crates/server/assets/runtime/` (`scripts/check-runtime-assets.sh`);
- `GET /v1/plugins/{name}/client.js?token=…` — бандл отдельным файлом для отладки;
- deep link `/?token=…&plugin=demo` открывает панель; страница подключается
  автоматически, когда токен есть в URL или localStorage.

Ограничения: React-компоненты не поддерживаются (ADR-013 §5) — бандл должен
экспортировать `default`/`apply`/`activate` или использовать шим
`__ModuleLoader__.load({ id, factory })` и сам строить DOM; `ownerProps`/hooks/
chain/hoc-семантика слотов DSH не воспроизводится — `props` ограничены
`{theme, slot, id, plugin, context}`; внешние модули (`dsh.client.external`) не
подключаются — бандл должен быть самодостаточным; произвольная инъекция в DOM
шелла недоступна (только регионы из таблицы).

### Установка npm-плагинов

```sh
dshr plugin npm add cordis --name cordis-core
dshr plugin npm add @scope/plugin@1.2.3
dshr plugin npm update [name]
dshr plugin npm remove name
dshr plugin npm list
```

Управление установленными Cordis-плагинами без удаления:

```sh
dshr plugin npm disable <name>
dshr plugin npm enable <name>
```

Поиск по реестру npm и метаданные:

```sh
dshr plugin npm search cordis
dshr plugin npm search cordis --limit 5 --json
dshr plugin npm info cordis
```

Пакет ставится в `<home>/cordis/packages/<profile>/<name>` (для `default` — без
подкаталога профиля); запись `plugins.cordis`
(entry — абсолютный путь к каталогу пакета, `enabled: true`) пишется в user config, а
при `--profile <name>` — в `<home>/profiles/<name>/profile.yaml`. Так же ведут себя
`plugin add/remove` для MCP.
Для `file:`, git- и URL-спеков имя пакета нужно указать через `--name`. После
установки перезапустите harness, чтобы sidecar подхватил плагин.

## Sidecar

Sidecar-плагины используют MCP (stdio или Streamable HTTP) — отдельный протокол не нужен:

```sh
dshr plugin add fs --command npx --arg -y --arg @modelcontextprotocol/server-filesystem --arg .
dshr plugin add remote --url https://mcp.example.com/mcp
```

Инструменты: `mcp__<server>__<tool>`, resources/prompts — как в [MCP](MCP.md).

## Ограничения

- Два ABI: legacy JSON core-модуль на wasmi и component model/WIT на wasmtime (оба — JSON через строку).
- WASM: нет hot reload (перезагрузка только рестартом), нет кэша грантов; memory cap, sha256 и Ed25519-подпись есть, но нет отзыва ключей и прозрачного журнала (CT-подобного).
- Cordis: hot reload есть (watcher + endpoint), capability-гранты есть; клиентская половина — iframe-мост с eager registry, региональным монтажом слотов (полный словарь DSH → 17 регионов, неизвестное имя → панель), capability-ассетами, host-действиями, `dsh.client.runtime` (свой UI-стек), общим рантаймом `dsh.client.host_modules` (React 18.3.1 с сервера) и C-lite-совместимостью: React-компоненты слотов с host-hooks/`ctx.get('sessions')`, частичный design-system шим, один реальный DSH-плагин в conformance; полный design-system, React-контекст шелла и SlotMap-семантика (`ownerProps`/framework-hooks) остаются тиром C-full по [ADR-040](adr/040-react-client-half.md).
- Sidecar не имеет отдельного manifest-формата — конфиг MCP.
