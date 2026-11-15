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

**Статус:** закрыто. Ядро — 12.11.2026 (112 тестов, живой tool-loop, TUI PTY-тест); `apply_patch`, `ask_user`, persistent bash, diff-view и eval-набор добавлены позже.

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
| 2.12 | Eval-набор: 10 задач + метрики успеха | 2 | ✅ `evals/tasks/*.json` + `crates/cli/tests/evals.rs`; успех 10/10, бюджет токенов на задачу, отчёт `target/evals/report.json`, джоба `evals` в CI |

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
| 3.6 | Compaction: токены, пороги, суммаризация | 3 | ✅ `crates/compaction/tests/live_gate_100k.rs`: сессия > 100k токенов сжимается, последние 2 turn'а сохраняются дословно, reopen и продолжение сессии |
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

**Статус:** закрыто. Linux Landlock + seccomp; Windows AppContainer — experimental opt-in, по умолчанию fail-closed (реальный Windows-хост недоступен); WASM hot reload, capability-гранты, аудит и Cordis-рантайм (sidecar + клиентская половина) сделаны.

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

**Статус:** частично: sidecar-рантайм (системный Node, настоящий `cordis` при наличии пакета), hot reload, capability-гранты и клиентская половина (eager registry + iframe на слот, 17 регионов, полный словарь DSH 0.2.2-rc.2, экспериментальный `dsh.client.runtime` — половина приносит свою модульную таблицу/React, PoC тира B `dsh.client.host_modules` — React 18.3.1 и таблица модулей от хоста, C-lite — React-компоненты слотов с host-hooks/`ctx.get('sessions')`, design-system примитивы (с раунда 93, C2.1 — reference-порт: атомы и все 204 иконки; с раунда 94, C2.2 — полный `Tooltip`), сервисы моста (`ctx.get('slots')`/`locale`, `ctx.inject`, диагностика `__DSHR_LAST_ERROR__`), React-контракт (типы `React.memo`/`forwardRef`, keyed-ids, `locale.register`/`t()`, `ctx.interval`/`ctx.timeout`) и conformance n=4 (раунд 82 — n=6: `dsh-session-superagent@0.10.12`, `dsh-mnemon@0.5.46`; раунд 83 — **n=9**, цель ADR-013 §6 8–12 достигнута: `dsh-plugin-market@1.2.2`, `dsh-plugin-codegraph-panel@0.6.5`, `@goodandready/dsh-plugin-notify@0.4.9`); с раунда 84 — **C1**: host HTTP-маршруты плагинов (loopback-сервер sidecar, allowlist-прокси dshr-сервера, `connect-src` на origin сервера для кадров с `host_modules`; реальная host-половина `@kt11/dsh-deepseek-balance` отвечает своим JSON), остатки — SSE/WebSocket/стриминг, отсутствие аутентификации; раунд 85 — **C2**: design-system шим вырос с 7 до 30+ экспортов по фактическим импортам (компоненты, именованные `Icon*` с Proxy-fallback, `writeClipboard`, хуки `useAnchoredPosition`/`useDismissOnOutsidePointer`), conformance n=11 (`dsh-plugin-debugging@0.8.1`, `dsh-plugin-studio@0.3.1` активируются без ошибок, их settings-UI был инертен без host-сервисов `pluginManager`/settings — с раунда 89 (C1.2) рендерится через seat-`inject` и `configForms`), smoke раунда 85 — 79/79; раунд 86 — хардненинг C1: prefix-маршруты, `routes` в `GET /v1/plugins/status`, фильтр MCP-псевдо-инструментов и loopback-only политика с opt-in `--allow-lan-plugin-routes`; раунд 87 — **C1.1**: сервисы `sessions`/`sessionProjections` в sidecar из внутреннего снимка сервера (usage-маршруты реальных плагинов считают статистику по сессиям dshr; задержка ≤ 5 с, проекции упрощены, содержимое сессий не мостится), раунд 89 — **C1.2**: seat-`inject`, Cordis-генераторы (`slots.inject`/`ctx.inject`/`ctx.effect`) и host-сервис `configForms` (in-memory writable-форма в кадре, файловая — у host-половин), поэтому settings-UI `dsh-plugin-studio`/`dsh-plugin-debugging` реально рендерится; smoke раунда 89 — 82/82; раунд 92 — **C1.4**: host-сервисы `webServer` (правила exact/prefix-маршрутов, duplicate-throw, origin-аксессоры), `ctx.baseUrl` (`file://` корня пакета) и `configForms` (reference-shape snapshot, числовая ревизия, `set`/`unset`) выровнены по reference-контракту, `registerFallback`/`registerUpgrade` не поддержаны; bridge harness 102 (+4), а раунд 93 — C2.1: атомы design-system и все 204 иконки портированы из reference-исходников (`deepseek-ai/deepseek-harness` @ `39163dd`, `packages/client/ui-primitives/src`), общий рантайм-артефакт на конец раунда 375 894 байта, а раунд 94 — C2.2: полный reference-порт `Tooltip` (placement-engine, `ShortcutKeys`, `input-modality`; артефакт 387 613 байт), поэтому полный `Menu` и `MarkdownText`/`JsonBlock` остаются упрощёнными; полная React-совместимость — C-full [ADR-040](adr/040-react-client-half.md), решение владельца) — сделаны; runtime pack, `@dshr/cordis-compat@1`, npm-резолвер/CAS и отчёт матрицы conformance — нет.

