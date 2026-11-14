# Журнал раундов паритета (1–79)

Цель: довести `dshr` до паритета с DSH 0.2.2-rc.2 / DSH Desktop.
Каждый раунд — код/документация + проверка. Итоговая сверка: 361 root-тест
(+3 ignored) и 21 desktop-тест = **382**, `clippy`/`fmt`/`deny` чисто,
release-бинарь воспроизводим.

| Раунд | Срез | Ключевые коммиты |
|---|---|---|
| 1–7 | Фундамент: ACP audio/stdio MCP, WASM component/WIT + Ed25519, Leptos UI, Tauri shell, релизные артефакты, durable jobs, `apply_patch`/`ask_user`, onboarding, provider setup, профильные плагины, скачивание обновлений | `592a5a3` `b5e69dc` `14a4434` `49f0f54` |
| 8 | Импорт DSH v4 (`sessions import`) | `9296202` |
| 9 | Детерминированные релизные архивы | `102b7b2` |
| 10 | LLM-заголовки в `run` | `804b2d8` |
| 11 | npm-поиск плагинов (`plugin npm search/info`) | `8ab7901` |
| 12 | Safe mode (пропуск MCP/Cordis/WASM) | `fb6321a` |
| 13 | Режимы окна и macOS material | `a371647` |
| 14 | Панель Plugins в Web UI + npm discovery | `f396c38` |
| 15 | MCP elicitation (stdio) | `8b9c436` |
| 16 | Last-known-good профиль и rollback | `e454037` |
| 17 | HTTP elicitation в POST-SSE | `c152079` |
| 18 | Elicitation в фоновом GET-SSE | `9f128ed` |
| 19 | LLM-заголовки в server | `b59e879` |
| 20 | LLM-заголовки в TUI | `bf1eb2f` |
| 21 | Linux/Windows desktop packages в release CI | `67f3f32` |
| 22 | Установка скачанных macOS-обновлений | `1377a62` |
| 23 | Воспроизводимый release-бинарь; recovery-меню | `d904709` `ebefd43` |
| 24 | Network/port в трее | `20ed6dd` |
| 25 | `ask_user` в ACP через elicitation | `3918587` |
| 26 | Extended window mode | `e45032d` |
| 27 | PARITY refresh; drag-and-drop папки | `c5b0c04` `ce58cd4` |
| 28 | Каналы обновлений Stable/Beta | `fdb1c9e` |
| 29 | Тема Web UI и версия сервера | `8f3a673` |
| 30 | Recovery-окно (data URL + Tauri-команда) | `729d858` |
| 31 | ACP `session/list` | `0c73f7d` |
| 32 | ACP `session/set_mode` и режимы | `75bbe89` |
| 33 | ACP `session/close` / `session/resume` | `832c512` |
| 34 | Профили/config в recovery-окне | `63d3b48` |
| 35 | Frameless extended + кастомный titlebar | `6416cf9` |
| 36 | Tool-карточки с summary/результатом в Web | `ec72e56` |
| 37 | Developer ID/notarization-тулинг | `73f0b27` |
| 38 | Diff-детали инструментов в TUI | `40bf803` |
| 39 | Persistent bash на Unix | `aebd8ed` |
| 40 | Disable/enable всех плагинов из recovery | `56142fe` |
| 41 | Массовый импорт DSH (`--all`) | `49aebb3` |
| 42 | `plugin npm enable/disable` | `a1e5d54` |
| 43 | По-плагинные переключатели в recovery | `2c2d915` |
| 44 | Windows sandbox в CI; Homebrew-бутылки | `c878451` `e7f6b56` |
| 45 | Cordis audit-log | `cac7a35` |
| 46 | Release dry-run; блокеры; terminal-шимы | `c1af38e` `e0a9f8b` `2ce54d3` |
| 47 | Bundle-слои профиля | `0fe234a` |
| 48 | Зависимости бандлов и cycle-check | `4cd6f73` |
| 49 | Патчи профиля | `edd1747` |
| 50 | Патчи бандлов | `a38e1d7` |
| 51 | Semver-требования бандлов | `b583185` |
| 52 | Cordis fs-гранты | `44e51e2` |
| 53 | Установка плагинов из Web UI | `8b68b48` |
| 54 | Web-менеджер установленных плагинов | `08b8813` |
| 55 | npm-инфо пакета в UI | `61773b4` |
| 56 | macOS DMG | `e5225ee` |
| 57 | Featured-каталог плагинов | `fa97784` |
| 58 | Финальная проверка (build/deny/repro/DMG/UI) | `96d2a30` |
| 59 | Локальные оценки плагинов; сверка счётчиков | `1cc955f` `f6793dd` |
| 60 | Итоговая сводка и финальная проверка | `47b3496` |

