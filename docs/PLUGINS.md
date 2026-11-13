# Плагины: WASM и sidecar

## WASM

Плагин — `.wasm` + YAML-манифест в `~/.dshr/plugins` или `<cwd>/.dshr/plugins`.

```yaml
name: echo
description: Echo plugin
wasm: echo.wasm
```

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
- fuel-лимит 10M инструкций;
- инструменты получают имена `wasm__<plugin>__<tool>`;
- модуль инстанцируется на каждый вызов (изоляция, но без кэша состояния).

## Sidecar

Sidecar-плагины используют MCP (stdio или Streamable HTTP) — отдельный протокол не нужен:

```sh
dshr plugin add fs --command npx --arg -y --arg @modelcontextprotocol/server-filesystem --arg .
dshr plugin add remote --url https://mcp.example.com/mcp
```

Инструменты: `mcp__<server>__<tool>`, resources/prompts — как в [MCP](MCP.md).

## Ограничения

- WASM ABI кастомный (JSON), не component model/WIT; wasmtime не используется.
- Нет hot reload, capability-grants, подписи и памяти-лимита кроме fuel.
- Sidecar не имеет отдельного manifest-формата — конфиг MCP.
