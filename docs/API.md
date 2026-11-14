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
| GET | `/v1/tools` | — | `{tools: [{name, description, input_schema}]}` |
| GET | `/v1/plugins/search` | `?q=` | `{packages: [{name, version, description}]}` (npm registry) |
| GET | `/v1/plugins/info` | `?name=` | npm metadata (version/description/homepage/repository) |
| GET | `/v1/plugins/catalog` | `?q=&category=&limit=` | `{packages: [{name, description, category, tags, verified}], total, categories}` |
| GET | `/v1/plugins/catalog/{name}` | — | `{package, rating, reviews: {average, count}, installed}` |
| GET | `/v1/plugins/ratings` | — | `{ratings: {name: 1..5}}` |
| POST | `/v1/plugins/rate` | `{name, rating}` | `{ok}` (сохраняет локальную оценку) |
| GET | `/v1/plugins/reviews` | `?name=` | `{average,count,reviews}` или `{plugins:{name:{average,count,reviews}}}` |
| POST | `/v1/plugins/review` | `{name,rating,text}` | `{ok,average,count}` (локальный отзыв ≤2000) |
| POST | `/v1/plugins/install` | `{name}` | `{ok, output}` — запускает `plugin npm add` (bearer-token) |
| GET | `/v1/plugins/installed` | — | `{plugins: [{name, enabled}]}` |
| POST | `/v1/plugins/toggle` | `{name, enabled}` | `{ok, output}` (enable/disable) |
| POST | `/v1/plugins/remove` | `{name}` | `{ok, output}` |
| GET | `/v1/config/model` | — | `{provider, active_provider, model, base_url, routes, note}` |
| POST | `/v1/config/model` | `{provider, model, base_url?}` | `{ok, restart_required, provider, model}` (пишет user-слой `config.yaml`, 0600; ключи не принимает) |
| GET | `/v1/sessions` | — | `{sessions: SessionSummary[]}` |
| POST | `/v1/sessions` | `{title?}` | `{session: SessionHeader}` |
| GET | `/v1/sessions/{id}` | — | `{session, messages}` |
| POST | `/v1/sessions/{id}/turns` | `{prompt}` | `{accepted: true}` (при `session.auto_title` заголовок генерируется в фоне) |
| GET | `/v1/sessions/{id}/events` | SSE | поток `ServerEvent` |
| POST | `/v1/sessions/{id}/cancel` | `{}` | `{cancelled}` |
| POST | `/v1/sessions/{id}/approvals/{approval_id}` | `{approved}` | `{resolved}` |
| POST | `/v1/sessions/{id}/questions/{question_id}` | `{answer}` | `{resolved}` |

`ServerEvent` (JSON, поле `type`): `text`, `tool_start`, `tool_end`, `approval`, `question`, `done`, `cancelled`, `error`.

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

`GET /` отдаёт Leptos CSR-приложение: список сессий, чат со стримом, карточки инструментов, approvals, панель Plugins (tools + поиск npm), переключатель темы light/dark
(localStorage, `?theme=light`) и версия сервера. Исходник — `crates/web` (trunk + wasm-bindgen), собранный `dist/` встроен в сервер. Пересборка: `cd crates/web && trunk build --release`.