## Пост-60

- GHCR: OCI-publisher бутылок (`scripts/publish-bottle-ghcr.py`), job
  `bottles-ghcr`, формула `dshr-ghcr.rb`; push проверен против mock-регистра.
- Market: локальные отзывы + агрегаты ★/count (`/v1/plugins/review(s)`).
- WASM: hot reload (fingerprint watcher + `/v1/plugins/reload`).
- GHCR: реальный push/pull OCI-бутылки подтверждён в Actions.
- Bottle: `package-bottle.sh` создаёт dist до записи; macOS build проверен.
- Desktop: Linux/Windows сборка и упаковка (tar/deb/zip) проверены в Actions;
  добавлен Windows `icon.ico`, vibrancy оставлен macOS-only.
- Windows: runtime-тест AppContainer прошёл на windows-latest (запись внутри/снаружи workspace) добавлен
  в CI; реальный Windows-хост по-прежнему недоступен.
- 61: `ToolRegistry` стал общим мутируемым handle'ом (Arc+RwLock, generation,
  `unregister`/`remove_prefix`/`register_all`), `dshr_cordis::CordisManager`
  владеет sidecar'ами и синхронизирует их с конфигом; watcher по mtime
  (пакеты плагинов + profile/user config), broadcast-события, `sync()`
  add/remove/restart. `a86b91b`
- 62: `GET /v1/plugins/status`, `POST /v1/plugins/reload`, live
  `/v1/plugins/installed`, клиентские маршруты; `dshr serve` собирает
  serving-реестр и супервизор (с тем же allow/deny), watcher включён по
  умолчанию (`--no-watch-plugins` отключает). `09d7d79`
- 63: клиентская половина `dsh.client` — sandboxed iframe (opaque origin, CSP
  без сети, bundle base64→blob), `postMessage`-мост v1, слот `panel`,
  `ctx.action(close|reload)`; панель плагинов в Web UI со статусом и Reload,
  deep link `?token=…&plugin=…`. `3aeb461`
- 64: мост v1.1 — референсная регистрация `{name,id,title,order}`,
  `ctx.slots.inject`/`ctx.effect` с dispose, `onTheme`, `deactivate` без потери
  регистраций; node-харнесс моста в `cargo test`; шелл: persistent stage с
  несколькими iframe, вкладки слотов, resize, live-тема, `immediately`-монтаж,
  действия compose/notify. `b8bf8ef` `19c83f3`
- 65: capability-ассеты для многофайловых бандлов (`/asset/{capability}/…`,
  TTL, confined, CSP по origin) и module-table шим; серверные тесты режима.
- 66: headless smoke `scripts/plugin-smoke.py` (28 проверок: слоты, resize, тема,
  действия, eager, inject-порядок, ошибки плагина, exports-карта, многофайловый и
  module-table бандлы, hot reload открытой панели); Replica-оценка клиентской
  половины 98.0/100 (must 12/12).

- 67: eval-набор (ROADMAP 2.12): `evals/tasks/*.json` (10 задач) +
  `crates/cli/tests/evals.rs`; гейт success 10/10 и бюджет токенов на задачу,
  отчёт `target/evals/report.json`, джоба `evals`.
- 68: perf-гейты §20: `crates/perf` (`cargo test -p dshr-perf --release`) —
  промпт (100 инструментов), append+fsync, поиск 10k событий, сборка
  turn-контекста и память на 100k токенов; регресс > 20 % — блокер, джоба `perf`.
- 69: live-гейт compaction 100k: `crates/compaction/tests/live_gate_100k.rs` —
  сессия > 100k токенов, сохранение последних turn'ов дословно, reopen и
  продолжение после сжатия.
