# Телеметрия (OpenTelemetry)

Трейсы экспортируются по OTLP/HTTP, если задан endpoint. Метрики и логи OTel не включены.

```sh
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318 \
OTEL_SERVICE_NAME=dshr \
RUST_LOG=info \
dshr run "hello"
```

Поддерживается стандартный env:

- `OTEL_EXPORTER_OTLP_ENDPOINT` или `OTEL_EXPORTER_OTLP_TRACES_ENDPOINT` — включает экспорт;
- `OTEL_SERVICE_NAME` — имя сервиса (по умолчанию `dshr`);
- `RUST_LOG` — фильтр.

Спаны:

- `agent.turn` — `session_id`, `provider`;
- `tool.execute` — `tool`.

JSON-логи по-прежнему идут в stderr. При завершении CLI вызывается `provider.shutdown()` для flush.

Ограничения:

- только traces (без metrics/logs);
- нет семплирования/атрибутов модели/токенов;
- при недоступном коллекторе экспорт логируется, работа не блокируется;
- содержимое сообщений/промптов в атрибуты не попадает.
