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