- 70: браузерный e2e `scripts/web-e2e.py` (headless Chrome/CDP, stdlib) +
  джоба `web-e2e`; попутно исправлен баг Web UI: «New session» не выбирала
  созданную сессию (отправка падала с «no session»).

- 71: plugin market: каталог `crates/server/src/catalog.json` получил
  `category`/`tags`/`homepage`/`verified`; `/v1/plugins/catalog` принимает
  `q`/`category`/`limit` и отдаёт категории; новый
  `GET /v1/plugins/catalog/{name}` — карточка пакета с оценкой, агрегатом
  отзывов и признаком установки; UI показывает категорию; тесты
  `crates/server/tests/plugins_market.rs` (4).

- 72: i18n остатка CLI: 24 новых метода `Lang` + `ExportLabels`; локализованы
  рантайм-сообщения `sessions list/show/export`, `plugin npm *`, `doctor`, префикс
  ошибки (`ошибка[code]`/`error[code]`); справка `plugin npm` и `sessions import`
  (+21 ключ × ru/en); 10 новых интеграционных тестов, `--json` не зависит от языка.

- 73: слоты клиентской половины: фикстура `demo8` с DSH-именами
  (`panel`/`message.action`/`settings.action`); smoke-проверка, что все три имени
  доходят до шелла и рендерятся как вкладки/активации (29/29). Региональный
  монтаж слотов и полный React-набор — вне scope по ADR-013 §5 (в ADR записана
  причина: ленивый монтаж половины не позволяет выбрать регион до загрузки
  бандла).

- 74: Web UI diff: сервер передаёт input инструмента в событии `tool_start`,
  карточка рисует `-old`/`+new` для `edit`, `+`-строки для `write` и тело
  `apply_patch`; e2e расширен вторым ходом (`edit`) и проверяет превью и
  фактическое изменение файла (12/12).

- 75: настройки провайдера в Web UI: `GET/POST /v1/config/model` (маршрут из
  известных, модель, `base_url`; запись user-слоя `config.yaml` атомарно, 0600),
  диалог Settings с подсказкой о перезапуске и явным «ключи — из окружения»;
  e2e расширен проверками загрузки/сохранения настроек (16/16).

- 76: региональный монтаж слотов клиентской половины — отложенная фаза
  [ADR-013](adr/013-cordis-compat.md) §5 («Региональный монтаж слотов»):
  - eager registry-кадр на каждую `dsh.client`-половину (скрытый iframe
    `mode=registry`, порядок `inject`, `immediately`-половины первыми среди
    корней) — панель, сайдбар и диалоги знают о регистрациях до открытия панели;
  - по sandboxed iframe `mode=slot` на каждую регистрацию и на каждое вхождение
    повторяющегося региона (`message.action`, `tool.card`,
    `sidebar.session.action`), с `slot`/`id`/`context`;
  - полный словарь DSH 0.2.2-rc.2 (116 ключей `@deepseek-ai/dsh-client-ui-*`)
    плюс legacy `panel`/`message.action` разложен по 17 регионам шелла
    (`panel`, `sidebar.top/footer/session.action`, `settings.section/icon/action`,
    `message.action`, `tool.card`, `conversation.header`, `composer.actions`,
    `shell.leading/bottom/overlay`, `plugins.add/detail`, `approval.detail`);
    незнакомое имя — fallback в панель плагина;
  - протокол моста v1.2: токен фрейма и режим в каждом сообщении, `region` в
    `slots`/`resize`, `context` повторяющихся регионов, адресный `action-result`
    тому фрейму, который спросил; host-страница встраивает
    `window.__DSHR_FRAME__`, `/v1/plugins/{name}/host` валидирует
    `frame`/`mode`/`slot`/`id`/`context` (400 на мусор).
  - Код: `crates/cordis/src/bridge.js` (таблица `SLOT_REGIONS`, регионы,
    slot-режим), `crates/cordis/src/client.rs` (`FrameSpec`/`FrameMode`),
    `crates/server/src/lib.rs` (валидация host-запроса),
    `crates/web/src/main.rs` (`SlotRegion`/`SlotFrame`/`RegistryFrames`/
    `PanelRegion`, монтаж/снятие регионов диалогов, live-тема, resize, reload).
  - Проверки: `crates/cordis/tests/bridge_harness.mjs` (покрытие каталога,
    slot-режим, регионы), `crates/server/tests/plugins_live.rs` (валидация),
    `scripts/plugin-smoke.py` — 45 проверок в headless Chrome,
    регресс e2e `scripts/web-e2e.py` — 16/16. Replica клиентской половины —
    **98.1/100** (must 13/13, `docs/replica/client-half-parity.md`).
  - React-компоненты, `ownerProps`/hooks/chain/hoc-семантика слотов и
    произвольная инъекция в DOM шелла по-прежнему вне scope.

