# Каталог тест-кейсов и покрытие

Документ описывает, как проверяется весь код `dshr`: формальные модели →
тест-кейсы → автоматические тесты → покрытие. Он же служит чек-листом для
регрессий: у каждого кейса есть ID, уровень и статус.

## Метод

1. **Формализация.** Протоколы с гонками и жизненным циклом формализуются в TLA+
   ([formal/](../formal/README.md)); TLC доказывает инварианты и liveness.
2. **Деривация.** На каждое действие/инвариант модели пишутся тест-кейсы: happy
   path, негативный, граничный и «после сбоя». ID кейса связан с моделью.
3. **Исполнение.** Rust: `#[test]` / `#[tokio::test]`; сквозные сценарии —
   headless Chrome (`scripts/plugin-smoke.py`); релизные — CI (`release.yml`, `ci.yml`).
4. **Измерение.** `cargo llvm-cov` по workspace; цель — не «100%», а отсутствие
   непроверенных ветвей жизненного цикла и безопасности.

Уровни: **U** unit, **I** integration, **E** end-to-end (браузер/процесс),
**T** формальная модель (TLC).

Статусы: ✅ автоматизирован · ⚠️ частично/вручную · 📋 запланирован (приоритет
S1 — блокер, S2 — функциональный пробел, S3 — косметика/редкий случай).

## Формальные модели → тест-кейсы

| Модель / действие | Инвариант или свойство | Тест-кейсы |
|---|---|---|
| `CordisSupervisor!Start/Connect/Register` | `RegisteredImpliesStarted` | SUP-01, SUP-02, REG-01 |
| `CordisSupervisor!Stop` | `StoppedHasNoTools` | SUP-03, REG-02 |
| `CordisSupervisor!Reload` | `RunningGenerationMatches`, `GenerationSound` | SUP-04, SUP-05 |
| `CordisSupervisor!Crash` | `InflightSound` (окно stale-инструментов) | SUP-06, REG-03 |
| `CordisSupervisor!Disable/Enable` | liveness `<>[] disabled ~> <>[] stopped` | SUP-07, SUP-08 |
| `ClientAssets!Issue` | `OneTicketPerCapability`, `IssuedExactness` | CLI-10, CLI-11 |
| `ClientAssets!Attempt` | `ServedIssued`, `ServedConfined`, `OutsideNeverAllowed` | CLI-12, CLI-13, SRV-20 |
| `ClientAssets!Tick` | `ServedWithinTTL`, `LateNeverAllowed` | CLI-11, CLI-14 |
| `ApprovalGate!Execute` | `NoUnapprovedExecution` | AGT-05, AGT-06 |
| `ApprovalGate!Deny` | `ExecutedNotDenied` | AGT-07 |
| `ApprovalGate!Cancel` | `FinishedTurnClean` | AGT-08 |
| `ApprovalGate` liveness | `PendingEventuallyDecided` | AGT-09 |

## config / core

| ID | Кейс | Ур. | Статус |
|---|---|---|---|
| CFG-01 | Слои конфига: defaults → bundles → bundle patches → profile → user → profile patches → CLI patches | U | ✅ `crates/config` |
| CFG-02 | `requires` бандлов: semver-диапазоны, отсутствие зависимости, цикл | U | ✅ `crates/config` |
| CFG-03 | `deny_unknown_fields`: неизвестный ключ — ошибка, не игнор | U | ✅ |
| CFG-04 | Валидация: пустые имена, некорректные пути, отрицательные лимиты | U | ✅ |
| CFG-05 | Битый YAML/JSON при hot-reload не применяется и не роняет процесс | I | ✅ `crates/cordis/tests/manager.rs` (`a_broken_config_keeps_running_plugins`) |
| CFG-06 | Переменные окружения и приоритет `--home/--profile/DSH_HOME` | I | ✅ `crates/cli` |
| CORE-01 | Коды ошибок `DshrError` → exit code и текст | U | ✅ `crates/core` |
| CORE-02 | `SessionId` уникален и сериализуется стабильно | U | ✅ |