| № | Задача | ИД | DoD |
|---|---|---|---|
| 5.13 | Runtime pack: движок, хеш и подпись, `dshr runtime install/remove/list`, офлайн-режим | 3 | На чистой машине install → плагин грузится; без пакета — внятная ошибка |
| 5.14 | Регистрация Cordis-рантайма в `PluginRuntime`: единый манифест, capabilities, health-check | 2.5 | Cordis-плагин в `dshr plugin list` наравне с WASM |
| 5.15 | Шим `@dshr/cordis-compat@1`: сервисы `ctx.*` поверх host-RPC | 3 | Пример плагина DSH отдаёт инструмент и срабатывает на хуке |
| 5.16 | Capability-шина builtins: `fs`/`exec`/`net`/`process`/`crypto` | 3 | Попытка выйти за workspace → типизированный отказ, видимый в аудите |
| 5.17 | npm-резолвер + CAS + lock (registry v2), без `postinstall` | 2.5 | Установка с транзитивными зависимостями; lock воспроизводим |
| 5.18 | HMR JS-тира: watch → reload → dispose/rebuild fiber | 1.5 | Правка плагина без рестарта host |
| 5.19 | Conformance-набор 8–12 реальных плагинов + nightly CI | 2 | ⚠️ числовая цель закрыта с запасом (C-lite/C2, раунды 79–85): 11 реальных плагинов в smoke — `@kt11/dsh-deepseek-balance@0.2.1` (рендер, CSS, клик), `dsh-remote@1.1.2` (React-слоты в `conversation.session.header.*`/`sidebar.workspaces.directoryFlow`, штатная деградация без опциональных сервисов), `@ruoyang/dsh-plugin-deepseek-balance@1.0.2` (плавающий бейдж в `shell.overlay`, `ctx.effect`/`ctx.interval`/`ctx.locale`), `dsh-genui@0.3.3` (тумблер в `conversation.input.left`, `React.memo` и keyed `conversation.chat.node`), `dsh-session-superagent@0.10.12` (composer dock `conversation.composer.dock`), `dsh-mnemon@0.5.46` (крупный multi-region: активация, публикация слотов и сверка shadow-регистраций через `ctx.slots.subscribe`/`getVersion`/`entriesOfSlot`), `dsh-plugin-market@1.2.2` (сайдбар), `dsh-plugin-codegraph-panel@0.6.5` (панель `settings.section`) и `dsh-plugin-notify@0.4.9` (строки `plugins.*`), `dsh-plugin-debugging@0.8.1` и `dsh-plugin-studio@0.3.1` (раунд 85, C2: примитивы-heavy половины активируются; с раунда 89, C1.2, их settings-UI рендерится через seat-`inject`/`configForms` и честно деградирует без host-маршрутов); источники — MIT, vendored fixtures, smoke-гейт в CI (`web-e2e`). Цель 8–12 достигнута; nightly-прогон и отчёт матрицы остаются. `@michengai/dsh-archive-manager` требует `'unsafe-eval'` в CSP, `@michengai/dsh-skills-manager` падает на инициализации — вне набора (TECH-DEBT) |
| 5.20 | Host HTTP-маршруты плагинов (C1): sidecar loopback-сервер, allowlist-прокси, `connect-src` для opt-in кадров | 2 | ⚠️ раунды 84–86: exact/prefix-маршруты проксируются (exact выигрывает, среди prefix — самый длинный), реальная host-половина `@kt11/dsh-deepseek-balance` отвечает своим JSON (500 `not-configured` без ключа, 200 для usage-маршрута); раунд 86 добавил per-plugin `routes` в `GET /v1/plugins/status`, фильтр MCP-псевдо-инструментов и loopback-only политику (403 без `--allow-lan-plugin-routes`), а раунд 87 (C1.1) — сервисы `sessions`/`sessionProjections` из внутреннего снимка сервера (usage-маршруты реальных плагинов считают статистику по сессиям dshr; задержка ≤ 5 с, проекции упрощены); нет SSE/WebSocket/стриминга, аутентификации на маршрутах |
| 5.21 | Контракты слотов для settings-UI (C1.2): seat-`inject`, Cordis-генераторы, host-сервис `configForms` | 1.5 | ✅ раунд 89: регистрация слота может отдать props фабрикой `inject: () => ({...})` (ошибка деградирует компонент, не плагин); `ctx.slots.inject`/`ctx.inject`/`ctx.effect` принимают `function*` (тело выполняется, `yield`-disposer'ы снимаются в обратном порядке); `configForms.get(entryId)` даёт форму `{getSnapshot, subscribe, mutate(ops, revision)}` со стабильным snapshot и revision-контрактом — в кадре in-memory writable, у host-половин файловая (`<home>/cordis/config-forms/<entry>.json`). Эффект: `dsh-plugin-studio` рендерит вкладки в Settings, `dsh-plugin-debugging` — карточку в Plugins с переключателем и Save; smoke 82/82 (+2), bridge harness 90 (+3). Границы: browser-форма in-memory (без reload-персистентности), семантика конфигурационного движка DSH не воспроизведена, провалы seat-inject только логируются |
| 5.22 | Слот-контракт по референсному каталогу (C1.3): сгенерированный `slot-catalog.json` и правила `SlotCore` | 2 | ✅ раунд 91: каталог слотов сгенерирован из `deepseek-ai/deepseek-harness` (`packages/extensions/cordis-client-runner/src/client/slot-catalog.ts`, коммит `39163dd`) — 122 слота с `kind` (single 45 / list 48 / keyed 22 / chain 4 / hoc 3), `scope` (root 57 / session 60 / session-maybe 5), framework standard-kit и `inject`-флагом (`crates/cordis/src/slot-catalog.json`, host-страница публикует как `window.__DSHR_SLOT_META__`); регистрация повторяет reference `SlotCore`: undeclared бросает (кроме dshr extension-пространств `panel`/`message.action`/`custom.*`/`x-*`/`dshr.*`), `keyed`/`list`/`hoc`/`chain` требуют `key`/`id`/`id`/`select`, повторная регистрация в ячейку на том же `priority` бросает, другой `priority` затеняет (lowest renders), `children`-декларации живут/каскадно снимаются вместе с записью, `ctx.slots.inject` ждёт объявления слота, shell получает только shadowing-winners, props = standard-kit → seat `inject` → `ownerProps` → `t`, добавлен `ctx.slots.spec(key)`. Эффект: собственные слоты `dsh-mnemon` (`mnemon.component.*`) работают по референсным правилам; bridge harness 98 (+8), smoke 82/82. Границы: chain select-диспетчеризация, hoc-вложение, store seats и ownerProps от реальных shell-сайтов — остаток C-full (TECH-DEBT) |
| 5.23 | Host-сервисы по reference-контракту (C1.4): правила и origin-аксессоры `webServer`, `ctx.baseUrl`, reference-shape `configForms` | 1.5 | ✅ раунд 92: `webServer` приведён к `packages/host/webserver/src/index.ts` — exact/prefix — независимые таблицы, дубликат `(kind, path)` бросает `webserver: duplicate <kind> route`, путь — абсолютный pathname без trailing slash/query/hash и вне reserved-префиксов dshr API, `getOrigin()`/`port`/`protocol` отдают origin dshr-сервера из `<home>/cordis/host.json` (host-bridge), `registerFallback`/`registerUpgrade` бросают явную ошибку (долг); `ctx.baseUrl` — `file://` URL корня пакета (loader-context reference Cordis); `configForms` приведён к `config-form-types.ts`: snapshot `{status, value, base, user, revision: number, writable}` со стабильной идентичностью, `mutate(ops, expectedRevision?)` с атомарными `set`/`unset` и числовым revision-fence, хелперы `set`/`unset`; форма в кадре in-memory, у host-половин — файловая `<home>/cordis/config-forms/<entry>.json`. Проверки: bridge harness 102 (+4), `plugins_live` 16 (proxy-тест расширен duplicate-route/trailing-slash), smoke 82/82, workspace 365+3. Границы: `registerFallback`/`registerUpgrade` не поддержаны, browser-`configForms` не персистится через reload, живого RPC-зеркала настроек нет; остаток reference-работы — реальный порт ui-primitives из `packages/client/ui-primitives`, store seats/renderer, chain dispatch/hoc-nesting, ownerProps от реальных shell-сайтов |
| 5.24 | Порт reference `ui-primitives` (C2.1): атомы design-system и все иконки из исходников reference | 2 | ✅ раунд 93: `primitives.js` переписан по `deepseek-ai/deepseek-harness` @ `39163dd` (`packages/client/ui-primitives/src`) — `Button` (variants primary/ghost/outline/toolbar, sizes md/sm, leading icon, forwardRef), `Input`, `Switch` (role switch, aria-label), `Checkbox` (native input + label, data-modal-autofocus), `Tag` (`data-tone`), `Pill`, `SegmentedControl` (tablist+indicator, roving tabindex, walk-keys, id-value/-panel), `SegmentedTabs`, `DisclosureRow` (expandOnRowClick/previewChevron/collapsedContent/headerAccessory/running shimmer), `TextShimmer`, `StateDot` (done/warning/ongoing/error/idle, SVG spinner, appearance step), `PathLabel`, `Toast` (portal, role=alert, holdMs+fade), `Modal` (portal, mask, Escape, focus), `Menu`, `MenuSurface` (forwardRef, data-menu-material/phase), `Tooltip` (cloneElement-якорь); все **204** `Icon*` сгенерированы из upstream `icons/artwork.ts` (точные SVG-пути, stroke regular 1 / medium 1.3, инлайн вместо спрайта, Proxy-fallback для будущих имён); стили — `--dsw-alias-*` токены с fallback'ами; synthetic-фикстуры приведены к reference-контрактам (`StateDot state:"done"`, `Tooltip` element-child), C2-probe проверяет структурные признаки (role=switch/aria-label, data-tone, tablist, svg, label.dshr-checkbox, path-label textContent); smoke 82/82 (два прогона подряд), bridge harness 102, cordis 26, server 16, workspace 365+3 (desktop 21 — итого 386), web-e2e 16/16; общий рантайм-артефакт 375 894 байта (было 176 725; после C2.2 — 387 613 байт). Остаток на конец раунда 93: полный `Menu` (submenu/focus) и `Tooltip` (placement; доведён в C2.2, 5.25), упрощённые `MarkdownText`/`JsonBlock` (upstream требует shiki/katex/mdast), `Spinner`/`useAnchoredPosition` — dshr-расширения; store seats/renderer, chain dispatch/hoc-nesting, ownerProps от shell-сайтов |
| 5.25 | Полный `Tooltip` (C2.2): placement-engine, `ShortcutKeys` и input-modality | 1 | ✅ раунд 94: `Tooltip` портирован из `packages/client/ui-primitives/src/Tooltip.tsx` — label-резолвер (вычисляется при показе), независимые hover/focus-триггеры (`delayMs`/`focusDelayMs`), `openOnClick`-pinning с outside-pointer/Escape/Tab-dismiss, `onlyWhenClipped`, `maxWidth`, `portal`, `data-side`/`data-pinned`, nested-suppression через контекст, `aria-describedby` при pinning, pointer-modality (фокус после указателя не поднимает бабл), touch-hover исключён, forward ref сквозь клон якоря; `tooltip-placement.ts` портирован как есть — requested side → opposite → perpendicular, viewport-clamp, overflow-fallback; `input-modality.ts` (`data-input-modality`, `pointerModality`) и `ShortcutKeys` (keycaps с `+`-разделителями) портированы и экспортированы. Проверки: smoke 82/82 (C2-probe + узлы `Tooltip`/`ShortcutKeys`), bridge harness 102, cordis 26, server 16, workspace 365+3 (desktop 21 — итого 386), web-e2e 16/16; рантайм-артефакт 387 613 байт. Остаток: полный `Menu` (submenu/focus), `MarkdownText`/`JsonBlock`, store seats/renderer, chain dispatch/hoc-nesting, ownerProps от shell-сайтов |

**Выход:** существующие плагины DSH ставятся и работают, их доступ к ОС ограничен и зааудирован; нативные `.node`-плагины получают диагностику и sidecar-путь.

---

## M6 — Server + Web UI + SDK (15–25 ИД)

**Статус:** закрыто. REST/SSE, auth, approvals, JSON-RPC, Leptos CSR Web UI, Dockerfile; браузерный e2e (`scripts/web-e2e.py`) в CI; reproducible-бинарь проверен; бутылки в GHCR-канал требуют внешних учётных данных.

| № | Задача | ИД | DoD |
|---|---|---|---|
| 6.1 | `axum`-сервер: сессии, turns, events (SSE) | 3 | OpenAPI, контрактные тесты |
| 6.2 | Auth, CORS, rate-limit, статика | 1.5 | Негативные тесты |
| 6.3 | Approvals по API + отмена | 1.5 | E2E |
| 6.4 | Web UI: чат + стрим | 3 | ✅ `scripts/web-e2e.py` (headless Chrome/CDP, без npm): чат → стрим → карточка tool |
| 6.5 | Web UI: карточки, diff, approvals | 3 | ✅ карточки tool_start/tool_end с превью diff (`-old`/`+new`) и модалка approval проверены e2e (Allow → выполнение) |
| 6.6 | Web UI: сессии, настройки провайдеров | 2 | ✅ сессии: «New session» выбирает сессию и обновляет сайдбар; диалог Settings — маршрут/модель/`base_url`, запись user-слоя `config.yaml` (0600); ключи — из окружения |
| 6.7 | SDK: JSON-RPC сервер/клиент | 2.5 | Пример Python |
| 6.8 | E2E-сценарий полного цикла | 2 | ✅ джоба `web-e2e` в CI: чат → approval → Allow → выполнение → запись файла → журнал, второй ход с diff и настройки провайдера (16 проверок) |
| 6.9 | Упаковка: cargo-dist, brew, Docker | 2 | Установка с нуля |
| 6.10 | LLM: Anthropic + Gemini адаптеры | 2.5 | Кассеты, tool-calls, reasoning |
| 6.11 | MCP-сервер (stdio, tools/resources) | 2 | Пример сервера + интеграционный тест |

**Выход:** браузерный интерфейс и встраиваемый SDK.

---

> **C-full ([ADR-040](adr/040-react-client-half.md), [ADR-041](adr/041-c-full-decision.md)):** полный design-system, React-контекст — отложены; атомы примитивов и все 204 иконки портированы из reference в C2.1 (раунд 93: общий артефакт на конец раунда 375 894 байта), в C2.2 — полный `Tooltip` (раунд 94: артефакт 387 613 байт), остаются полный `Menu` и markdown-компоненты; слот-контракт по референсному каталогу закрыт в C1.3 (раунд 91: 122 слота, правила `SlotCore`, children/deferred inject, winners, standard-kit + `ownerProps`), host-сервисы `webServer`/`ctx.baseUrl`/`configForms` выровнены по reference-контракту в C1.4 (раунд 92), остаток SlotMap — chain select-диспетчеризация, hoc-вложение, store seats (`PropsStore`/`BoundActions`) и ownerProps от реальных shell-сайтов. Возврат по критерию ≥3 реальных плагинов, которым не хватает этих возможностей (сейчас таких нет). Точечные кандидаты: host-сервисы `pluginManager`/`settings`, chain/hoc-рендеринг, store/renderer-семантика, streaming-прокси.

## M7 — Паритет (эпики, 84–145 ИД)

**Статус:** срез 15.11.2026 — M0–M7 закрыты (365 тестов, +3 ignored; с desktop 21 — итого 386); Leptos + Tauri + PWA + WIT; P2 Cordis — sidecar + hot reload + клиентская половина с региональным монтажом слотов, экспериментальным `dsh.client.runtime`, PoC тира B `dsh.client.host_modules` (React от хоста) и C-lite (React-компоненты слотов с host-hooks/`ctx.get('sessions')`, сервисы `ctx.get('slots')`/`locale`/`ctx.inject`, диагностика `__DSHR_LAST_ERROR__`, React-контракт: типы `React.memo`/`forwardRef`, keyed-ids, `locale.register`/`t()`, `ctx.interval`/`ctx.timeout`, design-system примитивы (с раунда 93, C2.1 — reference-порт: атомы и все 204 иконки из `deepseek-ai/deepseek-harness` @ `39163dd`; с раунда 94, C2.2 — полный `Tooltip`), conformance n=4 (раунд 82 — n=6, раунд 83 — **n=9**: цель ADR-013 §6 8–12 реальных плагинов достигнута); раунд 84 — C1 host HTTP-маршруты плагинов (sidecar + allowlist-прокси + `connect-src`, реальная host-половина balance-плагина), остатки — стриминг, отсутствие аутентификации; раунд 85 — C2: design-system шим с 30+ экспортами и conformance n=11 (`dsh-plugin-debugging`/`dsh-plugin-studio` активируются, их settings-UI был инертен без host-сервисов `pluginManager`/settings — с раунда 89 (C1.2) рендерится через seat-`inject`/`configForms`), smoke раунда 85 — 79/79; раунд 86 — хардненинг C1: prefix-маршруты, `routes` в `GET /v1/plugins/status`, фильтр MCP-псевдо-инструментов, loopback-only политика (403 без `--allow-lan-plugin-routes`); раунд 87 — C1.1: сервисы `sessions`/`sessionProjections` из внутреннего снимка сервера (usage-маршруты реальных плагинов считают статистику по сессиям dshr; задержка ≤ 5 с, проекции упрощены), раунд 89 — C1.2: seat-`inject`, Cordis-генераторы и `configForms` (in-memory форма в кадре, файловая — у host-половин), поэтому settings-UI studio/debugging реально рендерится; smoke раунда 89 — 82/82; раунд 91 — C1.3: слот-контракт по сгенерированному каталогу DSH (122 слота с kind/scope/standard-kit, правила `SlotCore`: undeclared/keyed/chain/duplicate, winners, children/deferred inject, standard-kit + ownerProps, `ctx.slots.spec(key)`); раунд 93 — C2.1: атомы design-system и все 204 иконки портированы из reference (`deepseek-ai/deepseek-harness` @ `39163dd`, `packages/client/ui-primitives/src`; артефакт на конец раунда 375 894 байта), раунд 94 — C2.2: полный reference-порт `Tooltip` (placement-engine, `ShortcutKeys`, `input-modality`; артефакт 387 613 байт), полный Menu и markdown-компоненты остаются упрощёнными); Replica 98.1/100, must 13/13; полная React-совместимость — решение по C-full ADR-040, а не остаток C-lite); Windows отложен. DoD-гейты M7 — см. ниже.

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