- 77: экспериментальный enabler React в клиентской половине —
  `dsh.client.runtime`: `package.json` половины может объявить один
  package-relative classic-скрипт; host-страница встраивает его в sandboxed-документ
  и исполняет **до** бандла, половина публикует `window.__DSHR_MODULES__`, а
  module-table бандлы (`__ModuleLoader__.load({id, factory})`) резолвят
  `require(specifier)` из этой таблицы (неизвестный id — явная ошибка с
  подсказкой про `dsh.client.runtime`). Валидация: путь confined к пакету,
  ≤ 8 МиБ, содержимое с `</script>` отклоняется (400), CSP остаётся network-free,
  отдельной подписи у runtime нет.
  - Код: `crates/cordis/src/client.rs` (`ClientHalf.runtime`/`runtime_path`,
    `host_page`/`host_page_assets` принимают байты runtime),
    `crates/server/src/lib.rs` (`plugin_client_host` читает и валидирует runtime,
    `contains_script_close`).
  - React 18.3.1 PoC проверен вручную (в репозиторий не коммитился): React UMD
    в `runtime.js` (143 197 байт) + module-table бандл — рендер в slot-iframe,
    `useState` (клик 0→1), `ctx.onTheme` (тема light) работают.
  - Проверки: cordis lib 21 (+4), server `plugins_live` 9 (+3), smoke — 48/48
    (+3: runtime discovery, host-page runtime-before-bundle, module-table
    require render); workspace 353 passed + 3 ignored (desktop 21, итого 374),
    web-e2e 16/16.
  - Границы: это enabler, а не полная React-совместимость — общей модульной
    таблицы шелла, design-system и SlotMap-семантики по-прежнему нет;
    [ADR-040](adr/040-react-client-half.md) (Proposed) фиксирует тиры A → B → C
    (D не рекомендуется), решение по B/C — за владельцем.

- 78: PoC тира B по [ADR-040](adr/040-react-client-half.md) — общая модульная
  таблица хоста `dsh.client.host_modules`:
  - половина объявляет массив id из allowlist (`react`, `react/jsx-runtime`,
    `react/jsx-dev-runtime`, `react-dom`, `react-dom/client`); незнакомый id
    отклоняется при discovery (`crates/cordis/src/client.rs`: `HOST_MODULES`,
    `HOST_RUNTIME_VERSION`, `host_runtime_url`, `PageRuntime`);
  - host-страница opt-in-кадра добавляет перед бандлом один classic-скрипт
    `GET /v1/runtime/react-18.3.1.js` — версионированный immutable-артефакт
    (React 18.3.1 UMD + ReactDOM UMD + `bootstrap.js`, ~143 КБ;
    `Cache-Control: public, max-age=31536000, immutable`), поэтому браузер
    скачивает React один раз на страницу, а не получает ~140 КБ внутри каждой
    host-страницы; кадры без `host_modules` остаются полностью network-free;
  - порядок исполнения: shared runtime → plugin-shipped `dsh.client.runtime`
    (может переопределять записи) → бандл; контракт `require` не меняется —
    рантайм публикует те же id в `window.__DSHR_MODULES__`;
  - CSP opt-in-кадра: `script-src 'unsafe-inline' blob: <origin сервера>`,
    `connect-src 'none'` и остальные директивы прежние; инъекция через
    Host-header закрыта тем же `request_origin` с валидацией, что и для assets;
  - исходники, MIT-лицензии и контрольные суммы — `crates/server/assets/runtime/`;
    `scripts/check-runtime-assets.sh` проверяет checksums и включён в
    `scripts/check.sh` и CI Linux test job;
  - JS-реалм у каждого iframe свой (изоляция не ломается), поэтому «одна версия
    на страницу» — это один артефакт и одна загрузка, а не общий JS-контекст.
  - Проверки: cordis lib 25 (+4: parse/dedup `host_modules`, отклонение
    незнакомого id, порядок shared → plugin runtime → bundle, отсутствие
    shared-скрипта без opt-in), server `plugins_live` 12 (+3: маршрут и
    заголовки/404, host-страница opt-in ссылается на рантайм и не встраивает
    React + CSP с origin, враждебный Host не инжектируется), smoke — 53/53
    (+5: discovery `host_modules`, immutable-артефакт, host-страница без
    встроенного React, реальный React рендерится из общего рантайма,
    `useState`-интерактивность); workspace 360 passed + 3 ignored (desktop 21,
    итого 381), web-e2e 16/16. Smoke-фикстура `hosted` не содержит React в
    бандле.
  - Границы тира B: design-system `@deepseek-ai/dsh-client-ui-*`, React-контекст
    и композиция шелла, SlotMap-семантика (`ownerProps`, framework-hooks,
    cardinality/scope) по-прежнему отсутствуют; версия React закреплена за
    сервером, у плагинов нет выбора версии; CSP opt-in-кадров открывает origin
    для скриптов. Решение по тиру C — за владельцем, D не рекомендуется.