## LLM-провайдеры

| ID | Кейс | Ур. | Статус |
|---|---|---|---|
| LLM-01 | Выбор маршрута provider (deepseek/openai/routerai) и модели | U | ✅ |
| LLM-02 | Стриминг: `text_delta`, tool calls по частям, `done`/`stop_reason` | U | ✅ `crates/llm*` |
| LLM-03 | Аргументы инструмента: пустые, битый JSON, вложенные объекты | U | ✅ |
| LLM-04 | Usage/кэш-токены аккумулируются, `max_tokens` соблюдается | U | ✅ |
| LLM-05 | HTTP 4xx/5xx, таймаут, обрыв стрима — типизированные ошибки | U | ✅ |
| LLM-06 | Мок-провайдер детерминирован для e2e | U | ✅ |
| LLM-07 | OpenAI-совместимый wire-формат (tools, reasoning) | I | ⚠️ S2: нет контрактного теста против реального провайдера |

## session / compaction

| ID | Кейс | Ур. | Статус |
|---|---|---|---|
| SES-01 | Append/read JSONL, устойчивость к последней неполной строке | U | ✅ `crates/session*` |
| SES-02 | resume/fork/rewind сохраняют/обрезают историю корректно | I | ✅ |
| SES-03 | Импорт DSH v4 (zstd JSONL, tool calls, title) | I | ✅ `crates/cli` |
| SES-04 | Массовый импорт `--all`, битые файлы пропускаются | I | ✅ |
| SES-05 | Compaction: порог, сохранение system/tool-пар, идемпотентность | U | ✅ `crates/compaction` |
| SES-06 | Заголовок сессии: провайдер, fallback на первую строку | I | ✅ |
| SES-07 | Одновременная запись двух процессов | I | 📋 S2 |

## tools / registry

| ID | Кейс | Ур. | Статус |
|---|---|---|---|
| TOOL-01 | `read`: бинарный файл отклоняется, лимиты, пути вне workspace | U | ✅ `crates/tools` |
| TOOL-02 | `write`: создание родителей, отказ вне workspace | U | ✅ |
| TOOL-03 | `edit`: уникальность совпадения, no-op, несовпадение | U | ✅ |
| TOOL-04 | `apply_patch`: add/update/delete/move, счётчики строк | U | ✅ |
| TOOL-05 | `bash`: cwd/env между вызовами, таймаут, отмена, код выхода | U | ✅ |
| TOOL-06 | `glob`/`grep`: glob-семантика, regex-ошибки, лимиты результатов | U | ✅ |
| TOOL-07 | `web`: deny localhost/приватных адресов, HTML→text, лимит размера | U | ✅ |
| TOOL-08 | Схема аргументов: неизвестные/пропущенные поля | U | ✅ |
| TOOL-09 | Вывод сверх лимита: spill-файл и хвост с ссылкой | U | ✅ |
| TOOL-10 | Регистр инструментов: allow/deny-фильтры, независимость `filtered` | U | ✅ |
| REG-01 | Атомарность `register_all`: читатель видит 0 или все 3 инструмента, не часть | U | ✅ новый (TLA `RegisteredImpliesStarted`) |
| REG-02 | `remove_prefix` снимает ровно инструменты плагина, `unregister` возвращает флаг | U | ✅ |
| REG-03 | Generation растёт на каждую мутацию, не убывает | U | ✅ |
| REG-04 | Гонка `execute` с `remove_prefix`: вызов доигрывает на старом Arc | I | 📋 S3 |

## agent / approvals

