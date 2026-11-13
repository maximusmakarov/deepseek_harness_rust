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

Файлы: если клиент объявил `clientCapabilities.fs.readTextFile`/`writeTextFile`, инструменты `read`/`write` идут через `fs/read_text_file`/`fs/write_text_file`; иначе используется локальный fallback.

Терминал: при `clientCapabilities.terminal=true` инструмент `bash` выполняет `terminal/create` → `wait_for_exit` → `output` → `release`; иначе локальный sandbox.

Сессии: `session/load` объявлен (`loadSession: true`) и проигрывает историю `user_message_chunk`/`agent_message_chunk`; `cwd` должен совпадать с сохранённым. `session/resume` открывает сессию без replay; `session/close` отменяет активный turn и идемпотентен.
Инструменты выполняются локально. При `workspace-write` опасные вызовы (shell, запись вне workspace) отправляют клиенту `session/request_permission`; клиент выбирает `allow-once`/`reject-once`. Без ответа через 300 с — отказ. `--danger` отключает запросы.

Изображения и аудио: блоки `image` и `audio` принимаются; блок `resource_link` (обязательный baseline ACP) превращается в текстовый блок с именем, URI, mime и описанием; встроенный `resource` не поддержан (`embeddedContext=false`). `promptCapabilities.image=true`, `promptCapabilities.audio` равен возможностям провайдера (включён для OpenAI-совместимых аудио-моделей, выключен для DeepSeek/Anthropic wire). Аудио-блок при провайдере без поддержки отклоняется с `-32602`, а не теряется молча.

MCP-серверы из `session/new`: поддерживаются **HTTP** (`type:http`, `url`, `headers`) и **stdio** (`command`, `args`, `env`); их инструменты доступны в turn. Транспорт SSE не поддержан. Capabilities объявляют `mcpCapabilities.http=true`, `mcpCapabilities.sse=false`.

`session/request_permission` реализован как запрос от агента клиенту; ответ `selected` + `optionId=allow-once` разрешает вызов.

Ошибки — стандартный JSON-RPC error; busy session → `-32000`.
