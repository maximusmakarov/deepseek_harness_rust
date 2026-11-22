# Журнал раундов паритета (1–149)

Цель: довести `dshr` до паритета с DSH 0.2.2-rc.3 / официальным `apps/desktop`
(и community-плагином `dsh-desktop`). Каждый раунд — код/документация +
проверка. Итоговая сверка: 422 root-тестов (+3 ignored), 38 desktop-тестов и
11 unit-тестов web-крейта = **471**; bridge harness 121, client-half smoke 89, `clippy`/`fmt`/`deny` чисто, release-бинарь
воспроизводим.

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

- 80: расширение C-lite conformance до **n=2** и сервисы моста, которых ждут
  референсные плагины ([ADR-040](adr/040-react-client-half.md),
  [PLUGINS.md](PLUGINS.md)):
  - `ctx.get('slots')` — тот же реестр, что `ctx.slots`
    (`inject`/`register`/`remove`/`list`), плюс `entries(key)`;
    `ctx.get('locale')` — минимальный сервис (`getSnapshot()` →
    `{active: "en", locale: "en"}`, `subscribe`); неизвестное имя по-прежнему
    `undefined` (`crates/cordis/src/bridge.js`).
  - `ctx.inject(deps, callback)` — референсный fiber-хелпер: `callback(ctx)`
    вызывается только если резолвятся **все** зависимости, иначе возвращается
    пустой dispose. Опциональные интеграции DSH-плагинов
    (`betterSidebar`/`sidebarRight`/`webTerminals`) в dshr не смонтированы —
    это ровно та штатная деградация, на которую рассчитаны плагины.
  - Диагностика: последняя ошибка кадра доступна изнутри как
    `window.__DSHR_LAST_ERROR__` (`{message, stack}`) без раунд-трипа к шеллу —
    так разбирается несовместимость реального плагина.
  - Conformance n=2: к `@kt11/dsh-deepseek-balance@0.2.1` добавлен
    `dsh-remote@1.1.2` (MIT; клиентская половина upstream без изменений,
    sha256 `5016e5d3…` в `package.json`, host-половина — заглушка, в манифесте
    только добавлен `host_modules`), фикстура
    `crates/cordis/tests/fixtures/dsh-remote/`. Плагин регистрирует React-слоты
    в `conversation.session.header.actions`/`utilities` и
    `sidebar.workspaces.directoryFlow` и активируется без ошибок при отсутствии
    опциональных сервисов.
  - Найденные ограничения conformance-набора (вне n=2, записаны в
    [TECH-DEBT.md](TECH-DEBT.md) и [LIMITATIONS.md](LIMITATIONS.md)):
    `@michengai/dsh-archive-manager` 1.3.6 использует `new Function`/eval →
    нужен `'unsafe-eval'` в CSP (сознательно не включён);
    `@michengai/dsh-skills-manager` 1.4.8 падает на инициализации бандла
    (`Object.assign` над undefined) — нужен триаж исходников.
  - Проверки: bridge harness 79 (+6: alias `ctx.get('slots')`, `register` через
    него, `entries`, `locale`, `ctx.inject`, `__DSHR_LAST_ERROR__`), cordis lib
    26, server `plugins_live` 12, smoke — 63/63 (+3: второй реальный плагин в
    discovery, его registry-кадр без ошибок, монтаж слота в регион
    `conversation.header`), workspace 361 passed + 3 ignored (desktop 21, итого
    382), web-e2e 16/16; shared runtime не менялся (148 140 байт).
  - Границы: набор всё ещё n=2 (цель M5b 5.19 — 8–12 плагинов); мостовые
    сервисы — подмножество (нет проекций `sessions`, `sessionStats` и т.п.);
    eval-based бандлы требуют ослабления CSP, SlotMap-семантика и инъекция в DOM
    шелла по-прежнему вне scope — решение по C-full за владельцем.

- 81: conformance C-lite до **n=4** и доработки React-контракта, найденные
  реальными плагинами ([ADR-040](adr/040-react-client-half.md),
  [PLUGINS.md](PLUGINS.md)):
  - Тип компонента: `React.memo`/`forwardRef` возвращают объект с `$$typeof`, а не
    функцию; `normalize` в host-React-режиме теперь принимает такие типы (раньше
    падало `slot: render is required`).
  - Keyed-регистрации: регистрация с `key` без `id` получает уникальный id
    `<slot>:<key>` (второй key того же слота больше не заменяет первый), а
    `ctx.get('slots').entries(key)` отдаёт референсную форму с `component` и
    `options` для кода, который ищет свои entries.
  - Локали: `ctx.locale.register(ns, dicts)` + `t()` в props компонента
    (английский словарь, `{name}`-подстановки); таймеры `ctx.interval`/
    `ctx.timeout` регистрируются во фрейме и снимаются вместе с ним
    (`crates/cordis/src/bridge.js`).
  - Primitives: шим расширен `JsonBlock`/`MarkdownText` (нужны genui) — теперь
    `Button`, `StateDot`, `Tooltip`, `Icon`, `Spinner`, `JsonBlock`,
    `MarkdownText`; общий рантайм-артефакт — 149 442 байта
    (`crates/server/assets/runtime/`, `checksums.txt` обновлён).
  - Conformance n=4: к `@kt11/dsh-deepseek-balance@0.2.1` и `dsh-remote@1.1.2`
    добавлены `@ruoyang/dsh-plugin-deepseek-balance@1.0.2` (MIT; клиентская
    половина upstream без изменений; плавающий бейдж в `shell.overlay`,
    `ctx.effect`/`ctx.interval`/`ctx.locale`; sha256 `ca700695…` в `package.json`)
    и `dsh-genui@0.3.3` (MIT; тумблер GenUI в `conversation.input.left`,
    регистрации через `React.memo` и keyed-слот `conversation.chat.node`; sha256
    `8f0500ad…`); фикстуры
    `crates/cordis/tests/fixtures/{ruoyang-balance,dsh-genui}/`, host-половины —
    заглушки.
  - Проверки: bridge harness 87 (+8: React-режим с memo-типом, host props/hooks,
    `t()`-место компонента, keyed-ids, форма `slots.entries`), smoke — 66/66
    (+3: discovery двух новых плагинов, рендер бейджа в `shell.overlay`, рендер
    тумблера GenUI в `conversation.input.left`), cordis lib 26, server
    `plugins_live` 12, workspace 361 passed + 3 ignored (desktop 21, итого 382),
    web-e2e 16/16.
  - Границы: набор n=4 (цель M5b 5.19 — 8–12 плагинов); по-прежнему нет полного
    design-system `@deepseek-ai/dsh-client-ui-*`, SlotMap-семантики
    (`ownerProps`/framework-hooks/cardinality/scope), React-контекста шелла и
    композиции между плагинами, host-маршрутов DSH-плагинов и выбора версии
    React; `@michengai/dsh-archive-manager` требует `'unsafe-eval'` в CSP
    (сознательно не включён), `@michengai/dsh-skills-manager` падает на
    инициализации бандла — оба вне набора и ждут триажа.

- 82: C-lite conformance до **n=6** — ещё два реальных MIT-плагина без изменений
  клиентской половины ([ADR-040](adr/040-react-client-half.md),
  [PLUGINS.md](PLUGINS.md)):
  - `dsh-session-superagent@0.10.12` коммитит composer dock
    (`conversation.composer.dock`) и активируется без ошибок; sha256
    `d58f0fa2…`, фикстура
    `crates/cordis/tests/fixtures/dsh-session-superagent/`, host-половина —
    заглушка;
  - `dsh-mnemon@0.5.46` (sha256 `d967402b…`, фикстура
    `crates/cordis/tests/fixtures/dsh-mnemon/`) — крупный multi-region плагин
    (шапка чата, composer, панель, настройки, `plugins.*`): активируется,
    публикует слоты и сверяет собственные shadow-регистрации через
    `ctx.slots.subscribe`/`getVersion`/`entriesOfSlot` — референсная поверхность
    слотов, добавленная в раунде 81, выдержала реальный плагин (правок моста не
    потребовалось).
  - Conformance-набор: 6 реальных MIT-плагинов (4 + 2), все fixture с sha256 в
    `package.json`; `@michengai/dsh-archive-manager` (`'unsafe-eval'`) и
    `@michengai/dsh-skills-manager` (падение на инициализации бандла) остаются
    вне набора.
  - Проверки: smoke — 69/69 (+3: discovery двух новых плагинов, монтаж composer
    dock `dsh-session-superagent`, активация `dsh-mnemon` с публикацией слотов),
    bridge harness 87, cordis lib 26, server `plugins_live` 12, workspace 361
    passed + 3 ignored (desktop 21, итого 382), web-e2e 16/16; общий
    рантайм-артефакт не менялся — 149 442 байта.
  - Границы: набор n=6 (цель M5b 5.19 — 8–12 плагинов); C-full по-прежнему
    открыт — полный design-system `@deepseek-ai/dsh-client-ui-*`, SlotMap-семантика
    (`ownerProps`/framework-hooks/cardinality/scope), React-контекст/состояние
    шелла и композиция между плагинами, host-маршруты DSH-плагинов, выбор версии
    React.

- 83: C-lite conformance до **n=9** — цель [ADR-013](adr/013-cordis-compat.md) §6
  (8–12 реальных плагинов) достигнута; ещё три MIT-плагина с upstream-клиентской
  половиной без изменений ([ADR-040](adr/040-react-client-half.md),
  [PLUGINS.md](PLUGINS.md)):
  - `@99galaxy/dsh-plugin-market@1.2.2` (sha256 `27abb2db…`, фикстура
    `crates/cordis/tests/fixtures/dsh-plugin-market/`) — запись в сайдбаре
    (`sidebar.panellist` → регион `sidebar.footer`), в манифесте только `react`;
  - `dsh-plugin-codegraph-panel@0.6.5` (sha256 `f834fadb…`, фикстура
    `crates/cordis/tests/fixtures/dsh-plugin-codegraph-panel/`) — панель
    `settings.section` (upstream-манифест: `immediately: true`), в фикстуре — только `react`;
  - `@goodandready/dsh-plugin-notify@0.4.9` (sha256 `7a2843fa…`, фикстура
    `crates/cordis/tests/fixtures/dsh-plugin-notify/`) — строки в `plugins.item`
    и `plugins.row.config` (регион `plugins.detail`), primitives + react.
  - Все три активируются, публикуют слоты и не оставляют ошибок; host-половины —
    заглушки, манифест фикстуры опирается на `host_modules`. Conformance-набор:
    9 реальных MIT-плагинов; `@michengai/dsh-archive-manager` (`'unsafe-eval'` в
    CSP) и `@michengai/dsh-skills-manager` (падение инициализации бандла)
    остаются вне набора.
  - Проверки: smoke — 72/72 (+3: discovery трёх новых половин, активация трёх
    плагинов с публикацией слотов, монтаж кадра market в регион
    `sidebar.footer`), bridge harness 87, cordis lib 26, server `plugins_live` 12,
    workspace 361 passed + 3 ignored (desktop 21, итого 382), web-e2e 16/16;
    общий рантайм-артефакт не менялся — 149 442 байта.
  - Границы: числовая цель ADR-013 §6 достигнута (n=9), но это по-прежнему
    C-lite. C-full (полный design-system `@deepseek-ai/dsh-client-ui-*`,
    SlotMap-семантика `ownerProps`/framework-hooks/cardinality/scope/chain/hoc,
    React-контекст/состояние шелла и композиция между плагинами, host-маршруты
    DSH-плагинов, выбор версии React) — решение владельца, а не незакрытая
    C-lite-работа.

- 84: **C1** — host HTTP-маршруты плагинов работают end-to-end
  ([ADR-040](adr/040-react-client-half.md), [PLUGINS.md](PLUGINS.md)):
  - sidecar поднимает **loopback-only** HTTP-сервер (`127.0.0.1`, случайный порт,
    per-process токен), bootstrap даёт `ctx.inject(['webServer'])` +
    `webServer.register({name, kind, path, handler})` с обычными Node
    `(req, res)` и `credentials.resolve(name)` из окружения (ключи не покидают
    sidecar); зарезервированные префиксы (`/v1`, `/dshr`, `/openapi`, UI-ассеты)
    не регистрируются;
  - менеджер после старта читает внутренний инструмент `__dshr_routes` (порт,
    токен, таблица путей), хранит её в `Running` и отдаёт `http_route(path)`
    (только exact-пути); внутренние `__dshr_*`-инструменты отфильтрованы из
    реестра инструментов агента;
  - fallback-прокси сервера: только allowlist, пробрасывает method/query/body
    (request ≤ 1 МиБ, response ≤ 8 МиБ, таймаут 15 с), reserved- и неизвестные
    пути → обычный dshr 404; permissive CORS для opaque-origin кадров (preflight
    отвечает глобальный `CorsLayer`); маршруты живые — reload/stop убирает их из
    allowlist;
  - CSP: кадры с `host_modules` получают `connect-src <origin сервера>` — реальная
    причина «Failed to fetch» в C-lite (браузерный fetch плагина к своему
    маршруту блокировался); внешняя сеть по-прежнему запрещена, кадры без
    `host_modules` остаются network-free;
  - реальная host-половина `@kt11/dsh-deepseek-balance@0.2.1` больше не
    застаблена: upstream host half вендорен дословно (host sha256 `8771ae90…`,
    client sha не менялся — `f0a4080…`); smoke через кадр плагина проверяет
    `/dsh-balance/deepseek` → `500 {ok:false, code:"not-configured"}`,
    `/dsh-usage/sessions?tz=UTC` → `200` c JSON и массивом `sessions`, неизвестный
    путь → dshr 404.
  - Проверки: smoke — 75/75 (+3: проксирование host-маршрута, query/status,
    неизвестный путь), bridge harness 87, cordis lib 26, server `plugins_live`
    14 (+2: `proxies_plugin_host_routes`, `plain_frames_stay_network_free`),
    workspace 363 passed + 3 ignored (desktop 21, итого 384), web-e2e 16/16;
    общий рантайм-артефакт не менялся — 149 442 байта.
  - Границы C1: сервисов `sessions`/`sessionProjections` нет (usage-маршруты,
    строящие статистику по сессиям, отдают пустые/ошибочные данные), нет
    SSE/WebSocket/стриминга, только exact-пути; plugin-маршруты не
    аутентифицированы (как в DSH — публичные для локального web-профиля) и в
    LAN-режиме доступны с интерфейса сервера; sidecar подключается через MCP,
    поэтому meta-инструменты MCP (`list_resources`/`read_resource`/
    `list_prompts`/`get_prompt`) по-прежнему видны в реестре инструментов
    host-only плагина. Полный C-full (design-system, SlotMap-семантика,
    React-контекст шелла, триаж отклонённых плагинов) — решение владельца.

- 85: **C2** — design-system шим расширен с 7 до 30+ экспортов по фактическим
  импортам реальных плагинов, conformance **n=11**
  ([ADR-040](adr/040-react-client-half.md), [PLUGINS.md](PLUGINS.md)):
  - компоненты `Input`, `Switch`, `Checkbox`, `Tag`, `Pill`, `Menu`,
    `MenuSurface` (forwardRef-контейнер), `Modal`, `Toast`, `SegmentedControl`
    (radiogroup + управление стрелками), `SegmentedTabs` (tablist),
    `DisclosureRow`, `PathLabel`, `TextShimmer` — семантика и props взяты из
    фактических вызовов `dsh-mnemon`/`dsh-session-superagent` (`Menu` c
    `items/onSelect/anchor/selectedId/align/side/portal`, `Modal` c
    `title/description/footer/onClose/closeLabel` и порталом, `Toast` c
    `text/tone/holdMs/onDone/icon` и авто-dismiss, `SegmentedControl` c
    `options/value/onChange`, `SegmentedTabs` c `items/value/onChange`,
    `DisclosureRow` c `title/open/expandable/collapsedContent/headerAccessory`);
    прежние `Button`/`StateDot`/`Tooltip`/`Icon`/`Spinner`/`JsonBlock`/
    `MarkdownText` сохранены, `Tooltip` стал `forwardRef`;
  - именованные `Icon*`-компоненты (close/chevron/search/settings/link/refresh/
    warning/skill/check/info/plus/trash/copy) с SVG-глифами и **Proxy-fallback**
    на генерик для любого неизвестного `Icon*`-имени — импорт больше не даёт
    `undefined` (частая причина краха бандла); плюс `writeClipboard`
    (navigator.clipboard + execCommand fallback) и хуки
    `useAnchoredPosition`/`useDismissOnOutsidePointer`;
  - conformance **n=11**: добавлены `dsh-plugin-debugging@0.8.1` (MIT, client
    sha `9b828160…`, фикстура `crates/cordis/tests/fixtures/dsh-plugin-debugging/`)
    и `dsh-plugin-studio@0.3.1` (MIT, client sha `1194c18b…`, фикстура
    `crates/cordis/tests/fixtures/dsh-plugin-studio/`) — клиентские половины
    upstream без изменений (sha256 в `package.json`), host-половины — заглушки;
    обе активируются без ошибок, но их UI, требующий host-сервисов
    (`pluginManager`/settings), остаётся несмонтированным — та же штатная
    деградация, что у `dsh-remote` с опциональными сервисами;
  - smoke: +2 проверки — дерево всех смоделированных примитивов рендерится в
    host-React кадре (12 текстовых маркеров, включая портальные Menu/Modal/Toast),
    а реальные вклады `dsh-plugin-notify`/`dsh-mnemon` монтируются в диалоге
    Plugins без `__DSHR_LAST_ERROR__`;
  - Проверки: smoke — 79/79 (+2), bridge harness 87, cordis lib 26, server
    `plugins_live` 14, workspace 363 passed + 3 ignored (desktop 21, итого 384),
    web-e2e 16/16; общий рантайм-артефакт вырос до **176 725 байт**
    (React + ReactDOM UMD + `bootstrap.js` + `primitives.js`).
  - Границы C2: полного design-system `@deepseek-ai/dsh-client-ui-*` и визуальных
    токенов нет (шим — независимая реализация по фактическим импортам, набор
    компонентов будет расти по мере роста conformance), host-сервисов
    `pluginManager`/`settings` нет (поэтому UI `dsh-plugin-debugging`/
    `dsh-plugin-studio` инертен), SlotMap-семантика, React-контекст шелла и
    композиция между плагинами остаются C-full — решение владельца.

- 86: **Хардненинг C1** — prefix-маршруты, живая таблица маршрутов в статусе,
  фильтр MCP-псевдо-инструментов и loopback-only политика для plugin-маршрутов
  ([ADR-040](adr/040-react-client-half.md) §«Хардненинг C1», [PLUGINS.md](PLUGINS.md)):
  - prefix-маршруты: `webServer.register` принимает `kind: "exact"` и
    `kind: "prefix"`; sidecar матчит exact раньше prefix, среди prefix — самый
    длинный; `CordisManager::http_route` и fallback-прокси сервера работают так
    же (`crates/cordis/src/bootstrap.mjs`, `crates/cordis/src/manager.rs`,
    `crates/server/src/lib.rs`); reserved-префиксы и неизвестные пути остаются
    404;
  - диагностика: `GET /v1/plugins/status` теперь отдаёт per-plugin `routes`
    (живая таблица host-маршрутов) рядом с `tools` — `PluginStatus.routes`
    (`crates/cordis/src/manager.rs`);
  - шум MCP: псевдо-инструменты моста `list_resources`/`read_resource`/
    `list_prompts`/`get_prompt` отфильтрованы из реестра инструментов
    Cordis-плагина (рядом с внутренними `__dshr_*`) — `crates/cordis/src/lib.rs`;
  - LAN-политика: plugin-маршруты неаутентифицированы (у iframe нет токена),
    поэтому при bind вне loopback прокси отвечает 403, пока явно не передан
    `--allow-lan-plugin-routes` / `ServerConfig.allow_plugin_routes_lan`
    (`crates/cli/src/serve_cmd.rs`, `crates/server/src/lib.rs`). Это закрывает
    записанный в TECH-DEBT пункт «plugin routes public as DSH / LAN exposure»;
  - Проверки: server `plugins_live` 15 (+1:
    `plugin_routes_are_loopback_only_unless_opted_in`; `proxies_plugin_host_routes`
    расширен проверками prefix-маршрутов, таблицы `routes` в статусе и фильтра
    meta-tools), workspace 364 passed + 3 ignored (desktop 21, итого 385),
    smoke 79/79 без изменений, bridge harness 87, cordis lib 26, web-e2e 16/16;
    общий рантайм-артефакт не менялся — 176 725 байт (React + ReactDOM UMD +
    `bootstrap.js` + `primitives.js`).
  - Остатки C1: сервисы `sessions`/`sessionProjections` (C1.1) для
    usage-маршрутов отсутствуют, нет SSE/WebSocket/стриминга через прокси;
    host-сервисов `pluginManager`/`settings` для UI `dsh-plugin-debugging`/
    `dsh-plugin-studio` по-прежнему нет. Полный C-full (design-system,
    SlotMap-семантика, React-контекст шелла) — решение владельца.