| ID | Кейс | Ур. | Статус |
|---|---|---|---|
| AGT-01 | Tool-loop: последовательные вызовы, max_steps, финальный текст | I | ✅ `crates/agent` |
| AGT-02 | Ошибка инструмента доставляется модели, цикл не падает | I | ✅ |
| AGT-03 | Отмена посреди шага: стрим закрыт, состояние консистентно | I | ✅ |
| AGT-04 | `ask_user` в headless — fail-closed | U | ✅ |
| AGT-05 | Политика `ask`: инструмент не исполняется до approval | I | ✅ (TLA `NoUnapprovedExecution`) |
| AGT-06 | Политика `never`: исполняется без ожидания | I | ✅ |
| AGT-07 | Отказ окончателен: повторной попытки нет, модель получает отказ | I | ✅ |
| AGT-08 | Отмена при pending-approval очищает ожидания | I | ✅ (TLA `FinishedTurnClean`) |
| AGT-09 | Каждое pending-решение рано или поздно принято (UI живой) | T | ✅ `ApprovalGateLive.cfg` |
| AGT-10 | Бюджет дерева субагентов, рекурсивный вызов tools | I | ✅ |
| AGT-11 | Prompt injection через tool result не меняет политику approvals | I | 📋 S2 |

## mcp

| ID | Кейс | Ур. | Статус |
|---|---|---|---|
| MCP-01 | stdio: initialize, tools/list, tools/call, ошибки сервера | I | ✅ `crates/mcp/tests` |
| MCP-02 | resources/prompts: list и read | I | ✅ |
| MCP-03 | HTTP POST-SSE и фоновый GET-SSE | I | ✅ |
| MCP-04 | Elicitation: stdio, POST-SSE, GET-SSE; отказ/таймаут | I | ✅ |
| MCP-05 | OAuth refresh, истёкший токен | I | ✅ |
| MCP-06 | Сервер упал/закрыл stdout: вызовы завершаются ошибкой | I | ✅ |
| MCP-07 | Sampling: запрос модели у сервера | I | 📋 S3 |

## Cordis: supervisor (TLA `CordisSupervisor`)

| ID | Кейс | Ур. | Статус |
|---|---|---|---|
| SUP-01 | `start` регистрирует инструменты плагина, статус running/gen=1 | I | ✅ `crates/cordis/tests/manager.rs` |
| SUP-02 | Повторный `start` заменяет инструменты, старая генерация не остаётся | I | ✅ |
| SUP-03 | `stop` снимает инструменты и завершает sidecar; статус running=false | I | ✅ |
| SUP-04 | `reload` подхватывает правку entry (v1→v2), registry-handle тот же | I | ✅ |
| SUP-05 | `sync`: add/remove/changed-spec, удалённый плагин исчезает из реестра | I | ✅ |
| SUP-06 | Падение sidecar (tool вызывает `process.exit`): вызов падает, reload восстанавливает | I | ✅ новый (`reload_recovers_after_the_sidecar_crashes`) |
| SUP-07 | Watcher: изменение файла плагина перезагружает только его | I | ✅ |
| SUP-08 | Watcher: `profile.yaml` добавляет/убирает плагины без рестарта | I | ✅ |
| SUP-09 | Битый конфиг не останавливает работающие плагины (retry на следующем тике) | I | ✅ |
| SUP-10 | Ошибка старта запоминается в статусе и повторяется при reload | I | ✅ |
| SUP-11 | Стоимость watcher'а: пакет > 4000 файлов не сканируется целиком | U | 📋 S3 |
| SUP-12 | Параллельные `reload` и `sync` не интерливают (ops-mutex) | I | 📋 S2 |

## Клиентская половина (TLA `ClientAssets`)

