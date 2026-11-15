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
| Плагины: WASM core/component, Ed25519, Cordis-sidecar | Cordis: audit-log, fs-гранты, hot reload (watcher + endpoint) и клиентская половина (eager registry-кадр на каждую половину + отдельный sandboxed iframe на слот, каталог DSH 0.2.2-rc.2 (с раунда 91 — 122 слота с `kind`/`scope`/standard-kit) и legacy `panel`/`message.action` (всего 124 ключа) разложены по 17 регионам шелла, незнакомое имя — fallback в панель; протокол v1.2: токен фрейма, `region`, slot-режим, `context`; `inject`/`effect`-lifecycle, capability-ассеты, host-действия, перезагрузка всех фреймов плагина; экспериментальный `dsh.client.runtime` — половина приносит свою модульную таблицу/UI-стек, включая React; PoC тира B `dsh.client.host_modules` — половина опционально берёт React 18.3.1 и таблицу модулей от хоста одним immutable-артефактом `/v1/runtime/react-18.3.1.js`; module-table бандлы резолвят `require` из `window.__DSHR_MODULES__`; C-lite (раунд 79): при host React `ctx.slots.register(entry, Component)` — React-компонент (один React-root на кадр, перерисовка на host-state, error boundary), props с `useTheme()`/`useSession()`/`useSessions()`/`useHostState()`/`wide`, `ctx.get('sessions')` (подмножество референсного сервиса), тогда частичный design-system шим `@deepseek-ai/dsh-client-ui-primitives`; раунд 80: сервисы моста `ctx.get('slots')` (реестр + `entries(key)`), `ctx.get('locale')` и `ctx.inject(deps, callback)` для опциональных интеграций, диагностика `window.__DSHR_LAST_ERROR__`, conformance n=2 (`@kt11/dsh-deepseek-balance@0.2.1` + `dsh-remote@1.1.2`); раунд 81: React-контракт моста — принимаются типы `React.memo`/`forwardRef`, keyed-регистрации без `id` получают уникальный id `<slot>:<key>`, `ctx.get('slots').entries(key)` отдаёт `component`/`options`, `ctx.locale.register(ns, dicts)` + `t()` в props, `ctx.interval`/`ctx.timeout` снимаются вместе с фреймом, primitives шим расширен `JsonBlock`/`MarkdownText` (всего 7 компонентов), conformance n=4 (к прежним двум добавлены `@ruoyang/dsh-plugin-deepseek-balance@1.0.2`, `dsh-genui@0.3.3`); раунд 82: conformance n=6 — `dsh-session-superagent@0.10.12` коммитит composer dock (`conversation.composer.dock`), а крупный multi-region `dsh-mnemon@0.5.46` активируется, публикует слоты и сверяет свои shadow-регистрации через `ctx.slots.subscribe`/`getVersion`/`entriesOfSlot` (референсная поверхность слотов раунда 81 выдержала реальный плагин); раунд 83: conformance n=9 — цель ADR-013 §6 (8–12 реальных плагинов) достигнута, добавлены `@99galaxy/dsh-plugin-market@1.2.2` (`sidebar.panellist` → регион `sidebar.footer`), `dsh-plugin-codegraph-panel@0.6.5` (`settings.section`; upstream `immediately: true`) и `@goodandready/dsh-plugin-notify@0.4.9` (`plugins.item`/`plugins.row.config` → регион `plugins.detail`), все три активируются и публикуют слоты без ошибок, а eval-based бандлы (`@michengai/dsh-archive-manager`) требуют `'unsafe-eval'` в CSP (сознательно не включён), `@michengai/dsh-skills-manager` падает на инициализации — оба вне набора; раунд 84: **C1** — host HTTP-маршруты плагинов работают end-to-end (loopback-сервер sidecar с per-process токеном, allowlist-прокси dshr-сервера (exact-маршруты; раунд 86 добавил prefix), `connect-src` на origin сервера для кадров с `host_modules`), реальная host-половина `@kt11/dsh-deepseek-balance` отвечает своим JSON через кадр плагина; раунд 85 — **C2**: design-system шим вырос с 7 до 30+ экспортов по фактическим импортам (компоненты `Input`/`Switch`/`Checkbox`/`Tag`/`Pill`/`Menu`/`MenuSurface`/`Modal`/`Toast`/`SegmentedControl`/`SegmentedTabs`/`DisclosureRow`/`PathLabel`/`TextShimmer`, именованные `Icon*` с Proxy-fallback, `writeClipboard`, хуки `useAnchoredPosition`/`useDismissOnOutsidePointer`), conformance **n=11** (`dsh-plugin-debugging@0.8.1`, `dsh-plugin-studio@0.3.1` активируются без ошибок, их settings-UI оставался инертным без host-сервисов `pluginManager`/settings до C1.2); раунд 86 — **хардненинг C1**: prefix-маршруты (`kind: "exact"`/`"prefix"`, exact выигрывает, среди prefix — самый длинный), per-plugin `routes` в `GET /v1/plugins/status`, фильтр MCP-псевдо-инструментов (`list_resources`/`read_resource`/`list_prompts`/`get_prompt`) из реестра инструментов Cordis-плагина и loopback-only политика plugin-маршрутов (403 при bind вне loopback без `--allow-lan-plugin-routes`); раунд 87 — **C1.1**: sidecar получает DSH-shaped `sessions`/`sessionProjections` из внутреннего снимка сервера (`/v1/internal/session-snapshot`, токен в `<home>/cordis/host.json`, обновление раз в 5 с), поэтому usage-маршруты реальных плагинов считают статистику по сессиям dshr (проекции упрощены: `llmMs`=0, без истории тредов; содержимое сессий намеренно не мостится); раунд 89 — **C1.2**: seat-`inject` в регистрациях слотов, Cordis-генераторы в `slots.inject`/`ctx.inject`/`ctx.effect` и host-сервис `configForms` (in-memory writable-форма в кадре, файловая — у host-половин), поэтому settings-UI `dsh-plugin-studio` (реальные вкладки в Settings) и `dsh-plugin-debugging` (карточка в Plugins с рабочим переключателем и Save) реально рендерится, а не молча инертен; раунд 91 — **C1.3**: слот-контракт переведён на сгенерированный каталог DSH (122 слота с `kind`/`scope`/framework standard-kit, `window.__DSHR_SLOT_META__`) и референсные правила `SlotCore` — undeclared/keyed/chain/duplicate-регистрации, shadowing-winners, children-декларации, deferred inject, standard-kit + `ownerProps`, `ctx.slots.spec(key)`; раунд 92 — **C1.4**: host-сервисы `webServer`/`ctx.baseUrl`/`configForms` по reference-контракту (exact/prefix-таблицы независимы, дубликат `(kind, path)` бросает, path-валидация — абсолютный pathname без trailing slash/query/hash, `getOrigin()`/`port`/`protocol` отдают origin dshr-сервера из host-bridge, `file://`-baseUrl, snapshot `{status, value, base, user, revision: number, writable}` с `mutate`/`set`/`unset` и числовым revision-fence; `registerFallback`/`registerUpgrade` в sidecar не поддержаны); раунд 93 — **C2.1**: атомы design-system (`Button`/`Input`/`Switch`/`Checkbox`/`Tag`/`Pill`/`SegmentedControl`/`SegmentedTabs`/`DisclosureRow`/`TextShimmer`/`StateDot`/`PathLabel`/`Toast`/`Modal`/`Menu`/`MenuSurface`/`Tooltip`) и все **204** `Icon*` портированы из reference-исходников (`deepseek-ai/deepseek-harness` @ `39163dd`, `packages/client/ui-primitives/src`) на `--dsw-alias-*` токенах, поэтому `@deepseek-ai/dsh-client-ui-primitives` — reference-порт, а не независимый шим (общий рантайм-артефакт на конец раунда 93 — 375 894 байта; раунд 94 — **C2.2**: полный reference-порт `Tooltip` (`Tooltip.tsx`: label-резолвер, `delayMs`/`focusDelayMs`, `openOnClick`-pinning, `onlyWhenClipped`/`maxWidth`, portal, nested-suppression, `aria-describedby`, pointer-modality, forward ref; `tooltip-placement.ts` — requested side → opposite → perpendicular, viewport-clamp, overflow-fallback; `input-modality.ts` и `ShortcutKeys` портированы и экспортированы), общий рантайм-артефакт — 387 613 байт; раунд 95 — **C2.3**: ядро `Menu`/`MenuItemButton` портировано по `Menu.tsx`/`MenuRow.tsx` (data/компонентные ряды, separators/labels, reserve-icon, selection `check`/`fill`, `danger`/`current`, disabled-reason через reference `Tooltip`, `ShortcutKeys`, footer-группа, `children`-ряды, one-level submenu, клавиатурный walk, outside/iframe-blur dismissal, `usePointerGrace`, `focusWithoutRing`, portal-позиционирование, dense/compact; `MENU_TRANSITION_MS`/`isMobileDevice` экспортированы), общий рантайм-артефакт — 405 323 байта; раунд 96 — **C2.4**: reference submenu page-layout (`menu-submenu-position.ts`: side-карточка, если помещается сбоку, иначе single-page с non-selecting back-row `data-menu-back`, Escape/ArrowLeft с фокусом на родителе), 200-мс hover grace между карточками и `MenuGroup`/`observeStickyMenuGroups` (sticky-заголовки через IntersectionObserver/ResizeObserver), общий рантайм-артефакт — 415 972 байта; раунд 97 — **C2.5**: `JsonBlock` портирован из reference (collapsible, `label`/`payload`/`defaultOpen`/`truncatedLabel`, cap 20 000 символов) и `MarkdownText` получил reference-поверхность props плюс dependency-free GFM-подмножество, общий рантайм-артефакт — 424 801 байт), упрощённой остаётся только детальная `useMenuSubmenu`-механика focus-transition/page-anchor, а upstream markdown-стек (micromark/mdast, shiki, KaTeX, footnotes, streaming) не входит); остатки C1 — SSE/WebSocket/стриминг, отсутствие аутентификации на plugin-маршрутах (риск снижен loopback-only политикой); полного design-system (C2.1 закрыл атомы и иконки, C2.2 — полный Tooltip, C2.3 — ядро Menu, C2.4 — submenu page-layout/hover grace/MenuGroup, C2.5 — reference JsonBlock и GFM-подмножество MarkdownText, а раунды 98–101 — submenu-механику C2.6 (side-card/single-page по измеренной ширине, `fixed`-позиционирование с flip/clamp, `activeList` для клавиатуры, Escape/ArrowLeft на родителя, 200-мс hover grace, `data-menu-back`), store-seats C3.1 (`@deepseek-ai/dsh-client-store` в module-таблице: `createSnapshotStore`/`defineStore`/`shallowEqual`; `store: handle\|factory`, framework-owned instance на scope-key, `useStore(selector, eq)` + baked `actions`, inject-аргументы `(sessionId, actions)`), fallback-маршруты C1.5 (`webServer.registerFallback`, одно место, проксирование не-зарезервированных несовпавших путей) и ownerProps-plumbing (`?ownerProps={...}` → `frame.ownerProps` → props после store/seat share), общий рантайм-артефакт — 430 139 байт; `registerUpgrade` (WebSocket) остаётся явно не поддержанным, ownerProps от реальных shell-сайтов — долг; остаются upstream markdown-стек и renderer-семантика), React-контекста шелла и SlotMap-остатков (chain select-диспетчеризация, hoc-вложение; store seats закрыты в C3.1, ownerProps — только plumbing) нет — тир C-full, решение владельца: по [ADR-041](adr/041-c-full-decision.md) отложен, дальше точечные инкременты); Replica-паритет клиентской половины 98.1/100 (must 13/13); WASM: hot reload (fingerprint watcher и `/v1/plugins/reload`) | ✅ |
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
`@goodandready/dsh-plugin-notify@0.4.9` (строки plugins-страницы); раунд 84 —
**C1**: host HTTP-маршруты плагинов работают частично (sidecar-сервер +
allowlist-прокси, `connect-src` origin для opt-in кадров, реальная host-половина
balance-плагина), с записанными остатками (сервисы `sessions`/
`sessionProjections`, стриминг, отсутствие аутентификации на plugin-маршрутах;
exact-only и LAN-экспозиция закрыты в раунде 86),
раунд 85 — **C2**: design-system шим вырос с 7 до 30+ экспортов (новые
компоненты, именованные `Icon*` с Proxy-fallback, хуки
`useAnchoredPosition`/`useDismissOnOutsidePointer`, `writeClipboard`), а
conformance доведён до **n=11** (`dsh-plugin-debugging@0.8.1`,
`dsh-plugin-studio@0.3.1` активируются без ошибок, их settings-UI оставался
несмонтированным без host-сервисов `pluginManager`/settings до C1.2), а раунд
86 —
**хардненинг C1**: prefix-маршруты (`kind: "exact"`/`"prefix"`, exact
выигрывает, среди prefix — самый длинный), per-plugin `routes` в
`GET /v1/plugins/status`, фильтр MCP-псевдо-инструментов и loopback-only
политика plugin-маршрутов с opt-in `--allow-lan-plugin-routes`; раунд 87 —
**C1.1**: мост `sessions`/`sessionProjections` (DSH-shaped сервисы из
внутреннего снимка сервера, задержка ≤ 5 с, упрощённые проекции, содержимое
сессий не мостится) — usage-маршруты реального balance-плагина считают
статистику по сессиям dshr; раунд 89 — **C1.2**: seat-`inject`, генераторы
Cordis и `configForms` (browser-форма in-memory, файловая — у host-половин),
так что settings-UI `dsh-plugin-studio`/`dsh-plugin-debugging` рендерится,
раунд 91 — **C1.3**: слот-контракт по референсному каталогу DSH (122 слота с
kind/scope/standard-kit, правила `SlotCore`: undeclared/keyed/chain/duplicate,
shadowing-winners, children-декларации, deferred inject, standard-kit +
`ownerProps`); раунд 92 — **C1.4**: host-сервисы `webServer`/`ctx.baseUrl`/`configForms` выровнены по reference-контракту (правила exact/prefix-маршрутов и duplicate-throw, origin dshr-сервера, `file://`-baseUrl, snapshot с числовой ревизией и `set`/`unset`; `registerFallback`/`registerUpgrade` в sidecar не поддержаны); раунд 93 — **C2.1**: атомы design-system и все 204 иконки портированы из reference-исходников (`deepseek-ai/deepseek-harness` @ `39163dd`, `packages/client/ui-primitives/src`), общий рантайм-артефакт на конец раунда — 375 894 байта, а раунд 94 — **C2.2**: полный reference-порт `Tooltip` (placement-engine, `ShortcutKeys`, `input-modality`; артефакт 387 613 байт), а раунд 95 — **C2.3**: ядро `Menu`/`MenuItemButton` по `Menu.tsx`/`MenuRow.tsx` (ряды/selection/aria, reserve-icon, disabled-reason через Tooltip, `ShortcutKeys`, footer/`children`-ряды, one-level submenu, клавиатурный walk, outside/iframe-blur dismissal, `usePointerGrace`/`focusWithoutRing`, portal-позиционирование, dense/compact; артефакт 405 323 байта), а раунд 96 — **C2.4**: reference submenu page-layout (side-карточка, если помещается сбоку, иначе single-page с non-selecting back-row `data-menu-back` и Escape/ArrowLeft на родителя), 200-мс hover grace между карточками и `MenuGroup`/`observeStickyMenuGroups` (sticky-заголовки через IntersectionObserver/ResizeObserver; артефакт 415 972 байта), а раунд 97 — **C2.5**: reference `JsonBlock` (collapsible, `label`/`payload`/`defaultOpen`/`truncatedLabel`, cap 20 000 символов с хвостом «truncated», безопасный `JSON.stringify` с fallback) и `MarkdownText` с reference-поверхностью props (`text`/`streaming`/`labels`/`fileMentions`/`pathImages`/`variant`/`omitHtml`/`textInteractionSource`) и dependency-free GFM-подмножеством (заголовки, списки, fenced code, blockquote, hr, inline-стили, ссылки/изображения только `http(s)`/`mailto`, raw HTML не рендерится; артефакт 424 801 байт), а раунды 98–101 — C2.6 submenu-механика (side-card/single-page по измеренной ширине, `fixed` flip/clamp, `activeList`, Escape/ArrowLeft на родителя, 200-мс hover grace), C3.1 store-seats (`@deepseek-ai/dsh-client-store` в module-таблице, `store: handle|factory`, instance на scope-key, `useStore`/`actions`, inject-аргументы `(sessionId, actions)`), C1.5 fallback-маршруты (`webServer.registerFallback`) и ownerProps-plumbing (`?ownerProps={...}` → `frame.ownerProps` → props; артефакт 430 139 байт), поэтому остаются upstream markdown-стек (micromark/mdast, shiki, KaTeX, footnotes, инкрементальный streaming), renderer-семантика, `registerUpgrade` (WebSocket) и ownerProps от реальных shell-сайтов; полной React-совместимости нет — полный
design-system, React-контекст шелла и SlotMap-остатки (chain select-диспетчеризация,
hoc-вложение; store seats закрыты в C3.1, ownerProps — только plumbing) ждут решения по
тиру C-full [ADR-040](adr/040-react-client-half.md), это вопрос владельца, а не
незакрытая C-lite-работа), приватные terminal
shim'ы.

