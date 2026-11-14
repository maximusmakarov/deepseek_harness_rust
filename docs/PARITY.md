# Паритет с DSH Desktop 0.2.2-rc.2

Матрица пользовательских сценариев оригинала и `dshr`. Источники: reference-клон
`dsh-desktop@233b76c` (`docs/user-guide.md`, `docs/architecture.md`), установленный
DSH 0.2.2-rc.2 и собственные ADR. Статусы: ✅ есть, ⚠️ частично, ❌ нет,
⏸ сознательно отложено.

## Desktop

| Сценарий оригинала | dshr | Статус |
|---|---|---|
| Первый запуск, профиль по умолчанию, настройка | авто-создание home/README/desktop-profile + нативный provider setup (маршрут/модель/ключ, `desktop.env 0600`); wizard'а бандлов нет | ✅ |
| Запуск с папкой (`dsh-desktop <folder>`, drag-and-drop) | аргумент, сохранённая `desktop-folder` и drag-and-drop папки на окно (рестарт с новым cwd) | ✅ |
| Профили: список в трее, переключение с рестартом, last-known-good | CLI `--profile`/`DSH_PROFILE`; трей: список, New profile…, переключение пишет `desktop-profile` и перезапускает shell | ✅ |
| Профиль как композиция бандлов/зависимостей/патчей | config-слои + `bundles` с `requires`/semver + bundle/profile `patches` + Cordis-sidecar | ⚠️ |
| Режимы окна (compatibility/extended/enhanced), материалы macOS | Compatibility/Enhanced/Extended: overlay title bar на macOS, frameless+кастомный titlebar на Linux/Windows; Material (macOS vibrancy) | ⚠️ |
| Локальный web-порт: random/фиксированный, LAN-подтверждение | трей Network: local/LAN с предупреждением, фиксированный/случайный порт; как в оригинале — без отдельного LAN-диалога подтверждения | ✅ |
| Трей: show/hide, profile, terminal, updates, quit; закрытие окна = hide | трей: show/hide, профили, браузер, терминал, диагностика, quit; close-to-tray | ⚠️ |
| Web UI: тема | light/dark через кнопку Theme, localStorage и `?theme=light`; live-тема для iframe-плагинов | ✅ |
| Web UI: настройки провайдера | диалог Settings: маршрут/модель/base_url, запись user-слоя `config.yaml` (0600) и подсказка о перезапуске; ключи — из окружения | ✅ |
| Web UI: карточки инструментов и diff | карточка `tool_start` показывает сводку и превью изменений (`-old`/`+new` для `edit`, `+` для `write`, тело `apply_patch`), `tool_end` — результат | ✅ |
| Plugin market | Plugins UI: каталог с категориями и тегами, серверный поиск `?q=`/`?category=`/`?limit=`, детальная карточка `GET /v1/plugins/catalog/{name}` (оценка, агрегат отзывов, признак установки), поиск/инфо npm, Install, менеджер установленных, оценки и отзывы с агрегатом ★/count; общая серверная база отзывов ограничена home'ом пользователя | ⚠️ |
| Плагины: `dsh plugin --profile add/remove/update`, рестарт | CLI add/remove/update/list/enable/disable + Web-менеджер (installed, toggle/remove/install) для активного профиля | ✅ |
| Терминал из трея с приватными shim'ами | terminal.sh с DSHR_HOME/профилем/PATH и shim `<home>/bin/dshr` | ✅ |
| Обновления: фоновая проверка, канал, installer, Linux → download page | проверка, уведомление, каналы Stable/Beta, скачивание macOS zip с проверкой SHA256SUMS, распаковка/reveal, установка через helper (macOS); notarization нет | ⚠️ |
| Recovery/safe mode: plugin management, rollback, switch profile, diagnostics | при сбое старта — recovery-окно (Retry/Safe mode/Diagnostics/Config/Disable+Enable всех plugins/по-плагинные Cordis/профили/Quit); safe-mode, last-known-good + Rollback profile | ⚠️ |
| `--export-diagnostics`: zip с логами, crash dumps, system-info | `--check`, `--export-diagnostics` (zip: логи, конфиг, system-info) | ✅ |
| Платформы: macOS/Windows installer, Linux deb/AppImage/tar.gz | macOS `.app` + DMG, Linux `.deb`/tar.gz и Windows zip в release CI; Windows-installer'а нет | ⚠️ |
| Автообновление/подпись/notarization | ad-hoc по умолчанию; Developer ID + notarization при наличии секретов (не прогонялось) | ⚠️ |

