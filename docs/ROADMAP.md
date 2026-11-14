# Roadmap `dshr` — задачи, оценки, Definition of Done

Декомпозиция [PLAN.md](PLAN.md). Оценки — идеальные дни (ИД). Статусы: `[ ]` не начато, `[~]` в работе, `[x]` готово.

---

## Правила ведения

- Одна задача = один PR (или серия атомарных PR), ≤ 3 ИД.
- Задача закрывается только при выполнении DoD: код + тест + документация + observability.
- Оценка > 3 ИД → декомпозировать.
- Изменение оценки > 50 % → запись в журнал спринта и пересмотр плана.

---

## M0 — Фундамент (3–5 ИД)

**Статус:** выполнен 12.11.2026 — Rust 1.100, 28 тестов, CI-конфигурация, ADR-001…013.

| № | Задача | ИД | DoD |
|---|---|---|---|
| 0.1 | `cargo new` workspace, `rust-toolchain.toml`, edition 2024 | 0.5 | `cargo build --workspace` |
| 0.2 | Линтеры: rustfmt, clippy (pedantic subset), `cargo-deny` | 0.5 | `cargo clippy -D warnings` зелёный |
| 0.3 | CI: macOS/Ubuntu/Windows, кэш, nextest, coverage | 0.5 | Матрица зелёная на пустом проекте |
| 0.4 | `dshr-core`: ids, ошибки с кодами, время, redaction | 0.5 | Unit-тесты, serde round-trip |
| 0.5 | `dshr-config`: YAML-слои + JSON Schema | 0.5 | Golden-тесты приоритета слоёв |
| 0.6 | CLI-каркас: `--version`, `doctor`, `config show` | 1 | Бинарь запускается, вывод стабилен |
| 0.7 | `tracing`: JSON-логи, уровни, поля | 0.5 | Логи парсятся, redaction работает |
| 0.8 | ADR-001…007, CONTRIBUTING, LICENSE, README | 0.5 | Файлы на месте, ссылки валидны |

**Выход:** CI зелёный, `dshr doctor` информативен, конфиг собирается из слоёв.

---

## M1 — LLM + журнал + headless (10–16 ИД)

**Статус:** выполнен 12.11.2026 — 70 тестов, live smoke DeepSeek, cassette replay.

| № | Задача | ИД | DoD |
|---|---|---|---|
| 1.1 | `dshr-llm`: Message/ContentBlock/Request/Usage/Capabilities | 1 | Round-trip serde, proptest |
| 1.2 | `StreamChunk` + `BlockAssembler` (текст, thinking, tool_use, tool_result) | 1.5 | Тесты на неполные JSON-фрагменты |
| 1.3 | Anthropic SSE-парсер (`ping`, `content_block_*`, `message_delta/stop`) | 1 | Fuzz 1M итераций без паник |
| 1.4 | Адаптер DeepSeek Messages (Anthropic wire; account `x-dsh-auth-token` + API-key) | 1.5 | Контрактные тесты на кассете; live smoke |
| 1.5 | OpenAI-совместимый адаптер (RouterAI/шлюзы) | 1 | Тот же набор тестов |
| 1.6 | Ретраи, таймауты, отмена, классификация ошибок | 1 | 12+ тестов, включая отмену |
| 1.7 | Mock-сервер + cassette record/replay | 2 | Прогон без сети, байт-идентичный журнал |
| 1.8 | `dshr-session`: события, `derive_messages()`, проекции | 2 | Golden-тесты derive |
| 1.9 | `dshr-session-jsonl`: append, fsync, repair хвоста | 2 | `kill -9` → reopen → continue |
| 1.10 | CLI `run` (стрим, `--json`, `--model`, `--session`) | 1 | E2E на кассете |
| 1.11 | `sessions list/show/export` | 1 | Форматы документированы |
| 1.12 | Живой smoke: `/models` + Messages + stream + tool round-trip | 0.5 | `#[ignore]`-тест, запуск с `DSH_DEEPSEEK_TOKEN` |

**Выход:** one-shot чат с сохранением и продолжением сессии; всё воспроизводимо офлайн.

---

## M2 — Инструменты + цикл + TUI (15–20 ИД)