- 79: C-lite PoC по [ADR-040](adr/040-react-client-half.md) — React-компоненты
  слотов поверх общего рантайма хоста, host-hooks/сервисы, частичный
  design-system шим и один реальный DSH-плагин в conformance:
  - React-режим моста: если в кадре есть host React (`host_modules` содержит
    `react`), `ctx.slots.register(entry, Component)` трактуется как
    React-компонент (референсный контракт), а не imperative-колбэк
    `render(element, props)`. Мост держит один React-root на кадр, перерисовывает
    его при смене темы/host-state/context, ловит ошибки рендера error boundary и
    сообщает их шеллу (`error`); в регионе показывается
    `Plugin render error: …` (`crates/cordis/src/bridge.js`).
  - Host-state и hooks: шелл рассылает `host-state` (`theme`, `wide`, `session`,
    `sessions`) при `ready`, смене сессии/списка и темы
    (`crates/web/src/main.rs`: `push_host_state_all`); в props компонента
    появляются `useTheme()`, `useSession()`, `useSessions(selector?)`,
    `useHostState(selector?)` и `wide` (`wide` сейчас всегда true — записанное
    ограничение). Перерисовка не ремоунтит кадр.
  - Сервисы: `ctx.get('sessions')` отдаёт подмножество референсного сервиса
    (`currentProvideInfo.getSnapshot()`/`subscribe`, `get(id)`, `list()`);
    неизвестное имя — `undefined`, поэтому простой плагин деградирует мягко.
  - Дизайн-система: общий рантайм (148 140 байт: React 18.3.1 UMD + ReactDOM UMD
    + `bootstrap.js` + новый `primitives.js`) публикует независимый **частичный**
    шим `@deepseek-ai/dsh-client-ui-primitives` (`Button`, `StateDot`, `Tooltip`,
    `Icon`, `Spinner` на host React и CSS-переменных темы; не пакет upstream);
    allowlist `HOST_MODULES` в `crates/cordis/src/client.rs` расширен этим id;
    `crates/server/assets/runtime/README.md` и `checksums.txt` обновлены.
  - Conformance: реальный MIT-плагин `@kt11/dsh-deepseek-balance@0.2.1` вендорен
    фикстурой `crates/cordis/tests/fixtures/deepseek-balance/` (upstream-клиентская
    половина без изменений, sha256 `f0a40800…` в `package.json`; host-половина —
    заглушка; в манифесте только добавлен `host_modules`). В smoke он рендерит
    React-карточку в `sidebar.footer.action`, вставляет свой CSS и открывает popup
    по клику; данные показывают ветку «查询失败», потому что host-маршрутов DSH в
    dshr нет — записанное ограничение, рендер и интерактивность не ломаются.
  - Новые smoke-фикстуры: `hooks` (host-hooks и `wide`) и `ui-kit`
    (design-system шим). Проверки: bridge harness 73 (включая
    host-state/`ctx.get`), cordis lib 26, server `plugins_live` 12, smoke —
    60/60 (+7, было 53/53), workspace 361 passed + 3 ignored (desktop 21,
    итого 382), web-e2e 16/16.
  - Границы: это C-lite, а не полный тир C. Остаются C-full: полный
    design-system `@deepseek-ai/dsh-client-ui-*` и SlotMap-семантика
    (`ownerProps`, framework-hooks, cardinality/scope/chain/hoc), React-контекст
    и состояние шелла, композиция между плагинами, host-маршруты DSH-плагинов,
    выбор версии React, настоящий `@dshr/cordis-compat`-шим и расширение
    conformance-набора (сейчас n=1). Решение — за владельцем, D не рекомендуется.