| ID | Кейс | Ур. | Статус |
|---|---|---|---|
| CLI-01 | `dsh.client` discovery: entry, exports-карта, platform=web | U | ✅ `crates/cordis/src/client.rs` |
| CLI-02 | Отказ при отсутствии манифеста/половины и при не-web платформе | U | ✅ |
| CLI-03 | Путь бандла канонизируется и не выходит за пакет | U | ✅ |
| CLI-04 | Режим доставки: один файл → inline, относительные импорты → assets, override | U | ✅ |
| CLI-05 | Inline host-страница: base64-бандл, escape `<`, ready/error маркеры | U | ✅ |
| CLI-06 | Мост: порядок слотов, `inject`-группы, `effect`-dispose, `deactivate` | I | ✅ `tests/bridge_harness.mjs` |
| CLI-07 | Мост: тема, resize, `ctx.action` round-trip и отказ | I | ✅ |
| CLI-08 | Module-table бандл активируется через `apply(ctx)` | E | ✅ smoke |
| CLI-09 | Eager-половина монтируется в порядке `inject` и переживает закрытие | E | ✅ smoke |
| CLI-10 | Capability выдаётся на один плагин: чужой плагин получает 403 | I | ✅ новый |
| CLI-11 | Capability истекает (TTL) и после этого не принимается | U | ✅ новый |
| CLI-12 | Traversal `%2e%2e` и absolute-путь в ассетах → 404 | I | ✅ |
| CLI-13 | Symlink внутри пакета, ведущий наружу, отклоняется | U | ✅ новый |
| CLI-14 | Неизвестный/некорректный capability → отказ, без утечки деталей | I | ✅ |
| CLI-15 | CSP asset-страницы содержит только origin сервера | I | ✅ |
| CLI-16 | Враждебный `Host` не инъектируется в CSP (fallback loopback) | I | ✅ новый |
| CLI-17 | Открытая панель подхватывает новый бандл после reload | E | ✅ smoke |
| CLI-18 | Слоты: несколько, переключение, `deactivate` при закрытии | E | ✅ smoke |
| CLI-19 | Ошибка бандла показывается в панели, не «пустой экран» | E | ✅ smoke |
| CLI-20 | `compose`/`notify` действия доходят до шелла | E | ✅ smoke |
| CLI-21 | 28 сквозных проверок в headless Chrome | E | ✅ `scripts/plugin-smoke.py` |

## WASM

| ID | Кейс | Ур. | Статус |
|---|---|---|---|
| WASM-01 | Core-ABI модуль: загрузка, вызов, ошибка гостя | U | ✅ `crates/wasm` |
| WASM-02 | Component model/WIT: типы, вызов | U | ✅ |
| WASM-03 | Подпись Ed25519: валидная/битая/чужой ключ | U | ✅ |
| WASM-04 | Лимиты: fuel, memory cap, таймаут | U | ✅ |
| WASM-05 | Hot reload каталогов: новый/изменённый/удалённый плагин | I | ✅ |
| WASM-06 | Stale-инструменты удалённого плагина не остаются в реестре | I | ✅ |
| WASM-07 | Негативный: модуль без capabilities не получает fs/net | I | ⚠️ S2 |

## server

| ID | Кейс | Ур. | Статус |
|---|---|---|---|
| SRV-01 | Auth: header/query token, отсутствие/неверный → 401 | I | ✅ `crates/server/tests` |
| SRV-02 | Rate limit: N запросов, затем 429 + Retry-After | I | ✅ |
| SRV-03 | Сессии: create/list/get, SSE-события, cancel | I | ✅ |
| SRV-04 | Approvals/questions endpoints: resolve, неизвестный id | I | ✅ |
| SRV-05 | `/v1/tools`, `/v1/rpc`, OpenAPI, PWA-манифест | I | ✅ |
| SRV-06 | Плагины: install/toggle/remove — валидация имён, CLI-вызов | I | ✅ |
| SRV-07 | `status`/`reload`: 200/404/502, без супервизора — 501 | I | ✅ `plugins_live.rs` |
| SRV-08 | `/v1/plugins/installed` следует за супервизором | I | ✅ |
| SRV-09 | `/v1/plugins/clients` и режимы доставки | I | ✅ |
| SRV-10 | `/host` inline: CSP без сети, bundle не ссылается наружу | I | ✅ |
| SRV-11 | `/asset/{cap}/{path}`: scope, TTL, traversal, размер | I | ✅ |
| SRV-12 | Враждебный Host не попадает в CSP | I | ✅ новый |
| SRV-13 | SSE keep-alive и реконнект клиента | I | 📋 S3 |
| SRV-14 | Больше лимита тела запроса → 413 | I | 📋 S3 |

