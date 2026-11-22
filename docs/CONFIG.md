# Конфигурация

Слои: defaults < profile < user < patch. Схема: `dshr config show --schema`.

```yaml
model:
  provider: deepseek-account
  name: deepseek-flash
  reasoning_effort: high   # null/пусто = дефолт провайдера
profile-композиция: `profiles/<name>/profile.yaml` может включать
`bundles: [base, tools]` — сначала применяются `<home>/bundles/<bundle>/config.yaml`
(с учётом `requires: [name@^1.0]`, semver-проверка и циклы запрещены), bundle-патчи `patches: [file.yaml]` (до профиля), затем `patches` профиля
(после user config),
затем профиль, пользовательский конфиг и патчи.

session:
  dir: ~/.dshr/sessions
  auto_title: true   # LLM-заголовок при первом prompt (run)
tools:
  output_limit_chars: 30000
search:
  api_key: null      # Brave Search для web_search; env BRAVE_SEARCH_API_KEY важнее
  base_url: null     # переопределение endpoint'а
ui:
  language: ru   # ru | en
plugins:
  trusted_keys: []           # Ed25519 public keys (32-byte hex)
  cordis:                    # JS/npm plugins через node-sidecar
    - name: demo
      entry: plugins/demo.mjs
      enabled: true
logging:
  level: info
  format: json
```

`ui.language` влияет на подсказки, статусы и approval-диалог TUI, а также на
секцию **Models** веб-UI (строки берутся из `dshr-i18n`, язык приходит из
`GET /v1/health`). Остальные экраны веб-UI пока английские. Для CLI язык задаётся `--lang en|ru` или `DSHR_LANG`; локализованы верхнеуровневая справка, аргументы `run`/`sessions`, вывод `sessions`, `doctor`, `plugin`, `schedule`, `run`-сводка и `serve`. Часть справки по остальным командам и ошибки остаются английскими.

`plugins.trusted_keys` включает режим подписи WASM-плагинов. Пустой список — подписи не проверяются (подписанный плагин без ключей отклоняется); непустой список — каждый WASM-плагин обязан иметь `signature` в манифесте, которая проверяется по одному из ключей. Отсутствие подписи или чужой ключ означает отказ загрузки. Ключи и подписи создаются командами `dshr plugin keygen` и `dshr plugin sign` — см. [PLUGINS.md](PLUGINS.md).

## Секреты: `<home>/credentials.json`

Ключи, введённые в Models (или в first-run onboarding), хранятся в
`<home>/credentials.json` (0600, атомарная запись) в виде
`{version, revision, values: {REF: value}}`. Переменная окружения с тем же
именем всегда важнее (так `desktop.env` продолжает работать), поэтому
`credentials::resolve` сначала читает env, затем документ.

- API отдаёт только `configured`/`source`/`masked` — значение не покидает
  сервер (`POST /v1/credentials/describe`).
- Запись — `PUT /v1/credentials/{reference}` (write-only, `1..=4096`
  printable ASCII, опциональный `expected_revision`), удаление —
  `DELETE` с ревизией.
- Итоговый ключ провайдера ищется как env → `credentials.json`; `dshr serve`
  читает оба источника. Остальные команды (`run`/`tui`) пока читают только
  env — это записано в TECH-DEBT.

## Models: провайдеры и списки моделей

Вкладка **Models** веб-UI (порт React-секции `ui-settings-models`) хранит
профили провайдеров в отдельном документе `<home>/models.yaml` (0600,
атомарная запись). Активный маршрут и модель остаются в `config.yaml`
(секция `model`), поэтому правка профиля не переписывает остальную
конфигурацию.

```yaml
version: 1
revision: 3
welcome_version: 1      # подтверждённая версия first-run notice
providers:
  deepseek-official:
    protocol: anthropic-messages   # anthropic-messages | openai-chat
    base_url: https://gateway.example/anthropic   # null = каталог/env
    api_key_env: MY_DEEPSEEK_KEY                   # null = каталог
    models:
      - id: deepseek-flash
        name: DeepSeek Flash
        context_window: 1000000
        max_tokens: 64000
        input: [text, image]
        reasoning_efforts:          # false | карта уровень → значение
          off: off
          high: high
    retry:                          # null = дефолт адаптера
      mode: normal                  # normal | always
      max_retries: 5
      retryable_codes: [EMPTY_RESPONSE, RATE_LIMIT, SERVER, TIMEOUT, TRANSPORT]
      backoff:
        initial_delay_ms: 500
        max_delay_ms: 10000
        jitter_ratio: 0.1
```

