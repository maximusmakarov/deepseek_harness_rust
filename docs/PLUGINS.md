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
node, `DSHR_CORDIS_ROOT` — корень (ставится автоматически). Горячей замены,
кэша грантов и клиентской UI-половины пока нет.

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
- Нет hot reload и capability-grants; memory cap, sha256 и Ed25519-подпись есть, но нет отзыва ключей и прозрачного журнала (CT-подобного).
- Sidecar не имеет отдельного manifest-формата — конфиг MCP.