- 87: **C1.1** — сервисы `sessions`/`sessionProjections` в sidecar
  ([ADR-040](adr/040-react-client-half.md) §«C1.1», [PLUGINS.md](PLUGINS.md)):
  - сервер: `serve()` генерирует per-run внутренний токен, пишет
    `<home>/cordis/host.json` (`{url, token}`) и публикует
    `GET /v1/internal/session-snapshot` — recent ≤50 сессий
    (id/title/created_at/model/turns/steps и usage-события
    assistant/turn/step/tokens); содержимое сообщений и промпты не покидают
    сервер; без настроенного токена эндпоинт отвечает 404, при неверном
    `x-dshr-internal-token` — 401 (`crates/server/src/lib.rs`:
    `write_host_bridge`, `internal_session_snapshot`,
    `ServerConfig.internal_token`);
  - sidecar: `connect()` передаёт `DSHR_HOST_BRIDGE`
    (`<home>/cordis/host.json`), bootstrap читает файл, раз в 5 с
    (unref-таймер) обновляет снимок и транслирует его в DSH-shaped сервисы
    через `ctx.get`: `sessions` (`list()`/`get(id)`, `header.createdAt` в
    epoch ms, DSH-shaped `log` с `request/context`, `session/title`,
    `assistant/message` usage) и `sessionProjections`
    (`snapshot(session).values` с `sessionStats` turns/steps/`llmMs`=0 и
    `tokenUsage`) (`crates/cordis/src/lib.rs`, `bootstrap.mjs`:
    `refreshHostSessions`, `dshSessionFromSnapshot`);
  - эффект: usage-маршруты реального `@kt11/dsh-deepseek-balance`
    (`/dsh-usage/sessions`) перечисляют сессии dshr и считают totals; smoke
    опрашивает маршрут из кадра плагина и требует `sessions` ≥ 1 и totals;
  - Проверки: smoke — 80/80 (+1: C1.1 bridge), bridge harness 87,
    cordis lib 26, server `plugins_live` 16 (+1:
    `internal_session_snapshot_feeds_the_host_bridge`, покрывает 401/404),
    workspace 365 passed + 3 ignored (desktop 21, итого 386), web-e2e 16/16;
    общий рантайм-артефакт не менялся — 176 725 байт.
  - Границы C1.1: это снимок, а не живой RPC — данные могут отставать до 5 с,
    проекции упрощены (`llmMs`=0, без истории тредов), содержимое сессий
    намеренно не мостится; SSE/WebSocket/стриминга по-прежнему нет, полный
    C-full (design-system, SlotMap-семантика, React-контекст шелла) —
    решение владельца.

- 88: **решение по C-full и полная верификация** — записан
  [ADR-041](adr/041-c-full-decision.md): C-full (полный design-system,
  SlotMap-семантика, React-контекст шелла, host-сервисы
  `pluginManager`/`settings`, streaming) **отложен**; дальнейшие шаги —
  точечные инкременты по спросу, критерий возврата — ≥3 реальных плагина,
  которым не хватает этих возможностей (сейчас таких нет). Полный
  `scripts/check.sh` зелёный («all gates passed»), reproducible-build
  проверен, release-артефакты пересобраны и проверены (`.app` проходит smoke
  80/80).

Решение по полной React-совместимости зафиксировано в
[ADR-041](adr/041-c-full-decision.md): C-full отложен, кандидаты следующих
точечных инкрементов — host-сервисы `pluginManager`/`settings` (закрывают UI
`dsh-plugin-debugging`/`dsh-plugin-studio`), SlotMap PoC `conversation.*` и
streaming-прокси.

- 89: **C1.2** — seat-`inject`, Cordis-генераторы и `configForms`
  ([ADR-040](adr/040-react-client-half.md) §«C1.2», [PLUGINS.md](PLUGINS.md)):
  - seat `inject`: регистрация слота может отдать компоненту дополнительные
    props фабрикой `inject: () => ({...})` (framework seat); они мержатся при
    монтировании (React-режим — вместе с host-props, imperative — в props
    колбэка). Если фабрика бросила (нет host-сервиса), компонент рендерится
    деградированно, событие пишется в лог кадра, а плагин **не** помечается
    упавшим (`crates/cordis/src/bridge.js`: `injectedProps`);
  - Cordis-генераторы: `ctx.slots.inject`, `ctx.inject` и `ctx.effect`
    принимают `function*`; тело исполняется, все `yield`-disposer'ы
    собираются и вызываются на teardown в обратном порядке (`cordisDisposer`).
    Это разблокировало `dsh-plugin-studio`, который регистрирует вкладки
    генератором;
  - `configForms`: browser-bridge отдаёт `ctx.get('configForms')` и
    `ctx.configForms` с `get(entryId)` → форма `{getSnapshot, subscribe,
    mutate(ops, revision)}`; snapshot-объект стабилен между нотификациями
    (требование `useSyncExternalStore`), revision-контракт соблюдается
    (`mutate` с устаревшей ревизией возвращает `false`). Форма в кадре
    in-memory и writable — правки **не переживают reload кадра**. Host-половинам
    bootstrap даёт файловый вариант `<home>/cordis/config-forms/<entry>.json`
    (`crates/cordis/src/bootstrap.mjs`);
  - эффект: `dsh-plugin-studio@0.3.1` рендерит реальные вкладки в Settings
    (`settings.plugins.tab`, свой CSS, честные error-состояния, когда
    host-маршруты — заглушки), а `dsh-plugin-debugging@0.8.1` — карточку
    настроек в диалоге Plugins (`plugins.bundle.config`): переключатель и Save
    проходят round-trip через `configForms`;
  - Проверки: smoke — 82/82 (+2: реальная settings-вкладка studio с seat
    inject; карточка debugging рендерится и сохраняется), bridge harness
    **90** (+3: seat-props, генераторный `slots.inject`, генераторный
    `effect`), cordis lib 26, server `plugins_live` 16, workspace 365 passed
    + 3 ignored (desktop 21, итого 386), web-e2e 16/16; общий
    рантайм-артефакт не менялся — 176 725 байт.
  - Границы C1.2: browser-`configForms` in-memory (без персистентности при
    reload кадра), семантика конфигурационного движка DSH не воспроизведена
    (host-сервисы `pluginManager`/`settings` по-прежнему отсутствуют, хотя
    settings-UI теперь рендерится и деградирует честно), провалы seat-inject
    только логируются; полный C-full по [ADR-041](adr/041-c-full-decision.md)
    отложен.

- 90: **chore(perf-gate)** — `scripts/check.sh` повторяет wall-clock perf-прогон
  один раз при падении (ложные красные на загруженном хосте: Spotlight-индексация
  поднимала load average до 20–45 и p95 append выходил за 5 мс); реальная
  регрессия валит оба прогона. Полный `scripts/check.sh` — «all gates passed»,
  perf после стабилизации нагрузки зелёный (p95 append в бюджете).

- 91: **C1.3** — слот-контракт по референсному каталогу
  ([ADR-040](adr/040-react-client-half.md) §«C1.3», [PLUGINS.md](PLUGINS.md)):
  - каталог слотов больше не рукописный: `crates/cordis/src/slot-catalog.json`
    сгенерирован из `deepseek-ai/deepseek-harness` (`packages/extensions/
    cordis-client-runner/src/client/slot-catalog.ts`, коммит `39163dd`) — 122
    слота с `kind` (single 45 / list 48 / keyed 22 / chain 4 / hoc 3), `scope`
    (root 57 / session 60 / session-maybe 5), framework standard-kit и флагом
    slot-`inject`; host-страница публикует его как
    `window.__DSHR_SLOT_META__` (`crates/cordis/src/client.rs`);
  - регистрация повторяет reference `SlotCore`: undeclared-слот бросает
    (исключение — dshr extension-пространства `panel`/`message.action`/
    `custom.*`/`x-*`/`dshr.*`), `keyed` требует `key`, `list`/`hoc` — `id`,
    `chain` — `select`; вторая регистрация в ту же ячейку (`single` — слот,
    `keyed` — key, иначе id) на том же `priority` бросает, другой `priority`
    затеняет (рендерится меньший);
  - регистрация может объявлять дочерние слоты
    `children: {key: {kind, scope}}`; объявление живёт ровно столько же,
    сколько запись, и каскадно снимает свои регистрации — это разблокировало
    собственные слоты `dsh-mnemon` (`mnemon.component.*`);
  - `ctx.slots.inject(key, factory)` ждёт **объявления** слота и
    перезапускается при появлении (reference registry semantics), а не просто
    «когда сервисы готовы»; новый `ctx.slots.spec(key)` отдаёт
    `{key, kind, scope, standardProps, inject}` для интроспекции, как в
    референсе (им пользуется mnemon);
  - shell получает только shadowing-winners (`single` — один, `keyed` — по
    одному на ключ, `list`/`chain`/`hoc` — все слои в порядке
    priority → order → регистрация), а props компонента собираются в
    референсном порядке: standard-kit из каталога (нереализованные hooks
    остаются вызываемыми и возвращают `undefined`, `sessionId` — из
    host-state) → seat `inject` → ownerProps из
    `window.__DSHR_FRAME__.ownerProps` → `t`;
  - словарь регионов пополнился 6 upstream-ключами (`conversation.input.
    model.pane`, `forge.*`) — 124 ключа включая 2 legacy; synthetic
    smoke-фикстуры получили обязательный `key` для keyed-слотов;
  - Проверки: bridge harness **98** (+8: undeclared/keyed/chain/duplicate
    throws, single shadowing, catalog standard kit, ownerProps, plain values),
    smoke 82/82 (три зелёных прогона подряд; реальные плагины регистрируются по
    тем же правилам, `dsh-mnemon` проходит дочерние слоты и deferred inject),
    cordis lib 26, server `plugins_live` 16, workspace 365 passed + 3 ignored
    (desktop 21, итого 386), web-e2e 16/16; общий рантайм-артефакт не менялся —
    176 725 байт.
  - Границы C1.3: `chain` не диспатчится по `select`, а монтируется слоями;
    `hoc` не вкладывает `children` между кадрами; store-seats
    (`PropsStore`/`BoundActions`) и mounted-model сессий не реализованы;
    ownerProps поставляет только frame-seam (harness/расширения shell), реальные
    render-сайты шелла (`conversation.chat.node` и т.п.) данных пока не дают.
    Полный C-full (design-system, store/renderer, chain/hoc-рендеринг) —
    [ADR-041](adr/041-c-full-decision.md).