- Маршрут без профиля использует встроенный каталог: `deepseek-account`,
  `deepseek-official`, `openai`, `openai-compatible`, `routerai` (алиас
  `deepseek`). Профиль может переопределить endpoint и имя переменной ключа.
- Пустой список `models` означает «каталог по умолчанию»; непустой — контракт:
  активную модель нельзя выбрать вне списка (`POST /v1/config/model` → 400).
- Маршрут вне каталога (кастомный провайдер) должен объявить `protocol` и
  `base_url`; `dshr serve` собирает его тем же адаптером.
- Приоритет endpoint'а: `providers.<route>.base_url` → `model.base_url` →
  `base_url_env` окружения (`DEEPSEEK_BASE_URL`/`OPENAI_BASE_URL`) → каталог.
- Ключи в файл не пишутся: профиль хранит **имя** переменной, а значение
  читается из окружения (`desktop.env` у десктопа). Веб-UI показывает только
  состояние «ключ найден/нет». Кнопка **Fetch available models** спрашивает
  endpoint о каталоге (`POST /v1/config/providers/discover`) тем же именем
  переменной и предлагает отметить модели; в файл попадает только выбор.
- `retry` — политика повторов запроса: `normal` повторяет только перечисленные
  коды (по умолчанию пять транзиентных) не больше `max_retries`, `always` —
  любую ошибку до успеха или отмены. Backoff — экспонента от
  `initial_delay_ms` с потолком `max_delay_ms` и симметричным джиттером
  (`0..1`); `Retry-After` провайдера учитывается и тоже ограничивается
  потолком. Пустые поля берут дефолты (5 / 500 / 10000 / 0.1), `null` —
  дефолт адаптера. Политика исполняется `dshr serve` в DeepSeek и
  OpenAI-совместимом адаптерах.
- `welcome_version` — последняя подтверждённая версия first-run notice
  (веб-UI показывает notice, пока значение меньше текущей версии); пишется
  отдельным `POST /v1/config/providers/welcome` и не участвует в revision-fence
  профилей.
- `revision` — монотонная ревизия; каждая запись через API присылает
  прочитанную ревизию, устаревшая получает `409`.
- Изменения профиля/списка применяются `dshr serve` при следующем запуске
  (адаптер и модель фиксируются на старте).
### `model.reasoning_effort`

Уровень reasoning effort активной модели живёт в `config.yaml` (поле
`model.reasoning_effort`), а не в `models.yaml`: провайдерский документ
описывает, **какие** уровни модель поддерживает (`reasoning_efforts`, карта
уровень → значение для провайдера), а выбор пользователя — одно значение.

- Пустое/отсутствующее поле (или пункт **Provider default** в Models) убирает
  ключ `model.reasoning_effort` из конфигурации: запрос уходит без
  reasoning-параметра.
- Значение обязано быть объявленным уровнем этой модели (иначе
  `POST /v1/config/model` → 400). Для модели, отсутствующей в профиле,
  проверяется общий список уровней: `off`, `minimal`, `low`, `medium`, `high`,
  `xhigh`, `max`.
- `dshr serve` резолвит уровень в значение провайдера из `models.yaml`
  (`reasoning_efforts.<level>`) и передаёт его адаптеру; необъявленный уровень
  не отбрасывается молча — `serve` предупреждает и отправляет как есть
  (permissive), а объявленный, но неподдерживаемый протоколом уровень
  отклоняется до сетевого запроса.
- На провод уходит по-разному: DeepSeek Messages — `thinking` плюс
  `output_config.effort` (для `off` — только `thinking: disabled`);
  OpenAI-compatible — плоский `reasoning_effort` для моделей `gpt-oss`,
  вложенный `reasoning.effort` для остальных `gpt-*`, ничего для прочих
  ([ADR-043](adr/043-reasoning-effort-wire.md)).
- Читает поле только `dshr serve`; `run`/`tui`/`acp` берут reasoning из
  env-ключей. Правка применяется после рестарта `dshr serve`.
