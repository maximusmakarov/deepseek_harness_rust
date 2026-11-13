# MCP

Поддерживаются два транспорта: stdio и Streamable HTTP. Инструменты получают имена `mcp__<server>__<tool>`; для каждого сервера также регистрируются `mcp__<server>__list_resources` и `mcp__<server>__read_resource`.

## stdio

```yaml
mcp:
  servers:
    - name: filesystem
      command: npx
      args: ["-y", "@modelcontextprotocol/server-filesystem", "."]
```

## HTTP

```yaml
mcp:
  servers:
    - name: remote
      url: "https://mcp.example.com/mcp"
      headers:
        Authorization: "Bearer ${TOKEN}"
```

`${TOKEN}` не раскрывается автоматически — подставьте значение через переменную окружения или конфиг-профиль. Клиент отправляет `Accept: application/json, text/event-stream`, поддерживает JSON и SSE-ответы, запоминает `Mcp-Session-Id`.

## CLI

```sh
dshr plugin add filesystem --command npx --arg -y --arg @modelcontextprotocol/server-filesystem --arg .
dshr plugin add remote --url https://mcp.example.com/mcp
dshr plugin list
```

## Ресурсы

- `mcp__<server>__list_resources` — список `uri/name/description`.
- `mcp__<server>__read_resource` — чтение по `uri` (текстовые contents; binary помечается).

## Границы

- Нет GET-SSE потока, prompts, sampling, elicitation и OAuth.
- HTTP-сессия живёт в памяти клиента; реконнект — новый `initialize`.
- MCP-серверы запускаются как обычные процессы/соединения без собственного sandbox.