## Host и инструменты

| Область | dshr | Статус |
|---|---|---|
| LLM-провайдеры, tool-loop, сессии, resume/fork/rewind, compaction | DeepSeek + OpenAI-совместимые | ✅ |
| Инструменты fs/shell/search/todo/present/web | есть | ✅ |
| `apply_patch` (DSH-формат) | есть, multi-file add/update/delete/move | ✅ |
| LLM-заголовки сессий | `run`, HTTP server и TUI генерируют заголовок провайдером (DSH-промпт), fallback — первая строка | ✅ |
| Импорт сессий DSH v4 | `sessions import` / `--all` (zstd JSONL → наша модель, title/tool calls) | ✅ |
| `ask_user` | TUI, Web, ACP elicitation (при capability клиента); headless fail-closed | ✅ |
| persistent bash, diff-view | `bash` — persistent shell на Unix (cwd/env между вызовами, изоляция сессий); Windows stateless; TUI diff-детали | ✅ |
| MCP: stdio/HTTP, tools/resources/prompts/sampling, OAuth refresh, elicitation | elicitation в stdio, HTTP POST-SSE и фоновом GET-SSE | ✅ |
| ACP v1: сессии, permissions, fs, terminal, image/audio/resource, MCP | есть; `session/list`, `session/set_mode`, `session/close`, `session/resume` | ✅ |
| Плагины: WASM core/component, Ed25519, Cordis-sidecar | Cordis: audit-log, fs-гранты, hot reload (watcher + endpoint) и клиентская половина (eager registry-кадр на каждую половину + отдельный sandboxed iframe на слот, полный словарь DSH 0.2.2-rc.2 (116 ключей) и legacy `panel`/`message.action` разложены по 17 регионам шелла, незнакомое имя — fallback в панель; протокол v1.2: токен фрейма, `region`, slot-режим, `context`; `inject`/`effect`-lifecycle, capability-ассеты, host-действия, перезагрузка всех фреймов плагина; экспериментальный `dsh.client.runtime` — половина приносит свою модульную таблицу/UI-стек, включая React; PoC тира B `dsh.client.host_modules` — половина опционально берёт React 18.3.1 и таблицу модулей от хоста одним immutable-артефактом `/v1/runtime/react-18.3.1.js`; module-table бандлы резолвят `require` из `window.__DSHR_MODULES__`; C-lite (раунд 79): при host React `ctx.slots.register(entry, Component)` — React-компонент (один React-root на кадр, перерисовка на host-state, error boundary), props с `useTheme()`/`useSession()`/`useSessions()`/`useHostState()`/`wide`, `ctx.get('sessions')` (подмножество референсного сервиса), частичный design-system шим `@deepseek-ai/dsh-client-ui-primitives`; раунд 80: сервисы моста `ctx.get('slots')` (реестр + `entries(key)`), `ctx.get('locale')` и `ctx.inject(deps, callback)` для опциональных интеграций, диагностика `window.__DSHR_LAST_ERROR__`, conformance n=2 (`@kt11/dsh-deepseek-balance@0.2.1` + `dsh-remote@1.1.2`); раунд 81: React-контракт моста — принимаются типы `React.memo`/`forwardRef`, keyed-регистрации без `id` получают уникальный id `<slot>:<key>`, `ctx.get('slots').entries(key)` отдаёт `component`/`options`, `ctx.locale.register(ns, dicts)` + `t()` в props, `ctx.interval`/`ctx.timeout` снимаются вместе с фреймом, primitives шим расширен `JsonBlock`/`MarkdownText` (всего 7 компонентов), conformance n=4 (к прежним двум добавлены `@ruoyang/dsh-plugin-deepseek-balance@1.0.2`, `dsh-genui@0.3.3`); раунд 82: conformance n=6 — `dsh-session-superagent@0.10.12` коммитит composer dock (`conversation.composer.dock`), а крупный multi-region `dsh-mnemon@0.5.46` активируется, публикует слоты и сверяет свои shadow-регистрации через `ctx.slots.subscribe`/`getVersion`/`entriesOfSlot` (референсная поверхность слотов раунда 81 выдержала реальный плагин); раунд 83: conformance n=9 — цель ADR-013 §6 (8–12 реальных плагинов) достигнута, добавлены `@99galaxy/dsh-plugin-market@1.2.2` (`sidebar.panellist` → регион `sidebar.footer`), `dsh-plugin-codegraph-panel@0.6.5` (`settings.section`; upstream `immediately: true`) и `@goodandready/dsh-plugin-notify@0.4.9` (`plugins.item`/`plugins.row.config` → регион `plugins.detail`), все три активируются и публикуют слоты без ошибок, а eval-based бандлы (`@michengai/dsh-archive-manager`) требуют `'unsafe-eval'` в CSP (сознательно не включён), `@michengai/dsh-skills-manager` падает на инициализации — оба вне набора; полного design-system, React-контекста шелла и SlotMap-семантики нет — тир C-full, решение владельца); Replica-паритет клиентской половины 98.1/100 (must 13/13); WASM: hot reload (fingerprint watcher и `/v1/plugins/reload`) | ✅ |
| Плагин-маркет/экосистема (fabric/market) | CLI `plugin npm search/info` + UI-панель Plugins; серверный каталог с категориями/тегами/поиском и карточкой пакета (`/v1/plugins/catalog*`); установка через CLI/UI; внешней общей базы отзывов нет | ⚠️ |
| Субагенты, teams, workflows, schedules, jobs (durable), LSP, Office, CUA | есть | ✅ |
| Windows sandbox/network | AppContainer backend есть, включается `DSHR_WINDOWS_SANDBOX`; fail-closed; CI гоняет argv- и runtime-тест записи внутри/снаружи workspace | ⏸ |
| i18n ru/en | TUI/CLI, справка и рантайм-сообщения ru/en; тексты ошибок en (по плану), `--json` не зависит от языка | ✅ |
| OTel, SDK, rate-limit, OpenAPI | есть | ✅ |

