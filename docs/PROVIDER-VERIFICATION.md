# Протокол DeepSeek: проверенные факты

Дата проверки: 12.11.2026 · статус: **пройдено** · живой endpoint

Документ фиксирует проводной протокол DeepSeek, который реализует `dshr-llm` в M1. Ключи и токены в репозиторий не попадают: тесты работают на кассетах, живой smoke — по переменной окружения.

---

## 1. Транспорт и аутентификация

| Параметр | Значение |
|---|---|
| Base URL | `https://api.deepseek.com/anthropic` |
| Протокол | Anthropic Messages (`/v1/messages`) |
| Заголовок версии | `anthropic-version: 2023-06-01` |
| Аутентификация | API-key (`Authorization: Bearer` / `x-api-key`) либо account-токен совместимого клиента (`x-dsh-auth-token`) |

Каталог моделей: `GET https://api.deepseek.com/models` с тем же auth.

---

## 2. Проверенные запросы

### 2.1. Каталог моделей

```http
GET https://api.deepseek.com/models
x-dsh-auth-token: <token>
User-Agent: dshr/0.1
```

Результат: `HTTP 200`, 2 модели:

```json
{
  "object": "list",
  "data": [
    { "id": "deepseek-flash",  "name": "DeepSeek-V4.1-Flash", "object": "model" },
    { "id": "deepseek-v4-pro", "name": "DeepSeek-V4-Pro",    "object": "model" }
  ]
}
```

Контекст обеих моделей по документации провайдера — 1 048 576 токенов.

### 2.2. Обычный ответ

```http
POST https://api.deepseek.com/anthropic/v1/messages
x-dsh-auth-token: <token>
anthropic-version: 2023-06-01
content-type: application/json

{"model":"deepseek-flash","max_tokens":32,
 "messages":[{"role":"user","content":"Reply with the single word: pong"}]}
```

Результат: `HTTP 200`, `stop_reason: end_turn`, блоки:

```json
{"content":[
  {"type":"thinking","thinking":""},
  {"type":"text","text":"pong"}
],
 "usage":{"input_tokens":38,"output_tokens":18,
          "cache_creation_input_tokens":0,"cache_read_input_tokens":0,
          "service_tier":"standard"}}
```

Вывод: это **Anthropic Messages API**, а не OpenAI Chat Completions. В ответе присутствует блок `thinking`, usage содержит cache-поля.

### 2.3. Стриминг

Тот же endpoint с `"stream": true`. Пришли SSE-события (8549 байт):

```text
message_start (1)
content_block_start (2)
ping (1)
content_block_delta (59)
content_block_stop (2)
message_delta (1)
message_stop (1)
```

Текст собрался в `1,2,3`; блоки — `thinking` и `text`. Парсер обязан игнорировать `ping` и не считать его концом потока.

### 2.4. Tool call и round-trip

Инструмент:

```json
{"name":"get_weather","description":"Get current weather for a city",
 "input_schema":{"type":"object","properties":{"city":{"type":"string"}},"required":["city"]}}
```

Запрос 1 → `stop_reason: tool_use`, блоки `thinking` + `tool_use` (`get_weather`, `{"city":"Paris"}`, id длиной 35).

Запрос 2 с историей `user → assistant(tool_use) → user(tool_result)` → `HTTP 200`, `stop_reason: end_turn`, текст:

> «The weather in Paris right now is 18°C with light rain…»

Вывод: полный цикл tool_use → tool_result работает; `tool_result.content` может быть строкой.

---

## 3. Что это меняет в M1

| Было в плане | Стало после проверки |
|---|---|
| DeepSeek-адаптер как OpenAI-compatible | Основной адаптер — **Anthropic Messages** (`/anthropic/v1/messages`, `anthropic-version: 2023-06-01`) |
| Только `Authorization: Bearer` | Два режима аутентификации: API-key и account-токен (`x-dsh-auth-token`) |
| `ContentBlock`: text/image/tool_call/tool_result/reasoning | Добавить `thinking` (reasoning) как отдельный блок; `tool_use`/`tool_result` — нативные имена провода |
| SSE: delta-события | Учитывать `message_start`, `content_block_start/delta/stop`, `message_delta`, `message_stop`, `ping` |
| Usage: prompt/completion | Добавить `cache_creation_input_tokens`, `cache_read_input_tokens`, `service_tier` |
| Каталог моделей — статический | `GET /models` с тем же auth; кэш с TTL, fallback на встроенный каталог |

Транспорт OpenAI-совместимого формата остаётся нужен для сторонних шлюзов — он реализуется вторым адаптером, а не заменяет DeepSeek.

---

## 4. Условия проверки

- Проверка выполнена на живом endpoint 12.11.2026; примеры выше воспроизводятся на совместимом доступе.
- В репозитории нет ключей и токенов; кассеты не содержат заголовков авторизации.
- Живой smoke-тест запускается только при наличии переменной окружения и не входит в CI.