## Проверка (15.11.2026)

- `cargo build --release --workspace --locked` и `cargo deny check` — чисто.
- `cargo test --workspace`: 366 passed + 3 ignored; desktop 21 passed (итого 387).
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
- Клиентская половина: `python3 scripts/plugin-smoke.py` — 82
  проверок в headless Chrome: eager registry-кадры всех половин, монтаж слота в
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
  `plugins.row.config`; все три активируются и публикуют слоты; раунд 84 (C1):
  реальная host-половина `@kt11/dsh-deepseek-balance` больше не застаблена, и
  из кадра плагина проверено, что её маршруты проксируются: `/dsh-balance/deepseek`
  → `500 {ok:false, code:"not-configured"}` (без ключа), `/dsh-usage/sessions?tz=UTC`
  → `200` с JSON и массивом `sessions`, неизвестный путь → dshr 404; серверный
  integration test `proxies_plugin_host_routes` покрывает GET/query/POST/OPTIONS/
  404/reserved-префиксы и снятие маршрутов при остановке плагина, а
  `plain_frames_stay_network_free` — что кадры без `host_modules` остаются с
  `connect-src 'none'`; раунд 85 (C2): дерево всех смоделированных примитивов
  рендерится в host-React кадре (12 текстовых маркеров, портальные
  Menu/Modal/Toast), а реальные вклады `dsh-plugin-notify`/`dsh-mnemon`
  монтируются в диалоге Plugins без `__DSHR_LAST_ERROR__`; conformance n=11 —
  `dsh-plugin-debugging@0.8.1` и `dsh-plugin-studio@0.3.1` активируются без
  ошибок, их settings-UI оставался инертным без host-сервисов
  `pluginManager`/settings до C1.2; общий рантайм-артефакт раунда 85 вырос до 176 725 байт (React + ReactDOM UMD + `bootstrap.js`
  + `primitives.js` с 30+ экспортами); раунд 86 (хардненинг C1): server
  integration tests покрывают prefix-маршруты (`/probe-prefix`, `/probe-prefix/a/b`),
  per-plugin `routes` в `GET /v1/plugins/status`, отсутствие MCP-псевдо-инструментов
  в `tools` и loopback-only политику (`0.0.0.0:0` без флага → 403, с
  `allow_plugin_routes_lan` → 200), smoke раунда 86 остался 79/79 (проверки не
  менялись); раунд 87 (C1.1) добавил проверку моста: smoke опрашивает
  `/dsh-usage/sessions?tz=UTC` из кадра реального balance-плагина и требует
  `sessions` ≥ 1 и totals — всего 80/80; раунд 89 (C1.2) добавил две проверки
  рендера реальных settings-UI: кадр `dsh-plugin-studio` в
  `settings.plugins.tab` внутри диалога Settings рендерит вкладки через
  seat-`inject` (и его собственный CSS), а карточка `dsh-plugin-debugging` в
  `plugins.bundle.config` рендерится, переключается и сохраняется через
  `configForms` — всего 82/82. Раунд 91 (C1.3) новых smoke-проверок не
  добавлял (набор остаётся 82/82; synthetic-фикстуры дополнены обязательным
  `key` для keyed-слотов, словарь — 124 ключа включая 2 legacy), а bridge
  harness вырос до **98** (+8: undeclared/keyed/chain/duplicate throws, single
  shadowing, catalog standard kit, ownerProps, plain values). Раунд 92 (C1.4)
  добавил 4 проверки контракта `configForms` (форма-контракт, стабильный
  snapshot, set с числовой ревизией, unset) — bridge harness **102**; smoke
  остался 82/82, а server `plugins_live` расширен правилами duplicate-route и
  trailing-slash. Раунд 93 (C2.1) новых проверок не добавлял: smoke остаётся
  82/82 (два зелёных прогона подряд), но C2-probe усилен до структурных
  проверок reference-контрактов — `role=switch` с `aria-label`, `data-tone`,
  `tablist`-табы, число `svg` (иконки инлайнены, а не спрайтом),
  `label.dshr-checkbox`, `textContent` у `[data-path-label]`; synthetic-фикстуры
  приведены к reference-контрактам (`StateDot state:"done"`, `Tooltip` с
  element-child), bridge harness 102, cordis lib 26, server `plugins_live` 16,
  workspace 366 passed + 3 ignored (desktop 21, итого 387), web-e2e 16/16;
  общий рантайм-артефакт на конец раунда 93 вырос до **375 894 байт** (React + ReactDOM UMD + `bootstrap.js` + портированный `primitives.js` с атомами и 204 иконками), раунд 94 (C2.2) — полный reference-порт `Tooltip` (placement-engine, `ShortcutKeys`, `input-modality`; smoke остаётся 82/82) — довёл его до **387 613 байт**, а раунд 95 (C2.3) — ядро `Menu`/`MenuItemButton` по reference-ядру (smoke остаётся 82/82) — до **405 323 байт**, а раунд 96 (C2.4) — reference submenu page-layout, 200-мс hover grace и `MenuGroup`/`observeStickyMenuGroups` (smoke остаётся 82/82; C2-probe рендерит `MenuGroup` и submenu-родителя и требует `menuRows >= 3`) — до **415 972 байт**, а раунд 97 (C2.5) — reference `JsonBlock` и dependency-free GFM-подмножество `MarkdownText` (smoke остаётся 82/82; C2-probe рендерит `MarkdownText` и `JsonBlock` и проверяет DOM/текст) — до **424 801 байта**, а раунды 98–101 (C2.6 submenu-механика, C3.1 store-seats, C1.5 fallback-маршруты и ownerProps-plumbing; smoke остаётся 82/82 с добавленным store-engine check, bridge harness 105, server `plugins_live` 17) — до **430 139 байт**.