## Дистрибутивы и CI

| Пункт | dshr | Статус |
|---|---|---|
| Бинарные архивы 5 таргетов, SHA256SUMS, Ed25519, provenance | есть | ✅ |
| Homebrew: бинарная формула | binary + бутылки; `bottles-ghcr` публикует OCI в GHCR; реальный push и анонимный pull манифеста подтверждены в Actions | ✅ |
| Reproducible builds | архивы детерминированы; бинарь воспроизводим при том же toolchain с remap/SOURCE_DATE_EPOCH (проверено локально); cross-Linux может отличаться | ✅ |
| Desktop в release CI | macOS `.app`; Linux/Windows desktop-сборки и упаковка проверены в Actions (tar+deb, zip) | ✅ |
| Windows/Linux desktop packages | Linux tar.gz + `.deb`, Windows zip; сборка/упаковка проверены на реальных раннерах; NSIS-инсталлятора нет | ⚠️ |

## Приоритеты паритета

P0 закрыто: трей/close-to-tray, профили, фиксированный порт/LAN, drag-and-drop папки,
диагностика, `apply_patch`, `ask_user` (включая ACP), каналы и установка обновлений,
recovery-окно с профилями/плагинами, first-run home. Осталось: прогон notarization
с секретами Apple.
P1: plugin market — каталог с категориями/поиском и карточкой пакета сделан; GUI-менеджер
плагинов есть; внешней (общей) базы отзывов нет — отзывы живут в home.
P2: прогон brew/desktop CI на теге, React-совместимость клиентских плагинов
(регионы и словарь имён — раунд 76, enabler `dsh.client.runtime` — раунд 77,
PoC тира B `dsh.client.host_modules` — раунд 78, C-lite — раунд 79: при host
React `ctx.slots.register(entry, Component)` рендерится как React-компонент с
host-hooks (`useTheme`/`useSession`/`useSessions`/`useHostState`/`wide`) и
`ctx.get('sessions')`, есть частичный design-system шим и первый реальный
DSH-плагин в conformance; раунд 80: сервисы `ctx.get('slots')`/`ctx.get('locale')`
и `ctx.inject`, диагностика `__DSHR_LAST_ERROR__`, conformance n=2
(`dsh-remote@1.1.2`); раунд 81: типы `React.memo`/`forwardRef`, keyed-ids,
`ctx.locale.register` + `t()`, `ctx.interval`/`ctx.timeout`, primitives шим из
7 компонентов и conformance n=4 (к прежним двум добавлены
`@ruoyang/dsh-plugin-deepseek-balance@1.0.2`, `dsh-genui@0.3.3`), раунд 82 —
conformance n=6 (`dsh-session-superagent@0.10.12` — composer dock,
`dsh-mnemon@0.5.46` — multi-region и референсная поверхность слотов), раунд 83 —
conformance **n=9**: цель ADR-013 §6 (8–12 реальных плагинов) достигнута, к
прежним шести добавлены `@99galaxy/dsh-plugin-market@1.2.2` (сайдбар),
`dsh-plugin-codegraph-panel@0.6.5` (панель настроек) и
`@goodandready/dsh-plugin-notify@0.4.9` (строки plugins-страницы); полной
React-совместимости нет — полный
design-system, React-контекст шелла и SlotMap-семантика ждут решения по
тиру C-full [ADR-040](adr/040-react-client-half.md), это вопрос владельца, а не
незакрытая C-lite-работа), приватные terminal
shim'ы.

