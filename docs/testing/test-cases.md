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
| CLI-31 | React-компоненты слотов (C-lite): при host React `ctx.slots.register(entry, Component)` рендерится как React-компонент в одном root на кадр; `host-state` (`theme`/`wide`/`session`/`sessions`) обновляет props/hooks (`useTheme`/`useSession`/`useSessions`/`useHostState`); ошибка рендера ловится error boundary и доходит до шелла; `ctx.get('sessions')` отдаёт подмножество референсного сервиса; принимаются React-типы-объекты (`React.memo`/`forwardRef` с `$$typeof`); регистрация с `key` без `id` получает уникальный id `<slot>:<key>`; компонент с `locale` получает `t()`-место (словарь из `ctx.locale.register`); `ctx.interval`/`ctx.timeout` снимаются вместе с фреймом; `ctx.get('slots').entries(key)` отдаёт `component`/`options` | I | ✅ `crates/cordis/src/bridge.js` + `tests/bridge_harness.mjs` (102 проверки) + smoke (фикстуры `hooks`, `hosted`, real plugins); `wide` пока всегда true — ограничение |
| CLI-32 | Conformance: реальный MIT-плагин `@kt11/dsh-deepseek-balance@0.2.1` (vendored fixture, клиентская половина upstream без изменений, sha256 в `package.json`) рендерит React-карточку в `sidebar.footer.action`, вставляет собственный CSS и открывает popup по клику; с раунда 84 host-половина тоже реальная (upstream дословно, host sha256 `8771ae90…`) и её маршруты отвечают через C1-прокси (без ключа — 500 `not-configured`) | E | ✅ smoke |
| CLI-33 | design-system примитивы `@deepseek-ai/dsh-client-ui-primitives`: id принят `HOST_MODULES`, общий рантайм публикует атомы (с раунда 93, C2.1 — портированы из reference-исходников `deepseek-ai/deepseek-harness` @ `39163dd`, `packages/client/ui-primitives/src`) и все **204** `Icon*` (точные SVG-пути, инлайн вместо спрайта, Proxy-fallback), бандл резолвит их через `require` | U/E | ✅ `crates/cordis/src/client.rs` (allowlist) + `crates/server/assets/runtime/primitives.js` + smoke (фикстура `ui-kit`, C2-probe) |
| CLI-34 | Conformance n=2: второй реальный MIT-плагин `dsh-remote@1.1.2` (vendored fixture, клиентская половина upstream без изменений, sha256 `5016e5d3…` в `package.json`, host-половина — заглушка) обнаруживается с примитивами в `host_modules`, его registry-кадр активируется без ошибок при отсутствии опциональных сервисов (`betterSidebar`/`sidebarRight`/`webTerminals`), а React-слот `conversation.session.header.actions` монтируется в регион `conversation.header` | E | ✅ smoke (фикстура `crates/cordis/tests/fixtures/dsh-remote/`) |
| CLI-35 | Сервисы моста для референсных плагинов: `ctx.get('slots')` — alias реестра слотов с `entries(key)`, `register` через него публикует слот; `ctx.get('locale')` отдаёт `{active: "en", locale: "en"}` и `subscribe` (с раунда 81 — ещё `register(ns, dicts)`); `ctx.inject(deps, callback)` вызывает callback только когда резолвятся все зависимости (неизвестный сервис — callback не вызывается); последняя ошибка кадра доступна как `window.__DSHR_LAST_ERROR__` (`{message, stack}`) | I | ✅ `crates/cordis/src/bridge.js` + `tests/bridge_harness.mjs` (102 проверки) |
| CLI-36 | React-контракт моста (раунд 81): `React.memo`/`forwardRef` типы принимаются как компонент слота в host-React-режиме; компонент получает референсные host-props (`wide`, `useTheme`, `useSession`) и рабочее `t()`-место для своего locale (`{name}`-подстановки); keyed-регистрации без `id` получают различные id `<slot>:<key>`; `ctx.get('slots').entries(key)` отдаёт `component` и `options` | I | ✅ `crates/cordis/src/bridge.js` + `tests/bridge_harness.mjs` (`reactScenario`, 5 проверок; всего 102) |
| CLI-37 | Conformance n=4: `@ruoyang/dsh-plugin-deepseek-balance@1.0.2` (MIT, vendored fixture, клиентская половина upstream без изменений, sha256 `ca700695…`) рендерит плавающий бейдж в `shell.overlay` через `ctx.effect`/`ctx.interval`/`ctx.locale`; `dsh-genui@0.3.3` (MIT, sha256 `8f0500ad…`) — тумблер GenUI в `conversation.input.left` и keyed-регистрация `conversation.chat.node`; оба кадра активируются без ошибок, два новых плагина видны в discovery и registry | E | ✅ smoke (фикстуры `crates/cordis/tests/fixtures/{ruoyang-balance,dsh-genui}/`) |
| CLI-38 | Conformance n=6: `dsh-session-superagent@0.10.12` (MIT, vendored fixture, клиентская половина upstream без изменений, sha256 `d58f0fa2…`, host-половина — заглушка) коммитит composer dock (`conversation.composer.dock`) и активируется без ошибок; `dsh-mnemon@0.5.46` (MIT, sha256 `d967402b…`) — крупный multi-region плагин: активируется, публикует слоты и сверяет собственные shadow-регистрации через референсную поверхность `ctx.slots.subscribe`/`getVersion`/`entriesOfSlot`; оба плагина видны в discovery, их registry-кадры поднимаются без ошибок | E | ✅ smoke (фикстуры `crates/cordis/tests/fixtures/{dsh-session-superagent,dsh-mnemon}/`, всего 69 проверок) |
| CLI-39 | Conformance n=9 (цель ADR-013 §6 8–12 достигнута): три новых реальных MIT-плагина с upstream-клиентской половиной без изменений (sha256 в `package.json`, host-половины — заглушки): `@99galaxy/dsh-plugin-market@1.2.2` (`sidebar.panellist` → регион `sidebar.footer`, только `react`) — активируется, публикует слоты, его sidebar-кадр монтируется; `dsh-plugin-codegraph-panel@0.6.5` (панель `settings.section`; upstream `immediately: true`; только `react`) — активируется и публикует слот; `@goodandready/dsh-plugin-notify@0.4.9` (`plugins.item`/`plugins.row.config` → регион `plugins.detail`, primitives + react) — активируется и публикует слот; все три видны в discovery, их registry-кадры поднимаются без ошибок | E | ✅ smoke (фикстуры `crates/cordis/tests/fixtures/{dsh-plugin-market,dsh-plugin-codegraph-panel,dsh-plugin-notify}/`, всего 75 проверок) |
| CLI-40 | **C1 host HTTP-маршруты плагинов**: host-половина регистрирует маршрут (`kind: "exact"`/`kind: "prefix"`) через `ctx.inject(['webServer'])`/`webServer.register`; sidecar поднимает loopback-only сервер (per-process токен), `__dshr_routes` отдаёт порт/токен/таблицу; dshr-сервер проксирует только allowlist-пути (GET/query/POST, request ≤ 1 МиБ, response ≤ 8 МиБ, таймаут 15 с, permissive CORS для opaque-origin кадров; reserved-префиксы `/v1`/`/dshr`/`/openapi`/UI-ассеты и неизвестные пути → dshr 404; reload/stop убирает маршруты); кадры с `host_modules` получают `connect-src <origin>`, кадры без него — `connect-src 'none'`; реальная host-половина `@kt11/dsh-deepseek-balance` отвечает через кадр плагина (`/dsh-balance/deepseek` → 500 JSON `{ok:false, code:"not-configured"}`, `/dsh-usage/sessions?tz=UTC` → 200 JSON с `sessions`, unknown → 404) | I/E | ✅ `plugins_live.rs` (`proxies_plugin_host_routes`, `plain_frames_stay_network_free`) + smoke 79/79 |
| CLI-41 | **C2: расширенный design-system шим и primitives-heavy conformance**: шим вырос с 7 до 30+ экспортов по фактическим импортам реальных плагинов — компоненты `Input`/`Switch`/`Checkbox`/`Tag`/`Pill`/`Menu`/`MenuSurface`/`Modal`/`Toast`/`SegmentedControl`/`SegmentedTabs`/`DisclosureRow`/`PathLabel`/`TextShimmer`, именованные `Icon*` с SVG-глифами и Proxy-fallback для неизвестных имён, `writeClipboard` и хуки `useAnchoredPosition`/`useDismissOnOutsidePointer`; smoke рендерит дерево всех смоделированных примитивов в host-React кадре (12 текстовых маркеров, портальные Menu/Modal/Toast) и проверяет, что реальные вклады `dsh-plugin-notify`/`dsh-mnemon` монтируются в диалоге Plugins без `__DSHR_LAST_ERROR__`; conformance n=11 — `dsh-plugin-debugging@0.8.1` (client sha256 `9b828160…`) и `dsh-plugin-studio@0.3.1` (client sha256 `1194c18b…`) активируются без ошибок; на момент C2 их UI на отсутствующих host-сервисах `pluginManager`/settings оставался несмонтированным, а раунд 89 (C1.2) его оживил — studio рендерит вкладки в Settings, debugging — карточку в Plugins (см. CLI-44) | U/E | ✅ `crates/server/assets/runtime/primitives.js` + smoke 79/79 на момент C2 (после C1.2 — 82/82; фикстуры `crates/cordis/tests/fixtures/{dsh-plugin-debugging,dsh-plugin-studio}/`) |
| CLI-42 | **Хардненинг C1**: prefix-маршруты матчатся в sidecar, `CordisManager::http_route` и fallback-прокси одинаково (`/probe-prefix` и `/probe-prefix/a/b` → 200, exact выигрывает у prefix, среди prefix — самый длинный); `GET /v1/plugins/status` отдаёт per-plugin `routes` (живая таблица host-маршрутов) рядом с `tools`; MCP-псевдо-инструменты `list_resources`/`read_resource`/`list_prompts`/`get_prompt` не попадают в реестр инструментов Cordis-плагина (реальные инструменты, например `cordis__demo__probe`, остаются); при bind вне loopback (`0.0.0.0:0`) plugin-маршрут отвечает 403, а с `ServerConfig.allow_plugin_routes_lan`/`--allow-lan-plugin-routes` — 200; reserved- и неизвестные пути по-прежнему 404 | I | ✅ `plugins_live.rs` 16 checks (раунд 86: `proxies_plugin_host_routes` расширен prefix/status/meta-tools, `plugin_routes_are_loopback_only_unless_opted_in`; раунд 87 добавил `internal_session_snapshot_feeds_the_host_bridge`) + `crates/cordis/src/bootstrap.mjs`/`manager.rs`/`lib.rs` + `crates/cli/src/serve_cmd.rs` |
| CLI-43 | **C1.1: мост `sessions`/`sessionProjections`**: сервер пишет `<home>/cordis/host.json` (`{url, token}`) и публикует `GET /v1/internal/session-snapshot` (recent ≤50 сессий: id/title/created_at/model/turns/steps и usage-события assistant; содержимое сообщений и промпты не покидают сервер; 404 без настроенного токена, 401 при неверном `x-dshr-internal-token`); sidecar получает путь через `DSHR_HOST_BRIDGE`, раз в 5 с обновляет снимок и строит DSH-shaped `ctx.get('sessions')` (`list()`/`get(id)`, `header.createdAt` в epoch ms, `log` с `request/context`, `session/title`, `assistant/message` usage) и `ctx.get('sessionProjections')` (`snapshot(session).values`: `sessionStats` turns/steps/`llmMs`=0 и `tokenUsage`; данные могут отставать до 5 с); usage-маршрут реального `@kt11/dsh-deepseek-balance` (`/dsh-usage/sessions?tz=UTC`) возвращает `sessions` ≥ 1 и totals | I/E | ✅ `plugins_live.rs` (`internal_session_snapshot_feeds_the_host_bridge`: usage-поля, 401, 404) + `crates/cordis/src/bootstrap.mjs` (`refreshHostSessions`/`dshSessionFromSnapshot`) + smoke 80/80 (C1.1 bridge) |
| CLI-44 | **C1.2: seat `inject`, Cordis-генераторы и `configForms`** — контракты, без которых settings-UI реальных плагинов оставался инертным: (1) регистрация слота может отдать компоненту дополнительные props фабрикой `inject: () => ({...})`; они мержатся при монтировании в React-режиме (вместе с host-props) и в imperative-колбэке, а брошенная фабрика (нет host-сервиса) деградирует компонент, пишет событие в лог кадра и **не** помечает плагин упавшим; (2) `ctx.slots.inject`, `ctx.inject` и `ctx.effect` принимают `function*` — тело выполняется, все `yield`-disposer'ы собираются и вызываются на teardown в обратном порядке; (3) `ctx.get('configForms')`/`ctx.configForms.get(entryId)` отдаёт форму `{getSnapshot, subscribe, mutate(ops, revision)}`: snapshot-объект стабилен между нотификациями (требование `useSyncExternalStore`), `mutate` соблюдает revision-контракт (устаревшая ревизия → `false`), форма в кадре in-memory writable (правки не переживают reload кадра), а host-половинам bootstrap пишет файловый вариант `<home>/cordis/config-forms/<entry>.json`. Эффект: real-плагин `dsh-plugin-studio@0.3.1` рендерит реальные вкладки в Settings (`settings.plugins.tab`, собственный CSS, честные error-состояния при заглушках host-маршрутов), а `dsh-plugin-debugging@0.8.1` — карточку настроек в Plugins (`plugins.bundle.config`), переключатель и Save проходят round-trip через `configForms` | I/E | ✅ `crates/cordis/src/bridge.js` (`injectedProps`, `cordisDisposer`, `browserConfigForm`, `configFormsService`), `crates/cordis/src/bootstrap.mjs` (файловый `configForms`), `tests/bridge_harness.mjs` 102 (+3 в C1.2: seat-props, генераторный `slots.inject`, генераторный `effect`; +8 в C1.3 — см. CLI-45; +4 в C1.4 — см. CLI-46) + smoke 82/82 (+2: settings-вкладка studio и карточка debugging) |
| CLI-45 | **C1.3: слот-контракт по референсному каталогу (`SlotCore`-semantics)** — регистрации проверяются по сгенерированному каталогу DSH (`deepseek-ai/deepseek-harness` @ `39163dd`, `cordis-client-runner/src/client/slot-catalog.ts`): 122 слота с `kind` (single 45 / list 48 / keyed 22 / chain 4 / hoc 3), `scope` (root 57 / session 60 / session-maybe 5), framework standard-kit и `inject`-флагом (`crates/cordis/src/slot-catalog.json`, host-страница публикует как `window.__DSHR_SLOT_META__`). (1) неизвестный слот → `slot "<name>" is not declared` (исключение — dshr extension-пространства `panel`/`message.action`/`custom.*`/`x-*`/`dshr.*`); (2) `keyed` требует `key`, `list`/`hoc` — `id`, `chain` — `select`; (3) повторная регистрация в ячейку (`single` — слот, `keyed` — key, иначе id) на том же `priority` бросает, другой `priority` затеняет (рендерится меньший); (4) регистрация может объявить дочерние слоты `children: {key: {kind, scope}}`, которые живут и каскадно снимаются вместе с записью — это разблокировало `dsh-mnemon` и его `mnemon.component.*`; (5) `ctx.slots.inject(key, factory)` ждёт объявления слота и перезапускается при появлении (reference registry semantics); (6) shell получает только shadowing-winners (`single` — один, `keyed` — по одному на ключ, `list`/`chain`/`hoc` — все слои в порядке priority → order → регистрация); (7) props компонента = standard-kit из каталога (нереализованные hooks остаются вызываемыми и возвращают `undefined`, `sessionId` — из host-state) → seat `inject` → `ownerProps` из `window.__DSHR_FRAME__.ownerProps` → `t`; (8) `ctx.slots.spec(key)` отдаёт `{key, kind, scope, standardProps, inject}`. Границы: `chain` не диспатчится по `select` (монтируется слоями), `hoc` не вкладывает `children` между кадрами, store seats (`PropsStore`/`BoundActions`) и ownerProps от реальных render-сайтов шелла — остаток C-full | I/E | ✅ `crates/cordis/src/bridge.js` (`SLOT_META`/`declaredSlots`, `declareChildSlots`/`undeclareChildSlots`, `normalize`, `winners`, `slotStandardProps`, `frameOwnerProps`, `slots.inject`/`spec`) + `tests/bridge_harness.mjs` 102 (+8: undeclared/keyed/chain/duplicate throws, single shadowing, catalog standard kit, ownerProps, plain values) + smoke 82/82 (реальные плагины регистрируются по тем же правилам; synthetic-фикстуры дополнены обязательным `key`, словарь — 124 ключа) |
| CLI-46 | **C1.4: host-сервисы по reference-контракту (`webServer`/`ctx.baseUrl`/`configForms`)** — сверка с исходниками `deepseek-ai/deepseek-harness`: (1) `webServer` (`packages/host/webserver/src/index.ts`) — exact- и prefix-таблицы независимы, дубликат `(kind, path)` бросает `webserver: duplicate <kind> route`, путь обязан быть абсолютным pathname без завершающего `/`, `?` и `#` и вне reserved-префиксов dshr API, `getOrigin()`/`port`/`protocol` отдают origin dshr-сервера из `<home>/cordis/host.json` (host-bridge), а не loopback-порт sidecar-прокси, `registerFallback`/`registerUpgrade` в sidecar не поддержаны и бросают явную ошибку; (2) `ctx.baseUrl` — loader-context свойство reference Cordis: `file://` URL корня пакета плагина (host-половины вроде `dsh-plugin-studio` резолвят от него пути); (3) `configForms` (`packages/client/ui-settings/src/client/config-form-types.ts`) — snapshot ровно `{status, value, base, user, revision: number, writable}` со стабильной идентичностью между чтениями, `mutate(ops, expectedRevision?)` с атомарными `set`/`unset` и числовым revision-fence, хелперы `set(field, value)`/`unset(field)`; форма в кадре in-memory (правки не переживают reload), у host-половин файловая `<home>/cordis/config-forms/<entry>.json`. Проверки: bridge harness 102 (+4: форма-контракт, стабильный snapshot, set с числовой ревизией, unset), server `plugins_live` 16 (proxy-тест расширен duplicate-route и trailing-slash правилами), smoke 82/82, cordis lib 26, workspace 365 passed + 3 ignored (desktop 21, итого 386), web-e2e 16/16; рантайм-артефакт на момент C1.4 не менялся — 176 725 байт (после C2.1, раунд 93, — 375 894 байта; после C2.2, раунд 94, — 387 613 байт). Границы: `registerFallback`/`registerUpgrade` не поддержаны (нужны чужим SPA/WS-плагинам), browser-`configForms` не персистится через reload, живого RPC-зеркала настроек нет; остаток reference-работы — реальный порт `ui-primitives` из `packages/client/ui-primitives` (следующий кандидат), store seats/renderer, chain dispatch/hoc-nesting, ownerProps от реальных shell-сайтов | I/E | ✅ `crates/cordis/src/bootstrap.mjs` (`validRoutePath`, `webRoutes`, `unsupportedWebServerFeature`, `ctx.baseUrl`, reference `configForms`), `crates/cordis/src/bridge.js` (`browserConfigForm`/`configFormsService` reference-shape), `crates/cordis/tests/bridge_harness.mjs` 102 (+4), `crates/server/tests/plugins_live.rs` (`proxies_plugin_host_routes`: duplicate-route/trailing-slash), smoke 82/82 |
| CLI-47 | **C2.1: атомы design-system и все иконки портированы из reference** — `primitives.js` переписан по `deepseek-ai/deepseek-harness` @ `39163dd` (`packages/client/ui-primitives/src`), а не независимый шим: `Button` (variants primary/ghost/outline/toolbar, sizes md/sm, leading icon, forwardRef), `Input` (wrapper + leading icon), `Switch` (role switch, `aria-label`, thumb без видимого текста), `Checkbox` (native input + label, `data-modal-autofocus`), `Tag` (`data-tone`: outline/solid/neutral/quiet/success/info/warning/danger), `Pill` (span/button по наличию `onClick`, `active`), `SegmentedControl` (tablist + indicator, roving tabindex, walk-клавиши, `<id>-<value>`/`-panel`), `SegmentedTabs` (Pill-табы, grid indicator, Left/Right/Home/End с переносом фокуса), `DisclosureRow` (header/row, `expandOnRowClick`, `previewChevron`, `collapsedContent`, `headerAccessory`, running-shimmer), `TextShimmer` (nested-context + decoration/sweep), `StateDot` (done/warning/ongoing/error/idle, SVG-spinner, `appearance: "step"`), `PathLabel` (directory/name split, clipped-маска, `title`), `Toast` (portal, `role=alert`, success check-circle, `holdMs`+fade, actions, anchor), `Modal` (portal, mask, Escape, focus, warning tone, headless, footer), `Menu` (data-rows/separators/labels, fixed-список, outside/Escape, `selectedId`/`selectedIds`, portal; полный submenu/focus-движок **не** портирован), `MenuSurface` (`forwardRef`, `data-menu-material`/`phase`), `Tooltip` (label-based клон якоря, hover/focus-бабл; placement-движок **не** портирован); все **204** `Icon*` сгенерированы из upstream `icons/artwork.ts` (точные SVG-пути, stroke regular 1 / medium 1.3, инлайн вместо document-спрайта, Proxy-fallback для будущих имён); стили — reference `--dsw-alias-*` токены с fallback'ами; synthetic-фикстуры приведены к reference-контрактам (`StateDot state:"done"`, `Tooltip` с element-child). Проверки: smoke 82/82 (два зелёных прогона подряд) — C2-probe проверяет структурные признаки, а не только текст (`role=switch` с `aria-label`, `data-tone`, `tablist`-табы, число `svg`, `label.dshr-checkbox`, `textContent` у `[data-path-label]`); bridge harness 102, cordis lib 26, server `plugins_live` 16, workspace 365 passed + 3 ignored (desktop 21, итого 386), web-e2e 16/16; общий рантайм-артефакт на конец C2.1 — **375 894 байта** (после C2.2, раунд 94, — 387 613 байт; было 176 725; cached immutable один раз на страницу). Границы C2.1 (полный `Tooltip` закрыт в C2.2, см. CLI-48; ядро `Menu` портировано в C2.3, см. CLI-49): полный `Menu` (submenu/focus) не портирован, `MarkdownText`/`JsonBlock` остаются упрощёнными dshr-расширениями (upstream требует shiki/katex/mdast), `Spinner`/`useAnchoredPosition` — тоже dshr-расширения; store seats/renderer, chain dispatch/hoc-nesting и ownerProps от shell-сайтов остаются C-full | I/E | ✅ `crates/server/assets/runtime/primitives.js` (порт reference-атомов + генерация 204 иконок) + `scripts/plugin-smoke.py` (C2-probe: структурные проверки) + `scripts/check-runtime-assets.sh` (375 894 байта на конец C2.1; 387 613 байт — CLI-48; 405 323 байта — CLI-49; 415 972 байта — CLI-50) + smoke (фикстура `ui-kit`) |
| CLI-48 | **C2.2: полный `Tooltip`** — последний упрощённый атом C2.1 доведён до reference-порта `packages/client/ui-primitives/src/Tooltip.tsx`: label-резолвер вычисляется только при показе; независимые hover/focus-триггеры с `delayMs`/`focusDelayMs`; `openOnClick`-pinning (клик открывает и фиксирует бабл) с outside-pointer/Escape/Tab-dismiss; `onlyWhenClipped`, `maxWidth`, `portal`, `data-side`/`data-pinned`, nested-suppression через контекст, `aria-describedby` при pinning, pointer-modality (фокус после указателя не поднимает бабл), touch-hover исключён, forward ref сквозь клон якоря. `tooltip-placement.ts` портирован как есть: requested side → opposite → perpendicular, viewport-clamp и overflow-fallback; `input-modality.ts` (`data-input-modality`, `pointerModality`) и `ShortcutKeys` (keycaps с `+`-разделителями) портированы и экспортированы; C2-probe дополнен узлом `Tooltip` в synthetic-фикстуре (`ShortcutKeys` экспортирован и отрисовывается внутри Tooltip) | U/E | ✅ `crates/server/assets/runtime/primitives.js` (`Tooltip`/`placeTooltip`/`ShortcutKeys`/input-modality) + `scripts/plugin-smoke.py` (C2-probe) + `scripts/check-runtime-assets.sh` (387 613 байт на конец C2.2; после C2.3 — 405 323 байта; после C2.4 — 415 972 байта), smoke 82/82 |
| CLI-49 | **C2.3: `Menu`/`MenuItemButton` по reference-ядру** — `Menu` переписан по `packages/client/ui-primitives/src/Menu.tsx`/`MenuRow.tsx`: data-`items` и компонентные `MenuItemButton`, separators/labels, `reserveIcon`-колонки (contextual), selection `check`/`fill`, `danger`/`current`, disabled-reason через reference `Tooltip`, `shortcut` + `ShortcutKeys`, aria (`menuitem`/`haspopup`/`expanded`/`disabled`/`describedby`/`keyshortcuts`/`current`), `children`-ряды и `footer`-группа, `dense`/`compact`, portal-позиционирование по anchor-rect (`minWidthFromAnchor`/`anchorPoint`/`align`/`side`), contextual-атрибут; клавиатура: Arrow/Home/End walk с roving focus, Escape закрывает и возвращает фокус (`focusWithoutRing`), Tab «settles» ряд / Shift+Tab уходит, ArrowRight/Left входят/выходят из submenu, `autoFocus`; one-level submenu открывается на hover/ArrowRight и закрывается по ArrowLeft/Escape/выбору; outside-pointer dismissal + закрытие при blur в iframe, `closeOnPointerLeave` с 200-мс grace (`usePointerGrace`); экспортированы `MenuItemButton`, `MENU_TRANSITION_MS`, `usePointerGrace`, `focusWithoutRing`, `isMobileDevice`. Границы на конец C2.3: полная submenu-машинерия (`useMenuSubmenu`: single-panel back-page, меж-уровневый hover grace) и `MenuGroup`/`observeStickyMenuGroups` не портированы — закрыто в C2.4 (CLI-50) | U/E | ✅ `crates/server/assets/runtime/primitives.js` (`Menu`/`MenuItemButton`/`usePointerGrace`/`focusWithoutRing`/`MENU_TRANSITION_MS`/`isMobileDevice`) + `scripts/plugin-smoke.py` (C2-probe) + `scripts/check-runtime-assets.sh` (405 323 байта; после C2.4 — 415 972 байта), smoke 82/82 |
| CLI-50 | **C2.4: reference submenu page-layout, focus-контракт, 200-мс hover grace и `MenuGroup`** — сверка с `packages/client/ui-primitives/src/{Menu.tsx,MenuGroup.tsx,menu-submenu-position.ts}`: (1) размещение submenu по `menu-submenu-position.ts` — если дочерняя карточка помещается сбоку родительской (справа или слева), это side-карточка; иначе single-page layout: дочерние ряды заменяют родительскую страницу, сверху non-selecting back-row (`data-menu-back`, chevron-left, `submenuBackLabel` или label родителя) + separator, а Escape/ArrowLeft возвращают на родительскую страницу с фокусом на родительском ряду; (2) focus-контракт — hover открывает submenu без перехвата фокуса, ArrowRight/клик фокусируют первый дочерний ряд; (3) 200-мс hover grace между родительской и дочерней карточками — уход курсора в дочернюю отменяет запланированное закрытие (`cancelSubmenuClose`); (4) `MenuGroup` + `observeStickyMenuGroups` портированы по `MenuGroup.tsx` — `section[role=group]` с `aria-labelledby`, `data-menu-group`/`data-menu-group-start`/`data-menu-group-heading`, sticky-заголовок (`data-stuck`) на IntersectionObserver/ResizeObserver; C2-probe рендерит `MenuGroup`, submenu-родителя и проверяет `menuRows >= 3` | U/E | ✅ `crates/server/assets/runtime/primitives.js` (`submenuNeedsPage`/`cancelSubmenuClose`/`data-menu-back`/`MenuGroup`/`observeStickyMenuGroups`/`data-stuck`) + `scripts/plugin-smoke.py` (C2-probe: `MenuGroup`, submenu-родитель, `menuRows >= 3`) + `scripts/check-runtime-assets.sh` (415 972 байта на конец C2.4; после C2.5 — 424 801 байт), smoke 82/82 |
| CLI-51 | **C2.5: reference `JsonBlock` и API-совместимое `MarkdownText`-подмножество** — сверка с `packages/client/ui-primitives/src/markdown/JsonBlock.tsx`: (1) `JsonBlock` — collapsible (▾/▸), props `label`/`payload`/`defaultOpen`/`truncatedLabel`, cap 20 000 символов с хвостом «truncated», безопасный `JSON.stringify` с fallback на `String` (циклический payload, `undefined`); (2) `MarkdownText` — reference-поверхность props (`text`/`streaming`/`labels`/`fileMentions`/`pathImages`/`variant`/`omitHtml`/`textInteractionSource`) и dependency-free рендерер GFM-подмножества: заголовки 1–6, абзацы, ul/ol, fenced code, blockquote, hr, inline `code`/`**bold**`/`*italic*`/`~~strike~~`, ссылки и изображения только `http(s)`/`mailto` (небезопасные протоколы `javascript:`/`data:` отбрасываются; относительные изображения не рендерятся), raw HTML не рендерится, `variant="compact"` даёт `data-markdown-variant`/`data-text-interaction-source`; (3) C2-probe рендерит `MarkdownText` (заголовок, bold, fenced code, ссылка) и `JsonBlock` (expanded JSON) и проверяет `.dshr-markdown h1`, `.dshr-markdown pre code`, `.dshr-json-block-body` и текст `"answer": 42`. Границы: upstream markdown-стек (micromark/mdast, shiki, KaTeX, footnotes, инкрементальный streaming) **не** портирован — `MarkdownText` остаётся API-совместимым подмножеством | I/E | ✅ `crates/server/assets/runtime/primitives.js` (`JsonBlock`/`MarkdownText`/`sanitizeHref`/`markdownBlocks`) + `scripts/plugin-smoke.py` (C2-probe: `MarkdownText`/`JsonBlock`) + `scripts/check-runtime-assets.sh` (424 801 байт после C2.5), smoke 82/82 |
| CLI-52 | **C2.6: submenu-механика по reference** — сверка с `packages/client/ui-primitives/src/{Menu.tsx,menu-submenu-position.ts}`: (1) side-card vs single-page выбирается по измеренной ширине дочерней карточки (панель измеряется скрытой, как в reference `menu-submenu-position.ts`); (2) side-card позиционируется `fixed` и flip/clamp против родительской карточки (right при нехватке места справа, left иначе; top-clamp по viewport с `SUBMENU_MARGIN`), `minWidth` от ширины родительского ряда; (3) клавиатура маршрутизируется через `activeList` — навигация идёт по родительскому списку или по дочерней странице в зависимости от фокуса; (4) Escape/ArrowLeft из дочерней страницы закрывают submenu и возвращают фокус на родительский ряд (Escape на корне закрывает меню и возвращает фокус на триггер); (5) 200-мс hover grace между родительской и дочерней карточками, наведение на другой ряд сворачивает открытый submenu, hover разрешён только для non-touch и `any-hover: hover` (`menuHoverAllowed`); (6) `data-menu-back`/`submenuBackLabel` non-selecting back-row в single-page layout. Проверки: smoke 82/82 (C2-probe), bridge harness 105, `plugins_live` 17, workspace 366 passed + 3 ignored (desktop 21, итого 387), web-e2e 16/16; рантайм-артефакт 430 139 байт | I/E | ✅ `crates/server/assets/runtime/primitives.js` (`menuHoverAllowed`, измерение ширины и `fixed`-flip/clamp, `activeList`, Escape/ArrowLeft, `cancelSubmenuClose`, `data-menu-back`) + `scripts/plugin-smoke.py` (C2-probe) |
| CLI-53 | **C3.1: store-seats по reference `packages/client/store`** — (1) общая module-таблица публикует `@deepseek-ai/dsh-client-store` (`createSnapshotStore`/`subscribe`/`getSnapshot`/`set`/`update`, `defineStore` с init+actions и baked callback'ами, `shallowEqual`); `zustand`/`immer`-семантика воспроизведена plain deep-copied draft'ом, `persist`-ключ принимается, но не персистится (ограничение); (2) регистрация слота может объявить `store: handle\|factory`; фрейм владеет одним instance на scope-key (`session`/`session-maybe` → id сессии, остальное — root), instance переиспользуется по `(id, scopeKey)`; (3) компоненту передаются `useStore(selector, equality?)` (React-подписка на `subscribe`, selector + кастомный `eq`) и baked `actions`; (4) `inject`-фабрика получает reference-аргументы: для session-scope — `(sessionId, actions)` при store и `(sessionId)` без store, для root — `(actions)`; ошибка store-фабрики репортится и деградирует seat, а не плагин. Проверки: bridge harness 105 (+3: `useStore`/`actions` в props, baked action мутирует framework-owned instance, inject получает `(sessionId, actions)`), smoke 82/82 (+store-engine check `defineStore` → `create` → `actions.inc` → snapshot `n === 1`), workspace 366 passed + 3 ignored (итого 387); артефакт 430 139 байт | I/E | ✅ `crates/server/assets/runtime/bootstrap.js` (`shallowEqual`/`createSnapshotStore`/`defineStore` в `window.__DSHR_MODULES__`) + `crates/cordis/src/bridge.js` (`resolveStoreInstance`/`storeProps`/`storeInstances`/`injectArguments`) + `crates/cordis/src/client.rs` (`HOST_MODULES` 7 id) + `tests/bridge_harness.mjs` 105 (+3) + smoke (store-engine check) |
| CLI-54 | **C1.5: fallback-маршруты + ownerProps plumbing** — (1) `webServer.registerFallback(handler)` как в reference `packages/host/webserver/src/index.ts`: одно fallback-место, второй владелец бросает `webserver: fallback already registered`, handler получает Node `(req, res)`, `unregister` освобождает место; sidecar отдаёт флаг `fallback` в `__dshr_routes`, `CordisManager::http_fallback()` находит единственного владельца (обход в порядке имён), а dshr проксирует в fallback только не-зарезервированные несовпавшие пути (reserved `/v1`/`/dshr`/UI и явный `/v1/fallback-probe` — 404, fallback не получает); `registerUpgrade` (WebSocket) остаётся явно не поддержанным и бросает `webServer.<name> is not supported by the dshr Cordis sidecar`; (2) ownerProps: shell может передать render-site owner props в `/v1/plugins/{name}/host?ownerProps={...}`; сервер валидирует JSON-объект (не-объект → 400 `ownerProps must be a JSON object`, лимит длины как у `context`), встраивает в `window.__DSHR_FRAME__.ownerProps`, а мост мержит их в props после store/seat share (порядок reference `PropsRuntime`: standard-kit → seat inject → store → ownerProps → `t`); реальные shell-сайты данных пока не дают — нужна chat/turn-модель Leptos-шелла (TECH-DEBT). Проверки: `plugins_live` 17 (+`proxies_the_plugin_fallback_seat`, ownerProps embed/отклонение не-объекта в frame-config тесте), bridge harness 105, smoke 82/82, workspace 366 passed + 3 ignored (итого 387); артефакт 430 139 байт | I/E | ✅ `crates/cordis/src/bootstrap.mjs` (`fallbackHandler`/`registerFallback`/`fallback`-флаг), `crates/cordis/src/lib.rs` (`SidecarHttp.fallback`), `crates/cordis/src/manager.rs` (`http_fallback`), `crates/server/src/lib.rs` (`plugin_http_proxy` fallback, `ownerProps` в `frame_spec`/`FrameConfig`), `crates/cordis/src/client.rs` (`FrameSpec.owner_props`), `crates/cordis/src/bridge.js` (`frameOwnerProps`), `crates/server/tests/plugins_live.rs` (`proxies_the_plugin_fallback_seat`, `slot_host_page_embeds_the_frame_config`), bridge harness 105 |
| CLI-55 | **C4.1: chat/turn-модель шелла, реальные owner shares и chain-элекция** — (1) **chat/turn-модель** (`crates/web/src/main.rs`): каждое user-сообщение открывает новый turn, ordinal пишется в `data-turn` каждого сообщения и в owner share; `SlotFrame`/`SlotRegion` получили `owner_props`, который едет в кадр как `&ownerProps=...` (URL-encoded JSON); (2) **реальные owner shares**: `sidebar.session.action` → `{wide:true}`; `message.action` для user-сообщений → `{source:{role,index}, kind:"user"}` (reference `UserBeforeOwnerProps`); `composer.actions` → `ComposerChainProps` `{sessionId, session:{id,model,turns}, pendingInteraction:null}`; (3) **chain-элекция** (`crates/cordis/src/bridge.js`): кадр chain-слота вызывает `select(ownerProps)` своей регистрации; `false` → `clear()`, пустой DOM и репорт `chain-unselected`, `true` → рендерится выбранная запись (owner-специфичный dispatch без серверного JS); сервер по-прежнему валидирует `ownerProps` как JSON-объект. Проверки: bridge harness **107** (+2: chain-select `false`/`true`), web-e2e **18** (+2: turn ordinal у сообщений, смонтированные `composer.actions`/`message.action` регионы), `plugins_live` 17 (ownerProps embed + отклонение не-объекта), smoke 82/82, cordis 26, workspace 366 passed + 3 ignored (итого 387); рантайм-артефакт не менялся — 430 139 байт. Границы: owner props с функциями (например `settings.section.close`) не передаются — нет callback-моста; несколько `select === true` рендерят несколько кадров (per-frame фильтрация, а не единый owner-renderer); `conversation.chat.node` и остальные conversation-слоты ждут полного chat-view | I/E | ✅ `crates/web/src/main.rs` (`owner_props` в `SlotFrame`/`SlotRegion`, узел turn/`data-turn`, owner shares `sidebar.session.action`/`message.action`/`composer.actions`), `crates/cordis/src/bridge.js` (`chainElected` → `select(frameOwnerProps)`, `chain-unselected`, `clear()`), `crates/cordis/tests/bridge_harness.mjs` 107 (+2), `scripts/web-e2e.py` 18 (+2), `crates/server/src/lib.rs`/`crates/server/tests/plugins_live.rs` (ownerProps-валидация) |

| CLI-56 | **C4.2: callback-мост owner props, единый owner-renderer chain, `hoc`-вложение, полный chat-view, `registerUpgrade`** — (1) **callback-мост** (`crates/cordis/src/bridge.js` + `crates/web/src/main.rs`): shell кодирует функцию в owner props маркером `{"$callback":"<name>"}`, сервер как и раньше требует JSON-объект, а мост материализует маркер в функцию, которая шлёт `ctx.action("callback", {name, args})` и резолвится `action-result` (промис-форма reference `openFile(path).then(...)`); диспетчер шелла: `settings.section.close` → закрыть настройки, `openFile` → инспектор нового `GET /v1/workspace/file?path=…` (workspace-confined после канонизации, cap 64 КиБ, `truncated`), `inspectCall` → инспектор tool-call'а из chat-view, `forkAt`/`loadImage` — честные ответы; материализованный колбэк сохраняет маркер, поэтому проброс owner props в дочерний слот ре-сериализует его, а owner share больше ~3000 символов (chat node) едет не в URL, а сообщением `owner-props` после `ready` (кадр перерисовывается); (2) **chain** — reference-элекция: побеждает первая регистрация (порядок priority → order → seq), чей `select(ownerProps)` не вернул `null`/`false`/`undefined`, компонент получает `{...ownerProps, matched}`; `SlotRegion` отдаёт каждый chain-слот одному `ChainHost` owner-renderer'у, который монтирует один кадр-кандидат и продвигается по `chain-unselected` (и по `error` кандидата), т.е. несколько `select === true` не рендерят несколько кадров; `slots`-сообщение несёт `kind`/`priority`/`key`/`children`, чтобы shell сортировал кандидатов сам; (3) **`hoc`-вложение** — запись с `children` получает `renderSlot`/`renderSlotChain` (только объявленные ключи, иначе throw, как reference `boundRenderSlot`); кадр репортит плейсхолдер (`anchor`: id/host/slot/ownerProps/rect — колбэки ре-сериализуются в маркеры), shell монтирует дочерние слот-кадры абсолютно над якорем внутри обёртки родителя и возвращает измеренную высоту (`anchor-size`) → родитель резервирует место в своём layout; дочерние слоты исключены из плоского монтажа в регионы; (4) **chat-view** — транскрипт (`Message`/`ContentBlock`) проецируется в turn/node-модель (`user`/`assistant-step` с text/thinking/images/tool_use, `tool_result` привязывается к call по id), SSE-события дописывают running-узел; conversation-слоты монтируются на узел: `conversation.chat.node` (keyed по kind; `user`/`assistant-step` — встроенный рендер шелла, kind-специфичные плагины заменяют его), `assistant-actions`, `message.images`/`user.images`, `reasoning.body`, `text-interaction`, `turnTail`, `user.before` (chain), `flow`, `markdown.fence` (hoc), `conversation.composer` (chain); новые регионы `chat.*`/`composer.chain`; (5) **`registerUpgrade`** — sidecar: exact-path таблица upgrade-маршрутов, дубликат пути бросает `webserver: duplicate upgrade route`, валидация пути как у HTTP-маршрутов, `upgrade`-листенер с проверкой `x-dshr-sidecar-token`, таблица в `__dshr_routes.upgrades`; `CordisManager::http_upgrade(path)`; dshr (`plugin_upgrade_proxy`) подменяет reserved/loopback-политику, туннелирует: replay request-head с токеном, чтение handshake апстрима, ответ 101 с его заголовками, `copy_bidirectional` через `hyper::upgrade`/`TokioIo`. Проверки: bridge harness **119** (+12), `plugins_live` **18** (+`proxies_plugin_websocket_upgrade`: 101 + `Sec-WebSocket-Accept` + туннель байтов через реальный сокет, duplicate/trailing rejection, `upgrades` в статусе), web-e2e **18/18**, smoke **88/88** (+owner callback закрывает диалог настроек; +3 hoc-пробы: дочерний кадр монтируется внутри обёртки родителя, получает измеренную высоту и рендерит объявленный дочерний слот), cordis 26, workspace 367 passed + 3 ignored (desktop 21, итого 388); артефакт 1 244 361 байт (не менялся). Границы: набор owner-callback'ов фиксирован шеллом, core-узлы чата рендерит шелл, owner share узла — снимок строки, `openFile`/`inspectCall` — инспекторы, не редактор | I/E | ✅ `crates/cordis/src/bridge.js` (`materializeOwnerProps`/`serializeOwnerProps`/`ownerCallback`, `chainElection`, `boundRenderSlot`/anchors, `anchor-size`, `owner-props`), `crates/cordis/tests/bridge_harness.mjs` 119 (+12), `crates/web/src/main.rs` (`ChainHost`/`ChainInstance`, `NestedSeat`/`SlotAnchor`/`nested`, `ChatNode`/`build_chat_nodes`/`ChatNodeRow`, `Callback`-диспетчер, `openFile`/`inspectCall` инспекторы, `/v1/workspace/file`-клиент), `crates/server/src/lib.rs` (`workspace_file`, `plugin_upgrade_proxy`/`parse_upgrade_head`), `crates/cordis/src/{bootstrap.mjs,lib.rs,manager.rs}` (`upgradeRoutes`, `SidecarHttp.upgrades`, `http_upgrade`, `PluginStatus.upgrades`), `crates/server/tests/plugins_live.rs` (+1) |
| CLI-57 | **C5.1: затенение встроенного рендера ядра** — `conversation.chat.node` решается по приоритету, как reference SlotMap: встроенный вид = запись с `CORE_RENDERER_PRIORITY = 0`, плагин затеняет только `priority < 0` (реальный `dsh-genui@0.3.3` регистрирует `assistant-step` с `-1`), приоритет по умолчанию оставляет встроенный вид; затеняются только `keyed`-ячейки, `single` остаются shell-owned; builtin-capture недоступен (записано в PLUGINS/ADR-042). Проверки: 6 host-target unit-тестов web-крейта (`core_kinds_keep_the_builtin_renderer_without_a_registration`, `a_default_priority_registration_does_not_take_the_builtin`, `a_negative_priority_shadows_a_core_kind`, `a_shadowed_other_key_does_not_replace_this_node`, `plugin_specific_kinds_render_their_keyed_registration`, `non_keyed_registrations_never_replace_a_node`), `trunk build --release`, smoke 88/88 | U/E | ✅ `crates/web/src/main.rs` (`CORE_RENDERER_PRIORITY`, `chat_node_replaced_by_plugin`, `renderer_tests`), `crates/web/dist/*` |
| CLI-58 | **C5.2: fallback упавшего рендерера** — ошибка кадра-победителя (`error` от iframe) пишется в `renderer_errors` по токену и резолвит keyed-ячейку в builtin, поэтому узел не остаётся пустым; строка получает `data-renderer-state="builtin|plugin"` и `data-renderer-token`; пустой (null) рендер уважается и не подменяется. Проверки: web-тесты 8 (`a_crashed_winner_falls_back_to_the_builtin_view`, `an_unrelated_crash_does_not_remove_the_winner` и матрица C5.1), smoke 88/88; browser-probe объединён с C5.3 | U/E | ✅ `crates/web/src/main.rs` (`renderer_errors`, `chat_node_renderer`, `renderer_tests`) |
| CLI-59 | **C5.3: живой поток сессий вместо 5-с опроса** — `GET /v1/internal/session-events` (SSE, `x-dshr-internal-token`) отдаёт `event: session` с той же записью, что снимок; сервер публикует её после создания сессии, генерации заголовка и завершения turn'а; при отставании подписчика — `event: resync`; sidecar гидрируется снимком, применяет дельты (`mergeHostSession`), на resync/обрыв перегидратируется и переподключается с backoff (0.5→10 с), постоянного таймера 5 с нет. Проверки: server +3 (401/404/200 и SSE-заголовок, создание сессии в потоке, usage после turn'а), cordis 26+1+8+2, smoke 88/88 (C1.1 usage-маршрут balance-плагина видит сессии) | I/E | ✅ `crates/server/src/lib.rs` (`feed`, `publish_session`, `internal_session_events`, `session_summary_entry`), `crates/cordis/src/bootstrap.mjs` (`runHostFeed`/`streamHostSessions`/`mergeHostSession`), `crates/server/tests/http.rs` (+3) |
| CLI-60 | **C5.4a: живой сайдбар сессий** — публичный `GET /v1/sessions/events` (bearer/query token) отдаёт тот же поток, что внутренний фид; шелл держит один `EventSource`, `merge_session_summary` заменяет известный id и вставляет новый newest-first по `created_at`, `resync` перечитывает `/v1/sessions`, чужие типы кадров игнорируются. Проверки: web 11 (+3: newest-first, replace-in-place, парсер), server 20 (+1: 401 без токена, событие создания в публичном фиде), smoke 88/88 | I/E | ✅ `crates/server/src/lib.rs` (`session_feed`, `session_feed_stream`), `crates/web/src/main.rs` (`connect_session_feed`, `merge_session_summary`, `parse_session_feed`, `session_feed_tests`), `crates/server/tests/http.rs` (+1) |
| CLI-61 | **C5.4b: реальный `llmMs` и история turn'ов** — `SessionEvent::AssistantMessage.latency_ms` (optional, `serde(default, skip_serializing_if)`; старые журналы реплеятся), агент замеряет провайдерский вызов, запись сессии несёт `llm_ms`, `turns_detail` и `latency_ms` на usage-событие, sidecar отдаёт `sessionStats.llmMs`; thread-модели в dshr нет — записанное отсутствие. Проверки: session 6 (+1), server 20 (+поля в фиде), smoke 88/88 | U/I | ✅ `crates/session/src/events.rs`, `crates/agent/src/lib.rs`, `crates/server/src/lib.rs` (`session_summary_entry`), `crates/cordis/src/bootstrap.mjs` |
| CLI-62 | **C5.5: host-сервис `pluginManager`** — `ctx.get('pluginManager')` с `list`/`listBundles`/`status`/`setPluginEnabled`/`installBundle`/`removeBundle`; запросы идут на `/v1/plugins/*` с `x-dshr-internal-token`, серверные plugin-эндпоинты принимают internal-token наравне с bearer (`authorize_service`). Проверки: фикстура `plugin-manager-probe` + stub-сервер (`manager.rs`: list и toggle идут по HTTP и возвращают данные), server 21 (+1 internal-token), cordis 26+1+9+2, smoke 88/88. Граница: фасад над CLI-операциями, не pnpm-менеджер upstream | I/E | ✅ `crates/cordis/src/bootstrap.mjs` (`pluginManagerService`), `crates/server/src/lib.rs` (`authorize_service`), `crates/cordis/tests/fixtures/plugin-manager-probe`, `crates/cordis/tests/manager.rs` (+1) |


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
| DESK-06 | Single-instance: `<home>/desktop.lock` эксклюзивен, второй запуск форвардит папку и выходит, рестарт передаёт lock за ≤ 3 с | U/E | ✅ desktop `instance::tests` + `tests/single_instance.rs` |
| DESK-07 | Login-shell env: парсер delimited `env -0`, фильтр probe/owned, таймаут `DSHR_DESKTOP_LOGIN_SHELL_TIMEOUT_MS`, живой capture | U/E | ✅ desktop `login_env::tests`, `--check` → `54 variables from /bin/zsh` |
| DESK-08 | CLI-launcher: `<home>/bin/dshr` с `--home`/`--profile`, права 0755, ссылка `~/.local/bin` без перезаписи чужой команды | U | ✅ desktop `cli::tests` |
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

