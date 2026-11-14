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
`tools`, `started_at`, `error`. Ошибка старта запоминается в статусе и
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

Маршруты:

- `GET /v1/plugins/clients` — список половин (`name`, `package`, `platform`,
  `entry`, `bundle`, `immediately`, `inject`) и ошибки разбора;
- `GET /v1/plugins/{name}/host?token=…&theme=light|dark[&frame=…&mode=registry|slot&slot=…&id=…&context=…]`
  — host-страница для `<iframe sandbox="allow-scripts">`; токен нужен только для
  её выдачи. `frame` — непрозрачный токен шелла (эхо в сообщениях), `mode=slot` +
  `slot`/`id`/`context` — рендер одной регистрации; невалидные значения → 400;
- `GET /v1/plugins/{name}/asset/{capability}/{path}` — файлы пакета по capability
  (без bearer-токена; путь канонизируется и не выходит за пакет; ≤ 8 МиБ);
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
- Cordis: hot reload есть (watcher + endpoint), capability-гранты есть; клиентская половина — iframe-мост с eager registry, региональным монтажом слотов (полный словарь DSH → 17 регионов, неизвестное имя → панель), capability-ассетами и host-действиями; React-компоненты и `ownerProps`/hooks-семантика слотов не поддерживаются.
- Sidecar не имеет отдельного manifest-формата — конфиг MCP.
