# Порт секции Models (`ui-settings-models`)

Эталон поведения: upstream `deepseek-harness@39163dd`, пакет
`packages/client/ui-settings-models` (README и 14 тыс. строк React/TS).
Цель — перенести пользовательскую поверхность в нативный web-UI `dshr`, а не
копировать React-код: у `dshr` нет React-контекста шелла, settings-движка DSH
и каталога провайдеров, поэтому эквиваленты строятся на Rust/Leptos.

## Что уже есть (раунды 140–148)

- Каталог из пяти провайдеров (`deepseek-account`, `deepseek-official`,
  `openai`, `openai-compatible`, `routerai`; алиас `deepseek`) с протоколом,
  endpoint'ом по умолчанию, env-переменными base URL и ключа, сводкой и
  стартовым списком моделей.
- Документ профилей `<home>/models.yaml`: `version`, монотонный `revision`,
  `providers` (protocol/base_url/api_key_env/models), атомарная запись 0600,
  revision-fence (`409`), валидация всех полей.
- API: `GET /v1/config/providers`, `PUT /v1/config/providers/{id}` (полная
  замена), `DELETE …?expected_revision=N`; `POST /v1/config/model` принимает
  кастомный маршрут с профилем и проверяет модель по списку.
- Web UI: строки провайдеров (точка ключа, теги active/defaults/protocol),
  отдельная карточка редактора (base URL, имя переменной ключа, список моделей
  с id/именем/контекстом/max output/input-типами и reasoning-уровнями),
  добавление из каталога, кастомный провайдер, активация, удаление профиля с
  подтверждением, сохранение с revision.
- Селектор reasoning effort у активной модели (раунд 148): уровень из
  `config.yaml` → `model.reasoning_effort` → значение для провайдера, обоими
  адаптерами.
- `dshr serve` резолвит профиль (endpoint, ключ, кастомный протокол, retry
  plan, reasoning effort) и честно предупреждает, если активная модель выпала
  из списка.

## Этапы

| Этап | Объём | Статус |
|---|---|---|
| 1 | Каталог, профили, список моделей, API, serve-резолв | ✅ раунд 140 |
| 2 | Кастомный провайдер в UI (id/protocol/endpoint/минимум одна модель) — ✅ раунд 141; discovery `/v1/models` с пикером — ✅ раунд 143 | ✅ |
| 3 | Редактор reasoning-уровней — ✅ раунд 141 (сохранение/валидация), отправка effort обоими адаптерами + селектор в UI — ✅ раунд 148; retry-policy (normal/always, maxRetries, коды, backoff, исполнение в адаптерах) — ✅ раунд 142 | ✅ |
| 4 | Модальные диалоги удаления и first-run notice (`welcome_version`) — ✅ раунд 144; слоты `settings.models.provider-card`/`.footer` в собственных регионах — ✅ раунд 145; DeepSeek-онбординг с ключом, i18n ru/en — ✅ раунд 151 | ✅ |
| 5 | Хранилище ключей (`credentials.json`, 0600, write-only API, env важнее) — ✅ раунд 146; применение без рестарта — ⏳ | ⏳ |

## Отличия от оригинала (осознанные)

- Модельный документ хранит только имя переменной; значение — в окружении или
  в отдельном `<home>/credentials.json` (0600, write-only API, env важнее).
- Активный маршрут один (`config.yaml` → `model`), а не набор live-адаптеров:
  `dshr serve` фиксирует провайдера на старте, поэтому правки просят рестарт.
- Каталог отражает только адаптеры `dshr` (DeepSeek Messages и
  OpenAI-compatible), а не полный pi-ai-каталог upstream; Anthropic/Gemini
  появятся вместе с адаптерами.
- React-примитивы/слоты upstream не переиспользуются: секция рендерится
  Leptos-шеллом; слоты для плагинов — этап 4.
- Effort уходит как есть из `models.yaml`: таблицы ремапа и per-model
  `thinkingLevelMap` upstream не переносятся, а OpenAI-адаптер при отсутствии
  effort не шлёт reasoning-параметр вовсе (upstream подставляет `high` и всегда
  шлёт `thinking`) — см. `docs/adr/043-reasoning-effort-wire.md`.
- Reasoning по-прежнему читается только в `serve`; `run`/`tui`/`acp` берут
  его из env-ключей (осознанно, `docs/TECH-DEBT.md`).