## Web UI

| ID | Кейс | Ур. | Статус |
|---|---|---|---|
| WEB-01 | Тема light/dark, сохранение в localStorage, `?theme=` | E | ✅ headless |
| WEB-02 | Подключение по токену, список сессий, стрим ответа | E | ✅ |
| WEB-03 | Approval/question диалоги | E | ✅ |
| WEB-04 | Панель Plugins: каталог, оценки/отзывы, install | E | ✅ |
| WEB-05 | Клиентские половины: вкладки слотов, resize, тема | E | ✅ smoke |
| WEB-06 | Deep link `?token=&plugin=` подключается автоматически | E | ✅ smoke |
| WEB-07 | Нет console error/5xx во время сценария | E | ✅ smoke (падает на ошибках) |
| WEB-08 | Клавиатурная доступность панелей | E | 📋 S3 |

## sandbox

| ID | Кейс | Ур. | Статус |
|---|---|---|---|
| SBOX-01 | macOS Seatbelt: запись внутри разрешена, вне — отказ | I | ✅ `crates/sandbox` + CLI tests |
| SBOX-02 | Linux Landlock: пути и сеть | I | ⚠️ CI-only |
| SBOX-03 | Linux seccomp: сетевые вызовы запрещены | I | ⚠️ CI-only |
| SBOX-04 | Windows AppContainer: запись внутри/снаружи (реальный раннер) | I | ✅ CI (`ci.yml`) |
| SBOX-05 | Fail-closed: неизвестная платформа → запрет | U | ✅ |
| SBOX-06 | `DangerFull` отключает ограничения только явным флагом | I | ✅ |

## jobs / team / workflow / schedule

| ID | Кейс | Ур. | Статус |
|---|---|---|---|
| JOB-01 | Durable job: старт, статус, resume после рестарта | I | ✅ `crates/jobs` |
| JOB-02 | Отмена и таймаут джобы | I | ✅ |
| TEAM-01 | Ретраи, бюджет, дерево субагентов | I | ✅ `crates/team` |
| WF-01 | Workflow: шаги, ветвления, ошибка шага | I | ✅ `crates/workflow` |
| SCHED-01 | Cron-парсинг, DST/таймзоны, блокировка одновременного запуска | I | ✅ `crates/schedule` |

## extensions (LSP / office / browser / search)

| ID | Кейс | Ур. | Статус |
|---|---|---|---|
| EXT-01 | LSP: hover/definition/references/diagnostics | I | ✅ `crates/lsp/tests` |
| EXT-02 | Office: docx/xlsx/pptx создание и чтение | I | ✅ `crates/office` |
| EXT-03 | Browser CUA: open/click/type/screenshot/close | I | ⚠️ требует браузера |
| EXT-04 | Search: FTS5-индекс, переиндексация | I | ✅ `crates/search/tests` |

## interfaces (ACP / SDK / TUI / i18n)

| ID | Кейс | Ур. | Статус |
|---|---|---|---|
| IFACE-01 | ACP: initialize, сессии, permission, fs/terminal | I | ✅ `crates/acp/tests` |
| IFACE-02 | ACP: `session/list`, `set_mode`, `close`, `resume` | I | ✅ |
| IFACE-03 | ACP elicitation для `ask_user` | I | ✅ |
| IFACE-04 | SDK-клиент: сессии, стрим, ошибки | I | ✅ `crates/sdk/tests` |
| IFACE-05 | TUI: рендер, diff-детали, i18n ru/en | U | ✅ `crates/tui`, `crates/i18n` |

## desktop / release

