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
| `ClientAssets!Attempt` | `ServedIssued`, `ServedConfined`, `OutsideNeverAllowed` | CLI-12, CLI-13, SRV-11 |
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
| CLI-21 | 82 сквозных проверки в headless Chrome | E | ✅ `scripts/plugin-smoke.py` |
| CLI-22 | Eager registry: скрытый `mode=registry`-кадр на каждую половину, порядок `inject`/`immediately` | E | ✅ smoke |
| CLI-23 | Региональный монтаж: регистрация монтируется в регион по таблице `slot key → регион` (17 регионов шелла) | E | ✅ smoke |
| CLI-24 | Полный словарь: каталог DSH 0.2.2-rc.2 (122 слота с `kind`/`scope`/standard-kit после раунда 91) + legacy `panel`/`message.action` (всего 124 ключа) покрыты таблицей регионов | U | ✅ `crates/cordis/tests/bridge_harness.mjs` + `crates/cordis/src/slot-catalog.json` |
| CLI-25 | Незнакомое имя слота → fallback в панель плагина (`panel`) | E | ✅ smoke |
| CLI-26 | `context` повторяющихся регионов: сообщение `{index, role}`, карточка инструмента, строка сессии | E | ✅ smoke |
| CLI-27 | Адресный `resize`: высота конкретного slot-кадра, кламп 24–640 px | E | ✅ smoke |
| CLI-28 | `reload`: версия поднимается у всех фреймов плагина, открытая панель остаётся | E | ✅ smoke |
| CLI-29 | `dsh.client.runtime`: discovery (поле `runtime` в `/v1/plugins/clients`), host-страница исполняет runtime до бандла, module-table бандл резолвит `require` из `window.__DSHR_MODULES__` | I | ✅ `crates/cordis/src/client.rs` + `plugins_live.rs` + smoke (React 18.3.1 PoC — вручную) |
| CLI-30 | `dsh.client.host_modules` (тир B): discovery (allowlist id, поле `host_modules` в `/v1/plugins/clients`, незнакомый id → отказ), host-страница ссылается на `/v1/runtime/react-18.3.1.js` и не встраивает React, порядок shared → plugin runtime → bundle, без opt-in shared-скрипта нет | I | ✅ `crates/cordis/src/client.rs` + `plugins_live.rs` + smoke (`hosted`: React из общего рантайма + `useState`) |
| CLI-31 | React-компоненты слотов (C-lite): при host React `ctx.slots.register(entry, Component)` рендерится как React-компонент в одном root на кадр; `host-state` (`theme`/`wide`/`session`/`sessions`) обновляет props/hooks (`useTheme`/`useSession`/`useSessions`/`useHostState`); ошибка рендера ловится error boundary и доходит до шелла; `ctx.get('sessions')` отдаёт подмножество референсного сервиса; принимаются React-типы-объекты (`React.memo`/`forwardRef` с `$$typeof`); регистрация с `key` без `id` получает уникальный id `<slot>:<key>`; компонент с `locale` получает `t()`-место (словарь из `ctx.locale.register`); `ctx.interval`/`ctx.timeout` снимаются вместе с фреймом; `ctx.get('slots').entries(key)` отдаёт `component`/`options` | I | ✅ `crates/cordis/src/bridge.js` + `tests/bridge_harness.mjs` (98 проверок) + smoke (фикстуры `hooks`, `hosted`, real plugins); `wide` пока всегда true — ограничение |
| CLI-32 | Conformance: реальный MIT-плагин `@kt11/dsh-deepseek-balance@0.2.1` (vendored fixture, клиентская половина upstream без изменений, sha256 в `package.json`) рендерит React-карточку в `sidebar.footer.action`, вставляет собственный CSS и открывает popup по клику; с раунда 84 host-половина тоже реальная (upstream дословно, host sha256 `8771ae90…`) и её маршруты отвечают через C1-прокси (без ключа — 500 `not-configured`) | E | ✅ smoke |
| CLI-33 | Частичный design-system шим `@deepseek-ai/dsh-client-ui-primitives`: id принят `HOST_MODULES`, общий рантайм публикует `Button`/`StateDot`/`Tooltip`/`Icon`/`Spinner`/`JsonBlock`/`MarkdownText` (с раунда 81 — 7 компонентов; с раунда 85 (C2) — 30+ экспортов: ещё `Input`/`Switch`/`Checkbox`/`Tag`/`Pill`/`Menu`/`MenuSurface`/`Modal`/`Toast`/`SegmentedControl`/`SegmentedTabs`/`DisclosureRow`/`PathLabel`/`TextShimmer`, `writeClipboard`, хуки `useAnchoredPosition`/`useDismissOnOutsidePointer` и 18 именованных `Icon*` с Proxy-fallback), бандл резолвит их через `require` | U/E | ✅ `crates/cordis/src/client.rs` (allowlist) + `crates/server/assets/runtime/primitives.js` + smoke (фикстура `ui-kit`, C2-проба всех примитивов) |
| CLI-34 | Conformance n=2: второй реальный MIT-плагин `dsh-remote@1.1.2` (vendored fixture, клиентская половина upstream без изменений, sha256 `5016e5d3…` в `package.json`, host-половина — заглушка) обнаруживается с примитивами в `host_modules`, его registry-кадр активируется без ошибок при отсутствии опциональных сервисов (`betterSidebar`/`sidebarRight`/`webTerminals`), а React-слот `conversation.session.header.actions` монтируется в регион `conversation.header` | E | ✅ smoke (фикстура `crates/cordis/tests/fixtures/dsh-remote/`) |
| CLI-35 | Сервисы моста для референсных плагинов: `ctx.get('slots')` — alias реестра слотов с `entries(key)`, `register` через него публикует слот; `ctx.get('locale')` отдаёт `{active: "en", locale: "en"}` и `subscribe` (с раунда 81 — ещё `register(ns, dicts)`); `ctx.inject(deps, callback)` вызывает callback только когда резолвятся все зависимости (неизвестный сервис — callback не вызывается); последняя ошибка кадра доступна как `window.__DSHR_LAST_ERROR__` (`{message, stack}`) | I | ✅ `crates/cordis/src/bridge.js` + `tests/bridge_harness.mjs` (98 проверок) |
| CLI-36 | React-контракт моста (раунд 81): `React.memo`/`forwardRef` типы принимаются как компонент слота в host-React-режиме; компонент получает референсные host-props (`wide`, `useTheme`, `useSession`) и рабочее `t()`-место для своего locale (`{name}`-подстановки); keyed-регистрации без `id` получают различные id `<slot>:<key>`; `ctx.get('slots').entries(key)` отдаёт `component` и `options` | I | ✅ `crates/cordis/src/bridge.js` + `tests/bridge_harness.mjs` (`reactScenario`, 5 проверок; всего 98) |
| CLI-37 | Conformance n=4: `@ruoyang/dsh-plugin-deepseek-balance@1.0.2` (MIT, vendored fixture, клиентская половина upstream без изменений, sha256 `ca700695…`) рендерит плавающий бейдж в `shell.overlay` через `ctx.effect`/`ctx.interval`/`ctx.locale`; `dsh-genui@0.3.3` (MIT, sha256 `8f0500ad…`) — тумблер GenUI в `conversation.input.left` и keyed-регистрация `conversation.chat.node`; оба кадра активируются без ошибок, два новых плагина видны в discovery и registry | E | ✅ smoke (фикстуры `crates/cordis/tests/fixtures/{ruoyang-balance,dsh-genui}/`) |
| CLI-38 | Conformance n=6: `dsh-session-superagent@0.10.12` (MIT, vendored fixture, клиентская половина upstream без изменений, sha256 `d58f0fa2…`, host-половина — заглушка) коммитит composer dock (`conversation.composer.dock`) и активируется без ошибок; `dsh-mnemon@0.5.46` (MIT, sha256 `d967402b…`) — крупный multi-region плагин: активируется, публикует слоты и сверяет собственные shadow-регистрации через референсную поверхность `ctx.slots.subscribe`/`getVersion`/`entriesOfSlot`; оба плагина видны в discovery, их registry-кадры поднимаются без ошибок | E | ✅ smoke (фикстуры `crates/cordis/tests/fixtures/{dsh-session-superagent,dsh-mnemon}/`, всего 69 проверок) |
| CLI-39 | Conformance n=9 (цель ADR-013 §6 8–12 достигнута): три новых реальных MIT-плагина с upstream-клиентской половиной без изменений (sha256 в `package.json`, host-половины — заглушки): `@99galaxy/dsh-plugin-market@1.2.2` (`sidebar.panellist` → регион `sidebar.footer`, только `react`) — активируется, публикует слоты, его sidebar-кадр монтируется; `dsh-plugin-codegraph-panel@0.6.5` (панель `settings.section`; upstream `immediately: true`; только `react`) — активируется и публикует слот; `@goodandready/dsh-plugin-notify@0.4.9` (`plugins.item`/`plugins.row.config` → регион `plugins.detail`, primitives + react) — активируется и публикует слот; все три видны в discovery, их registry-кадры поднимаются без ошибок | E | ✅ smoke (фикстуры `crates/cordis/tests/fixtures/{dsh-plugin-market,dsh-plugin-codegraph-panel,dsh-plugin-notify}/`, всего 75 проверок) |
| CLI-40 | **C1 host HTTP-маршруты плагинов**: host-половина регистрирует маршрут (`kind: "exact"`/`kind: "prefix"`) через `ctx.inject(['webServer'])`/`webServer.register`; sidecar поднимает loopback-only сервер (per-process токен), `__dshr_routes` отдаёт порт/токен/таблицу; dshr-сервер проксирует только allowlist-пути (GET/query/POST, request ≤ 1 МиБ, response ≤ 8 МиБ, таймаут 15 с, permissive CORS для opaque-origin кадров; reserved-префиксы `/v1`/`/dshr`/`/openapi`/UI-ассеты и неизвестные пути → dshr 404; reload/stop убирает маршруты); кадры с `host_modules` получают `connect-src <origin>`, кадры без него — `connect-src 'none'`; реальная host-половина `@kt11/dsh-deepseek-balance` отвечает через кадр плагина (`/dsh-balance/deepseek` → 500 JSON `{ok:false, code:"not-configured"}`, `/dsh-usage/sessions?tz=UTC` → 200 JSON с `sessions`, unknown → 404) | I/E | ✅ `plugins_live.rs` (`proxies_plugin_host_routes`, `plain_frames_stay_network_free`) + smoke 79/79 |
| CLI-41 | **C2: расширенный design-system шим и primitives-heavy conformance**: шим вырос с 7 до 30+ экспортов по фактическим импортам реальных плагинов — компоненты `Input`/`Switch`/`Checkbox`/`Tag`/`Pill`/`Menu`/`MenuSurface`/`Modal`/`Toast`/`SegmentedControl`/`SegmentedTabs`/`DisclosureRow`/`PathLabel`/`TextShimmer`, именованные `Icon*` с SVG-глифами и Proxy-fallback для неизвестных имён, `writeClipboard` и хуки `useAnchoredPosition`/`useDismissOnOutsidePointer`; smoke рендерит дерево всех смоделированных примитивов в host-React кадре (12 текстовых маркеров, портальные Menu/Modal/Toast) и проверяет, что реальные вклады `dsh-plugin-notify`/`dsh-mnemon` монтируются в диалоге Plugins без `__DSHR_LAST_ERROR__`; conformance n=11 — `dsh-plugin-debugging@0.8.1` (client sha256 `9b828160…`) и `dsh-plugin-studio@0.3.1` (client sha256 `1194c18b…`) активируются без ошибок; на момент C2 их UI на отсутствующих host-сервисах `pluginManager`/settings оставался несмонтированным, а раунд 89 (C1.2) его оживил — studio рендерит вкладки в Settings, debugging — карточку в Plugins (см. CLI-44) | U/E | ✅ `crates/server/assets/runtime/primitives.js` + smoke 79/79 на момент C2 (после C1.2 — 82/82; фикстуры `crates/cordis/tests/fixtures/{dsh-plugin-debugging,dsh-plugin-studio}/`) |
| CLI-42 | **Хардненинг C1**: prefix-маршруты матчатся в sidecar, `CordisManager::http_route` и fallback-прокси одинаково (`/probe-prefix` и `/probe-prefix/a/b` → 200, exact выигрывает у prefix, среди prefix — самый длинный); `GET /v1/plugins/status` отдаёт per-plugin `routes` (живая таблица host-маршрутов) рядом с `tools`; MCP-псевдо-инструменты `list_resources`/`read_resource`/`list_prompts`/`get_prompt` не попадают в реестр инструментов Cordis-плагина (реальные инструменты, например `cordis__demo__probe`, остаются); при bind вне loopback (`0.0.0.0:0`) plugin-маршрут отвечает 403, а с `ServerConfig.allow_plugin_routes_lan`/`--allow-lan-plugin-routes` — 200; reserved- и неизвестные пути по-прежнему 404 | I | ✅ `plugins_live.rs` 16 checks (раунд 86: `proxies_plugin_host_routes` расширен prefix/status/meta-tools, `plugin_routes_are_loopback_only_unless_opted_in`; раунд 87 добавил `internal_session_snapshot_feeds_the_host_bridge`) + `crates/cordis/src/bootstrap.mjs`/`manager.rs`/`lib.rs` + `crates/cli/src/serve_cmd.rs` |
| CLI-43 | **C1.1: мост `sessions`/`sessionProjections`**: сервер пишет `<home>/cordis/host.json` (`{url, token}`) и публикует `GET /v1/internal/session-snapshot` (recent ≤50 сессий: id/title/created_at/model/turns/steps и usage-события assistant; содержимое сообщений и промпты не покидают сервер; 404 без настроенного токена, 401 при неверном `x-dshr-internal-token`); sidecar получает путь через `DSHR_HOST_BRIDGE`, раз в 5 с обновляет снимок и строит DSH-shaped `ctx.get('sessions')` (`list()`/`get(id)`, `header.createdAt` в epoch ms, `log` с `request/context`, `session/title`, `assistant/message` usage) и `ctx.get('sessionProjections')` (`snapshot(session).values`: `sessionStats` turns/steps/`llmMs`=0 и `tokenUsage`; данные могут отставать до 5 с); usage-маршрут реального `@kt11/dsh-deepseek-balance` (`/dsh-usage/sessions?tz=UTC`) возвращает `sessions` ≥ 1 и totals | I/E | ✅ `plugins_live.rs` (`internal_session_snapshot_feeds_the_host_bridge`: usage-поля, 401, 404) + `crates/cordis/src/bootstrap.mjs` (`refreshHostSessions`/`dshSessionFromSnapshot`) + smoke 80/80 (C1.1 bridge) |
| CLI-44 | **C1.2: seat `inject`, Cordis-генераторы и `configForms`** — контракты, без которых settings-UI реальных плагинов оставался инертным: (1) регистрация слота может отдать компоненту дополнительные props фабрикой `inject: () => ({...})`; они мержатся при монтировании в React-режиме (вместе с host-props) и в imperative-колбэке, а брошенная фабрика (нет host-сервиса) деградирует компонент, пишет событие в лог кадра и **не** помечает плагин упавшим; (2) `ctx.slots.inject`, `ctx.inject` и `ctx.effect` принимают `function*` — тело выполняется, все `yield`-disposer'ы собираются и вызываются на teardown в обратном порядке; (3) `ctx.get('configForms')`/`ctx.configForms.get(entryId)` отдаёт форму `{getSnapshot, subscribe, mutate(ops, revision)}`: snapshot-объект стабилен между нотификациями (требование `useSyncExternalStore`), `mutate` соблюдает revision-контракт (устаревшая ревизия → `false`), форма в кадре in-memory writable (правки не переживают reload кадра), а host-половинам bootstrap пишет файловый вариант `<home>/cordis/config-forms/<entry>.json`. Эффект: real-плагин `dsh-plugin-studio@0.3.1` рендерит реальные вкладки в Settings (`settings.plugins.tab`, собственный CSS, честные error-состояния при заглушках host-маршрутов), а `dsh-plugin-debugging@0.8.1` — карточку настроек в Plugins (`plugins.bundle.config`), переключатель и Save проходят round-trip через `configForms` | I/E | ✅ `crates/cordis/src/bridge.js` (`injectedProps`, `cordisDisposer`, `browserConfigForm`, `configFormsService`), `crates/cordis/src/bootstrap.mjs` (файловый `configForms`), `tests/bridge_harness.mjs` 98 (+3 в C1.2: seat-props, генераторный `slots.inject`, генераторный `effect`; +8 в C1.3 — см. CLI-45) + smoke 82/82 (+2: settings-вкладка studio и карточка debugging) |
| CLI-45 | **C1.3: слот-контракт по референсному каталогу (`SlotCore`-semantics)** — регистрации проверяются по сгенерированному каталогу DSH (`deepseek-ai/deepseek-harness` @ `39163dd`, `cordis-client-runner/src/client/slot-catalog.ts`): 122 слота с `kind` (single 45 / list 48 / keyed 22 / chain 4 / hoc 3), `scope` (root 57 / session 60 / session-maybe 5), framework standard-kit и `inject`-флагом (`crates/cordis/src/slot-catalog.json`, host-страница публикует как `window.__DSHR_SLOT_META__`). (1) неизвестный слот → `slot "<name>" is not declared` (исключение — dshr extension-пространства `panel`/`message.action`/`custom.*`/`x-*`/`dshr.*`); (2) `keyed` требует `key`, `list`/`hoc` — `id`, `chain` — `select`; (3) повторная регистрация в ячейку (`single` — слот, `keyed` — key, иначе id) на том же `priority` бросает, другой `priority` затеняет (рендерится меньший); (4) регистрация может объявить дочерние слоты `children: {key: {kind, scope}}`, которые живут и каскадно снимаются вместе с записью — это разблокировало `dsh-mnemon` и его `mnemon.component.*`; (5) `ctx.slots.inject(key, factory)` ждёт объявления слота и перезапускается при появлении (reference registry semantics); (6) shell получает только shadowing-winners (`single` — один, `keyed` — по одному на ключ, `list`/`chain`/`hoc` — все слои в порядке priority → order → регистрация); (7) props компонента = standard-kit из каталога (нереализованные hooks остаются вызываемыми и возвращают `undefined`, `sessionId` — из host-state) → seat `inject` → `ownerProps` из `window.__DSHR_FRAME__.ownerProps` → `t`; (8) `ctx.slots.spec(key)` отдаёт `{key, kind, scope, standardProps, inject}`. Границы: `chain` не диспатчится по `select` (монтируется слоями), `hoc` не вкладывает `children` между кадрами, store seats (`PropsStore`/`BoundActions`) и ownerProps от реальных render-сайтов шелла — остаток C-full | I/E | ✅ `crates/cordis/src/bridge.js` (`SLOT_META`/`declaredSlots`, `declareChildSlots`/`undeclareChildSlots`, `normalize`, `winners`, `slotStandardProps`, `frameOwnerProps`, `slots.inject`/`spec`) + `tests/bridge_harness.mjs` 98 (+8: undeclared/keyed/chain/duplicate throws, single shadowing, catalog standard kit, ownerProps, plain values) + smoke 82/82 (реальные плагины регистрируются по тем же правилам; synthetic-фикстуры дополнены обязательным `key`, словарь — 124 ключа) |

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
| SRV-15 | `/host`: валидация `frame`/`mode`/`slot`/`id`/`context`, 400 на мусор; slot-режим требует `slot`+`id` | I | ✅ `plugins_live.rs` |
| SRV-16 | `/host` с `dsh.client.runtime`: `</script>` в runtime → 400; runtime больше 8 МиБ → 413 | I | ✅ `plugins_live.rs` (`rejects_a_runtime_that_closes_the_script_tag`, `rejects_an_oversized_plugin_runtime`) |
| SRV-17 | `GET /v1/runtime/react-18.3.1.js`: immutable-кэш и `nosniff`, неизвестный файл/версия → 404; opt-in host-страница ссылается на рантайм и не встраивает React; враждебный `Host` не попадает в CSP opt-in-кадра | I | ✅ `plugins_live.rs` (маршрут, заголовки, 404, CSP с origin) |
| SRV-18 | Fallback-прокси plugin host-маршрутов: allowlist из `CordisManager` (exact и prefix, exact выигрывает, среди prefix — самый длинный), method/query/body пробрасываются, reserved- и неизвестные пути → 404, внутренние `__dshr_*`-инструменты и MCP-псевдо-инструменты (`list_resources`/`read_resource`/`list_prompts`/`get_prompt`) не попадают в реестр, `GET /v1/plugins/status` отдаёт live `routes`; маршруты снимаются при stop плагина; при bind вне loopback без `allow_plugin_routes_lan` прокси отвечает 403 | I | ✅ `plugins_live.rs` (`proxies_plugin_host_routes`, `plugin_routes_are_loopback_only_unless_opted_in`, `plain_frames_stay_network_free`) |

## Web UI

| ID | Кейс | Ур. | Статус |
|---|---|---|---|
| WEB-01 | Тема light/dark, сохранение в localStorage, `?theme=` | E | ✅ headless |
| WEB-02 | Подключение по токену, список сессий, стрим ответа | E | ✅ |
| WEB-03 | Approval/question диалоги | E | ✅ |
| WEB-04 | Панель Plugins: каталог, оценки/отзывы, install | E | ✅ |
| WEB-05 | Клиентские половины: региональный монтаж слотов (17 регионов), eager registry, resize, тема | E | ✅ smoke |
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
| Строки | 15 303 | 20 984 | **72.9%** |
| Функции | 1 626 | 2 429 | 66.9% |
| Регионы | 13 980 | 18 601 | 75.2% |

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
| `server/src/lib.rs` | 70.0% |
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

