# HTTP/SSE server

```sh
dshr serve --bind 127.0.0.1 --port 4317 --rate-limit 60
```

- Bearer auth: `Authorization: Bearer <token>`; токен генерируется, если не задан, и печатается при старте.
- `/v1/health` не требует auth и не лимитируется.
- Rate limit: `--rate-limit N` или `DSHR_RATE_LIMIT` — N запросов/мин **на bearer-токен**; `0` отключает. Неавторизованные запросы делят bucket `anonymous`.
- При превышении: `429`, `Retry-After: 60`, JSON `{"error":"rate limit exceeded"}`.
- Счётчик — фиксированное окно 60 с в памяти процесса; несколько инстансов не координируются.

## API

`GET /openapi.json` — OpenAPI 3.1 спецификация.

`/v1/sessions`, `/v1/sessions/{id}`, `/v1/sessions/{id}/turns`, `/v1/sessions/{id}/events` (SSE), `/v1/sessions/{id}/cancel`, approvals, `/v1/rpc`.

`/v1/config/model` (GET/POST) — активный маршрут/модель/base URL и reasoning effort (`reasoning_effort`: пустое значение убирает ключ, непустое обязано быть объявленным уровнем модели, иначе `400`).

`/v1/config/providers` (GET) — каталог провайдеров, сохранённые профили и
активный выбор из `<home>/models.yaml`; блок `active` содержит `provider`,
`model` и `reasoning_effort` (уровень активной модели или `null`), а элементы
моделей — `reasoning_efforts` (объявленные уровни) и каталоговые
`reasoning_levels` (см. [CONFIG](CONFIG.md#models-провайдеры-и-списки-моделей)).
`POST /v1/credentials/describe` (`references`) — состояние и маска до 64
ключей (без значений); `PUT /v1/credentials/{reference}`
(`{value, expected_revision}`) — write-only запись, `DELETE` с ревизией.
`POST /v1/config/providers/welcome` (`version`) — подтверждение first-run
notice (durable, без revision-fence).
`POST /v1/config/providers/discover` (`protocol`, `base_url`, `api_key_env`) —
спрашивает endpoint о списке моделей (`data`/`models`, OpenAI и
Anthropic-форматы) и возвращает кандидатов; ключ читается только из окружения.
`PUT /v1/config/providers/{id}` заменяет профиль (`expected_revision`,
`protocol`, `base_url`, `api_key_env`, `models`, `retry`), `DELETE` с
`?expected_revision=N` возвращает каталог по умолчанию; устаревшая ревизия —
`409`, неизвестный маршрут — `404`/`400`.