- Server `plugins_live`: 17 (+2: `internal_session_snapshot_feeds_the_host_bridge` —
  снимок сессий с usage-полями, 401 при неверном токене и 404 без него;
  `proxies_the_plugin_fallback_seat` — fallback-маршрут отвечает на
  не-зарезервированный путь, reserved-путь в fallback не уходит; frame-config
  тест проверяет `ownerProps` и отклонение не-объекта;
  `plugin_routes_are_loopback_only_unless_opted_in`; `proxies_plugin_host_routes`
  расширен prefix/status/meta-tools, а раунд 92 (C1.4) — ещё и правилами
  duplicate-route (`/probe-route` бросает) и отклонения trailing-slash).
- Replica-оценка клиентской половины (`replica-diff`): 98.1/100, must 13/13 —
  `docs/replica/client-half-parity.md` (три добавленные could-строки про
  plugin-shipped runtime, shared host module table и C-lite React-компоненты —
  dshr-дополнения вне скоринга; раунд 80 расширил строку C-lite сервисами моста
  и conformance n=2, раунд 81 — доработками React-контракта и conformance n=4,
  раунд 82 — conformance n=6, раунд 83 — conformance n=9, раунд 84 — C1
  host-маршрутами, раунд 85 — C2 (30+ экспортов design-system шима и
  conformance n=11), раунд 86 — хардненингом C1 (prefix-маршруты, `routes` в
  статусе, фильтр MCP-псевдо-инструментов, loopback-only политика), раунд 87 —
  мостом `sessions`/`sessionProjections` (C1.1, снимок сервера), а раунд 89 —
  C1.2 (seat-`inject`, Cordis-генераторы, `configForms` и оживший settings-UI
  studio/debugging) — та же строка, новых скоринговых строк не добавлено;
  раунд 91 — C1.3 (сгенерированный каталог слотов DSH + SlotMap-регистрация:
  kinds/scopes/standard-kit, undeclared/keyed/chain/duplicate, winners,
  children/deferred inject), а раунд 92 — C1.4 (host-сервисы `webServer`/
  `ctx.baseUrl`/`configForms` по reference-контракту), а раунд 93 — C2.1
  (атомы design-system и все 204 иконки портированы из reference), а раунд 94 —
  C2.2 (полный reference-порт `Tooltip`: placement-engine, `ShortcutKeys`,
  `input-modality`), а раунд 95 — C2.3 (ядро `Menu`/`MenuItemButton` по
  reference: ряды/selection/aria, клавиатурный walk, one-level submenu,
  dismissal), а раунд 96 — C2.4 (reference submenu page-layout: single-page
  back-row, 200-мс hover grace, `MenuGroup`/`observeStickyMenuGroups`)
  расширили ту же строку — additions остаются `original=no`,
  знаменатель не растёт;
  арифметика та же:
  must 13 × 3 = 39, should 6 × 2 = 12, should partial 1 × 2 × ½ = 1,
  could 1 × 1 = 1 → 39 + 12 + 1 + 1 = 53 из 54 = 98.15 → 98.1).