## Проверка (15.11.2026)

- `cargo build --release --workspace --locked` и `cargo deny check` — чисто.
- `cargo test --workspace`: 361 passed + 3 ignored; desktop 21 passed (итого 382).
- `scripts/check-reproducible.sh`: два независимых билда дали одинаковый sha256
  (`abdca627…`).
- `package-desktop.sh` + `package-dmg.sh`: `.app` проходит `codesign --verify`,
  DMG собран (UDZO); установочный/архивный dry-run проверен ранее.
- Headless Chrome: light-тема, версия, Plugins; `/v1/tools`, `/v1/plugins/catalog`,
  `/v1/plugins/ratings` отвечают.
- Браузерный e2e (`scripts/web-e2e.py`, stdlib CDP): «New session» → промпт →
  модалка approval для `bash` → Allow → выполнение → стрим ответа → карточка
  инструмента → файл реально записан → сессия в журнале; 16/16 проверок
  (добавлены второй ход с `edit`/diff и загрузка/сохранение настроек провайдера),
  джоба `web-e2e` в CI.
- Linux/Windows desktop: [run 43591784883](https://github.com/maximusmakarov/deepseek_harness_rust/actions/runs/43591784883)
  — обе сборки и упаковка успешны; `.deb` проверен `dpkg-deb --info`.
- Cordis hot reload проверен на живом `serve`: правка entry перезагружает плагин
  (генерация 1→2), добавление/удаление плагина в `profile.yaml` стартует/останавливает
  sidecar, `POST /v1/plugins/reload` отвечает, `GET /v1/plugins/status` отражает
  состояние; `/v1/plugins/installed` следует за супервизором.
- Клиентская половина: `python3 scripts/plugin-smoke.py` — 72
  проверки в headless Chrome: eager registry-кадры всех половин, монтаж слота в
  регион по таблице `slot key → регион` (17 регионов), fallback незнакомого имени
  в панель, `context` повторяющихся регионов, адресный `resize`, live-тема,
  `ctx.action` (`close`/`reload`/`compose`/`notify` и отказ для неизвестного),
  открытие/закрытие диалогов монтирует/снимает их регионы, многофайловый
  capability-бандл и module-table бандл рендерятся, reload поднимает версию всех
  фреймов плагина; plugin-shipped runtime (`dsh.client.runtime`) обнаруживается
  в `/v1/plugins/clients`, host-страница исполняет его до бандла, а
  module-table бандл резолвит `require` из `window.__DSHR_MODULES__` и рендерит
  свой mini-framework; PoC тира B: `host_modules` виден в `/v1/plugins/clients`,
  `/v1/runtime/react-18.3.1.js` отдаётся с immutable-кэшем, opt-in host-страница
  ссылается на рантайм и не встраивает React, а smoke-фикстура `hosted` без React
  в бандле рендерит реальный интерактивный React-компонент (`useState`) из общего
  рантайма (React 18.3.1 PoC раунда 77 — отдельно, вручную); C-lite (раунд 79):
  React-компонент слота рендерится через `ctx.slots.register(entry, Component)`,
  hooks-фикстура получает `useTheme()`/`useSession()`/`useSessions()`/`wide` из
  `host-state` и `ctx.get('sessions')`, ui-kit-фикстура берёт
  `@deepseek-ai/dsh-client-ui-primitives` из общего рантайма, а реальный плагин
  `@kt11/dsh-deepseek-balance@0.2.1` рендерит карточку в
  `sidebar.footer.action`, вставляет свой CSS и открывает popup; раунд 80:
  `dsh-remote@1.1.2` обнаруживается в `/v1/plugins/clients` (с
  примитивами в `host_modules`), его registry-кадр активируется без ошибок при
  отсутствии опциональных сервисов, а React-слот `conversation.session.header.actions`
  монтируется в регион `conversation.header`; раунд 81 (C-lite n=4):
  `@ruoyang/dsh-plugin-deepseek-balance@1.0.2` рендерит плавающий бейдж в
  `shell.overlay` (`t()` из `ctx.locale.register`, `ctx.interval`), а
  `dsh-genui@0.3.3` — тумблер GenUI в `conversation.input.left` с keyed-слотом
  `conversation.chat.node` (принимаются типы `React.memo`/`forwardRef`; общий
  рантайм-артефакт — 149 442 байта с шимом из 7 примитивов); раунд 82 (C-lite
  n=6): `dsh-session-superagent@0.10.12` монтирует composer dock в регион
  `composer.actions`, а `dsh-mnemon@0.5.46` (крупный multi-region) активируется,
  публикует слоты и сверяет собственные shadow-регистрации через
  `ctx.slots.subscribe`/`getVersion`/`entriesOfSlot`; раунд 83 (C-lite n=9):
  `dsh-plugin-market@1.2.2` монтирует кадр в регион `sidebar.footer`,
  `dsh-plugin-codegraph-panel@0.6.5` публикует панель `settings.section`, а
  `@goodandready/dsh-plugin-notify@0.4.9` — строки `plugins.item`/
  `plugins.row.config`; все три активируются и публикуют слоты; рантайм-артефакт
  не менялся — 149 442 байта.
- Replica-оценка клиентской половины (`replica-diff`): 98.1/100, must 13/13 —
  `docs/replica/client-half-parity.md` (три добавленные could-строки про
  plugin-shipped runtime, shared host module table и C-lite React-компоненты —
  dshr-дополнения вне скоринга; раунд 80 расширил строку C-lite сервисами моста
  и conformance n=2, раунд 81 — доработками React-контракта и conformance n=4,
  раунд 82 — conformance n=6, раунд 83 — conformance n=9; новых скоринговых
  строк не добавлено; арифметика
  та же:
  must 13 × 3 = 39, should 6 × 2 = 12, should partial 1 × 2 × ½ = 1,
  could 1 × 1 = 1 → 39 + 12 + 1 + 1 = 53 из 54 = 98.15 → 98.1).
- Формализация: `./formal/run_tlc.sh` — 3 модели / 5 конфигов TLC без нарушений
  (`CordisSupervisor` safety+liveness, `ClientAssets`, `ApprovalGate` safety+liveness);
  модели и абстракции — `docs/formal/README.md`.
- Каталог тест-кейсов `docs/testing/test-cases.md`: ~150 кейсов с ID, уровнями,
  привязкой к инвариантам TLA+ и статусами; baseline покрытия —
  `cargo llvm-cov --workspace --summary-only`: 72.9% строк (66.9% функций).

## Итог 83 раунда

Закрыты: desktop (трей, профили/композиция бандлов, порт/LAN, папка/drag-and-drop,
терминал с шимами, обновления stable/beta + установка, recovery-окно с профилями
и плагинами, режимы окна/материал, тема), plugins (npm/Cordis install/update/
enable/disable, Web-менеджер, featured-каталог, оценки, Cordis audit/fs-гранты,
клиентская половина: региональный монтаж слотов, `dsh.client.runtime`, тир B
`dsh.client.host_modules` и C-lite — React-компоненты слотов, сервисы моста,
React-контракт, conformance n=4, раунд 82 — n=6 (`dsh-session-superagent`,
`dsh-mnemon`) и раунд 83 — n=9: цель ADR-013 §6 (8–12 реальных плагинов)
достигнута (`dsh-plugin-market`, `dsh-plugin-codegraph-panel`,
`dsh-plugin-notify`); C-full — решение владельца), host (`ask_user` в TUI/Web/ACP,
`apply_patch`,
LLM-заголовки везде, elicitation stdio/HTTP/GET-SSE, DSH import, persistent bash,
TUI diff), дистрибутивы (детерминированные архивы, reproducible-бинарь, bottles,
DMG, Linux/Windows desktop-джобы, notarization-тулинг).

## Известные блокеры

- **GHCR-бутылки:** нет tap-репозитория/учётных данных пакетов; бутылки публикуются
  как GitHub release-ассеты.
- **Windows sandbox:** нужен реальный Windows-хост; локально проверяется только
  компиляция, ignored-тест вынесен в Windows CI.
- **Notarization:** нужны Apple Developer ID, сертификат и app-specific password.
- **Desktop CI/brew:** release-джобы выполняются только на теге; Linux/Windows
  desktop-сборка и упаковка отдельно проверены в Actions, локально — dry-run.

Каждый срез: код + тест/сборка + обновление этой матрицы.
