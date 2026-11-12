# 010. DeepSeek official — Anthropic Messages

- **Статус:** Accepted
- **Дата:** 2026-11-12

## Контекст

Живая проверка 12.11.2026 показала: `POST https://api.deepseek.com/anthropic/v1/messages`, auth `x-dsh-auth-token` (account) или API-key; ответы содержат `thinking`.

## Решение

Основной DeepSeek-адаптер реализует Anthropic Messages wire и два режима auth; OpenAI-совместимый адаптер — отдельный, для RouterAI/шлюзов.

## Последствия

Точное соответствие провайдеру; нужен Anthropic-совместимый транспорт, а не только OpenAI.

## Альтернативы

- Оставить как в DSH Desktop — отклонено: другой рантайм и/или несовместимый ABI.
- Отложить решение — отклонено: блокирует M0/M1.
