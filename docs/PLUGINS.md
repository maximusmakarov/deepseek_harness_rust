# Плагины: WASM и sidecar

## WASM

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

## Sidecar

Sidecar-плагины используют MCP (stdio или Streamable HTTP) — отдельный протокол не нужен:

```sh
dshr plugin add fs --command npx --arg -y --arg @modelcontextprotocol/server-filesystem --arg .
dshr plugin add remote --url https://mcp.example.com/mcp
```

Инструменты: `mcp__<server>__<tool>`, resources/prompts — как в [MCP](MCP.md).

## Ограничения

- WASM ABI кастомный (JSON), не component model/WIT; wasmtime не используется.
- Нет hot reload и capability-grants; memory cap, sha256 и Ed25519-подпись есть, но нет отзыва ключей и прозрачного журнала (CT-подобного).
- Sidecar не имеет отдельного manifest-формата — конфиг MCP.
