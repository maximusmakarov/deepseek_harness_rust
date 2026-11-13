# ACP (Agent Client Protocol)

`dshr acp` — ACP v1 сервер поверх stdio JSON-RPC.

```sh
dshr acp --danger
# или
dshr acp --cassette test.json --model deepseek-flash
```

Поддержано:

| Метод | Поведение |
|---|---|
| `initialize` | protocolVersion 1, capabilities, agentInfo |
| `session/new` | создаёт сессию, возвращает `sessionId` |
| `session/prompt` | запускает turn, стримит `session/update`, возвращает `stopReason` |
| `session/cancel` | отменяет активный turn |

Уведомления: `agent_message_chunk`, `tool_call`, `tool_call_update`.
Инструменты выполняются локально. При `workspace-write` опасные вызовы (shell, запись вне workspace) отправляют клиенту `session/request_permission`; клиент выбирает `allow-once`/`reject-once`. Без ответа через 300 с — отказ. `--danger` отключает запросы.

Не поддержано: `session/load`, `fs/*`, `terminal/*`, изображения/аудио, MCP-серверы от клиента.

`session/request_permission` реализован как запрос от агента клиенту; ответ `selected` + `optionId=allow-once` разрешает вызов.

Ошибки — стандартный JSON-RPC error; busy session → `-32000`.