| ID | Кейс | Ур. | Статус |
|---|---|---|---|
| DESK-01 | Трей: show/hide, профили, выход; close-to-tray | I | ✅ `crates/desktop` |
| DESK-02 | Recovery-окно: retry/safe mode/диагностика/плагины | I | ✅ |
| DESK-03 | Provider wizard → `desktop.env` 0600, перезапуск | I | ✅ |
| DESK-04 | Обновления: канал, скачивание, SHA256, установка | I | ✅ |
| DESK-05 | Режимы окна и материалы | E | ⚠️ ручная проверка |
| REL-01 | Детерминированные архивы и воспроизводимый бинарь | I | ✅ `scripts/check-reproducible.sh` |
| REL-02 | 5 таргетов, SHA256SUMS, Ed25519, provenance | I | ✅ CI |
| REL-03 | Бутлы/формула/ghcr-публикация (mock-регистр) | I | ✅ CI |
| REL-04 | DMG/`.app`, `codesign --verify` | I | ✅ |
| REL-05 | Notarization + staple + `spctl` | I | ⏸ внешний блокер (нет Apple Developer Program) |

## Покрытие

Инструмент: `cargo-llvm-cov` (llvm-tools-preview установлены).

```sh
cargo llvm-cov --workspace --summary-only          # таблица по крейтам
cargo llvm-cov --workspace --html --open           # отчёт с ветвями
cargo llvm-cov --workspace --fail-under-lines 80   # гейт для CI
```

Пороговые значения, которые имеет смысл закрепить в CI:

| Область | Цель по строкам | Обоснование |
|---|---|---|
| `tools`, `config`, `session*`, `llm*` | ≥ 90% | чистые функции, дешёвые тесты |
| `cordis`, `server`, `agent` | ≥ 80% | есть I/O и процессы, часть ветвей — защитные |
| `desktop`, `tui`, `cli` | ≥ 60% | GUI/интерактив, проверяются e2e |
| весь workspace | ≥ 80% | базовый гейт |

Текущий замер (этот прогон) — см. таблицу ниже; непокрытые строки в первую
очередь ищутся в: обработчиках ошибок I/O, ветках платформенных sandbox-бэкендов,
редких состояниях SSE/стриминга и в коде релизной упаковки.

### Базовый замер (2026-11-14, `cargo llvm-cov --workspace --summary-only`)

| Метрика | Покрыто | Всего | % |
|---|---|---|---|
| Строки | 15 346 | 20 996 | **73.1%** |
| Функции | 1 633 | 2 433 | 67.1% |
| Регионы | 14 019 | 18 616 | 75.3% |

Самые слабые места (по строкам):

| Файл | Строки |
|---|---|
| `cli/src/tui_cmd.rs` | 0% (не исполняется в тестах) |
| `tui/src/lib.rs` | 39.4% |
| `lsp/src/lib.rs` | 52.0% |
| `wasm/src/manager.rs` | 53.9% |
| `llm-openai/src/lib.rs` | 58.6% |
| `llm-deepseek/src/lib.rs` | 62.8% |
| `sdk/src/lib.rs` | 64.6% |
| `server/src/lib.rs` | 70.1% |
| `cordis/src/manager.rs` | 75.8% |

Приоритеты закрытия пробелов:

| Приоритет | Область | Что покрыть | Кейсы |
|---|---|---|---|
| S2 | LLM wire | ошибки/редиректы/таймауты провайдеров, reasoning-блоки | LLM-02, LLM-05, LLM-07 |
| S2 | server | SSE-реконнект, лимиты тела, ошибки стрима | SRV-13, SRV-14 |
| S2 | cordis manager | ветки ошибок `sync`/`reload`, конкурирующие операции | SUP-10, SUP-12 |
| S2 | wasm manager | ошибки загрузки/подписи в hot-reload пути | WASM-05, WASM-06 |
| S2 | lsp | ошибки запуска сервера, таймауты, рестарт | EXT-01 |
| S3 | tui | рендер-снапшоты состояний | IFACE-05 |
| S3 | CLI-команды | `tui`/`prompt`/`doctor` happy-path | — |

Гейт для CI (после закрытия S2): `--fail-under-lines 80` для workspace и
`--fail-under-lines 90` для `tools`, `config`, `session*`, `llm`,
`cordis/src/client.rs`.

