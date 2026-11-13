# 033. OpenTelemetry: OTLP/HTTP traces

- **Статус:** Accepted
- **Дата:** 2026-11-13

## Контекст

Нужна наблюдаемость в распределённых сценариях (server, workflows, teams), но CLI не должен требовать коллектор.

## Решение

1. **Opt-in:** OTLP включается только при `OTEL_EXPORTER_OTLP_ENDPOINT`/`..._TRACES_ENDPOINT`.
2. **OTLP/HTTP + protobuf** через `opentelemetry-otlp`; batch exporter.
3. **tracing layer:** `tracing-opentelemetry` + JSON fmt в stderr.
4. **Спаны:** `agent.turn` (`session_id`, `provider`) и `tool.execute` (`tool`).
5. **Завершение:** `SdkTracerProvider::shutdown()` после команды.
6. **Границы:** только traces; метрики, логи OTel, семплирование и токен-атрибуты не включены.

## Последствия

- Без env поведение и зависимости CLI не меняются (сеть не трогается).
- Атрибуты не содержат промптов/секретов.
- При недоступном коллекторе ошибки экспорта не ломают команду.

## Альтернативы

- OTLP/gRPC — отложено: HTTP проще для локального коллектора.
- Собственный JSON-трейс — отклонено: OTel совместим с Jaeger/Tempo.