**DoD-гейты M7** (критерий выхода: eval-набор + perf-бюджеты как CI-гейт):

| № | Гейт | Статус |
|---|---|---|
| 7.1 | Общий eval-набор: 10 задач, success rate 100 %, бюджет токенов на задачу | ✅ `crates/cli/tests/evals.rs`, джоба `evals` |
| 7.2 | Perf-бюджеты §20 как CI-гейт: промпт, append+fsync, поиск 10k, turn-context 100k, память 100k | ✅ `crates/perf/tests/budgets.rs`, джоба `perf`, регресс > 20 % — блокер |
| 7.3 | Матрица §4 заполнена для T3 | ⏳ заполняется, см. PLAN §4 |

---

## Контрольные точки

| Точка | Когда | Проверка |
|---|---|---|
| G0 | конец M0 | ✅ пройдена 12.11.2026: `doctor`, конфиг, тесты |
| G1 | конец M1 | ✅ пройдена 12.11.2026: cassette replay, живой smoke DeepSeek |
| G2 | конец M2 | ✅ ядро: tool-loop + approvals + TUI e2e; eval-набор отложен |
| G3 | конец M3 | ✅ live-гейт 100k: `crates/compaction/tests/live_gate_100k.rs` (compaction + reopen + продолжение) |
| G4 | конец M4 | ✅ MCP mock-сервер + субагент из кассеты; HTTP/resources отложены |
| G5 | конец M5 | ✅ Seatbelt, Landlock+seccomp и AppContainer: escape + workspace-write + сеть Linux |
| G6 | конец M6 | ✅ HTTP/SSE + встроенный UI; браузерный e2e `scripts/web-e2e.py` в CI (CDP, без Playwright-пакета) |
| G7 | M7 | матрица возможностей заполнена |

---

## Журнал изменений плана

| Дата | Изменение | Причина |
|---|---|---|
| 12.11.2026 | v1.0 — исходный план | Старт проекта |
| 12.11.2026 | v1.1 — улучшенная Rust-реализация; M0 выполнен | Уточнение позиционирования; каркас собран |
| 12.11.2026 | M1 выполнен: LLM, сессии, headless | 70 тестов; DeepSeek проверен живым прогоном |
| 12.11.2026 | M2 ядро: инструменты, агент, approvals, TUI | 112 тестов; живой tool-loop |
