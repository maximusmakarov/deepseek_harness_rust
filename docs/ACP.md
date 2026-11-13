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
Инструменты выполняются локально (файлы/шелл), approvals **автоматические** (`AutoApprover`), поэтому ACP-клиент не запрашивает разрешения. Для ограничения используйте `--danger` только осознанно; без него действует `workspace-write` + sandbox.

Не поддержано: `session/load`, `fs/*`, `terminal/*`, `session/request_permission`, изображения/аудио, MCP-серверы от клиента.

Ошибки — стандартный JSON-RPC error; busy session → `-32000`.