## Что осталось

Закрыто по плану 14.11.2026: eval-набор (2.12), perf-гейты §20, live-гейт
compaction 100k, браузерный e2e, plugin market (каталог/поиск/карточка),
матрица §4 для T3, синхронизация TECH-DEBT/ROUNDS. Раунды 74–79 закрыли
визуальный diff в Web UI, экран настроек провайдера, региональный монтаж
слотов клиентской половины, enabler `dsh.client.runtime` (плагин приносит свой
UI-стек, включая React, внутрь слота), PoC тира B — общую модульную таблицу
хоста `dsh.client.host_modules` (React 18.3.1 с сервера одним immutable-артефактом
на страницу) — и C-lite: React-компоненты слотов с host-hooks/`ctx.get('sessions')`,
частичный design-system шим и первый реальный DSH-плагин в conformance.

Открыто и не заблокировано:

- Общая серверная база отзывов plugin market (сейчас home-scoped файл).
- Экспериментальные контракты клиентской половины (`dsh.client.runtime` и
  `dsh.client.host_modules`: classic script, без ESM и отдельной подписи, версия
  React закреплена за сервером, CSP opt-in-кадров открывает origin для скриптов)
  могут поменяться до полного тира C по [ADR-040](adr/040-react-client-half.md).
- C-lite-ограничения: design-system шим частичный (5 компонентов), `wide` в
  host-state сейчас всегда true, host-маршрутов DSH-плагинов нет (реальный плагин
  в smoke показывает ветку ошибки данных), conformance-набор — n=1.

Ожидает решения (ADR-040 Proposed, тир C/C-full):

- **Полный тир C** — C-lite PoC сделан (раунд 79): React-компоненты слотов,
  host-hooks/`ctx.get('sessions')` и частичный design-system шим работают.
  Чего по-прежнему нет: полного design-system `@deepseek-ai/dsh-client-ui-*`,
  SlotMap-семантики (`ownerProps`, framework-hooks, cardinality/scope/chain/hoc),
  React-контекста/состояния шелла и композиции между плагинами, host-маршрутов
  DSH-плагинов, выбора версии React и настоящего `@dshr/cordis-compat`-шима.
  Выбор полного тира C (React-остров в Leptos) — за владельцем; D не
  рекомендуется.

Заблокировано внешним условием:

- **Windows sandbox/сеть** — нужен реальный Windows-хост; CI-шаг есть, локально
  только компиляция.
- **Notarization** — нужны Apple Developer ID, сертификат и app-specific password.
- **GHCR-tap/brew на реальном GHCR** — нужны tap-репозиторий и учётные данные пакетов.

Подробная матрица и блокеры: [PARITY.md](PARITY.md), реестр долгов — [TECH-DEBT.md](TECH-DEBT.md).

## Формализация и каталог тестов (14.11.2026)

- `formal/` — три TLA+-модели (`CordisSupervisor` safety+liveness, `ClientAssets`,
  `ApprovalGate` safety+liveness), 5 конфигов TLC; `./formal/run_tlc.sh` →
  «all models checked». Абстракции и результаты: `docs/formal/README.md`.
- `docs/testing/test-cases.md` — каталог ~150 кейсов с ID, уровнями, привязкой к
  инвариантам и статусами; baseline `cargo llvm-cov`: 72.9% строк, 66.9% функций.
- Деривированные тесты: атомарность `register_all` под читателями, symlink-escape
  в ассетах, восстановление после падения sidecar, CSP-инъекция через `Host`,
  scope/TTL capability.