- 92: **C1.4** — host-сервисы webServer/baseUrl/configForms по reference-контракту
  ([ADR-040](adr/040-react-client-half.md) §«C1.4», [PLUGINS.md](PLUGINS.md)):
  - `webServer` (`packages/host/webserver/src/index.ts`): exact- и prefix-таблицы
    теперь независимы, дубликат `(kind, path)` бросает
    `webserver: duplicate <kind> route`; путь обязан быть абсолютным pathname
    без завершающего `/`, `?` и `#` и не пересекаться с reserved-префиксами
    dshr API; `getOrigin()`/`port`/`protocol` отдают origin dshr-сервера из
    `<home>/cordis/host.json` (host-bridge), а не loopback-порт
    sidecar-прокси; `registerFallback`/`registerUpgrade` в sidecar не поддержаны
    и бросают явную ошибку (`crates/cordis/src/bootstrap.mjs`: `validRoutePath`,
    `webRoutes`, `unsupportedWebServerFeature`) — реальным плагинам набора не
    нужны, долг записан;
  - `ctx.baseUrl` — loader-context свойство reference Cordis: `file://` URL
    корня пакета плагина, нужен host-половинам (например,
    `dsh-plugin-studio`), которые резолвят пути от `baseUrl`;
  - `configForms` (`packages/client/ui-settings/src/client/config-form-types.ts`):
    snapshot ровно `{status, value, base, user, revision: number, writable}` со
    стабильной идентичностью между чтениями, `mutate(ops, expectedRevision?)` с
    атомарными `set`/`unset` и числовым revision-fence, хелперы
    `set(field, value)`/`unset(field)`; форма в кадре in-memory (правки не
    переживают reload), у host-половин — файловая
    `<home>/cordis/config-forms/<entry>.json` (`bridge.js`, `bootstrap.mjs`);
  - Проверки: bridge harness **102** (+4: форма-контракт, стабильный snapshot,
    set с числовой ревизией, unset), server `plugins_live` 16 (proxy-тест
    расширен правилами duplicate-route и trailing-slash), smoke 82/82, cordis
    lib 26, workspace 365 passed + 3 ignored (desktop 21, итого 386), web-e2e
    16/16; общий рантайм-артефакт не менялся — 176 725 байт.
  - Границы C1.4: `registerFallback`/`registerUpgrade` не поддержаны в sidecar
    (нужны для чужих SPA/WS-плагинов), browser-`configForms` не персистится
    через reload кадра, живого RPC-зеркала настроек нет (in-memory/файловые
    stand-in'ы); остаток reference-работы — реальный порт `ui-primitives` из
    `packages/client/ui-primitives` (следующий кандидат), store seats/renderer,
    chain dispatch/hoc-nesting и ownerProps от реальных shell-сайтов.

- 93: **C2.1** — атомы design-system и все иконки портированы из
  reference-исходников ([ADR-040](adr/040-react-client-half.md) §«C2.1»,
  [PLUGINS.md](PLUGINS.md)). Раньше `primitives.js` был независимым «похожим»
  шимом; теперь он портирован из `deepseek-ai/deepseek-harness` @ `39163dd`,
  `packages/client/ui-primitives/src`:
  - **атомы** — `Button` (variants `primary`/`ghost`/`outline`/`toolbar`,
    sizes `md`/`sm`, leading `icon`, `forwardRef`), `Input` (wrapper + leading
    icon), `Switch` (role switch, `aria-label`, thumb без видимого текста),
    `Checkbox` (native input + label, `data-modal-autofocus`), `Tag`
    (`data-tone`: outline/solid/neutral/quiet/success/info/warning/danger),
    `Pill` (span/button по наличию `onClick`, `active`), `SegmentedControl`
    (tablist + indicator, roving tabindex, walk-клавиши, `<id>-<value>`/
    `-panel`), `SegmentedTabs` (Pill-табы, grid indicator, Left/Right/Home/End
    с переносом фокуса), `DisclosureRow` (header/row, `expandOnRowClick`,
    `previewChevron`, `collapsedContent`, `headerAccessory`, running-shimmer),
    `TextShimmer` (nested-context + decoration/sweep), `StateDot`
    (done/warning/ongoing/error/idle, SVG-spinner, `appearance: "step"`),
    `PathLabel` (directory/name split, clipped-маска, `title`), `Toast`
    (portal, `role=alert`, success check-circle, `holdMs`+fade, actions,
    anchor), `Modal` (portal, mask, Escape, focus, warning tone, headless,
    footer), `Menu` (data-rows/separators/labels, fixed-позиционирование,
    outside/Escape, `selectedId`/`selectedIds`, portal; полный
    submenu/focus-движок **не** портирован), `MenuSurface` (`forwardRef`,
    `data-menu-material`/`phase`), `Tooltip` (label-based клон якоря,
    hover/focus-бабл; placement-движок **не** портирован);
  - **иконки** — все **204** reference-экспорта `Icon*` сгенерированы из
    upstream `icons/artwork.ts` (точные SVG-пути, stroke regular 1 / medium 1.3)
    и инлайнены вместо document-спрайта; для будущих имён остаётся
    Proxy-fallback;
  - **стили** — те же `--dsw-alias-*` токены, что использует reference (с
    fallback'ами), поэтому тема dshr управляет видом;
  - **совместимость** — synthetic-фикстуры приведены к reference-контрактам
    (`StateDot` с `state:"done"`, `Tooltip` с element-child), а C2-probe теперь
    проверяет структурные признаки, а не только текст: `role=switch` с
    `aria-label`, `data-tone`, `tablist`-табы, число `svg`, `label.dshr-checkbox`,
    `textContent` у `[data-path-label]`;
  - **расширения dshr (не из reference)** — `Spinner`, упрощённые
    `JsonBlock`/`MarkdownText` (upstream зависит от shiki/katex/mdast),
    `useAnchoredPosition`; `writeClipboard` и `useDismissOnOutsidePointer`
    портированы по reference-алгоритмам;
  - Проверки: smoke **82/82** (два зелёных прогона подряд; C2-probe усилен до
    структурных проверок), bridge harness 102, cordis lib 26, server
    `plugins_live` 16, workspace 365 passed + 3 ignored (desktop 21, итого 386),
    web-e2e 16/16; общий рантайм-артефакт вырос с прежних 176 725 до
    **375 894 байт** (React + ReactDOM UMD + `bootstrap.js` + `primitives.js`;
    cached immutable один раз на страницу).
  - Границы C2.1: полный `Menu` (submenu/focus-машинерия) и полный `Tooltip`
    (placement-engine) не портированы — работают упрощённые варианты;
    `MarkdownText`/`JsonBlock` остаются упрощёнными dshr-расширениями;
    store-seats (`PropsStore`/`BoundActions`)/renderer-семантика,
    `chain`-диспетчеризация/`hoc`-вложение и `ownerProps` от реальных
    shell-сайтов остаются C-full.

- 94: **C2.2** — полный Tooltip
  ([ADR-040](adr/040-react-client-half.md) §«C2.2», [PLUGINS.md](PLUGINS.md)).
  Единственный упрощённый атом из C2.1 доведён до reference-порта
  `packages/client/ui-primitives/src/Tooltip.tsx`:
  - `Tooltip` — label-резолвер (вычисляется только при показе), независимые
    hover/focus-триггеры с `delayMs`/`focusDelayMs`, `openOnClick`-pinning с
    outside-pointer/Escape/Tab-dismiss, `onlyWhenClipped`, `maxWidth`,
    `portal`, `data-side`/`data-pinned`, nested-suppression через контекст,
    `aria-describedby` при pinning, pointer-modality (фокус после указателя не
    поднимает бабл), touch-hover исключён, forward ref сквозь клон якоря;
  - `tooltip-placement.ts` портирован как есть: requested side → opposite →
    perpendicular, viewport-clamp и overflow-fallback;
  - `input-modality.ts` (`data-input-modality`, `pointerModality`) и
    `ShortcutKeys` (keycaps с `+`-разделителями) портированы и экспортированы;
  - C2-probe дополнен узлом `Tooltip` в synthetic-фикстуре (`ShortcutKeys` экспортирован и отрисовывается внутри Tooltip);
  - Проверки: smoke **82/82**, bridge harness 102, cordis lib 26, server
    `plugins_live` 16, workspace 365 passed + 3 ignored (desktop 21, итого 386),
    web-e2e 16/16; общий рантайм-артефакт вырос с **375 894** до
    **387 613 байт**.
  - Границы C2.2: полный `Menu` (submenu/focus-машинерия) по-прежнему не
    портирован; `MarkdownText`/`JsonBlock` остаются упрощёнными
    dshr-расширениями (upstream требует shiki/katex/mdast); store
    seats/renderer, `chain`-диспетчеризация/`hoc`-вложение и `ownerProps` от
    реальных shell-сайтов — C-full.

- 95: **C2.3** — `Menu`/`MenuItemButton` по reference-ядру
  ([ADR-040](adr/040-react-client-half.md) §«C2.3», [PLUGINS.md](PLUGINS.md)).
  Последний упрощённый компонент набора переписан по
  `packages/client/ui-primitives/src/Menu.tsx`/`MenuRow.tsx`:
  - **ряды** — data-`items` и компонентные `MenuItemButton`, separators/labels,
    `reserveIcon`-колонки (contextual), selection `check`/`fill`, `danger`,
    `current`, disabled-reason через reference `Tooltip`, `shortcut` +
    `ShortcutKeys`, aria (`menuitem`/`haspopup`/`expanded`/`disabled`/
    `describedby`/`keyshortcuts`/`current`);
  - **структура** — `children`-ряды, `footer`-группа, `dense`/`compact`,
    portal-позиционирование по anchor-rect (`minWidthFromAnchor`, `anchorPoint`,
    `align`/`side`), contextual-атрибут;
  - **клавиатура** — Arrow/Home/End walk с roving focus, Escape закрывает и
    возвращает фокус (`focusWithoutRing`), Tab «settles» ряд / Shift+Tab
    уходит, ArrowRight/Left входят/выходят из submenu, `autoFocus`;
  - **submenu** — one-level панель, открывается hover/ArrowRight и
    закрывается по ArrowLeft/Escape/выбору; **не** портированы полная
    `useMenuSubmenu` машинерия (single-panel back-page, меж-уровневый grace)
    и `MenuGroup`/`observeStickyMenuGroups`;
  - **поведение** — outside-pointer dismissal, закрытие при blur в iframe,
    `closeOnPointerLeave` с 200-мс grace (`usePointerGrace`), возврат фокуса
    на триггер после выбора/Escape; экспортированы `MenuItemButton`,
    `MENU_TRANSITION_MS`, `usePointerGrace`, `focusWithoutRing`,
    `isMobileDevice`;
  - Проверки: smoke **82/82** (menu-рендер в C2-probe и реальные плагины),
    bridge harness 102, cordis lib 26, server `plugins_live` 16, workspace
    365 passed + 3 ignored (desktop 21, итого 386), web-e2e 16/16; общий
    рантайм-артефакт вырос с **387 613** до **405 323 байт**.
  - Границы C2.3: полная submenu-машинерия + `MenuGroup`/sticky-groups не
    портированы; `MarkdownText`/`JsonBlock` остаются упрощёнными
    dshr-расширениями (upstream требует shiki/katex/mdast); store
    seats/renderer, `chain`-диспетчеризация/`hoc`-вложение и `ownerProps` от
    реальных shell-сайтов — C-full.

- 96: **C2.4** — reference submenu page-layout и `MenuGroup`
  ([ADR-040](adr/040-react-client-half.md) §«C2.4», [PLUGINS.md](PLUGINS.md)).
  Закрыты оставшиеся части reference-`Menu`:
  - **submenu layout** по `menu-submenu-position.ts`: если сбоку от
    родительской карточки дочерняя не помещается ни справа, ни слева,
    включается single-page layout — дочерние ряды заменяют родительскую
    страницу, сверху non-selecting back-row (`data-menu-back`, chevron-left,
    `submenuBackLabel` или label родителя) + separator; Escape/ArrowLeft
    возвращают на родительскую страницу с фокусом на родительском ряду;
  - **focus-контракт**: hover открывает submenu без перехвата фокуса, а
    ArrowRight/клик — с фокусом на первом дочернем ряду (как в reference);
  - **hover grace**: 200-мс окно между родительской и дочерней карточками —
    уход курсора в дочернюю отменяет закрытие (`cancelSubmenuClose`);
  - **`MenuGroup` + `observeStickyMenuGroups`** портированы по
    `MenuGroup.tsx`: `section[role=group]` с `aria-labelledby`,
    `data-menu-group`/`data-menu-group-start`/`data-menu-group-heading` и
    sticky-заголовком (`data-stuck`) на IntersectionObserver/ResizeObserver;
  - C2-probe теперь рендерит `MenuGroup`, submenu-родителя и проверяет
    `menuRows >= 3`.
  - Проверки: smoke **82/82** (C2-probe с `MenuGroup`/submenu-родителем и
    `menuRows >= 3`), bridge harness 102, cordis lib 26, server `plugins_live`
    16, workspace 365 passed + 3 ignored (desktop 21, итого 386), web-e2e
    16/16; общий рантайм-артефакт вырос с **405 323** до **415 972 байт**.
  - Границы C2.4: детальная `useMenuSubmenu`-механика focus-transition/
    page-anchor остаётся упрощённой (у нас CSS/JS-позиции вместо полного
    reference-движка); `MarkdownText`/`JsonBlock` на конец C2.4 оставались
    упрощёнными dshr-расширениями (upstream требует shiki/katex/mdast); store
    seats/renderer, `chain`-диспетчеризация/`hoc`-вложение и `ownerProps` от
    реальных shell-сайтов — C-full.

- 97: **C2.5** — reference `JsonBlock` и API-совместимое подмножество
  `MarkdownText`
  ([ADR-040](adr/040-react-client-half.md) §«C2.5», [PLUGINS.md](PLUGINS.md)).
  Закрыт markdown-долг C2.1–C2.4 настолько, насколько это возможно без
  upstream-стека:
  - **`JsonBlock`** портирован из `markdown/JsonBlock.tsx`: collapsible
    (▾/▸), props `label`/`payload`/`defaultOpen`/`truncatedLabel`, cap
    20 000 символов с хвостом «truncated», безопасный `JSON.stringify` с
    fallback на `String` (циклические payload'ы и `undefined`);
  - **`MarkdownText`** получил reference-поверхность
    (`text`/`streaming`/`labels`/`fileMentions`/`pathImages`/`variant`/
    `omitHtml`/`textInteractionSource`) и dependency-free рендерер
    GFM-подмножества: заголовки 1–6, абзацы, ul/ol, fenced code, blockquote,
    hr, inline `code`/`**bold**`/`*italic*`/`~~strike~~`, ссылки и изображения
    (только `http(s)`/`mailto`; `javascript:`/`data:` отбрасываются, а относительные
    изображения не рендерятся), raw HTML не рендерится (как при `omitHtml`);
    `variant="compact"` даёт `data-markdown-variant`/`data-text-interaction-source`;
  - **не** портирован upstream-стек: micromark/mdast, shiki-подсветка, KaTeX,
    footnotes и инкрементальный streaming — записано как ограничение
    (MarkdownText — не полный рендерер DSH, а API-совместимое подмножество);
  - C2-probe теперь рендерит `MarkdownText` (заголовок, bold, fenced code,
    ссылка) и `JsonBlock` (expanded JSON) и проверяет DOM/текст
    (`.dshr-markdown h1`, `.dshr-markdown pre code`, `.dshr-json-block-body`).
  - Проверки: smoke **82/82** (C2-probe с MarkdownText/JsonBlock), bridge
    harness 102, cordis lib 26, server `plugins_live` 16, workspace 365 passed
    + 3 ignored (desktop 21, итого 386), web-e2e 16/16; общий
    рантайм-артефакт вырос с **415 972** до **424 801 байта**.
  - Границы C2.5: markdown-стек upstream (mdast/shiki/KaTeX/footnotes/
    streaming) не входит; детальная `useMenuSubmenu`-механика
    focus-transition/page-anchor остаётся упрощённой; store seats/renderer,
    `chain`-диспетчеризация/`hoc`-вложение и `ownerProps` от реальных
    shell-сайтов — C-full; `Spinner`/`useAnchoredPosition` — dshr-расширения.

- 98–101: **C2.6/C3.1/C1.5 и ownerProps**
  ([ADR-040](adr/040-react-client-half.md) §«C2.6/C3.1/C1.5», [PLUGINS.md](PLUGINS.md)).
  Четыре контракта, у которых был точный reference-ориентир:
  - **C2.6 submenu-механика** (`primitives.js`): side-card vs single-page
    выбирается по измеренной ширине дочерней карточки
    (`menu-submenu-position.ts`), submenu позиционируется `fixed` с
    flip/clamp относительно родительской карточки (right/left, top-clamp по
    viewport), клавиатура маршрутизируется через `activeList` (родительский
    список или дочерняя страница), Escape/ArrowLeft из дочерней страницы
    возвращают фокус на родительский ряд и закрывают submenu, hover grace
    200 мс между уровнями, наведение на другой ряд сворачивает открытый
    submenu, `data-menu-back`/`submenuBackLabel` — как в reference; hover
    проверяется через `menuHoverAllowed` (не touch, `any-hover: hover`);
  - **C3.1 store-seats**: в общую module-таблицу добавлен
    `@deepseek-ai/dsh-client-store` (`createSnapshotStore`, `defineStore`,
    `shallowEqual`; reference `packages/client/store`) — `zustand`/`immer`-
    семантика воспроизведена plain deep-copied draft'ом, `persist`-ключ
    принимается, но не персистится; регистрация слота может объявить
    `store: handle|factory`, фрейм владеет одним instance на scope-key
    (`session`/`session-maybe` → id сессии, остальное — root), компонент
    получает `useStore(selector, eq)` и baked `actions`, а `inject`-фабрика
    для session-scope получает reference-аргументы `(sessionId, actions)`;
    instance переиспользуется по `(id, scopeKey)`;
  - **C1.5 fallback-маршруты**: `webServer.registerFallback(handler)`
    реализован как в reference (одно fallback-место; второй владелец
    бросает `webserver: fallback already registered`), sidecar отдаёт флаг
    `fallback` в `__dshr_routes`, `CordisManager::http_fallback()` находит
    единственного владельца, dshr проксирует в fallback только
    не-зарезервированные несовпавшие пути (reserved-префиксы dshr API/UI
    по-прежнему 404); `registerUpgrade` остаётся явно не поддержанным;
  - **ownerProps plumbing**: шелл может передать render-site owner props в
    кадр (`?ownerProps={...}` в `/v1/plugins/{name}/host`) — сервер
    валидирует JSON-объект и лимит длины как у `context`, встраивает в
    `window.__DSHR_FRAME__.ownerProps`, мост мержит их в props после
    store/seat share (порядок reference `PropsRuntime`); реальные
    render-сайты шелла (`conversation.chat.node` и т.п.) данных пока не
    поставляют — нужна chat/turn-модель Leptos-шелла (записано в долг).
  - Проверки: smoke **82/82** (C2-probe получил store-engine check:
    `defineStore` → `create` → `actions.inc` → snapshot `n === 1`), bridge
    harness **105** (+3 store-seat: `useStore`/`actions` в props, мутация
    framework-owned instance, inject-аргументы `(sessionId, actions)`),
    server `plugins_live` **17** (+1 fallback-маршрут, reserved-путь не
    уходит в fallback; frame-config тест проверяет `ownerProps` и
    отклонение не-объекта), cordis lib 26, workspace **366** passed +
    3 ignored (desktop 21, итого **387**), web-e2e 16/16; общий
    рантайм-артефакт вырос с **424 801** до **430 139 байт**.
  - Границы: C2.6 закрыл submenu-механику (side-card/single-page,
    focus-transition/page-anchor, hover grace) — упрощённых остатков
    submenu больше нет; C3.1 закрыл store seats, но draft — plain deep-copy
    (без `immer`-патчей) и `persist` не персистится; C1.5 закрыл
    `registerFallback`, а `registerUpgrade` (WebSocket) остаётся явно не
    поддержанным; ownerProps работает как plumbing (frame-seam), но
    реальные shell-сайты данных не дают; upstream markdown-стек
    (mdast/shiki/KaTeX/footnotes/streaming) и chain dispatch/hoc nesting в
    frame-per-slot архитектуре остаются как записано выше.

- 102: **C4.1** — chat/turn-модель в Leptos-шелле и chain-элекция по owner
  props ([ADR-040](adr/040-react-client-half.md) §«C4.1»,
  [PLUGINS.md](PLUGINS.md)). Шелл начал поставлять реальный render-site
  owner-контекст, а мост — owner-специфичный dispatch:
  - **chat/turn-модель** (`crates/web/src/main.rs`): каждое user-сообщение
    открывает новый turn, ordinal пишется в `data-turn` каждого сообщения и в
    owner share; `SlotFrame`/`SlotRegion` получили `owner_props`, который едет
    в кадр как `&ownerProps=...` (URL-encoded JSON);
  - **реальные owner shares**: `sidebar.session.action` → `{wide:true}`;
    `message.action` для user-сообщений → `{source:{role,index}, kind:"user"}`
    (референсный `UserBeforeOwnerProps`); `composer.actions` →
    `ComposerChainProps` `{sessionId, session:{id,model,turns},
    pendingInteraction:null}`;
  - **chain-элекция** (`crates/cordis/src/bridge.js`): кадр chain-слота
    вызывает `select(ownerProps)` своей регистрации; `false` → `clear()`,
    пустой DOM и репорт `chain-unselected`, `true` → рендерится выбранная
    запись, то есть owner-специфичный dispatch без серверного JS;
  - Проверки: bridge harness **107** (+2: chain-select `false`/`true`),
    web-e2e **18** (+2: turn ordinal у сообщений, смонтированные
    `composer.actions`/`message.action` регионы), server `plugins_live` 17
    (ownerProps-валидация), smoke 82/82, cordis lib 26, workspace 366 passed
    + 3 ignored (desktop 21, итого 387); общий рантайм-артефакт не менялся —
    430 139 байт (общих правок рантайма в раунде не было).
  - Границы: owner props с функциями (например `settings.section.close`) не
    передаются — нет callback-моста; при нескольких `select === true`
    рендерятся все кадры (per-frame фильтрация, а не единый owner-renderer);
    `conversation.chat.node` и остальные conversation-слоты ждут полного
    chat-view; upstream markdown-стек, `registerUpgrade` (WebSocket) и
    детальная renderer-механика остаются как записано выше.

- 103: **C2.7** — upstream markdown-стек. Вместо dependency-free GFM-подмножества
  модульная таблица получает отдельный `markdown.js` на 814 222 байта (сборка
  `scripts/build-markdown-runtime.mjs`), который переопределяет `MarkdownText`:
  micromark/mdast, shiki, KaTeX, footnotes и инкрементальный streaming; C2-probe
  проверяет KaTeX, shiki, footnotes и streaming. Общий рантайм-артефакт вырос с
  430 139 до **1 244 361 байта**. Остаётся renderer-семантика ядра (какие слоты
  рендерит шелл и как компоненты делят состояние) — это часть C-full
  ([ADR-041](adr/041-c-full-decision.md)), а не порт библиотек.
- 104: **C4.2** — callback bridge, chain owner-renderer, hoc nesting, полный
  chat-view и `registerUpgrade`. Owner props с функциями едут маркерами
  `{"$callback":"<name>"}` и материализуются мостом в функции, которые шлют
  `ctx.action("callback", …)` в шелл (`settings.section.close`, `openFile` →
  `/v1/workspace/file`, `inspectCall`, `forkAt`, `loadImage`); большие owner
  shares идут сообщением `owner-props` после `ready`. Транскрипт проецируется в
  turn/node-модель, conversation-слоты (`conversation.chat.node`,
  assistant-actions, images, reasoning, text-interaction, turnTail, user.before,
  flow, markdown.fence, composer-chain) монтируются на узел, два core-узла чата
  рендерит шелл; `hoc` вкладывает `children` через якоря родительского кадра,
  chain-слот обслуживает единый owner-renderer. `webServer.registerUpgrade`
  поддержан: sidecar держит exact-path таблицу upgrade-маршрутов (дубликат
  бросает, путь валидируется, `x-dshr-sidecar-token`), публикует её в
  `__dshr_routes.upgrades` и per-plugin `upgrades`, dshr-сервер туннелирует
  upgraded-соединение (`plugin_upgrade_proxy`: replay request-head, handshake
  апстрима, `hyper::upgrade` + `copy_bidirectional`). Проверки: bridge harness
  **119** (+12), smoke **84/84** (+owner callback закрывает Settings), web-e2e
  **18/18**, `plugins_live` **18** (+`proxies_plugin_websocket_upgrade`).
- 105: **S1+S2 (desktop hardening по официальному `apps/desktop`)** — раунд
  начат со сверки с upstream `39163dd` (0.2.2-rc.3) и закрыл три поведения
  официальной оболочки, которые у `dshr` отсутствовали:
  - `crates/desktop/src/instance.rs`: эксклюзивный `<home>/desktop.lock` на всё
    время работы; второй запуск форвардит папку из argv через
    `<home>/desktop-pending-folder` и выходит, а работающий shell перезапускает
    сервер с этой папкой; при рестарте из трея lock ждётся до 3 с (передача
    между поколениями);
  - `crates/desktop/src/login_env.rs`: один прогон логин-шелла `<shell> -ilc
    'env -0'` между delimiters, таймаут `DSHR_DESKTOP_LOGIN_SHELL_TIMEOUT_MS`
    (1–2 147 483 647 мс, по умолчанию 10 000), фильтр `PWD`/`OLDPWD`/`SHLVL`/
    `_`, probe-переменных и owned-имён (`DSHR_*`/`TAURI_*`/`DSH_HOME`);
    окружение передаётся `dshr serve` до `desktop.env`, поэтому ключ
    провайдера из `~/.zshrc` наконец виден GUI-запуску;
  - `crates/desktop/src/cli.rs`: `<home>/bin/dshr` (+`.cmd`) — launcher к
    бинарю из бандла с `--home`/`--profile`, работает при закрытом приложении,
    перезаписывается на старте (переезд бандла подхватывается); трей
    **Install CLI command…** линкует его в `~/.local/bin/dshr`, не перезаписывая
    чужую команду.
  - Документация: пин upstream в [REFERENCE.md](../reference/REFERENCE.md)
    (`66491b05`/rc.1 → `39163dd`/rc.3, +клоны `apps/desktop`), раздел
    «Официальный `apps/desktop`» в [PARITY.md](PARITY.md), [DESKTOP.md](DESKTOP.md).
  - Проверки: desktop-тесты **38** — 35 unit + 3 интеграционных (было 21;
    +17 на lock/forward, парсер env, shim, ссылку и второй запуск бинаря), `--check` живьём: `desktop check ok`, в `desktop.log`
    `login shell environment: 54 variables from /bin/zsh`; `fmt`/`clippy -D
    warnings` чисто.
  - Отложено владельцем 16.11.2026: NSIS-инсталлятор, Windows-подпись,
    mandatory-policy и дифференциальные обновления официальной оболочки.
    Остаётся решением владельца C-full ([ADR-041](adr/041-c-full-decision.md)).

- 106: **C5.1 — затенение встроенного рендера ядра** (первый срез фазы C-full,
  [ADR-042](adr/042-c-full-phase.md)). Встроенный вид `conversation.chat.node`
  больше не захардкожен за `user`/`assistant-step`: шелл решает ячейку по
  приоритету (`CORE_RENDERER_PRIORITY = 0`), как reference SlotMap.
  - `crates/web/src/main.rs`: `chat_node_replaced_by_plugin` /
    `core_renderer_shadowed` / `keyed_registration`; плагин затеняет встроенный
    рендер только `priority < 0` (так делает реальный `dsh-genui` для
    `assistant-step`), регистрация с приоритетом по умолчанию оставляет
    встроенный вид; затеняются только `keyed`-ячейки, `single` остаются
    shell-owned.
  - Границы записаны в [PLUGINS.md](PLUGINS.md) §«Затенение встроенного рендера
    ядра»: builtin-capture невозможен (Leptos-вид нельзя отдать в iframe как
    React-компонент), поэтому `dsh-genui` деградирует до собственного рендера;
    отказ кадра-победителя → встроенный вид — следующий срез C5.2.
  - Проверки: 6 unit-тестов web-крейта (host-target, `cargo test --bins`),
    `trunk build --release`, пересборка `dshr` (dist встроен в сервер);
    client-half smoke **88/88**, browser e2e — 17/18 локально: единственный
    провал `the tool really wrote the file` — вложенный seatbelt
    (`sandbox-exec: Operation not permitted`) в текущей песочнице агента, в CI
    e2e идёт без вложенной песочницы; web-e2e получил opt-in
    `DSHR_E2E_DANGER=1` для таких окружений.
  - Открыто: C5.2 (fallback на ошибку/пустой кадр + `__DSHR_RENDERER_STATE__`),
    C5.3 (живой поток сессий), C5.4 (mounted-model в шелле), C5.5
    (`pluginManager`), C5.6 (`settings`), C5.7 (закрытие фазы).

- 107: **C5.2 — fallback упавшего рендерера** (продолжение фазы C-full,
  [ADR-042](adr/042-c-full-phase.md)). Ошибка кадра-победителя больше не
  оставляет пустой узел:
  - `crates/web/src/main.rs`: `UiState.renderer_errors` (токен кадра → текст
    ошибки) заполняется в обработчике `error` (до статус-бара и цепочной
    элекции), а `chat_node_renderer` резолвит ячейку в `Builtin`, если токен
    победителя упал; случаи «нет регистрации», «приоритет 0 у core-узла» и
    «чужая ошибка» остаются прежними.
  - Диагностика на строке сообщения: `data-renderer-state="builtin|plugin"` и
    `data-renderer-token` — видно, кто отрисовал узел и что шелл откатился.
  - Осознанное ограничение: пустой (null) рендер — решение плагина, шелл его не
    подменяет (reference тоже); fallback только на ошибку.
  - Проверки: web-тесты **8** (+2: упавший победитель → builtin, чужая ошибка
    не снимает победителя), `trunk build --release`, пересборка `dshr`,
    client-half smoke **88/88**. Browser-probe с падающим `conversation.chat.node`
    требует chat+плагин в одном сценарии — объединён с C5.3, чтобы не строить
    отдельный стенд.
  - Открыто: C5.3 (живой поток сессий), C5.4 (mounted-model), C5.5
    (`pluginManager`), C5.6 (`settings`), C5.7 (закрытие).

- 108: **C5.3 — живой поток сессий** (фаза C-full, [ADR-042](adr/042-c-full-phase.md)).
  Плагинные host-половины больше не живут на 5-секундном опросе снимка:
  - `crates/server/src/lib.rs`: `AppState.feed` (broadcast, 256) +
    `publish_session` — после создания сессии, генерации заголовка и
    завершения turn'а в поток уходит **та же запись, что в снимке**
    (`session_summary_entry` вынесен и переиспользуется снимком), поэтому
    sidecar применяет дельту без второй модели данных. Обработчик
    `GET /v1/internal/session-events` (SSE, тот же `x-dshr-internal-token`,
    404 без токена) отдаёт `event: session`; при отставании подписчика —
    `event: resync`.
  - `crates/cordis/src/bootstrap.mjs`: начальная гидратация снимком, затем
    подписка на поток (fetch-stream, парсер SSE-кадров), `mergeHostSession`
    (replace/prepend), `resync`/обрыв → перегидратация и reconnect с backoff
    (0.5→10 с). Постоянного таймера 5 с больше нет; опрос снимка остаётся
    только как восстановление. `runHostFeed` стартует после `readline` на
    stdin, чтобы unref-timer не был единственным handle и sidecar не выходил.
  - Проверки: server **+3** (`internal_session_events_require_the_bridge_token`,
    `creating_a_session_pushes_it_to_the_internal_feed`,
    `a_finished_turn_pushes_usage_to_the_internal_feed`), cordis 26+1+8+2,
    client-half smoke **88/88** (включая C1.1: usage-маршрут реального
    balance-плагина перечисляет сессии). Итог: root 369 (+3 ignored).
  - Осталось в C5.4: live-проекции в шелле (`llmMs`, thread history, ≤ 1 с) и
    замена упрощённых `sessionStats`; затем C5.5 `pluginManager`, C5.6
    `settings`, C5.7 закрытие.

- 109: **C5.4a — mounted-model сессий в шелле** (фаза C-full,
  [ADR-042](adr/042-c-full-phase.md)). Сайдбар больше не ждёт действий
  пользователя, чтобы увидеть созданную/переименованную/завершённую сессию:
  - сервер отдаёт тот же поток и обычному клиенту: `GET /v1/sessions/events`
    (bearer, `token` в query) через общий `session_feed_stream`; внутренний
    эндпоинт C5.3 использует его же;
  - шелл подключает один `EventSource` (`connect_session_feed`) и применяет
    кадры: `merge_session_summary` заменяет известный id на месте и вставляет
    новый newest-first по `created_at`; `resync` → перечитывание снимка
    `/v1/sessions`; `parse_session_feed` отбрасывает чужие типы;
  - проверки: web **11** (+3: вставка newest-first, замена на месте, парсер
    кадров), server **20** (+1: 401 без токена и событие создания в публичном
    фиде), client-half smoke **88/88**; итог root 370 (+3 ignored).
  - Осталось в C5.4b: `sessionStats.llmMs` из латентности шага (сейчас
    `llmMs=0`) и история turn'ов; отдельной thread-модели в dshr нет
    (субагентские сессии — самостоятельные сессии), это будет записано как
    отсутствие, а не как долг. Затем C5.5 `pluginManager`, C5.6 `settings`.

- 110: **C5.4b — проекции: `llmMs` и история turn'ов** (фаза C-full,
  [ADR-042](adr/042-c-full-phase.md)). `sessionStats.llmMs=0` был
  заглушкой; теперь латентность шага — факт журнала:
  - `SessionEvent::AssistantMessage` получил
    `#[serde(default, skip_serializing_if = "Option::is_none")] latency_ms`;
    старые журналы реплеятся без поля (тест backward-compat), `None` не
    сериализуется;
  - `dshr-agent` замеряет провайдерский вызов (`Instant` до `stream`, после
    `assembler.finish()`) и пишет `Some(ms)`;
  - запись сессии (`session_summary_entry`) теперь несёт `llm_ms` (сумма
    латентностей), `turns_detail` (turn → steps/llm_ms/tokens) и
    `latency_ms` в каждом `usage_events`; sidecar отдаёт
    `sessionStats.llmMs` из `llm_ms`;
  - thread-модели (DSH threads) в dshr нет: субагентские сессии — отдельные
    сессии, поэтому это **отсутствие**, а не долг; per-turn история едет в
    `turns_detail`/`log`.
  - Проверки: session 6 (+1 backward-compat/round-trip), server 20 (+поля в
    фиде), agent/session-jsonl/acp зелёные, smoke **88/88**; итог root 371
    (+3 ignored). Дальше C5.5 `pluginManager`, C5.6 `settings`, C5.7 закрытие.

- 111: **C5.5 — host-сервис `pluginManager`** (фаза C-full,
  [ADR-042](adr/042-c-full-phase.md)). Плагин больше не обязан знать про CLI:
  - `crates/cordis/src/bootstrap.mjs`: сервис `pluginManager` —
    `list()`/`listBundles()`/`status()` (GET) и
    `setPluginEnabled(name, enabled)`/`installBundle(spec)`/`removeBundle(name)`
    (POST) через `http://<host>/v1/plugins/*` с per-run внутренним токеном из
    `host.json` (таймаут 10 с, ошибки возвращаются как `{ok:false,error}`);
  - `crates/server/src/lib.rs`: plugin-эндпоинты принимают
    `x-dshr-internal-token` наравне с bearer
    (`authorize_service`), поэтому host-половина не нуждается в
    пользовательском токене;
  - conformance-фикстура `crates/cordis/tests/fixtures/plugin-manager-probe`:
    host-половина с инструментами `plugin_manager_probe` (list) и
    `plugin_manager_toggle` (setPluginEnabled); тест `manager.rs` поднимает
    stub-сервер, проверяет GET/POST и возвращаемые данные.
  - Проверки: cordis **26+1+9+2** (manager +1), server **21** (+1 internal-token
    на `/v1/plugins/installed`), smoke **88/88**; итог root 373 (+3 ignored).
  - Границы (записаны): upstream `PluginManager` — pnpm-движок (bundle-патчи,
    registry-планы, `allowBuilds`, version exemptions); dshr отдаёт фасад над
    своими CLI-операциями, а не порт pnpm-менеджера. Осталось: C5.6
    `settings`-движок (схема/слои/миграции вместо in-memory `configForms`) и
    C5.7 закрытие фазы.

- 112: **C5.6a — settings store** (фаза C-full, [ADR-042](adr/042-c-full-phase.md)).
  Конфигурационные формы получили серверный источник истины вместо
  процесса-писателя:
  - `GET /v1/config-forms/{entry}` и `POST` с `ops` (`set`/`unset`, путь
    строкой через `.` или массивом), numeric `expected_revision`
    (несовпадение → `409`), атомарная запись через `.json.tmp` + rename,
    документ `<home>/cordis/config-forms/<entry>.json` в формате host-половин
    (`{value, revision}`, легаси-объект читается как revision 0);
  - доступ — `authorize_service` (пользовательский bearer или
    `x-dshr-internal-token`), имя entry валидируется
    (`[A-Za-z0-9._:-]{1,128}`, иначе `404`);
  - проверки: server **23** (+2: round-trip с 409 на устаревшую ревизию,
    unset, сохранение между запросами; bad entry и internal-token), smoke
    **88/88**; итог root 375 (+3 ignored).
  - Осталось: C5.6b — browser-`configForms` через host-relay на этот store
    (сейчас форма в кадре по-прежнему in-memory, правки не переживают reload),
    затем C5.7 — закрытие фазы.

- 113: **C5.6b — browser-`configForms` через host-relay** (фаза C-full,
  [ADR-042](adr/042-c-full-phase.md)). Правки форм больше не теряются при
  reload кадра:
  - `crates/cordis/src/bridge.js`: форма стала представлением серверного
    store — при создании шлёт `config-form-read`, `mutate` шлёт
    `config-form-mutate` с `expected_revision` и применяет полученный
    `config-form-snapshot`; локальное состояние обновляется только из ответа,
    `useSyncExternalStore` получает стабильный snapshot;
  - `crates/web/src/main.rs`: обработчики `config-form-read`/`-mutate`
    (shell держит пользовательский токен и ходит в `/v1/config-forms/{entry}`),
    ответ — `config-form-snapshot` тому же кадру. Поймано: numeric
    `expected_revision` приходит как f64, и `0.0` сервер отвергал как float —
    теперь конвертируется в u64 до сериализации;
  - harness **121** (+2: relay-контракт и отклонение устаревшей ревизии),
    client-half smoke **89/89** (+1: после Save в карточке
    `dsh-plugin-debugging` файл `<home>/cordis/config-forms/*.json` реально
    создан с `revision >= 1`).
  - Осталось: C5.7 — закрытие фазы (сверка трёх направлений, матрица, отчёт,
    решение о complete).

- 114: **C5.7 — фаза C-full закрыта** (16.11.2026,
  [ADR-042](adr/042-c-full-phase.md)). Сведены доказательства по трём
  направлениям и зафиксированы границы:
  - **renderer-семантика ядра:** встроенный вид — запись keyed-ячейки с
    `CORE_RENDERER_PRIORITY`; плагин затеняет её только `priority < 0`;
    упавший кадр-победитель откатывается в builtin (`data-renderer-state`);
    `single`-ячейки shell-owned; builtin-capture невозможен (Leptos-вид в
    iframe) — записано как граница, а не долг (коммиты `3e563b1`, `c3e24f6`);
  - **mounted-model сессий:** серверный поток `session` (внутренний и
    публичный), sidecar без 5-с опроса (`resync` → снимок), шелл применяет
    дельты в сайдбар, `llmMs` считается из латентности шага, `turns_detail`
    даёт per-turn историю; thread-модели DSH нет (субагентские сессии
    самостоятельны) — отсутствие, не долг (коммиты `7408db2`, `2d0bf02`,
    `ab79227`);
  - **host-сервисы:** `pluginManager` (фасад над CLI-операциями, internal
    token), settings store `/v1/config-forms/{entry}` с revision-fence и
    атомарной записью, browser-формы через relay и персистентность
    (коммиты `8c8b276`, `338df29`, `ee78493`).
  - **Гейты (чистое окружение, 16.11.2026):** полный `cargo test --workspace`
    — **376 passed + 3 ignored, 0 failed**; desktop **38**, web **11**, bridge
    harness **121**, client-half smoke **89/89**, browser e2e **18/18**;
    fmt/clippy/deny чисто. Прежние 373 passed и 17/18 e2e в песочнице агента
    были ограничением вложенного `sandbox-exec`/PTY — чистый прогон это
    подтвердил.
  - **Вне объёма C5 (LIMITATIONS/TECH-DEBT):** порт pnpm-движка upstream
    `PluginManager`, семантика конфигурационного движка DSH (схемы/слои/
    миграции), builtin-capture, thread-модель.
  - «Что осталось» ниже относится к прежним трекам (Cordis-совместимость,
    дистрибутивы, блокеры), а не к C-full.

## Что осталось

Закрыто по плану 14.11.2026: eval-набор (2.12), perf-гейты §20, live-гейт
compaction 100k, браузерный e2e, plugin market (каталог/поиск/карточка),
матрица §4 для T3, синхронизация TECH-DEBT/ROUNDS. Раунды 74–102 закрыли
визуальный diff в Web UI, экран настроек провайдера, региональный монтаж
слотов клиентской половины, enabler `dsh.client.runtime` (плагин приносит свой
UI-стек, включая React, внутрь слота), PoC тира B — общую модульную таблицу
хоста `dsh.client.host_modules` (React 18.3.1 с сервера одним immutable-артефактом
на страницу) — и C-lite: React-компоненты слотов с host-hooks/`ctx.get('sessions')`,
частичный design-system шим и первый реальный DSH-плагин в conformance; раунд 80
довёл conformance до n=2 (`dsh-remote@1.1.2`) и добавил сервисы моста
(`ctx.get('slots')`/`locale`, `ctx.inject`, диагностика `__DSHR_LAST_ERROR__`);
раунд 81 довёл conformance до n=4 и закрыл пробелы React-контракта
(типы `React.memo`/`forwardRef`, keyed-ids, `locale.register`/`t()`,
`ctx.interval`/`ctx.timeout`) и расширил primitives шим до семи компонентов;
раунд 82 довёл conformance до n=6 (`dsh-session-superagent@0.10.12`,
`dsh-mnemon@0.5.46`) и подтвердил референсную поверхность слотов
(`subscribe`/`getVersion`/`entriesOfSlot`) реальным плагином; раунд 83 довёл
conformance до **n=9** — целевой диапазон ADR-013 §6 (8–12 реальных плагинов)
закрыт (`dsh-plugin-market@1.2.2`, `dsh-plugin-codegraph-panel@0.6.5`,
`@goodandready/dsh-plugin-notify@0.4.9`; смонтированы сайдбар, панель настроек и
строки plugins-страницы), а раунд 84 — **C1**: host HTTP-маршруты плагинов
работают end-to-end (loopback-сервер sidecar + allowlist-прокси dshr-сервера,
`connect-src` на origin сервера для кадров с `host_modules`; реальная
host-половина `@kt11/dsh-deepseek-balance` отвечает своим JSON через кадр
плагина), а раунд 85 — **C2**: design-system шим расширен с 7 до 30+ экспортов
по фактическим импортам реальных плагинов (компоненты, именованные `Icon*` с
Proxy-fallback, хуки, `writeClipboard`), conformance доведён до **n=11**
(`dsh-plugin-debugging@0.8.1`, `dsh-plugin-studio@0.3.1` активируются без
ошибок; их settings-UI оставался несмонтированным до раунда 89, C1.2), а раунд
86 — **хардненинг C1**: prefix-маршруты (exact выигрывает, среди prefix — самый
длинный), живая таблица `routes` в
`GET /v1/plugins/status`, фильтр MCP-псевдо-инструментов
(`list_resources`/`read_resource`/`list_prompts`/`get_prompt`) из реестра
инструментов Cordis-плагина и loopback-only политика plugin-маршрутов (403 при
bind вне loopback без явного `--allow-lan-plugin-routes`), а раунд 87 — **C1.1**:
sidecar получает DSH-shaped сервисы `sessions`/`sessionProjections` из
внутреннего снимка сервера (`/v1/internal/session-snapshot`, токен в
`<home>/cordis/host.json`, обновление раз в 5 с), поэтому usage-маршруты
реальных плагинов считают статистику по сессиям dshr (проекции упрощены,
содержимое сессий не мостится), а раунд 89 — **C1.2**: seat-`inject` для
регистраций слотов, Cordis-генераторы в `slots.inject`/`ctx.inject`/`ctx.effect`
и host-сервис `configForms` (in-memory форма в кадре и файловая у host-половин),
поэтому settings-UI `dsh-plugin-studio` и `dsh-plugin-debugging` реально
рендерится; раунд 91 (**C1.3**) перевёл слот-контракт на сгенерированный каталог
DSH (122 слота с kind/scope/standard-kit) и референсные правила `SlotCore`
(undeclared/keyed/chain/duplicate, shadowing-winners, children-декларации,
deferred inject, standard-kit + ownerProps) — bridge harness 98 (+8); раунд 92
(**C1.4**) выровнял host-сервисы `webServer`/`ctx.baseUrl`/`configForms` по
reference-контракту — правила exact/prefix-маршрутов и duplicate-throw,
origin-аксессоры из host-bridge, `file://`-baseUrl, snapshot
`{status, value, base, user, revision, writable}` с числовой ревизией и
`set`/`unset` (bridge harness 102, +4); раунд 93 (**C2.1**) заменил независимый
design-system шим портированными атомами и всеми 204 иконками из
`deepseek-ai/deepseek-harness` @ `39163dd`
(`packages/client/ui-primitives/src`), поэтому `@deepseek-ai/dsh-client-ui-primitives`
больше не «похожий» шим, а reference-порт; раунд 94 (**C2.2**) довёл до
полного reference-порта `Tooltip` (placement-engine, `ShortcutKeys`,
`input-modality`); раунд 95 (**C2.3**) портировал ядро `Menu`/`MenuItemButton`
по `Menu.tsx`/`MenuRow.tsx` (ряды, selection, aria, клавиатурный walk,
one-level submenu, dismissal; рантайм-артефакт 405 323 байта), а раунд 96
(**C2.4**) закрыл reference submenu page-layout (single-page back-row с
`data-menu-back`, Escape/ArrowLeft с фокусом на родителе), hover grace
200 мс и `MenuGroup`/`observeStickyMenuGroups` (sticky-заголовки через
IntersectionObserver/ResizeObserver; рантайм-артефакт 415 972 байта), а раунд
97 (**C2.5**) портировал `JsonBlock` из reference и заменил упрощённый
markdown-шим API-совместимым dependency-free подмножеством `MarkdownText`
(GFM-подмножество, sanitize URL, reference-поверхность props; рантайм-артефакт
424 801 байт), поэтому на конец C2.5 остаток — упрощённая детальная
`useMenuSubmenu`-механика focus-transition/page-anchor, upstream markdown-стек
(mdast/shiki/KaTeX/footnotes/streaming не входит),
store/renderer, `chain`/`hoc`, `ownerProps` от реальных
shell-сайтов. Раунды 98–101 одним блоком закрыли submenu-механику (C2.6:
side-card/single-page по измеренной ширине, `fixed`-flip/clamp,
`activeList`-маршрутизация клавиатуры, Escape/ArrowLeft, 200-мс hover grace),
store seats (C3.1: `@deepseek-ai/dsh-client-store` в module-таблице и
`store: handle|factory` в регистрациях слотов), fallback-маршруты (C1.5:
`webServer.registerFallback` с одним местом и проксированием несовпавших
не-зарезервированных путей) и ownerProps-plumbing (`?ownerProps={...}` →
`frame.ownerProps` → props компонента, порядок reference `PropsRuntime`);
рантайм-артефакт — 430 139 байт. Раунд 102 (C4.1) добавил chat/turn-модель
Leptos-шелла (user-сообщение открывает turn, ordinal в `data-turn` и owner
share), реальные owner shares трёх render-сайтов (`sidebar.session.action`,
`message.action` для user-сообщений, `composer.actions` с
`ComposerChainProps`) и chain-элекцию в мосте (`select(ownerProps)`, `false` →
пустой DOM + `chain-unselected`), поэтому owner-специфичный dispatch теперь
есть без серверного JS. Итоговый остаток: upstream markdown-стек,
renderer-семантика, `hoc`-вложение, owner props с функциями (нет
callback-моста), per-frame фильтрация цепочек (несколько `select === true`
рендерят несколько кадров) и полный chat-view для `conversation.chat.node`
(`registerUpgrade` — явно не поддержан).

Открыто и не заблокировано:

- Общая серверная база отзывов plugin market (сейчас home-scoped файл).
- Экспериментальные контракты клиентской половины (`dsh.client.runtime` и
  `dsh.client.host_modules`: classic script, без ESM и отдельной подписи, версия
  React закреплена за сервером, CSP opt-in-кадров открывает origin для скриптов и
  `connect-src` — для host-маршрутов плагина) могут поменяться до полного тира C
  по [ADR-040](adr/040-react-client-half.md).
- C-lite/C2/C2.1–C2.6-ограничения: с раунда 93 (C2.1) design-system **портирован из
  reference-исходников** (`deepseek-ai/deepseek-harness` @ `39163dd`,
  `packages/client/ui-primitives/src`): атомы `Button`, `Input`, `Switch`,
  `Checkbox`, `Tag`, `Pill`, `SegmentedControl`, `SegmentedTabs`,
  `DisclosureRow`, `TextShimmer`, `StateDot`, `PathLabel`, `Toast`, `Modal`,
  `Menu`, `MenuSurface`, `Tooltip` и **204** `Icon*` (точные SVG-пути,
  regular/medium stroke) — вместо прежней независимой реализации 30+
  экспортов с 18 иконками. Частичным остаётся: ядро `Menu` портировано в C2.3
  (раунд 95), а в C2.4 (раунд 96) — reference submenu page-layout
  (single-page back-row) с hover grace и `MenuGroup`/`observeStickyMenuGroups`
  (полный `Tooltip` закрыт в C2.2,
  раунд 94), а раунд 97 (C2.5) закрыл markdown-долг настолько, насколько это
  возможно без upstream-стека: `JsonBlock` — порт из reference, `MarkdownText` —
  API-совместимое dependency-free GFM-подмножество с reference-поверхностью
  props (upstream micromark/mdast, shiki, KaTeX, footnotes и инкрементальный
  streaming **не** входят), поэтому C2.6 (раунды 98–101) закрыл и detail-механику submenu
  (side-card/single-page по измеренной ширине, focus-transition, hover grace,
  `activeList`-маршрутизация), упрощённых остатков submenu больше нет;
  `Spinner`/`useAnchoredPosition` —
  dshr-расширения; `wide`
  в host-state сейчас всегда true, conformance-набор — **n=11**
  (цель ADR-013 §6 8–12 достигнута; остальные smoke-половины синтетические),
  host-сервисов `pluginManager`/`settings` нет, но с раунда 89 (C1.2) UI
  `dsh-plugin-debugging`/`dsh-plugin-studio` рендерится через seat-`inject` и
  `configForms`: studio отдаёт вкладки в Settings, debugging — карточку в
  Plugins с рабочим переключателем и Save; при отсутствии host-маршрутов
  плагины показывают честные error-состояния, а не молча инертны,
  SlotMap-остатки (`hoc`-вложение и renderer-семантика; `chain`-элекция по
  `select(ownerProps)` и owner shares трёх реальных сайтов поставлены в C4.1,
  раунд 102, но несколько `select === true` рендерят несколько кадров, owner
  props с функциями не передаются, а полный chat-view для
  `conversation.chat.node` ещё нужен; store seats закрыты в C3.1, раунды
  98–101; каталог слотов, регистрация, kinds, winners, `children`/deferred
  inject закрыты в C1.3) и React-контекст шелла по-прежнему отсутствуют, мостовые сервисы — подмножество референсных (проекции
  `sessionStats` упрощены: `llmMs`=0, без истории тредов), eval-based бандлы
  требуют ослабления CSP,
  а `@michengai/dsh-skills-manager` падает на инициализации бандла — нужен триаж.
  Дальнейшее расширение (полный design-system/SlotMap-остатки/React-контекст) — уже не
  C-lite, а C-full: решение владельца.
- C1.2-остатки (раунд 89): browser-`configForms` — in-memory writable-форма,
  правки не переживают reload кадра (файловая персистентность есть только у
  host-половин в `<home>/cordis/config-forms/`); семантика конфигурационного
  движка DSH (схемы, слои, миграции) не воспроизведена; провалы seat-inject
  только логируются в лог кадра — плагин при этом остаётся живым.
- C1.3-остатки (раунд 91): chain-элекция реализована в C4.1 (раунд 102):
  кадр chain-слота вызывает `select(ownerProps)` регистрации, `false` даёт
  пустой DOM и репорт `chain-unselected`, но это per-frame фильтрация —
  единого owner-renderer нет, и несколько `select === true` рендерят
  несколько кадров. `hoc` по-прежнему не вкладывает `children` между
  кадрами; store-seats
  (`PropsStore`/`BoundActions`) закрыты в C3.1 (раунды 98–101: framework-owned
  instance на scope-key, `useStore(selector, eq)`, baked `actions`,
  reference-аргументы `inject`), остаются renderer-семантика и mounted-model
  сессий; ownerProps — plumbing `?ownerProps={...}` → `frame.ownerProps` →
  props и с C4.1 (раунд 102) реальные render-сайты шелла поставляют
  `{wide:true}` (`sidebar.session.action`), `{source,kind:"user"}`
  (`message.action`) и `ComposerChainProps` (`composer.actions`), но owner
  props с функциями (например `settings.section.close`) не передаются (нет
  callback-моста), а `conversation.chat.node` и остальные conversation-слоты
  ждут полного chat-view.
- C1.4-остатки (раунд 92): `webServer.registerFallback` поддержан с C1.5
  (раунды 98–101: одно fallback-место, второй владелец бросает, dshr
  проксирует только не-зарезервированные несовпавшие пути), `registerUpgrade`
  (WebSocket) по-прежнему явно не поддержан (нужен чужим SPA/WS-плагинам;
  реальным плагинам набора не нужен); browser-`configForms` по-прежнему
  in-memory — правки не переживают reload кадра, живого RPC-зеркала настроек
  нет (in-memory/файловые stand-in'ы, семантика конфигурационного движка DSH не
  воспроизведена). Порт `ui-primitives` закрыт в C2.1 (раунд 93): атомы и все
  204 иконки портированы из reference, а в C2.2 (раунд 94) доведён полный
  `Tooltip`, в C2.3 (раунд 95) — ядро `Menu`/`MenuItemButton`, в C2.4
  (раунд 96) — submenu page-layout с hover grace и `MenuGroup`/
  `observeStickyMenuGroups`, а в C2.5 (раунд 97) — `JsonBlock` из reference и
  dependency-free GFM-подмножество `MarkdownText` с reference-поверхностью props;
  submenu-механика закрыта в C2.6 (раунды 98–101), остаются upstream
  markdown-стек (mdast/shiki/KaTeX/footnotes/streaming) и renderer; store
  seats закрыты в C3.1 (раунды 98–101), fallback-маршруты — в C1.5; C4.1
  (раунд 102) добавил chain-элекцию по owner props и owner shares трёх
  реальных сайтов, поэтому следующий кандидат reference-работы — renderer,
  `hoc`-вложение, callback-мост для функциональных owner props и полный
  chat-view.
- C2.1–C2.6-остатки (раунды 93–101): портированы атомы и все 204 иконки;
  полный `Tooltip` доведён в C2.2, а в C2.3 — ядро `Menu`
  (`MenuItemButton`, data/компонентные ряды, separators/labels, reserve-icon,
  selection `check`/`fill`, `danger`/`current`, disabled-reason через Tooltip,
  `ShortcutKeys`, footer-группа, `children`-ряды, one-level submenu, клавиатурный
  walk, outside/iframe-blur dismissal, `usePointerGrace`, `focusWithoutRing`,
  portal-позиционирование, dense/compact), в C2.4 — reference submenu
  page-layout (single-page back-row `data-menu-back`, Escape/ArrowLeft на
  родителя), 200-мс hover grace между карточками и
  `MenuGroup`/`observeStickyMenuGroups` (sticky-заголовки через
  IntersectionObserver/ResizeObserver), а в C2.5 (раунд 97) — `JsonBlock`
  (collapsible, cap 20 000 символов, `truncatedLabel`) и API-совместимое
  dependency-free GFM-подмножество `MarkdownText` (заголовки/списки/fenced
  code/blockquote/hr/inline-стили, sanitize URL, raw HTML не рендерится).
  С C2.6 (раунды 98–101) submenu-механика закрыта: side-card/single-page
  выбирается по измеренной ширине, submenu позиционируется `fixed` с
  flip/clamp, клавиатура идёт через `activeList` (родитель/дочерняя
  страница), Escape/ArrowLeft возвращают фокус на родительский ряд, hover
  grace 200 мс действует между уровнями, наведение на другой ряд сворачивает
  submenu; остаются upstream markdown-стек (micromark/mdast, shiki, KaTeX,
  footnotes, инкрементальный streaming) **не** входящий в `MarkdownText`,
  `Spinner`/`useAnchoredPosition` — dshr-расширения; store-seats
  (`PropsStore`/`BoundActions`) закрыты в C3.1 (раунды 98–101:
  `@deepseek-ai/dsh-client-store` в module-таблице, `store: handle|factory`,
  instance на scope-key, `useStore(selector, eq)` + baked `actions`,
  reference-аргументы `inject`), fallback-маршруты — в C1.5; C4.1 (раунд 102)
  закрыл chain-элекцию (`select(ownerProps)` в кадре chain-слота) и поставил
  owner shares трёх реальных сайтов шелла, так что остаются
  renderer-семантика, `hoc`-вложение, callback-мост для функциональных owner
  props и полный chat-view (`conversation.chat.node`) — C-full.
- C1-остатки (раунды 84–87): раунд 87 (C1.1) закрыл отсутствие сервисов
  `sessions`/`sessionProjections` — sidecar строит их из снимка сервера
  (задержка ≤ 5 с, упрощённые проекции: `llmMs`=0, без истории тредов,
  содержимое сессий намеренно не мостится). Остаются: нет
  SSE/WebSocket/стриминга через прокси (fallback-прокси буферизует ответ).
  Раунд 86 закрыл ограничение exact-only (появились prefix-маршруты), шум
  MCP-псевдо-инструментов в реестре и LAN-экспозицию: plugin-маршруты
  по-прежнему неаутентифицированы (как в DSH), поэтому при bind вне loopback
  прокси отвечает 403 без явного `--allow-lan-plugin-routes`
  (`ServerConfig.allow_plugin_routes_lan`).

Ожидает решения (ADR-040 Proposed, тир C/C-full):

- **Полный тир C** — C-lite PoC сделан (раунд 79: React-компоненты слотов,
  host-hooks/`ctx.get('sessions')` и частичный design-system шим; раунд 80:
  conformance n=2 и сервисы моста `ctx.get('slots')`/`locale`/`ctx.inject`;
  раунд 81: React-контракт — типы `memo`/`forwardRef`, keyed-ids, локали
  `register`/`t()`, таймеры — и conformance n=4; раунд 82: conformance n=6
  (`dsh-session-superagent@0.10.12` — composer dock, `dsh-mnemon@0.5.46` —
  multi-region и референсная поверхность слотов); раунд 83: conformance n=9 —
  цель ADR-013 §6 достигнута (`dsh-plugin-market@1.2.2`,
  `dsh-plugin-codegraph-panel@0.6.5`, `@goodandready/dsh-plugin-notify@0.4.9`);
  раунд 84: **C1** — host HTTP-маршруты плагинов работают end-to-end
  (loopback-сервер sidecar, allowlist-прокси, `connect-src` на origin сервера,
  реальная host-половина balance-плагина), с честно записанными остатками
  (сервисы `sessions`/`sessionProjections`, стриминг); раунд 85: **C2** —
  design-system шим вырос с 7 до 30+ экспортов по фактическим импортам
  (компоненты, именованные `Icon*` с Proxy-fallback, хуки, `writeClipboard`),
  conformance n=11 (`dsh-plugin-debugging@0.8.1`, `dsh-plugin-studio@0.3.1`
  рендерятся с раунда 89 через seat-`inject`/`configForms`, хотя
  host-сервисов `pluginManager`/settings по-прежнему нет); раунд
  86 — хардненинг C1: prefix-маршруты, таблица `routes` в
  `GET /v1/plugins/status`, фильтр MCP-псевдо-инструментов и loopback-only
  политика plugin-маршрутов с opt-in `--allow-lan-plugin-routes`; раунд 87 —
  **C1.1**: мост `sessions`/`sessionProjections` из внутреннего снимка сервера
  (задержка ≤ 5 с, упрощённые проекции, содержимое сессий не мостится); раунд
  89 — **C1.2**: seat-`inject`, Cordis-генераторы и `configForms` (in-memory
  форма в кадре, файловая — у host-половин), что сняло инертность
  settings-плагинов; раунд 91 — **C1.3**: слот-контракт по референсному
  каталогу (122 слота с kind/scope/standard-kit; правила `SlotCore`:
  undeclared/keyed/chain/duplicate, shadowing-winners, children-декларации,
  deferred inject, standard-kit + ownerProps); раунд 92 — **C1.4**: host-сервисы
  `webServer`/`ctx.baseUrl`/`configForms` по reference-контракту (exact/prefix-
  таблицы и правила путей с duplicate-throw, `getOrigin()`/`port`/`protocol` из
  host-bridge, `file://`-baseUrl, snapshot
  `{status, value, base, user, revision: number, writable}` с `mutate`/`set`/
  `unset` и числовым revision-fence; `registerFallback`/`registerUpgrade` в
  sidecar не поддержаны); раунд 93 — **C2.1**: атомы design-system и все 204
  иконки портированы из `deepseek-ai/deepseek-harness` @ `39163dd`
  (`packages/client/ui-primitives/src`), поэтому шим примитивов стал
  reference-портом, а не независимой реализацией (рантайм-артефакт на конец
  раунда — 375 894 байта); раунд 94 — **C2.2**: полный reference-порт `Tooltip` (`Tooltip.tsx`: label-резолвер,
  `delayMs`/`focusDelayMs`, `openOnClick`-pinning, `onlyWhenClipped`/
  `maxWidth`, portal, nested-suppression, `aria-describedby`, pointer-modality,
  forward ref; `tooltip-placement.ts` и `input-modality.ts` портированы как
  есть, `ShortcutKeys` экспортирован), рантайм-артефакт — 387 613 байт; раунд
  95 — **C2.3**: ядро `Menu`/`MenuItemButton` портировано по
  `Menu.tsx`/`MenuRow.tsx` (ряды/selection/aria, клавиатурный walk, one-level
  submenu, dismissal; `usePointerGrace`/`focusWithoutRing`/`MENU_TRANSITION_MS`/
  `isMobileDevice` экспортированы), рантайм-артефакт — 405 323 байта; раунд
  96 — **C2.4**: reference submenu page-layout (`menu-submenu-position.ts`:
  side-карточка или single-page с non-selecting back-row `data-menu-back`,
  Escape/ArrowLeft с фокусом на родителе), 200-мс hover grace между
  карточками и `MenuGroup`/`observeStickyMenuGroups` (sticky-заголовки через
  IntersectionObserver/ResizeObserver), рантайм-артефакт — 415 972 байта; раунд
  97 — **C2.5**: `JsonBlock` портирован из
  `packages/client/ui-primitives/src/markdown/JsonBlock.tsx` (collapsible,
  `label`/`payload`/`defaultOpen`/`truncatedLabel`, cap 20 000 символов,
  безопасный `JSON.stringify`), а `MarkdownText` получил reference-поверхность
  props и dependency-free рендерер GFM-подмножества (заголовки, абзацы, списки,
  fenced code, blockquote, hr, inline-стили, ссылки/изображения только
  `http(s)`/`mailto`, raw HTML не рендерится), рантайм-артефакт — 424 801 байт;
  раунды 98–101 — **C2.6/C3.1/C1.5 + ownerProps**: submenu-механика
  (`menu-submenu-position.ts`, `--dsh-submenu`-позиционирование с flip/clamp,
  `activeList`, Escape/ArrowLeft, 200-мс hover grace, `data-menu-back`),
  store-seats (`@deepseek-ai/dsh-client-store` в module-таблице:
  `createSnapshotStore`/`defineStore`/`shallowEqual`, `store: handle|factory`,
  instance на scope-key, `useStore`/`actions`, inject-аргументы
  `(sessionId, actions)`), fallback-маршруты (`webServer.registerFallback`,
  одно место, проксирование не-зарезервированных несовпавших путей) и
  ownerProps-plumbing (`?ownerProps={...}` → `frame.ownerProps` → props после
  store/seat share); рантайм-артефакт — 430 139 байт; раунд 102 — **C4.1**:
  chat/turn-модель Leptos-шелла (turn ordinal в `data-turn` и owner share),
  реальные owner shares (`sidebar.session.action`, `message.action` для user,
  `composer.actions`) и chain-элекция в мосте (`select(ownerProps)` → пустой
  DOM + `chain-unselected` при `false`), рантайм-артефакт не менялся.
  Чего по-прежнему нет: полного design-system `@deepseek-ai/dsh-client-ui-*`
  (C2.1 закрыл атомы и иконки, C2.2 — полный Tooltip, C2.3 — ядро Menu,
  C2.4 — submenu page-layout/hover grace/MenuGroup, C2.5 — reference JsonBlock и
  markdown-подмножество, C2.6 — позиционирование/`activeList`/focus-transition
  механика submenu; остаются upstream markdown-стек:
  mdast/shiki/KaTeX/footnotes/streaming),
  SlotMap-остатков (`hoc`-вложение, renderer-семантика; chain-элекция и owner
  shares трёх реальных сайтов поставлены в C4.1, раунд 102, но несколько
  `select === true` рендерят несколько кадров, owner props с функциями не
  передаются (нет callback-моста), полного chat-view для
  `conversation.chat.node` нет; store seats закрыты в C3.1),
  `registerUpgrade` (WebSocket; `registerFallback` закрыт в C1.5),
  React-контекста/состояния шелла и композиции между плагинами, выбора версии
  React и настоящего `@dshr/cordis-compat`-шима. Это вопрос объёма/приоритета
  для владельца (C-full по ADR-040 остаётся Proposed); D не рекомендуется.

Заблокировано внешним условием:

- **Windows sandbox/сеть** — нужен реальный Windows-хост; CI-шаг есть, локально
  только компиляция.
- **Notarization** — нужны Apple Developer ID, сертификат и app-specific password.

Подробная матрица и блокеры: [PARITY.md](PARITY.md), реестр долгов — [TECH-DEBT.md](TECH-DEBT.md).

## Раунд 115 — внешние треки: GHCR-индекс и release-dry-run (16.11.2026)

Внешние треки закрыты настолько, насколько это возможно без учётных данных;
остаток — действия владельца (tap-репозиторий и push тега).

Состояние внешней стороны, перепроверенное по публичному API GitHub 16.11.2026:

- тегов и релизов в `maximusmakarov/deepseek_harness_rust` нет — workflow
  `Release` ни разу не запускался на теге, то есть desktop-джобы тега ни разу
  не выполнялись;
- пакет `ghcr.io/maximusmakarov/deepseek_harness_rust/dshr` существует: тег
  `0.0.1` (одна платформа `x86_64_linux`), push и анонимный pull подтверждены
  run `43585903547`; 16.11.2026 `tags/list` (`{"tags":["0.0.1"]}`) и манифест
  индекса читаются без токена, то есть пакет публичный;
- все шесть попыток macOS-верификации GHCR 14.11.2026 (`ghcr-mac-verify`)
  упали, поэтому «push macOS-бутылки» в Actions не подтверждён;
- tap-репозитория `homebrew-dshr` нет (404) — единственный настоящий блокер
  `brew install`;
- `gh`, `GITHUB_TOKEN` и SSH-ключ в локальном окружении отсутствуют
  (`git@github.com` → `Permission denied (publickey)`), поэтому ни тег, ни tap
  отсюда создать нельзя.

Исправлено в релизном пути:

- `scripts/publish-bottle-ghcr.py`: индекс версии собирается из всех бутылок
  одним push. Прежний цикл в `release.yml` пушил бутылки по одной, а каждый push
  перезаписывает тег `<version>` индексом с единственной платформой — на реальном
  релизе 3 из 4 платформ остались бы неустановимыми;
- `release.yml`, `desktop-linux`: выгружались только `dist/*.tar.gz*`, поэтому
  обещанный `dshr_<version>_amd64.deb` не попадал ни в релиз, ни в `SHA256SUMS`;
  теперь выгружается и `dist/*.deb*`;
- `publish` больше не зависит молча от GHCR: job ждёт `bottles-ghcr`, но бинарные
  артефакты публикует и при его падении, а GHCR-формула рендерится только при
  успешном push (`needs.bottles-ghcr.result == 'success'`);
- `Release` получил `workflow_dispatch` с входом `version`: собираются
  build/desktop/бутылочные джобы, `bottles-ghcr` и `publish` пропускаются —
  desktop-трек проверяется без тега и без релиза;
- `scripts/release-version.sh` — единая нормализация версии (`v0.1.0` → `0.1.0`,
  мусор отвергается);
- `scripts/sync-homebrew-tap.sh` + `packaging/homebrew-tap/README.md` — установка
  GHCR-формулы в tap (`root_url`, bottle-`sha256`, `ruby -c`, коммит
  `dshr <version>`, `--push`, `--dry-run`) и порядок одноразовой настройки;
- формула приведена к требованиям `brew audit --strict` (Homebrew 7.0.13,
  проверено на локальном tap): блок `bottle do` сортируется
  (ARM macOS → Intel macOS → ARM Linux → Intel Linux), digests выравниваются по
  самому длинному тегу, лишняя пустая строка убрана, строка `version` удалена
  (brew сканирует её из URL) — версия в `sync-homebrew-tap.sh` читается из URL
  релиза. Заодно подтверждено, что brew выводит из
  `root_url "https://ghcr.io/v2/<owner>/<repo>"` стратегию
  `CurlGitHubPackagesDownloadStrategy` и путь `…/<formula>/manifests/<version>`,
  совпадающий с публикатором.

Тесты: `scripts/test-publish-bottle-ghcr.py` проверяет две платформы в одном
индексе (CI), `scripts/test-sync-homebrew-tap.sh` — tap-синхронизацию и
`release-version.sh` (CI и `scripts/check.sh`).

## Раунд 116 — внешние треки закрыты: релиз v0.1.0 и GHCR-tap (16.11.2026)

Оба трека, которые числились «заблокированными внешним условием», проверены на
реальных артефактах.

- Запушены `main` (`b17091e..458220d`, 12 коммитов) и тег `v0.1.0`; workflow
  `Release` (run `43832256936`, 19m59s) прошёл целиком — 5 архивов, macOS
  `.app`+DMG, Linux tar+deb, Windows zip, 4 бутылки, GHCR-push и `publish`.
- Релиз `v0.1.0` содержит 17 ассетов, в том числе `dshr_0.1.0_amd64.deb`
  (терялся до фикса раунда 115), `dshr-ghcr.rb`, `dshr.rb`, `SHA256SUMS`.
- Индекс `ghcr.io/maximusmakarov/deepseek_harness_rust/dshr:0.1.0` содержит все
  четыре платформы (`arm64_sequoia`, `sonoma`, `aarch64_linux`, `x86_64_linux`) —
  агрегирующий push проверен на реальном релизе, а не только на mock-регистре.
- Tap `maximusmakarov/homebrew-dshr` создан; формула установлена
  `scripts/sync-homebrew-tap.sh --push` (коммит `dshr 0.1.0`), `brew tap
  maximusmakarov/dshr` видит формулу, `brew audit --strict` — 0 замечаний.
- `brew fetch maximusmakarov/dshr/dshr` скачал манифест индекса и бутылку
  `arm64_sequoia` из GHCR анонимно, sha256 совпал с формулой и ассетом релиза;
  `brew install maximusmakarov/dshr/dshr` налил бутылку
  (`poured_from_bottle: true`), `brew test dshr` и `dshr --version` проходят —
  установка из GHCR проверена end-to-end.
- Найдено ограничение Homebrew 7: сторонний tap требует явного доверия —
  `brew trust maximusmakarov/dshr`, иначе `Refusing to load formula … from
  untrusted tap`. Записано в [RELEASE](RELEASE.md#tap) и
  [INSTALL](INSTALL.md#homebrew).

Остались заблокированными только Windows-хост (sandbox/сеть) и notarization
(Apple Developer ID). Полный `brew install` — одна команда владельца.

## Раунд 117 — runtime pack для Cordis: `dshr runtime` (16.11.2026)

Закрыт последний пункт M5b, который не требует решения владельца: sidecar больше
не зависит от системного Node.

- `crates/cordis/src/runtime.rs`: реестр паков `<home>/runtimes/<engine>/<version>/`
  с `receipt.json` и активной версией в `<engine>/current`. Установка
  fail-closed: обязательный sha256, опциональная Ed25519-подпись над байтами
  архива, распаковка `.tar.gz` отклоняет абсолютные пути, `..`, симлинки,
  выходящие за границы пака, и спецфайлы, ограничивает распакованный объём
  2 ГиБ, ставится через staging + rename (упавшая установка не оставляет
  половинчатый пак).
- `dshr runtime install|list|use|remove`: `--from` — локальный `.tar.gz`
  (офлайн) или `https://` (кэш в `<home>/cache/runtime`), `--sha256`
  обязателен, `--signature`/`--public-key` опциональны, `list --json` для
  скриптов.
- sidecar: `DSHR_NODE` → активный пак `node` → `node` из PATH; когда нет ничего,
  предупреждение содержит подсказку с командой установки.
- проверено вживую: настоящий `node-v22.11.0-darwin-arm64.tar.gz` (47,8 МБ,
  sha256 из `SHASUMS256.txt`) поставлен по https, `node --version` → v22.11.0,
  `npm --version` → 10.9.0 (симлинки пака распакованы и разрешаются); с активным
  паком и `PATH` без Node демо-плагин (`cordis__demo__ping`) загружается, а после
  снятия `current` sidecar падает с `No such file or directory` — бинарь
  действительно берётся из пака.
- тесты: +9 unit (установка/активация, sha256, подпись, `--force`, откат при
  неудачной замене, remove с fallback, отказ небезопасным путям и симлинкам,
  `node_program`) и +2 CLI (round-trip install/list/remove, отказ при неверном
  sha256).

Осталось по M5b: полный шим `@dshr/cordis-compat@1`, собственный npm-резолвер/CAS,
отчёт матрицы «инструменты/хуки»; C-full — решение владельца ([ADR-040](adr/040-react-client-half.md)).

## Раунд 118 — npm-резолвер и CAS вместо npm (16.11.2026)

Закрыт второй пункт M5b: установка Cordis-плагинов больше не требует бинаря
`npm`.

- Новый крейт `dshr-npm`: клиент registry v2 (abbreviated packument, кэш в
  `<home>/npm/registry`, `DSHR_NPM_REGISTRY` для зеркал), разбор спеков
  (`name`, `name@range`, `@scope/name@range`, dist-теги) и npm-синтаксиса
  диапазонов — пробелы (`>= 2.1.2 < 3.0.0` превращаются в
  `>=2.1.2, <3.0.0`), дефисные диапазоны (`1.2.3 - 2.0.0`) и `||`.
- Резолвинг графа: транзитивные `dependencies`, переиспользование совместимой
  версии наверху и вложение конфликтующей под зависимым; `optionalDependencies`
  best-effort, `devDependencies` игнорируются, `peerDependencies` — нет
  (записано в TECH-DEBT).
- Store: SRI-проверка `dist.integrity` (sha512, fallback sha1), распаковка один
  раз в `<home>/npm/cas/<digest>/`, кэш тарболов; распаковка отклоняет
  абсолютные пути, `..`, ссылки за пределы пакета и спецфайлы (потолок 512 МиБ).
- Материализация жёсткими ссылками, а не симлинками: Node разрешает симлинк в
  реальный путь, поэтому симлинк в store ломал импорты зависимостей изнутри
  пакета (ошибка воспроизведена на реальном `dsh-remote`; лечится как в pnpm —
  реальные каталоги плюс hardlink на файлы).
- `dshr plugin npm add|update` переведены на резолвер: `add` пишет
  `package.json`, ставит дерево и lock, `update` пере-резолвит из
  `package.json`; спеки `file:`/`git+` по-прежнему идут через бинарь npm.
- Lock `dshr-npm-lock.json`: `lockfileVersion`, `roots`, точные версии,
  integrity и пути; lifecycle-скрипты пакетов не выполняются вовсе.

Проверено на живом registry: `lodash` → 4.18.1 (загружается через Node и репортит
свою версию), `@kt11/dsh-deepseek-balance@0.2.1`, `dsh-mnemon@0.5.46` — 26
пакетов, из них scoped-зависимости резолвятся изнутри пакета, `dsh-remote@1.2.0`
— 12 пакетов; `npm update dsh-mnemon` пере-резолвил те же 26 пакетов.

Найденная граница совместимости: `dsh-remote` импортирует host-пакет
`@deepseek-ai/dsh-tools`, которого нет в registry-дереве — sidecar падает с
`Cannot find package '@deepseek-ai/dsh-tools'`. Это остаток шима
`@dshr/cordis-compat@1`, а не резолвера; записано в TECH-DEBT.

Тесты: +12 unit (парсинг спеков, npm-диапазоны, порядок слотов, integrity,
link-safety) и +5 интеграционных на локальном registry-стабе (транзитивные
зависимости, конфликт версий, дедупликация CAS, отказ при неверной integrity,
офлайн-переустановка из кэша) плюс CLI-тест, доказывающий, что резолвер не
вызывает `npm`.

## Раунд 119 — первая половина шима `@dshr/cordis-compat`: реальный плагин работает (16.11.2026)

Впервые опубликованный DSH-плагин с host-импортами загружается и **исполняет**
свои инструменты через sidecar.

- **peerDependencies**: `dshr-npm` ставит и их (npm ≥ 7 так делает), при этом
  peer'ы — хостовые синглтоны: ставятся на верхнем уровне, конфликтующая копия
  пропускается с предупреждением, недоступный peer не валит установку (плагины
  пинят peer к неопубликованным версиям хоста). От циклов peer'ов — лимиты графа
  (1500 пакетов, глубина 64) с типизированной ошибкой: до лимитов `dsh-mnemon`
  висел бесконечно.
- **Поверхность `ctx`**: `effect`, `on`/`emit`, `provide`, `tools.register`/
  `tools.list`, `systemPrompt.section`, `sandbox.confine`, `config`; `apply`
  получает конфиг, разрешённый из **именованного** `export const Config`
  (значения по умолчанию схемы) — раньше второй аргумент был `undefined` и
  плагины падали на `config.host`.
- **Форма инструментов**: `addTool` принимает хостовую форму `defineTool`
  (`parameters`/`execute`) наравне с dshr-формой (`inputSchema`/`handler`).
- **Точные версии**: `Requirement::parse("0.0.1-rc.1")` больше не читается как
  caret-диапазон semver — npm-семантика: голая версия точная (иначе ставился
  `rc.5` вместо запрошенного `rc.1`).

Проверено:

- `dsh-remote@1.2.0`: 59 пакетов (12 своих + peer'ы и их транзитивные), sidecar
  поднимается на node из runtime pack (PATH без Node), регистрирует **21**
  инструмент `cordis__dsh-remote__rw_*`; MCP-вызов `rw_info` вернул настоящий
  ответ плагина: «Remote host: <user>@<host>:22 … No host + workspace configured
  — call rw_connect with a host to get started.»;
- `dsh-mnemon@0.5.46`: ставится (54 пакета, один peer-конфликт пропущен с
  предупреждением), но падает на отсутствующем сервисе `subagents` и на
  `configForms.get(...).writable` — записано как граница шима в TECH-DEBT;
- регрессионный тест `exposes_the_reference_plugin_surface` проверяет
  `effect`/`on`/`provide`/`tools.register`/`systemPrompt.section`/`sandbox.confine`
  и резолвинг именованного `Config` без сети;
- в `dshr-npm` добавлен тест peer-зависимостей (required ставится, недоступный
  optional пропускается).

## Раунд 120 — сервис `subagents` в sidecar (16.11.2026)

- В `ctx` появился сервис `subagents`: `list()`, `getProvider(name)`,
  `registerProvider(provider)` (возвращает диспозер и эмитит
  `subagent/provider-added|removed` через `ctx.emit`); доступен и как
  `ctx.subagents`, и как `ctx.get('subagents')`. Это реестр, а не рантайм:
  дерево агентов живёт в процессе хоста, поэтому `start`/`startActivation`
  намеренно отсутствуют — плагины проверяют их `typeof` и деградируют.
- Config-формы создаются лениво и никогда не отдают `undefined`: реальные
  плагины читают `.writable` у снапшота без проверки на `null`. Мёртвый
  fallback-плейсхолдер, добавленный по первому впечатлению от `dsh-mnemon`,
  убран после проверки — падение было не в `configForms`, а в отсутствующем
  сервисе форм настроек.
- Проверка на живом плагине: `dsh-mnemon@0.5.46` проходит гейт `subagents`
  (ошибки `subagents.list` больше нет) и упирается в следующий сервис —
  `agents` (`agents.list`, `agents.get`, `agents.roots`, `agents.isOwnedBy`,
  `agents.getProvider`, …). Это следующий инкремент шима, записан в TECH-DEBT;
  `dsh-remote@1.2.0` по-прежнему монтируется и отдаёт 21 инструмент.
- Тест `exposes_subagent_registry_and_safe_config_forms`: регистрация
  провайдера, `list`/`getProvider`, диспозер и ленивая config-форма.

## Раунд 121 — `agents`: мост к дереву агентов хоста (16.11.2026)

- `dshr-core`: `AgentRegistry`/`LiveAgent` — реестр живых агентов (id, session,
  parent, root, depth, title, model, started_at, монотонная revision) с
  RAII-гардом на удаление; юнит-тесты на parentage/roots/root_of/revision.
- Сервер: `AppState` держит реестр, `start_turn` регистрирует корневого агента
  на время turn'а (title из сессии, model — имя провайдера) и снимает его
  гардом; новый внутренний endpoint `GET /v1/internal/agents` (тот же
  bridge-токен) отдаёт `{agents, revision}`.
- Sidecar: сервис `agents` гидратируется из этого endpoint'а раз в 2 с,
  нормализует snake_case/camelCase, эмитит `agent/created`/`agent/disposed` на
  дельте и отдаёт `list`/`roots`/`get`/`isOwnedBy`/`rootOf`/`resolveName`/
  `getProvider`. Через мост идут только метаданные.
- Заодно добавлен сервис `commands` (реестр слэш-команд: `register`/`list`/
  `get`/`run`), потому что `dsh-mnemon` регистрирует команду на старте.

Проверено:

- `dsh-mnemon@0.5.46` **монтируется** и отдаёт 3 инструмента
  (`mnemon_view_route`, `mnemon_view_action`, `mnemon_subagent_result`), без
  предупреждений; `dsh-remote@1.2.0` по-прежнему 21 инструмент. Итого два
  реальных host-плагина в conformance под sidecar.
- Тесты: `AgentRegistry` (2), серверный endpoint (`/v1/internal/agents` требует
  токен; живые агенты видны, после drop гарда список пуст) — 2, интеграционный
  тест sidecar с локальным HTTP-стабом: плагин видит `count=2`, `roots=[s1]`,
  `rootOf=s1`, `owned=true`, `parentId=s1`.
- Остаток, записанный в TECH-DEBT: сервис форм настроек (`dsh-mnemon` пишет
  не-фатальное «could not persist displayMode») и вывод команд в UI/агенту
  (реестр есть, исполнение через `commands.run`).

## Раунд 122 — сервис `settings` (16.11.2026)

- `ctx.settings` (он же `ctx.get('settings')`) реализует контракт в объёме,
  нужном плагинам: `register(ns, schema, {base, applies})` возвращает scope с
  `get`/`watch`/`update`/`replace`; сервис отдаёт `describe()`, `get(ns)`,
  `update`, `replace`, `mutate`, `writable` и `documentPath`.
- Разрешение послойное, как в DSH: умолчания схемы → композиционный `base` →
  пользовательский слой документа. Документ один —
  `<home>/cordis/settings.json`, пишется атомарно (`.part` + rename);
  `revision` в дескрипторе растёт на каждом коммите, `expectedRevision`
  отбивает устаревшую запись, `watch` получает `(next, prev)`.
- `configure()` добавлен как совместимый no-op: плагины, собранные против более
  нового API формы настроек, вызывают его на старте и получают диспозер вместо
  падения.

Проверено: `dsh-mnemon@0.5.46` монтируется **без единого предупреждения**
(3 инструмента), `dsh-remote@1.2.0` — по-прежнему 21. Регрессионный тест
`settings_service_resolves_and_persists` проверяет умолчания+base, `watch`,
`update`, дескрипторы и запись в `settings.json`.

## Раунд 123 — команды плагинов: от реестра к инструментам и HTTP (16.11.2026)

- Sidecar: `commands.register(...)` теперь регистрирует и MCP-инструмент
  `__dshr_command__<name>` (схема `{args, agent}`), который исполняет хендлер
  команды; диспозер снимает и команду, и инструмент.
- Хост: `dshr-cordis` пропускает эти инструменты в реестр агента под именем
  `cmd__<plugin>__<name>` (остальные `__dshr_*` по-прежнему скрыты) и
  подкладывает в вызов дескриптор сессии `{id, sessionId}` из `ToolCtx` —
  у sidecar'а нет доступа к контексту хоста.
- HTTP: `GET /v1/commands` отдаёт список команд (имя, описание, схема),
  `POST /v1/commands/{name}` с `{args, session_id?}` исполняет команду и
  возвращает `{output, is_error, files}`; имена не из `cmd__*` — 400.

Проверено: `dsh-mnemon` публикует команду `mnemon` как `cmd__dsh-mnemon__mnemon`
(в промпте рядом с 3 инструментами); регрессионные тесты —
`plugin_commands_become_tools` (sidecar→tool, подстановка сессии) и
`plugin_commands_are_listed_and_run_over_http` (list + run через axum).

Остаток: слэш-ввод в TUI (сейчас команды доступны агенту и HTTP, но не как
`/команда` в интерактивном терминале) — записан в TECH-DEBT.

## Раунд 124 — матрица conformance и три найденных бага (16.11.2026)

- `scripts/conformance.py`: воспроизводимый прогон по 11 плагинам набора — чистый
  home, `plugin npm add`, `prompt`, сбор инструментов/команд/предупреждений;
  результат — [CONFORMANCE.md](CONFORMANCE.md).
- Итог матрицы: **четыре плагина с host-половиной работают end-to-end** —
  `dsh-remote` (21 инструмент, 3 команды), `dsh-mnemon` (3 инструмента,
  1 команда), `dsh-plugin-debugging` (8 инструментов), `dsh-plugin-notify`
  (2 инструмента, 1 команда). Четыре — клиентские половины без host-инструментов
  (`dsh-genui`, `dsh-plugin-codegraph-panel`, `dsh-plugin-studio`,
  `@ruoyang/...balance`). `@kt11/...balance` отдаёт HTTP-маршруты (C1).
  `dsh-session-superagent` требует Node ≥ 22.15 (`zstdDecompressSync` из
  `node:zlib`) — с Node 26 монтируется. `dsh-plugin-market` не ставится по вине
  апстрима (`workspace:^`/`link:` спеки в опубликованном пакете).

Найдено и исправлено по ходу:

- **stdout ломал MCP**: `dsh-plugin-notify` печатает `[notify] …` через
  `console.log`, и JSON-RPC-канал разваливался («invalid JSON: expected ident»).
  Bootstrap теперь заворачивает `process.stdout.write` (в stdout проходит только
  JSON) и уводит `console.log/info/debug/warn` в stderr.
- **Дублирующиеся записи в tar**: установка падала с `File exists` на пакете,
  в архиве которого повторяется путь. Теперь дубликат перезаписывается (и в
  npm-store, и в runtime pack), а недокачанный CAS-каталог без `package.json`
  самолечится при следующей установке.
- **Подсказка про Node**: если плагин падает на отсутствующем экспорте
  (`does not provide an export named`), предупреждение sidecar прямо говорит,
  что нужен более новый Node, вместо голого стектрейса.

## Раунд 125 — слэш-команды в TUI и nightly-матрица (16.11.2026)

- TUI понимает `/`-ввод: `/help` печатает встроенные (`/help`, `/quit`) и все
  команды плагинов (`cmd__*`), `/quit` выходит, `/<name> args` находит
  инструмент `cmd__<plugin>__<name>` и исполняет его в контексте текущей сессии
  (cwd = workspace сессии, `session_id`, лимит вывода), результат печатается в
  транскрипт. Неизвестная команда отвечает подсказкой, а не молчанием.
- Команды плагинов теперь доступны на всех трёх поверхностях: агент
  (`cmd__*` инструменты, раунд 123), HTTP (`/v1/commands`, раунд 123) и TUI
  (`/name`, этот раунд).
- `.github/workflows/conformance.yml`: nightly-прогон матрицы на ubuntu с
  `DSHR_NODE=node` и выгрузкой `conformance.json`/Markdown артефактом;
  `scripts/conformance.py` получил `--markdown` и теперь пишет таблицу.
- Тесты: unit на разбор `/name args` и сопоставление `cmd__*` имён; expectrl-
  сценарий `tui_lists_commands_for_a_slash_input` проверяет `/help` в реальном
  терминале.

## Раунд 126 — матрица сервисов и честные заглушки (16.11.2026)

- Инвентаризация: по всем 11 плагинам набора собрано, какие сервисы они
  запрашивают (`ctx.get(...)`/`ctx.<name>`) — `sessions`, `loader`,
  `agentPresets`, `attachments`, `sessionPersistence`, `credentials`,
  `approval`, `profileContext`, `llm`, `agents`, `jobs`, `sessionQuery`,
  `pluginPackages`, `agentDefaultModel`, `workspaceRegistry` и другие.
- Добавлены честные реализации/заглушки: `loader` (`loaded: true`, `await()`
  — no-op), `profileContext`, `workspaceRegistry` (один workspace — cwd сессии),
  `agentPresets`/`pluginPackages`/`jobs`/`sessionPersistence`/`sessionQuery`
  (пустые результаты), `credentials` (пусто: keyring живёт в хосте),
  `approval` (fail-closed `deny`), `agentDefaultModel` (`null`). Это убирает
  `TypeError`-падения у плагинов, которые читают свойства сервиса сразу.
- Матрица «сервис → статус → что отдаёт» записана в
  [PLUGINS.md §Матрица сервисов](PLUGINS.md#матрица-сервисов); отсутствующие
  сервисы (`llm`, `conversation`, `attachments`, `inputTriggers`, `spillStore`,
  `sandboxPolicy`, `directoryPicker`, `configEditor`, `connection`) перечислены
  явно, чтобы не создавать иллюзию поддержки.
- Регрессия не появилась: `dsh-remote` — 21 инструмент, `dsh-mnemon` — 3,
  `dsh-plugin-debugging` — 8, `dsh-plugin-notify` — 2; предупреждений нет.
- Тест `compatibility_services_answer_honestly` проверяет формы всех заглушек.

## Раунд 127 — desktop: attach-режим по умолчанию (16.11.2026)

- `dshr-desktop` больше не является супервизором своего сервера: по умолчанию
  он **подключается** к уже работающему `dshr serve`. Настройки —
  `<home>/desktop-server.json` (`{"mode":"attach","url":…,"token":…}`),
  переопределяются `DSHR_DESKTOP_SERVER_URL`/`DSHR_DESKTOP_TOKEN`; при пустом
  URL открывается нативный диалог, позже — трей **Server setup… (attach)**.
- В attach-режиме приложение не запускает сервер, не настраивает провайдера, не
  пишет `desktop-last-good` и не останавливает сервер при выходе; проверка —
  `GET {url}/v1/health` (recovery-окно с ошибкой, если недоступен). Окно
  открывается на `{url}/?token=…`, «Open in browser» — туда же.
- Локальный режим (супервизия loopback-сервера) сохранён для dev/CI:
  `"mode":"local"`, `DSHR_DESKTOP_LOCAL=1` или `--local`; `--check` учитывает
  режим.
- Попутно починены два флакающих тест-харнесса (найдены при прогонах):
  у десктоп-крейта не было `tempfile` в dev-dependencies (тесты вообще не
  компилировались), а `temp_home()` в `single_instance` мог выдать двум
  параллельным тестам одно имя (clock-разрешение) — добавлен счётчик;
  в npm-интеграционных тестах macOS отдаёт сокеты неблокирующего listener'а
  неблокирующими, из-за чего stub иногда читал пустой запрос и отвечал 404 —
  теперь сокет переводится в blocking, а маршруты ставятся до старта accept.
  Стресс: npm 30/30, single_instance 10/10 без падений.

Проверено вживую: `desktop check ok: http://127.0.0.1:4317/?token=…` против
запущенного сервера; GUI-запуск пишет в `desktop.log` `attach to
http://127.0.0.1:4317` и не порождает дочерний `dshr serve`; после выхода из
приложения сервер продолжает отвечать 200. Ошибочные пути: пустой конфиг →
«attach mode: no server URL…», недоступный URL → «server … is not reachable».

## Раунд 128 — desktop: настройки сервера внутри приложения (16.11.2026)

- Как в официальном DSH (`Settings` внутри приложения): вместо нативных
  osascript-диалогов появилось окно **dshr server** — URL/токен attach-сервера
  и переключатель «Attach to a server» / «Run a local server (dev/CI)».
  Сохранение идёт через Tauri-команду `save_server_form`: валидация схемы
  (`http(s)://`), проверка `/v1/health` для attach, запись
  `desktop-server.json` (0600) и перезапуск shell.
- Если attach-URL не задан, приложение **сразу открывает это окно** (вместо
  блокирующего нативного диалога) — первый запуск без конфигурации больше не
  зависит от osascript и работает одинаково на macOS/Linux/Windows.
- Валидация вынесена в `validated_server_settings` и покрыта тестами
  (attach требует http(s)-URL, локальный режим игнорирует URL/токен, токен
  тримится). Десктоп-тесты: 40 bin + 3 integration.

## Раунд 129 — desktop: самодостаточное приложение и Settings как в официальном (16.11.2026)

- По умолчанию десктоп снова **самодостаточен**, как официальный DSH: сам
  поднимает встроенный `dshr serve`, ждёт `/v1/health` и открывает UI; отдельный
  сервер не нужен. Attach к внешнему серверу остался опцией.
- Нативные osascript-диалоги провайдера убраны: окно **Settings** (трей →
  Settings…, на первом запуске открывается само) содержит секции **Account**
  (провайдер, модель, account token/API key, Base URL) и **Server** (встроенный
  сервер / attach URL+токен). Сохранение — Tauri-команда `save_settings_form`:
  валидация схемы и ключа, проверка `/v1/health` для attach, запись
  `desktop.env` (0600) и `desktop-server.json` (0600), перезапуск shell.
- Валидация вынесена в чистые функции `validated_provider_form` и
  `validated_server_settings` и покрыта тестами: DeepSeek account/API-key,
  OpenAI-совместимый base URL, неизвестный провайдер, сохранение ранее
  записанного секрета при пустом поле.
- Проверено: первый запуск без креденшелов открывает Settings (в
  `desktop.log` — «provider is not configured … opening settings») и приложение
  остаётся живым; с `desktop.env` (dummy-токен) `--check` поднимает встроенный
  сервер: `desktop check ok: http://127.0.0.1:53127/?token=…`.
- Осознанное отличие от официального: нет OAuth-входа в аккаунт DeepSeek (у нас
  нет клиента официального account-сервиса) — токен/ключ вводится в Settings и
  хранится в `desktop.env` с правами 0600.

## Раунд 130 — desktop: настоящий «Sign in with DeepSeek» и секции Settings (16.11.2026)

- Найден и проверен публичный OAuth официального клиента
  (`@deepseek-ai/dsh-deepseek-account` 0.2.2-rc.3): `POST
  /auth-api/v0/dsh/auth_init` (PKCE S256, `state`, `redirect_uri`,
  `login_source: desktop`, пять заголовков `x-client-*`) и
  `POST /auth-api/v0/dsh/auth_exchange` (`code`, `code_verifier`,
  `redirect_uri`, `device_id`, `device_model`, `os_version`) →
  `{token, authorized_url}`; профиль — `GET /auth-api/v0/users/current` с
  `x-dsh-auth-token`. Живой `auth_init` вернул 200 и `authorize_url` при
  loopback-redirect.
- В десктоп добавлен модуль `signin.rs`: локальный callback-сервер, PKCE и
  `state`, открытие системного браузера, проверка callback, exchange, сохранение
  grant как `DSH_DEEPSEEK_TOKEN` в `desktop.env` (0600) и перезапуск.
  Кнопка **Sign in with DeepSeek** — в Settings.
- Settings получил секции официального приложения: **Models** (провайдер,
  модель, Base URL), **Web search** (Brave key → `BRAVE_SEARCH_API_KEY`),
  **Plugins** (список Cordis-плагинов с переключателями через
  `/v1/plugins/installed` и `/v1/plugins/toggle` — те же эндпоинты, что у
  веб-UI). Модели/ключи по-прежнему пишутся в `desktop.env`, плагины — через
  сервер в конфиг профиля.
- Тесты: 45 unit (включая конверт платформы, разбор callback с `state`,
  S256-челлендж) + 3 integration. Полный browser round-trip не прогонялся:
  нужен живой вход в аккаунт.

## Раунд 131 — Settings: Models/Web search/Plugins в веб-UI (16.11.2026)

- Веб-UI получил единый экран **Settings** с секциями официального приложения:
  Account (подсказка), **Models** (route/model/base URL — `/v1/config/model`),
  **Web search** (Brave-ключ — новый `/v1/config/search`, GET показывает
  `configured`/маску/источник, POST пишет ключ и **перерегистрирует
  `web_search` в живом сервере**), **Plugins** (переход в раздел плагинов).
  Открывается по deep-link `#settings`, поэтому десктоп может на него ссылаться.
- В схеме конфига появилась секция `search: {api_key, base_url}` (env
  `BRAVE_SEARCH_API_KEY` важнее), `AppState::new` регистрирует `web_search` с
  ключом из конфига, а POST-эндпоинт обновляет инструмент без рестарта.
- Десктопное окно Settings сокращено до **Account + Server** и кнопки
  **Open app settings** (Tauri-команда `open_app_settings` открывает
  веб-UI на `#settings`). Провайдер/модель больше не передаются флагами
  `--provider/--model`: источник — `config.yaml` (веб-UI Models), окружение
  поставляет только креденшелы.
- Тесты: серверные `search_config` (запись/маска/валидация), 45+3 десктопных,
  workspace-тесты и `check.sh quick` зелёные; `trunk build --release` для
  веб-UI проходит.
- Живая проверка контракта OAuth: `auth_exchange` с заведомо неверным кодом
  отвечает `HTTP 200 {biz_code:1, biz_msg:"INVALID_CODE"}` — формат запроса
  принят, для входа нужен только код после подтверждения в браузере.
- Смоук-тест живого сервера: `/` → 200, `GET /v1/config/search` →
  `configured:false`, `POST` с ключом → `configured:true`, маска `smok…-123`.

## Раунд 132 — Settings в официальном стиле: сайдбар секций (16.11.2026)


- Экран Settings веб-UI переделан в один экран с сайдбаром (Account / Models /
  Web search / Plugins) и телом активной секции — как в официальном приложении,
  вместо длинной страницы. Табы переключают секции, `#settings` открывает экран
  сразу, стили добавлены в `crates/web/style.css` (`.settings*`).
- Десктопное окно Settings ведёт в этот экран кнопкой **Open app settings**;
  Account/Server остаются в окне десктопа.
- Пересобраны веб-ассеты (`trunk build --release`) и `dshr-cli` (dist
  встраивается через `include_bytes!`/`include_str!`); workspace-гейт и
  `check.sh quick` зелёные.

## Раунд 133 — отмена входа, таймаут и вкладка Server (16.11.2026)

- Вход можно отменить: Tauri-команда `cancel_sign_in` выставляет флаг, цикл
  ожидания callback завершается с `sign-in cancelled`; таймаут настраивается
  `DSHR_DESKTOP_LOGIN_TIMEOUT_SECS` (5–3600 с, по умолчанию 600). Тест
  `cancellation_ends_the_flow` покрывает отмену, полный round-trip — как раньше.
- В сайдбар Settings веб-UI добавлена вкладка **Server**: origin, статус
  `/v1/health` и активная модель/провайдер из `/v1/config/model`; режим
  (встроенный/attach) по-прежнему задаётся в окне десктопа, о чём вкладка и
  сообщает. Deep-link `#settings` подгружает и её.
- Десктоп: 46 + 3 тестов зелёные; веб-ассеты пересобраны (`trunk build
  --release`), `dshr-cli` пересобран со встроенным dist.

## Раунд 134 — Node-runtime pack по умолчанию (16.11.2026)

- Cordis-sidecar перестал зависеть от системного `node`: `ensure_node` находит
  `DSHR_NODE`, затем установленный пак, затем **скачивает запиненный Node
  24.21.0** (macOS/Linux, arm64/x64) с `nodejs.org`, сверяет sha256 из таблицы
  `node_pack_spec` и активирует пак; при ошибке — предупреждение и `node` из
  PATH. Отключается `DSHR_RUNTIME_AUTO=0`. Windows (zip) остаётся на системном
  `node`.
- Живая проверка: `dshr plugin npm add @kt11/dsh-deepseek-balance` в чистом home
  + запуск `serve` → в логе `dshr: installed node 24.21.0 runtime pack at
  /tmp/pack-home/runtimes/node/24.21.0`, бинарь отвечает `v24.21.0`;
  повторный старт пак переиспользует.
- Тесты: таблица платформ (4 поддерживаемых, windows/freebsd → None),
  `download_pack` из локального `file://` (установка, `current`, удаление
  временного файла) и отказ на подменённом архиве.

## Раунд 135 — i18n оболочки, About и DevTools (16.11.2026)

- Оболочка десктопа получила ru/en: `DSHR_DESKTOP_LANG` важнее системной локали
  (`LC_ALL`/`LANG`/`AppleLocale`), переведены пункты трея и диалоги
  (`tr_for`, тест на маппинг и фолбэк). Число языков — 2, а не 8 как у
  официальной оболочки.
- В трее появились **About dshr** (версия, home, профиль, платформа) и
  **Developer tools** (`open_devtools`, включён feature `devtools` у Tauri).
- Десктоп-тесты: 47 + 3.

## Раунд 136 — origin-граница plugin-маршрутов (16.11.2026)

- Попытка ввести cookie-аутентификацию для plugin-маршрутов провалилась на
  реальности: кадры плагинов живут в `sandbox="allow-scripts"` без
  `allow-same-origin`, поэтому браузер не отправляет ни cookie, ни токен.
  Cookie-подход откатан; граница — **origin**: `Origin: null` (песочный кадр),
  loopback-origin и запросы без `Origin` разрешены, чужой сайт получает 403
  `cross-origin plugin request`; loopback-only политика сохранена.
- Тест `proxies_plugin_host_routes` проверяет 403 для `https://evil.example` и
  200 для `null`/`http://127.0.0.1:4317`; весь набор `plugins_live` — 18/18.
- Границы записаны в [TECH-DEBT](TECH-DEBT.md) и [PLUGINS](PLUGINS.md).

## Раунд 137 — SSE для plugin-маршрутов и черновик NSIS (16.11.2026)

- Plugin-маршруты теперь **стримят** ответы: если клиент просит
  `text/event-stream`, ответ плагина приходит SSE-типом или без
  `content-length`, прокси отдаёт `Body::from_stream` без буферизации и без
  15-секундного таймаута; обычные ответы остаются буферизованными с лимитом
  8 MiB. Тест `streams_server_sent_events_from_plugin_routes` проверяет, что
  первый `data: one` приходит, пока плагин держит соединение (второй — через
  300 мс), всего `plugins_live` — 19/19.
- Черновик Windows-инсталлятора: `packaging/windows/dshr.nsi` (per-user,
  ru/en, ярлыки, HKCU-uninstall, опциональный PATH, `~/.dshr` не трогается) и
  ручной workflow `windows-installer.yml`. Локально `makensis` нет, на Windows
  не прогонялось — job не входит в релизный пайплайн (см. RELEASE.md).

## Раунд 138 — Settings: ключ и сервер на одном экране (16.11.2026)

- Окно Settings десктопа снова **одна страница**: провайдер (DeepSeek /
  OpenAI-совместимый), модель, тип аутентификации, API-ключ, base URL и тут же
  сервер (встроенный/attach, URL, токен) — с одной кнопкой **Save and restart**.
  Валидатор `validated_provider_settings` пишет `desktop.env` (0600) и
  восстанавливает флаги `--provider/--model`, поэтому выбранный маршрут
  применяется сразу; `BRAVE_SEARCH_API_KEY` сохраняется при правке ключа.
- Веб-UI: вкладки Account и Server слиты в одну **Connection** (origin, health,
  активная модель + пояснение, что ключи/сервер задаются в Settings десктопа);
  сайдбар: Connection / Models / Web search / Plugins.
- Тесты: `provider_form_keeps_the_chosen_key_and_route` (deepseek + openai,
  флаги, base URL, отказ на `ftp://` и неизвестном провайдере), `free_port`
  сделан устойчивым к гонке за эфемерный порт; десктоп 46 + 3.

## Раунд 139 — пиксельный паритет Settings (16.11.2026)

- Окно Settings десктопа пересобрано на **официальных стилях**: из app.asar
  0.2.2-rc.3 извлечены `design-platform.css` (токены `--dsw-alias-*`),
  `base.css` (радиусы/шрифты) и `ModelsSection.module.css` (карточки, точки
  состояния, редактор, `keyInput`/`keyToggle`), вшиты в страницу без правок.
  Разметка повторяет официальную: `section/title/intro/rows/rowCard/rowHead/
  rowIdentity/credentialDot/rowTag/rowActions/editor/field/editorActions`.
- Ключ показывается маской сохранённого значения (как `ApiKeyInput`), поле
  черновика с кнопкой-глазом, пустое поле сохраняет прежний секрет; карточки
  DeepSeek / OpenAI-compatible / Server с точками состояния; сервер-only
  сохранение больше не перезаписывает маршрут (пустой provider = сохранить).
- Проверка: страница отрендерена headless Chrome (два скриншота в
  `docs/screenshots/`), тест `settings_ui_assets_are_the_official_design_system`
  фиксирует наличие плейсхолдеров и ключевых селекторов/токенов; десктоп
  47 + 3.

## Раунд 140 — Models: каталог провайдеров и списки моделей (16.11.2026)

- Фундамент «settings namespaces» для моделей: `dshr-config::models` —
  документ `<home>/models.yaml` (`version`, `revision`, `providers`),
  атомарная запись 0600, revision-fence (устаревшая ревизия → `409`),
  валидация маршрута/base URL/имени переменной/моделей, встроенный каталог
  `deepseek-account`/`deepseek-official`/`openai`/`openai-compatible`/`routerai`
  (алиас `deepseek`) и `resolve` с приоритетом профиль → `model.base_url` →
  env → каталог.
- Сервер: `GET /v1/config/providers` (каталог + строки + активный выбор,
  состояние ключа только как «найден/нет»), `PUT /v1/config/providers/{id}`
  (полная замена профиля, `expected_revision`), `DELETE …?expected_revision=N`;
  `POST /v1/config/model` теперь принимает кастомный маршрут с профилем и
  проверяет модель по непустому списку. Пять интеграционных тестов.
- `dshr serve` собирает провайдера из профиля: endpoint, имя переменной ключа,
  кастомный протокол (`anthropic-messages`/`openai-chat`), fallback на прежние
  env-ключи; предупреждение, если активная модель выпала из списка.
- Web UI: вкладка Models — строки провайдеров (точка ключа, теги `active`/
  `defaults`/протокол), редактор (base URL, имя переменной ключа, таблица
  моделей с input-типами, добавление/удаление/сброс списка), активация
  маршрута, удаление профиля с подтверждением, добавление из каталога,
  сохранение с revision. Официальные `--provider/--model`-флаги десктопа
  работают как раньше.
- Проверки: config 6 новых, server 5 новых; web-e2e переписан под Models
  (провайдер, профиль, добавление, удаление, `models.yaml`); браузерный прогон
  17/18 — единственный красный чек (bash-инструмент `timed out`) не связан с
  Models и воспроизводится в этой среде при любом UI.
- Осталось (следующие раунды): кастомный провайдер в UI, reasoning-уровни,
  retry-policy, модальные диалоги, discovery моделей, онбординг, слоты секции,
  ключи через хранилище; см. [TECH-DEBT](TECH-DEBT.md).

## Раунд 141 — Models: кастомный провайдер и reasoning-уровни (17.11.2026)

- **Кастомный провайдер** в web-UI: режим **Custom model API** создаёт профиль
  с произвольным маршрутом (`[a-z][a-z0-9-]*`), protocol
  (`openai-chat`/`anthropic-messages`), обязательным base URL, именем
  переменной ключа и хотя бы одной моделью; серверная валидация уже принимала
  такие маршруты, UI-форма выровнена с ней. Редактор карточки вынесен из
  строки провайдера в отдельную `models-editor-card`, поэтому одна и та же
  форма обслуживает и adopt из каталога, и правку, и создание.
- **Reasoning-уровни** per-model: переключатель off/on, строки
  уровень → API-значение (7 уровней, без дублей, непустые значения,
  `false` для выключенного), «Add level»; `draft_body` валидирует перед
  записью, `ModelEntry.reasoning_efforts` сохраняется в `models.yaml` и
  возвращается API. Рантайм-адаптеры пока не отправляют effort (в `Request`
  нет поля) — это отдельный слой.
- Проверки: web 16 тестов (+5: custom/reasoning-хелперы), web-e2e 18/19
  (новый чек «custom provider persisted with protocol, model and reasoning
  levels»; единственный FAIL — прежний bash `timed out` среды). Сервер/config
  не менялись, их наборы остаются зелёными.
- Осталось: discovery моделей endpoint'а и пикер, retry-policy, модальные
  диалоги, онбординг, слоты секции, хранение ключей.

## Раунд 142 — Models: retry-policy (17.11.2026)

- Новая политика повторов запроса в `models.yaml` (per-provider):
  `mode: normal|always`, `max_retries`, `retryable_codes`, `backoff`
  (`initial_delay_ms`/`max_delay_ms`/`jitter_ratio`). Дефолты как у upstream:
  normal, 5 попыток, пять транзиентных кодов, 500 мс / 10 000 мс / 0.1;
  `always` повторяет любую ошибку до успеха/отмены. Валидация: непустые
  уникальные коды, положительные задержки, `initial <= max`, jitter `0..1`.
- `dshr-llm` получил `RetryPlan` (чистые типы и тайминги): классификация
  `RetryCode` (429 → RATE_LIMIT, 5xx → SERVER, timeout/transport), лимит
  попыток, экспонента с потолком, детерминированный джиттер и учёт
  `Retry-After`. `llm-deepseek` и `llm-openai` больше не используют локальный
  `MAX_ATTEMPTS = 3`: оба исполняют план, поэтому `normal/always`, коды и
  backoff применяются на реальных запросах. `serve` собирает план из профиля.
- API: `retry` в строке провайдера и в `PUT /v1/config/providers/{id}`;
  серверные интеграционные тесты проверяют round-trip и отказ на
  `initial > max`. Web-UI: раскрывающийся блок **Retry policy** в карточке
  (inherit / transient / always, maxRetries, чекбоксы кодов с подписями,
  backoff-поля), клиентская валидация, сохранение в `models.yaml`.
- Проверки: `dshr-llm` 4 новых теста плана, `llm-openai` интеграционный тест
  «500 → повтор → успешный стрим», config +1, server round-trip; web 17
  тестов; web-e2e 18/19 (custom-провайдер теперь сохраняет и retry-policy;
  единственный FAIL — прежний bash timeout среды).
- Осталось: discovery `/v1/models` и пикер, модальные диалоги, онбординг,
  слоты секции, ключи, отправка reasoning effort.

## Раунд 143 — Models: discovery моделей и пикер (17.11.2026)

- Сервер: `POST /v1/config/providers/discover` (`protocol`, `base_url`,
  `api_key_env`) спрашивает endpoint о каталоге. OpenAI-формат — `{base}/models`
  с Bearer; Anthropic-формат — `{base}/v1/models` с `x-api-key` +
  `anthropic-version`, а для DeepSeek-базы с суффиксом `/anthropic` —
  `{root}/models` как fallback. Разбираются `data`/`models` (массив или карта
  id→name), дубликаты отбрасываются, лимит 500. Ключ читается только из
  окружения по имени переменной; 15-секундный таймаут, `404` пробует
  следующий URL, прочие ошибки — `502` с диагностикой.
- Web-UI: кнопка **Fetch available models** в карточке редактора; ответ
  открывает пикер с фильтром, Select all/Deselect all и **Update list**.
  Слияние: отмеченные добавляются, снятые (из перечисленных endpoint'ом)
  удаляются, настроенные, но не перечисленные строки сохраняются; попытка
  опустошить непустой список отклоняется. Помощник `merge_discovered`
  вынесен и покрыт тестами.
- Проверки: server `provider_settings` 6 (+discovery на локальном endpoint,
  отказ на `ftp://` и неизвестном протоколе); web 18 (+merge-тест); web-e2e
  **19/20** — новый чек «discovery lists the endpoint models», custom-провайдер
  получает модели через пикер; единственный FAIL — прежний bash timeout среды.
- Осталось: модальные диалоги удаления, онбординг/welcome, слоты секции,
  i18n, хранение ключей, отправка reasoning effort.

## Раунд 144 — Models: модальные удаления и first-run notice (17.11.2026)

- Удаление провайдера и строки модели идёт через **модальное подтверждение**
  (`ConfirmAction` → `.models-modal`): диалог называет провайдера/модель,
  Delete выполняет действие только после подтверждения, Cancel закрывает;
  кнопка Delete заблокирована на время записи. Inline-подтверждение удалено.
- **First-run notice**: `ModelsDocument.welcome_version` в `models.yaml`
  (durable, без revision-fence), `POST /v1/config/providers/welcome`
  (`version` 1..1000), поле в `GET /v1/config/providers`. Веб-UI показывает
  модальный «Welcome to Models», пока версия меньше текущей; Continue пишет
  версию, notice больше не появляется. Если ни у одного провайдера нет
  ключа, notice добавляет подсказку про `DEEPSEEK_API_KEY`/
  `DSH_DEEPSEEK_TOKEN`/desktop Settings. Полноценный DeepSeek-онбординг с
  вводом ключа невозможен без хранилища секретов — граница записана.
- Проверки: server provider_settings 7 (+durable welcome round-trip, отказ на
  version 0); web 18; web-e2e **21/22** (+«first-run notice acknowledged
  durably», +«model deletion names the model» с отменой; провайдерское
  удаление теперь через модалку). Единственный FAIL — прежний bash timeout
  среды.
- Осталось: слоты секции, i18n, хранение ключей, отправка reasoning effort.

## Раунд 145 — Models: дочерние слоты секции (17.11.2026)

- Секция Models объявила два extension-места из upstream-контракта:
  `settings.models.provider-card` (keyed по route, рендерится внутри каждой
  карточки провайдера) и `settings.models.footer` (после списка и Add).
  Владельцу карточки уходят `provider` (id/displayName/protocol/settingsPath/
  active/defaultBaseURL), `configured` и `keyConfigured`; context — `{provider}`.
- Регионы: bridge-таблица получила `settings.models.card` (provider-card) и
  `settings.models.footer` (footer) вместо плоских `settings.section`/
  `settings.action`, поэтому вкладка больше не смешивает карточки с общими
  панелями Settings. Harness каталога слотов обновлён (31 регион), shell CSS,
  PLUGINS-таблица регионов и счётчики в PARITY/LIMITATIONS/ROADMAP — тоже.
- Проверки: `dshr-cordis` 9 (bridge harness с новыми регионами), web-e2e
  **22/23** (+«models mounts its child slot regions»; единственный FAIL —
  прежний bash timeout среды). Web-сборка/бинарник пересобраны.
- Осталось: i18n ru/en, DeepSeek-онбординг с ключом (нужно хранилище
  секретов), отправка reasoning effort.

## Раунд 146 — Models: хранилище ключей и ключ в онбординге (17.11.2026)

- Новый `dshr-config::credentials`: `<home>/credentials.json` (0600, atomic,
  `version`/`revision`/`values`), env важнее документа, маска `abcd…wxyz`,
  валидация reference (`UPPER_SNAKE`, ≤96) и секрета (printable ASCII,
  1..4096). API: `POST /v1/credentials/describe` (до 64 refs, только
  configured/source/masked), `PUT /v1/credentials/{reference}` (write-only,
  revision-fence), `DELETE` с ревизией. Значения не возвращаются и не
  логируются.
- Строка провайдера теперь отдаёт `credential_masked`/`credential_source`;
  `dshr serve` и discovery читают ключ через `credentials::resolve` (env →
  файл). Профиль по-прежнему хранит только имя переменной.
- Web: поле **API key** в карточке редактора (пароль + Show/Hide, saved-маска,
  «Remove saved key»); при Save профиль пишется первым, затем ключ под
  выбранной/производной `<ROUTE>_API_KEY`; частичный сбой показывает
  «profile saved, but the key was not stored». First-run onboarding получил
  поле DeepSeek-ключа и сохраняет его как `DEEPSEEK_API_KEY` перед
  подтверждением notice. Удаление профиля убирает сохранённый ключ только если
  профиль называл производную `<ROUTE>_API_KEY`; кастомные/inherited
  reference'ы остаются.
- Проверки: config 25 (+3 credentials, 0600/маска/revision), server 8
  (+write-only round-trip, join в строку, 409), web 19, web-e2e **24/25**
  (+«onboarding stores the key write-only», +«the credentials file is
  private», ключ из редактора openai); единственный FAIL — прежний bash
  timeout среды.
- Осталось: i18n, отправка reasoning effort, ключи в `run`/`tui`.

## Раунд 147 — Models: ручная проверка в живом UI и два фикса (20.11.2026)

- Независимая ручная проверка (сабагент, headless Chrome + CDP) прошла 7/7
  сценариев: welcome/онбординг с ключом, список провайдеров, редактор и
  сохранение, кастомный провайдер + discovery, модальные удаления, отсутствие
  JS-ошибок, скриншоты. Репозиторий не менялся.
- Найдены и исправлены два дефекта:
  1. **HIGH:** в welcome-модалке поле ключа сжималось до ~18 px, а подсказка
     наезжала на него (grid `140px minmax(0,1fr) auto` в узкой карточке).
     Добавлено `.models-welcome .models-field { grid-template-columns: minmax(0,1fr) }`
     + подсказка отдельной строкой; копирайт welcome обновлён под
     `credentials.json`.
  2. **LOW:** jitter показывался как `0.10000000149011612` — persisted/resolved
     `jitter_ratio` переведён с f32 на f64 (serve кастует в f32 для
     `RetryPlan`), API/UI/`models.yaml` теперь пишут `0.1`.
- Повторная проверка тех же двух сценариев: PASS (input width 422 px,
  hint без overlap; jitter `0.1` в поле, после Save+reopen, в API и YAML).
- Проверки: config 25, server 9, web 19, e2e 24/25 (прежний bash timeout
  среды); clippy/fmt чисто.

## Раунд 148 — Reasoning effort на проводе + гейты `cargo deny` и doc-тестов (22.11.2026)

- Закрыт записанный техдолг «адаптеры не отправляют reasoning effort»:
  `model.reasoning_effort` в `config.yaml` (пусто = ключ убирается, значение
  обязано быть объявленным уровнем, иначе 400) резолвится `serve` в значение
  провайдера из `models.yaml` и живёт на конфиге соединения
  (`DeepSeekConfig`/`OpenAiConfig.reasoning_effort`), поэтому
  `dshr_llm::Request`, `crates/agent` и `title.rs` не тронуты —
  [ADR-043](adr/043-reasoning-effort-wire.md).
- DeepSeek: `thinking` + `output_config.effort` (при `off` только
  `thinking: disabled`); OpenAI-compatible: плоский `reasoning_effort` для
  `gpt-oss`, вложенный `reasoning.effort` для прочих `gpt-*`, ничего для
  остальных. Неподдерживаемый уровень падает до сетевого ввода-вывода; при
  отсутствии уровня reasoning-параметр не отправляется вовсе (upstream
  подставляет `high` — расхождение записано в PARITY).
- Models UI: селектор уровня активной модели с пунктом **Provider default**,
  показывается только при ≥ 2 объявленных уровнях; `GET /v1/config/providers`
  отдаёт `active.reasoning_effort`, поэтому селектор не сбрасывается после
  сохранения. `run`/`tui`/`acp` остались env-only (записано в TECH-DEBT).
- Живая проверка (headless Chrome + CDP, 14/14): уровень появляется/скрывается
  по метаданным модели, сохранение пишет `config.yaml`, «Provider default»
  убирает ключ, рестарт `dshr serve` применяет значение.
- Гейты: `scripts/check.sh` догнал CI — добавлены `cargo deny check` (с
  понятным сообщением, если бинарь не установлен: он не workspace-зависимость)
  и `cargo test --workspace --doc`. Сам `cargo deny` был красным ещё до этой
  работы: цепочка wasmtime 50.0.1 → 50.0.3 (сняты `RUSTSEC-2026-0566..0573`,
  `-0587`), yanked `wasmi` 2.0.0 → 2.0.2, unmaintained `chrono` — обоснованный
  ignore в `deny.toml`.
- Ретроспектива того же раунда закрыла четыре расхождения. `crates/web`
  исключён из workspace, поэтому ни `cargo test --workspace`, ни nextest его не
  видели, хотя `dist` вкомпилирован в сервер через `include_bytes!`: добавлены
  шаг `web_check` в `check.sh` (хост-тесты + `cargo check --target
  wasm32-unknown-unknown`, ~0.2 с) и джоб `web` в CI. `workspace_tests()`
  выбирает тот же раннер, что CI (nextest, иначе громкий откат на `cargo test`),
  чтобы не гонять набор дважды. `rust-version` поднят до реального пола 1.97
  (его задаёт `wasmtime-wasi 50.0.3`; `crates/desktop` — отдельный workspace с
  собственным lockfile, его 1.90 не тронут). Удалён мёртвый ignore
  `RUSTSEC-2024-0370` (advisory больше ничего не матчит — проверено зелёным
  `cargo deny check` без него), устаревший `chrono` заведён строкой в
  TECH-DEBT. Там же всплыл свежий `RUSTSEC-2026-0592` (`url` 2.5.8: переполнение
  стека в `Url::origin()` на вложенных `blob:`-URL) — по политике это апгрейд,
  а не ignore, `url` 2.5.9 + `form_urlencoded` 1.2.3. В `plugin-smoke.py`
  добавлен блок Models API (8 проверок:
  `active.reasoning_effort`, каталог маршрутов с уровнями, round-trip уровня,
  отказ на необъявленном уровне до рестарта, снятие ключа при «Provider
  default»).
- Инцидент этого раунда: `scripts/check.sh quick` выглядел зелёным, хотя
  `cargo fmt --all -- --check` печатал расхождения — вывод шёл через `| tail
  -40`, и статусом процесса был `tail`. Правило («код возврата берётся у самой
  команды, `set -o pipefail` + `${PIPESTATUS[0]}`») записано в
  `docs/DEVELOPMENT.md`; гейты с тех пор запускаются без пайпа.
- Проверки: config 26, llm-deepseek 15, llm-openai 13, provider_settings 10,
  web 23, wasm 11, doc-тесты 0 passed/5 suites, `check.sh quick` зелёный,
  e2e 25/25, plugin-smoke 97/97.

## Раунд 149 — Windows-only ветки: реальный `E0308` и механический гейт (22.11.2026)

- `test (windows-latest)` был красным как минимум четыре прогона подряд:
  `error[E0308]` в `crates/npm/src/install.rs`, «expected
  `Result<(), DshrError>`, found `Result<u64, DshrError>`». Баг пришёл из
  `216aedc` (резолв Cordis-плагинов без npm) и жил неделю: локальный хостинг
  `cargo clippy --workspace` его не компилирует вовсе, а Linux/macOS-ноги
  матрицы тоже. Красной была ровно одна нога из девяти.
- Причина в форме кода, а не в типе: тело
  `#[cfg(not(unix))] fn create_symlink` целиком было закрыто атрибутом, поэтому
  компилятор проверял его только на Windows. Остальные девять платформенных
  веток в воркспейсе (`crates/cordis/src/runtime.rs`, `crates/npm/src/store.rs`,
  `crates/desktop/*`) оказались типово корректны — единственная ошибка была в
  `install.rs`.
- Фикс: копирование вынесено в `copy_instead_of_link`, скомпилированный на
  каждом хосте, а `cfg` остался на однострочной обёртке. Два юнит-теста гоняют
  эту функцию на хосте, так что тип возврата теперь виден локально.
- Гейт: `scripts/check-platform-branches.py` валит сборку, если тело под
  `#[cfg(windows)]`/`#[cfg(not(unix))]` длиннее 6 строк. Он стоит первым шагом
  в `check.sh` и в каждой ноге CI, потому что Windows-раннер для этого не
  нужен; правило записано в `docs/DEVELOPMENT.md`.
- Побочно: `nextest.log is missing` в тех же логах — следствие того, что шаг
  `clippy` упал раньше `nextest`, а не отдельный баг.
- Проверки: `dshr-npm` 15+6 тестов (включая два новых), clippy/fmt чисто,
  `check-platform-branches` — 10 платформенных веток, все тела ≤ 6 строк.

## Раунд 150 — зелёная Windows-лега: фикстуры пака, копия симлинка, `file://` (22.11.2026)

- Windows-лега была красной неделю, но аннотации `nextest` показывали не больше
  двух падений из семи: `grep ... | head -30` в шаге аннотаций обрезал список.
  Корень один — `create_symlink` под `#[cfg(not(unix))]` возвращал `Err`, и
  фикстура пака с `bin/npm` роняла каждый тест, который её распаковывал.
- Фикстуры разделены: обычная (`bin/node` + `lib/npm-cli.js`) собирается
  `pack_archive`, симлинковая — `pack_archive_symlinked` под `#[cfg(unix)]`.
  Хелпер гейтится тем же `cfg`, что и его единственный вызывающий: иначе
  Windows-clippy с `-D warnings` падает на `dead_code`.
- Семантика не-unix симлинка приведена к `crates/npm`: копирование содержимого
  (`copy_instead_of_link`, скомпилировано на каждом хосте) вместо ошибки.
  Тело под `cfg` — однострочный делегат, как требует `MAX_BLOCK_LINES = 6`.
- `file://`-URL тестов собирались через `Path::display`, а на Windows это
  обратные слэши, непарсимые curl. Введён `local_url`: `file:///C:/...` для
  диска, `file:///...` для позы. Продакшн-код `file://` не строит.
- Следом Windows-лега открыла вторую ошибку — `report.path_of` и пути в
  `package-lock.json` собирались из нативного `Path`, то есть с `\` на
  Windows. Оба текста кросс-платформенные, поэтому путь теперь строится из
  прямослэшевого представления (`posix`). Воспроизведено симуляцией: обратные
  слэши в `posix` дают ровно те же два падения, что и CI.
- `web e2e` падал по гонке: `delete` пишет текст статуса до `busy.set(false)`,
  проверка кликала по ещё выключенной кнопке и теряла клик. Ожидание переведено
  с текста статуса на включённую кнопку.
- Проверки: прогон `45020562286` на `e07c38c` — все 9 джоб зелёные, включая
  `test (windows-latest)`; `dshr-npm` 6 интеграционных + юнит-тесты,
  `web-e2e` 25/25.

## Раунд 151 — i18n секции Models: ru/en через `ui.language` (22.11.2026)

- Секция Models была единственным экраном, где строки писались в разметке
  литералами. 103 подписи вынесены в `dshr-i18n` структурой `ModelsLabels` по
  образцу `ExportLabels`: одно поле `&'static str` на подпись и `match self`
  по языку, поэтому пропуск строки в одном языке не компилируется, а тест
  `models_labels_are_complete_in_both_languages` дополнительно проверяет, что в
  английском плече нет кириллицы. Ещё 27 сообщений с подстановками остались
  методами, возвращающими `String`, — `&'static str` не удержит `format!`.
- Язык берётся из `ui.language`: `Lang::parse` уже применялся в
  `crates/cli/src/tui_cmd.rs`, повторно использован здесь. Дефолт в
  `UiConfig` — `ru`, поэтому секция русская по умолчанию.
- Канал языка — `GET /v1/health`. Бандл `crates/web` собирается в статику и
  вшивается в сервер через `include_bytes!`, конфиг он прочитать не может, а
  серверного шаблонизатора в разметке нет. Поэтому `health` отдаёт
  `{status, version, language}`, UI стартует с `Ru` и переключается, когда
  ответ придёт; поле `#[serde(default)]` — чтобы старый сервер без `language`
  не ломал UI.
- `RwSignal<Lang>` передаётся в `ModelsSettings` пропом, подписи собираются
  один раз через `Signal::derive(move || lang.get().models_labels())`. Внутри
  замыканий, которые уже захватили `busy`/`editing`, императивные обработчики
  читают язык через `get_untracked()`, иначе `Lang` пришлось бы клонировать в
  каждую ветку.
- Текст не переводится там, где он не текст: ключи и значения протокола
  (`deepseek-account`, `openai-chat-completions`), имена переменных ключа,
  коды ошибок и CSS/ARIA-строки остались как есть. Заголовок активного
  маршрута сохраняет `{} · {} — {}` с непереведёнными провайдером и моделью.
- `scripts/web-e2e.py` пиннит `ui.language: en` в конфиг харнесса (иначе
  английские селекторы перестали бы совпадать) и получил отдельную проверку
  на русском home: секция должна содержать кириллицу и сохранять
  `deepseek-account`. Проверка сначала читала `innerText` до прихода
  провайдеров и падала на плейсхолдере — теперь ждёт строку каталога.
- Побочно: `busy.set(false)` перенесён перед записью статуса в четырёх
  обработчиках (`save`, `remove_saved_key`, `delete`, make-active). Гонка была
  закрыта в харнессе в раунде 150, но корень — в источнике: строка статуса
  появлялась раньше, чем кнопка снова становилась кликабельной.
- `GET /v1/health` получил собственный тест: `health_reports_the_configured_language`
  проверяет, что поле `language` доезжает до публичного рукопожатия, — раньше
  форма ответа была проверена только вручную.
- Проверки: `dshr-i18n` 8, `web` 23 (все на `Lang::En`), wasm-check чистый,
  `dshr-server --test http` 27, `web-e2e` 27/27, `plugin-smoke` 97/97,
  `check.sh` зелёный. Артефакты `crates/web/dist` пересобраны
  `trunk build --release` (wasm 1.87 МБ); дев-сборка даёт 43 МБ и в коммит не
  годится.

## Формализация и каталог тестов (14.11.2026)

- `formal/` — три TLA+-модели (`CordisSupervisor` safety+liveness, `ClientAssets`,
  `ApprovalGate` safety+liveness), 5 конфигов TLC; `./formal/run_tlc.sh` →
  «all models checked». Абстракции и результаты: `docs/formal/README.md`.
- `docs/testing/test-cases.md` — каталог ~150 кейсов с ID, уровнями, привязкой к
  инвариантам и статусами; baseline `cargo llvm-cov`: 72.9% строк, 66.9% функций.
- Деривированные тесты: атомарность `register_all` под читателями, symlink-escape
  в ассетах, восстановление после падения sidecar, CSP-инъекция через `Host`,
  scope/TTL capability.