- Формализация: `./formal/run_tlc.sh` — 3 модели / 5 конфигов TLC без нарушений
  (`CordisSupervisor` safety+liveness, `ClientAssets`, `ApprovalGate` safety+liveness);
  модели и абстракции — `docs/formal/README.md`.
- Каталог тест-кейсов `docs/testing/test-cases.md`: ~150 кейсов с ID, уровнями,
  привязкой к инвариантам TLA+ и статусами; baseline покрытия —
  `cargo llvm-cov --workspace --summary-only`: 72.9% строк (66.9% функций).

## Итог 101 раунда

Закрыты: desktop (трей, профили/композиция бандлов, порт/LAN, папка/drag-and-drop,
терминал с шимами, обновления stable/beta + установка, recovery-окно с профилями
и плагинами, режимы окна/материал, тема), plugins (npm/Cordis install/update/
enable/disable, Web-менеджер, featured-каталог, оценки, Cordis audit/fs-гранты,
клиентская половина: региональный монтаж слотов, `dsh.client.runtime`, тир B
`dsh.client.host_modules` и C-lite — React-компоненты слотов, сервисы моста,
React-контракт, conformance n=4, раунд 82 — n=6 (`dsh-session-superagent`,
`dsh-mnemon`) и раунд 83 — n=9: цель ADR-013 §6 (8–12 реальных плагинов)
достигнута (`dsh-plugin-market`, `dsh-plugin-codegraph-panel`,
`dsh-plugin-notify`), раунд 84 — C1: host HTTP-маршруты плагинов работают
(loopback-сервер sidecar, allowlist-прокси, `connect-src` origin для opt-in
кадров) с записанными остатками, раунд 85 — C2: design-system шим вырос с 7 до
30+ экспортов по фактическим импортам, conformance n=11
(`dsh-plugin-debugging`, `dsh-plugin-studio` — активация без ошибок, их
settings-UI был инертен без host-сервисов), раунд 86 — хардненинг C1:
prefix-маршруты, per-plugin `routes` в `GET /v1/plugins/status`, фильтр
MCP-псевдо-инструментов и loopback-only политика plugin-маршрутов с opt-in
`--allow-lan-plugin-routes`, раунд 87 — C1.1: мост `sessions`/`sessionProjections`
из внутреннего снимка сервера (usage-маршруты реального balance-плагина считают
статистику по сессиям dshr; задержка ≤ 5 с, проекции упрощены, содержимое
сессий не мостится), раунд 89 — C1.2: seat-`inject`, Cordis-генераторы и
`configForms`, поэтому settings-UI `dsh-plugin-studio` (вкладки в Settings) и
`dsh-plugin-debugging` (карточка в Plugins с переключателем и Save) теперь
реально рендерится (browser-`configForms` in-memory, семантика
DSH-конфигурации не воспроизведена), раунд 91 — C1.3: слот-контракт по
референсному каталогу DSH (122 слота с kind/scope/standard-kit; правила
`SlotCore` — undeclared/keyed/chain/duplicate, shadowing-winners,
children-декларации, deferred inject, standard-kit + `ownerProps`), раунд 92 —
C1.4: host-сервисы `webServer`/`ctx.baseUrl`/`configForms` по
reference-контракту (exact/prefix-правила маршрутов и origin-аксессоры,
`file://`-baseUrl, snapshot с числовой ревизией и `set`/`unset`), раунд 93 —
C2.1: атомы design-system и все 204 иконки портированы из reference-исходников
(`deepseek-ai/deepseek-harness` @ `39163dd`, `packages/client/ui-primitives/src`)
на `--dsw-alias-*` токенах, поэтому шим примитивов стал reference-портом (общий
рантайм-артефакт на конец раунда — 375 894 байта); раунд 94 — C2.2: полный
reference-порт `Tooltip` (placement-engine, `ShortcutKeys`, `input-modality`;
артефакт 387 613 байт), а раунд 95 — C2.3: ядро `Menu`/`MenuItemButton` по
`Menu.tsx`/`MenuRow.tsx` (ряды/selection/aria, reserve-icon, disabled-reason
через Tooltip, `ShortcutKeys`, footer/`children`-ряды, one-level submenu,
клавиатурный walk, outside/iframe-blur dismissal, `usePointerGrace`/
`focusWithoutRing`, portal-позиционирование, dense/compact; артефакт
405 323 байта), а раунд 96 — C2.4: reference submenu page-layout
(`menu-submenu-position.ts`: side-карточка или single-page с non-selecting
back-row `data-menu-back`, Escape/ArrowLeft на родителя), 200-мс hover grace
между карточками и `MenuGroup`/`observeStickyMenuGroups` (sticky-заголовки
через IntersectionObserver/ResizeObserver; артефакт 415 972 байта), а раунд 97
(C2.5) — reference `JsonBlock` (collapsible, cap 20 000 символов) и
API-совместимое dependency-free GFM-подмножество `MarkdownText` с
reference-поверхностью props (артефакт 424 801 байт), а раунды 98–101 —
C2.6 submenu-механика (side-card/single-page, `fixed` flip/clamp, `activeList`,
Escape/ArrowLeft, 200-мс hover grace), C3.1 store-seats
(`@deepseek-ai/dsh-client-store` в module-таблице, `useStore`/`actions`,
inject-аргументы `(sessionId, actions)`) и C1.5 fallback-маршруты
(`webServer.registerFallback`) плюс ownerProps-plumbing (артефакт 430 139
байт), поэтому остаются upstream markdown-стек (mdast/shiki/KaTeX/footnotes/
streaming), renderer-семантика, `registerUpgrade` и ownerProps от реальных
shell-сайтов; C-full — решение владельца), host (`ask_user` в
TUI/Web/ACP, `apply_patch`,
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