**Статус:** ядро выполнено 12.11.2026 — 112 тестов, живой tool-loop, TUI PTY-тест. `apply_patch` и `ask_user` добавлены позже; отложено: persistent bash, diff-view, eval-набор.

| № | Задача | ИД | DoD |
|---|---|---|---|
| 2.1 | Реестр инструментов, scoped-регистрация, схемы | 1.5 | Регистрация/скрытие тестами |
| 2.2 | Пайплайн: validate → approval → execute → spill | 1.5 | Тесты на каждую стадию |
| 2.3 | `read`/`write`/`edit`/`apply_patch` | 1.5 | Граничные случаи: CRLF, BOM, бинарь |
| 2.4 | `glob`/`grep` (`ignore`), лимиты, бинарные файлы | 1.5 | Тесты на .gitignore, размеры |
| 2.5 | `bash`: таймаут, отмена, persistent-сессия | 2 | Группа процессов убивается целиком |
| 2.6 | `todo`, `ask_user`, `present` | 1 | Интеграция в TUI |
| 2.7 | Агентный цикл: turn/step, батчи, max steps | 2.5 | 20+ сценариев, включая ошибки |
| 2.8 | Отмена, паники, консистентность журнала | 1.5 | Свойство: журнал валиден после отмены |
| 2.9 | Approval-политика (3 режима, правила, remember) | 1.5 | Матрица 12+ тестов |
| 2.10 | TUI: стрим, карточки, diff, модалки, статус | 2.5 | `expectrl`-сценарий |
| 2.11 | Демо-сценарий «баг → фикс → тесты → diff» | 1 | Записанное демо, README |
| 2.12 | Eval-набор: 10 задач + метрики успеха | 2 | Прогон в CI, отчёт по успеху/стоимости |

**Выход:** полноценная интерактивная сессия; ни одной записи без политики.

---

## M3 — Промпт, сессии, compaction, skills (10–15 ИД)

**Статус:** ядро выполнено 12.11.2026 — 127 тестов; prompt/AGENTS, skills, fork/rewind/title/search, compaction, FTS5 ([ADR-016](adr/016-search-backend.md)). Отложено: allowlist схем, live-гейт 100k; авто-заголовки (run) и импорт DSH v4 добавлены позже.

| № | Задача | ИД | DoD |
|---|---|---|---|
| 3.1 | Секции промпта + окружение + `AGENTS.md` | 2 | Golden-снимки, стабильный префикс |
| 3.2 | Допуск схем инструментов по политике | 1 | Нет «протёкших» инструментов |
| 3.3 | Resume/fork/rewind | 2 | Round-trip, границы turn |
| 3.4 | Заголовки сессий (async провайдер) | 1 | Не блокирует turn |
| 3.5 | SQLite-индекс + FTS5 + перестройка | 2 | Поиск < 100 мс на 10k событий |
| 3.6 | Compaction: токены, пороги, суммаризация | 3 | Длинная сессия продолжается |
| 3.7 | Pruning tool-результатов + spill-ссылки | 1 | Контекст сокращается, ссылки читаемы |
| 3.8 | Skills: discovery, каталог, `skill`-инструмент | 2 | 3 источника, ленивая загрузка |
| 3.9 | Экспорт Markdown/JSON, импорт DSH (флаг) | 1 | Импорт не ломает журнал |

**Выход:** длинные сессии, ветвление, поиск, навыки.

---

## M4 — MCP, web, jobs, субагенты (10–15 ИД)

**Статус:** ядро выполнено; MCP HTTP, resources, prompts, sampling и OAuth refresh. Отложено: MCP elicitation.

| № | Задача | ИД | DoD |
|---|---|---|---|
| 4.1 | MCP-клиент stdio + HTTP (`rmcp`) | 2.5 | Тест с echo-сервером |
| 4.2 | Namespacing, scoping, таймауты, reconnect | 2 | Деградация без падения |
| 4.3 | MCP-ресурсы и инструкции в промпт | 1 | Контрактные тесты |
| 4.4 | `web_search` + провайдер | 1.5 | Mock-провайдер в CI |
| 4.5 | `web_fetch` + readability + SSRF-guard | 2 | Набор SSRF-кейсов |
| 4.6 | Jobs: spawn, логи, kill, персистентность | 2 | Переживает рестарт |
| 4.7 | Субагенты: spawn/fork, дерево, сообщения | 2.5 | Демо с параллельной подзадачей |
| 4.8 | Бюджеты дерева (токены/время) | 1 | Превышение → корректная остановка |

