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
      oauth:
        token_url: "https://auth.example.com/token"
        client_id: "dshr"
        client_secret: "${CLIENT_SECRET}"
        scope: "mcp"
```

`${TOKEN}` не раскрывается автоматически — подставьте значение через переменную окружения или конфиг-профиль. Клиент отправляет `Accept: application/json, text/event-stream`, поддерживает JSON и SSE-ответы, запоминает `Mcp-Session-Id`.

## CLI

```sh
dshr plugin add filesystem --command npx --arg -y --arg @modelcontextprotocol/server-filesystem --arg .
dshr plugin add remote --url https://mcp.example.com/mcp
dshr plugin list
```

## Ресурсы и prompts

- `mcp__<server>__list_resources` — список `uri/name/description`.
- `mcp__<server>__read_resource` — чтение по `uri` (текстовые contents; binary помечается).
- `mcp__<server>__list_prompts` — список prompts и их аргументов.
- `mcp__<server>__get_prompt` — получить prompt по имени (`name`, `arguments`) и вернуть сообщения.

## Sampling

Сервер может запросить генерацию через `sampling/createMessage` по stdio или HTTP. Для HTTP клиент держит GET-SSE поток и отвечает POST-ом; capability объявляется только при наличии провайдера. Harness вызывает модель и возвращает:

```json
{"role":"assistant","content":{"type":"text","text":"..."},"model":"...","stopReason":"endTurn"}
```

HTTP sampling работает через GET-SSE вне клиентского вызова; stdio — во время ожидания ответа.

## Границы

- Нет elicitation; OAuth client-credentials поддерживается.
- OAuth: client-credentials, токен запрашивается при подключении; refresh/expiry не реализованы.
- GET-SSE используется для HTTP sampling; прочие server-notifications игнорируются.
- Prompts поддерживаются для stdio и HTTP.
- HTTP-сессия живёт в памяти клиента; реконнект — новый `initialize`.
- MCP-серверы запускаются как обычные процессы/соединения без собственного sandbox.
