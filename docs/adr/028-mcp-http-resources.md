# 028. MCP: Streamable HTTP и resources

- **Статус:** Accepted
- **Дата:** 2026-11-13

## Контекст

M4 закрыл только stdio-tools. Для удалённых MCP-серверов и чтения ресурсов нужен HTTP-транспорт и методы `resources/*`, иначе harness остаётся локальным.

## Решение

1. **Транспорт выбирается по `url`:** если он задан — Streamable HTTP, иначе stdio.
2. **HTTP-клиент:** POST JSON-RPC, `Accept: application/json, text/event-stream`, разбор JSON и SSE, заголовок `Mcp-Session-Id`, произвольные headers.
3. **Единый backend:** `McpBackend::{Stdio, Http}`; адаптеры инструментов и ресурсов не зависят от транспорта.
4. **Resources и prompts:** для каждого сервера регистрируются `list_resources`, `read_resource`, `list_prompts`, `get_prompt`; текстовые contents склеиваются, binary помечается.
5. **Плагины:** `plugin add --url` пишет HTTP-сервер в конфиг.
6. **Sampling:** stdio-клиент обрабатывает `sampling/createMessage` во время ожидания ответа и вызывает провайдера; HTTP sampling нет. Полностью асинхронный server-initiated цикл — нет.
7. **Границы:** нет GET-SSE, elicitation, OAuth.

## Реализация HTTP sampling

GET-SSE поток открывается при наличии провайдера; `sampling/createMessage` обрабатывается и отправляется POST-ом. OAuth client-credentials реализован; elicitation и refresh не реализованы.

## Последствия

- Удалённые MCP-серверы и ресурсы доступны без отдельного SDK.
- HTTP-клиент не переподключается автоматически; ошибка сессии видна инструменту.
- MCP остаётся недоверенным: таймауты, лимиты вывода, approvals.

## Альтернативы

- Использовать `rmcp` — отложено: текущий stdio-клиент уже покрывает протокол, HTTP добавлен точечно.
- GET-SSE для server-initiated сообщений — отложено: не требуется для tools/resources.