**Выход:** внешние инструменты, фоновые задачи, делегирование.

---

## M5 — Песочница + плагины + композиция (15–25 ИД)

**Статус:** ядро выполнено; Linux Landlock + seccomp; Windows AppContainer — experimental opt-in, по умолчанию fail-closed. Отложено: Windows sandbox, hot reload, capability-grants/аудит, Cordis-рантайм.

| № | Задача | ИД | DoD |
|---|---|---|---|
| 5.1 | Модель политики + `ConfinedArgv` | 1.5 | Типизированные отказы |
| 5.2 | macOS Seatbelt-бэкенд | 3 | Escape suite: запись/чтение/сеть |
| 5.3 | Linux Landlock (файловые эффекты) | 2 | Suite на runner с Landlock; иначе типизированный skip + self-hosted/VM |
| 5.4 | Linux seccomp-bpf (syscalls) | 2 | Фильтр + тесты; помечен как расширение dshr |
| 5.5 | Windows ACL + Job Objects (best effort) | 3 | Базовые ограничения |
| 5.6 | Fail-closed при недоступном backend | 1 | Тест: backend сломан → отказ |
| 5.7 | WASM-host: wasmtime + WIT, один диспетчерский импорт `host.call` | 2 | Пример плагина из docs; новая возможность = новая операция, а не новый импорт |
| 5.8 | Лимиты, манифест (`api_version`), integrity-check, горячая замена | 2 | Память 64 МиБ/fuel/epoch по умолчанию; битая подпись → плагин не активируется; откат версии |
| 5.9 | Capability-модель, кэш грантов, аудит | 2 | Нет ambient authority; каждое использование гранта — событие аудита; встроенные плагины не исключение |
| 5.10 | Sidecar-плагины (JSON-RPC) | 2 | Пример на Python |
| 5.11 | Профили/бандлы/патчи + `--dump-config` | 2 | Golden-тесты 4 слоёв |
| 5.12 | `plugin add/remove/list`, lock-файл; `oci://` и локальный путь | 2 | Установка из локального пути и по `oci://`-ссылке |

**Выход:** изоляция процессов и расширяемость без пересборки.

---

## M5b — Совместимость npm/Cordis (12–18 ИД)

Решение — [ADR-013](adr/013-cordis-compat.md). Отдельная фаза: WASM-тир и песочница не ждут JS-рантайма.

| № | Задача | ИД | DoD |
|---|---|---|---|
| 5.13 | Runtime pack: движок, хеш и подпись, `dshr runtime install/remove/list`, офлайн-режим | 3 | На чистой машине install → плагин грузится; без пакета — внятная ошибка |
| 5.14 | Регистрация Cordis-рантайма в `PluginRuntime`: единый манифест, capabilities, health-check | 2.5 | Cordis-плагин в `dshr plugin list` наравне с WASM |
| 5.15 | Шим `@dshr/cordis-compat@1`: сервисы `ctx.*` поверх host-RPC | 3 | Пример плагина DSH отдаёт инструмент и срабатывает на хуке |
| 5.16 | Capability-шина builtins: `fs`/`exec`/`net`/`process`/`crypto` | 3 | Попытка выйти за workspace → типизированный отказ, видимый в аудите |
| 5.17 | npm-резолвер + CAS + lock (registry v2), без `postinstall` | 2.5 | Установка с транзитивными зависимостями; lock воспроизводим |
| 5.18 | HMR JS-тира: watch → reload → dispose/rebuild fiber | 1.5 | Правка плагина без рестарта host |
| 5.19 | Conformance-набор 8–12 реальных плагинов + nightly CI | 2 | Отчёт матрицы «загрузился / инструменты / хуки», регресс ловится |

**Выход:** существующие плагины DSH ставятся и работают, их доступ к ОС ограничен и зааудирован; нативные `.node`-плагины получают диагностику и sidecar-путь.

