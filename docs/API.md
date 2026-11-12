# HTTP API

`dshr serve` поднимает HTTP/SSE-сервер и встроенный Web UI на одном порту.

```sh
dshr serve --bind 127.0.0.1 --port 55678
# token печатается при старте; можно задать DSHR_API_TOKEN
```

Аутентификация — `Authorization: Bearer <token>`. Для SSE допускается `?token=<token>`, потому что `EventSource` не умеет задавать заголовки. `/v1/health` публичный.

## REST

| Метод | Путь | Тело / параметры | Ответ |
|---|---|---|---|
| GET | `/v1/health` | — | `{status, version}` |
| GET | `/v1/sessions` | — | `{sessions: SessionSummary[]}` |
| POST | `/v1/sessions` | `{title?}` | `{session: SessionHeader}` |
| GET | `/v1/sessions/{id}` | — | `{session, messages}` |
| POST | `/v1/sessions/{id}/turns` | `{prompt}` | `{accepted: true}` |
| GET | `/v1/sessions/{id}/events` | SSE | поток `ServerEvent` |
| POST | `/v1/sessions/{id}/cancel` | `{}` | `{cancelled}` |
| POST | `/v1/sessions/{id}/approvals/{approval_id}` | `{approved}` | `{resolved}` |

`ServerEvent` (JSON, поле `type`): `text`, `tool_start`, `tool_end`, `approval`, `done`, `cancelled`, `error`.

Пример:

```sh
TOKEN=... BASE=http://127.0.0.1:55678
SID=$(curl -s -X POST $BASE/v1/sessions -H "Authorization: Bearer $TOKEN" \
  -H 'content-type: application/json' -d '{}' | jq -r .session.id)
curl -s -X POST $BASE/v1/sessions/$SID/turns -H "Authorization: Bearer $TOKEN" \
  -H 'content-type: application/json' -d '{"prompt":"hello"}'
curl -N "$BASE/v1/sessions/$SID/events?token=$TOKEN"
```

## JSON-RPC

`POST /v1/rpc` принимает JSON-RPC 2.0. Реализованы методы:

- `session.list` → `{sessions}`
- `session.get` `{id}` → `{session, messages}`
- `session.create` `{title?}` → `{session}`

Ошибки — стандартный объект `error` с кодами `-32601` (метод не найден), `-32602` (параметры), `-32000` (внутренняя ошибка).

## Web UI

`GET /` отдаёт встроенную страницу: список сессий, чат со стримом, карточки инструментов, approvals. Это vanilla JS без сборки; Leptos-версия отложена.