---

## M6 — Server + Web UI + SDK (15–25 ИД)

**Статус:** ядро выполнено 12.11.2026 — 149 тестов; REST/SSE, auth, approvals, JSON-RPC подмножество, Leptos CSR Web UI, Dockerfile. Отложено: браузерный e2e, ghcr-бутылки и reproducible builds.

| № | Задача | ИД | DoD |
|---|---|---|---|
| 6.1 | `axum`-сервер: сессии, turns, events (SSE) | 3 | OpenAPI, контрактные тесты |
| 6.2 | Auth, CORS, rate-limit, статика | 1.5 | Негативные тесты |
| 6.3 | Approvals по API + отмена | 1.5 | E2E |
| 6.4 | Web UI: чат + стрим | 3 | Playwright |
| 6.5 | Web UI: карточки, diff, approvals | 3 | Playwright |
| 6.6 | Web UI: сессии, настройки провайдеров | 2 | Playwright |
| 6.7 | SDK: JSON-RPC сервер/клиент | 2.5 | Пример Python |
| 6.8 | E2E-сценарий полного цикла | 2 | CI, скриншоты |
| 6.9 | Упаковка: cargo-dist, brew, Docker | 2 | Установка с нуля |
| 6.10 | LLM: Anthropic + Gemini адаптеры | 2.5 | Кассеты, tool-calls, reasoning |
| 6.11 | MCP-сервер (stdio, tools/resources) | 2 | Пример сервера + интеграционный тест |

**Выход:** браузерный интерфейс и встраиваемый SDK.

---

## M7 — Паритет (эпики, 84–145 ИД)

**Статус:** срез 13.11.2026 — M0–M7 закрыты (300 тестов); Leptos + Tauri + PWA + WIT; P2 Cordis — sidecar-прототип; Windows отложен.

| Эпик | Содержание | ИД |
|---|---|---|
| Workflows | скриптовый движок (Rhai/WASM), фазы, вложенные агенты | 12–20 |
| Расписания | cron-парсер, durable-очередь, доставка в сессию | 6–10 |
| Agent Teams | общая доска задач, владение, DAG | 10–16 |
| Worktrees/Forge | git-ветки, PR-статусы | 6–10 |
| LSP | 4 операции, провайдеры | 6–10 |
| Office | docx/xlsx/pptx рендер и конвертация | 10–20 |
| CUA/browser | Playwright/CDP-провайдер | 10–20 |
| OTel | traces/metrics, OTLP | 4–8 |
| Desktop | Tauri-обёртка, автообновление | 10–15 |
| ACP | протокол, совместимость | 6–10 |
| i18n | ru/en, вынос строк | 4–6 |

---

## Контрольные точки

| Точка | Когда | Проверка |
|---|---|---|
| G0 | конец M0 | ✅ пройдена 12.11.2026: `doctor`, конфиг, тесты |
| G1 | конец M1 | ✅ пройдена 12.11.2026: cassette replay, живой smoke DeepSeek |
| G2 | конец M2 | ✅ ядро: tool-loop + approvals + TUI e2e; eval-набор отложен |
| G3 | конец M3 | ✅ ядро: compaction unit-тест; live-гейт 100k отложен |
| G4 | конец M4 | ✅ MCP mock-сервер + субагент из кассеты; HTTP/resources отложены |
| G5 | конец M5 | ✅ Seatbelt, Landlock+seccomp и AppContainer: escape + workspace-write + сеть Linux |
| G6 | конец M6 | ✅ HTTP/SSE + встроенный UI; Playwright e2e отложен |
| G7 | M7 | матрица возможностей заполнена |

---

## Журнал изменений плана

| Дата | Изменение | Причина |
|---|---|---|
| 12.11.2026 | v1.0 — исходный план | Старт проекта |
| 12.11.2026 | v1.1 — улучшенная Rust-реализация; M0 выполнен | Уточнение позиционирования; каркас собран |
| 12.11.2026 | M1 выполнен: LLM, сессии, headless | 70 тестов; DeepSeek проверен живым прогоном |
| 12.11.2026 | M2 ядро: инструменты, агент, approvals, TUI | 112 тестов; живой tool-loop |
