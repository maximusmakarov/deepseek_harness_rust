# Tech debt register

Обновляется по мере закрытия; приоритеты — по влиянию на безопасность и основные сценарии.

## Закрыто

| Долг | Решение |
|---|---|
| Linux shell не работал (fail-closed) | Landlock через `__sandbox-exec` — [ADR-024](adr/024-linux-landlock.md) |
| Нет общего бюджета токенов на дерево агентов | `Budget` в `AgentConfig`, флаг `--max-total-tokens` — [ADR-025](adr/025-guardrails.md) |
| Нельзя ограничить набор инструментов | `tools.allowed` / `tools.denied` — [ADR-025](adr/025-guardrails.md) |
| Два scheduler-демона могли работать одновременно | lock-файл `schedules.lock` — [ADR-025](adr/025-guardrails.md) |
| Безымянные сессии после `run` | заголовок из первой строки prompt |
| LSP-навигация отсутствовала | `dshr-lsp` — [ADR-023](adr/023-lsp.md) |
| Linux network не ограничивался | seccomp-bpf deny сетевых syscalls — [ADR-027](adr/027-linux-seccomp-network.md) |
| MCP HTTP/resources отсутствовали | Streamable HTTP + resource tools — [ADR-028](adr/028-mcp-http-resources.md) |
| Team-доска терялась, workflow не повторялся | durable state + `retries` — [ADR-029](adr/029-durable-teams-retries.md) |
| Office-документы не поддерживались | docx/xlsx/pptx tools — [ADR-030](adr/030-office.md) |
| CUA/browser отсутствовал | headless Chrome tools — [ADR-031](adr/031-browser-cua.md) |
| Поиск только сканом | FTS5-индекс по гейту — [ADR-016](adr/016-search-backend.md) |
| Не было WASM-плагинов | wasmi + JSON ABI — [ADR-032](adr/032-wasm-plugins.md) |
| Не было телеметрии | OTLP/HTTP traces — [ADR-033](adr/033-opentelemetry.md) |
| Не было ACP | ACP v1 stdio + permissions + fs — [ADR-034](adr/034-acp.md) |
| Не было i18n | TUI ru/en — [ADR-035](adr/035-tui-i18n.md) |
| Не было desktop | PWA-манифест — [ADR-036](adr/036-pwa-desktop.md) |
| MCP prompts отсутствовали | `list_prompts`/`get_prompt` — [ADR-028](adr/028-mcp-http-resources.md) |
| MCP sampling отсутствовал | stdio + HTTP GET-SSE `sampling/createMessage` — [ADR-028](adr/028-mcp-http-resources.md) |
| MCP OAuth отсутствовал | client-credentials — [ADR-028](adr/028-mcp-http-resources.md) |
| ACP audio/stdio MCP отсутствовали | audio-блоки и stdio MCP из `session/new` — [ADR-034](adr/034-acp.md) |
| Не было криптоподписи WASM | Ed25519 над именем и sha256, `plugins.trusted_keys` — [ADR-032](adr/032-wasm-plugins.md) |
| Не было LLM-заголовков | `generate_title` (DSH-промпт), `session.auto_title` — `dshr-agent` |
| Не было `ask_user` | observer-канал в TUI/Web, fail-closed в headless — `dshr-agent` |
| Не было `apply_patch` | multi-file `*** Begin Patch` (add/update/delete/move), тесты — `dshr-tools` |
| Jobs не переживали рестарт | состояние и логи в `<home>/jobs`, running → interrupted при загрузке |
| Не было component model/WIT | wasmtime + WIT-мир `dshr:plugin@1.0.0`, пустой WASI; legacy wasmi сохранён — [ADR-032](adr/032-wasm-plugins.md) |
| Не было desktop-оболочки | `crates/desktop`: Tauri v2 над loopback `dshr serve` — [ADR-011](adr/011-desktop.md) |
| Не было Leptos UI | `crates/web` (Leptos CSR, trunk), dist встроен в сервер — [ADR-003](adr/003-ui-stack.md) |
| ACP не принимал embedded `resource` | текст/blob → text/image/audio — [ADR-034](adr/034-acp.md) |
| ACP игнорировал `resource_link` | блок разворачивается в текст с именем/URI — [ADR-034](adr/034-acp.md) |
| Не было стабильных бинарных дистрибутивов | Архивы на 5 таргетов, SHA256SUMS, Ed25519-подпись, provenance, бинарная формула — [ADR-039](adr/039-release-artifacts.md) |
| Региональный монтаж слотов клиентской половины был отложен (только слоты-вкладки панели) | Eager registry-кадр на каждую `dsh.client`-половину + отдельный sandboxed iframe на регистрацию; полный словарь DSH 0.2.2-rc.2 (116 ключей) и legacy `panel`/`message.action` разложены по 17 регионам шелла, незнакомое имя — fallback в панель; протокол моста v1.2 (frame token, `region`, slot-режим, `context`). Мостовая модель вместо React-контракта — решение [ADR-013](adr/013-cordis-compat.md) §5; полная React-совместимость — открытый Deferred-пункт по [ADR-040](adr/040-react-client-half.md). Replica 98.1/100. |
| Не было общего рантайма модулей хоста (React в каждом фрейме) | Тир B PoC (раунд 78): `dsh.client.host_modules` — один версионированный immutable-артефакт `/v1/runtime/react-18.3.1.js` (React + ReactDOM UMD + `bootstrap.js`), opt-in CSP с origin сервера; полный design-system, React-контекст и SlotMap-семантика остаются открытыми (частичный design-system шим добавлен в C-lite, раунд 79) — [ADR-040](adr/040-react-client-half.md). |
| Не было React-компонентов слотов и design-system (C-lite) | C-lite PoC (раунд 79, [ADR-040](adr/040-react-client-half.md)): при host React `ctx.slots.register(entry, Component)` — React-компонент (один root на кадр, перерисовка на host-state, error boundary), host-state `theme`/`wide`/`session`/`sessions`, hooks `useTheme`/`useSession`/`useSessions`/`useHostState`, `ctx.get('sessions')`; частичный независимый шим `@deepseek-ai/dsh-client-ui-primitives` (`Button`, `StateDot`, `Tooltip`, `Icon`, `Spinner`) в общем рантайме; conformance — реальный MIT-плагин `@kt11/dsh-deepseek-balance@0.2.1` (vendored fixture). Остаток полного тира C (C-full) — ниже. |
| Референсные плагины падали на отсутствующих сервисах моста | Раунд 80 ([ADR-040](adr/040-react-client-half.md)): `ctx.get('slots')` (тот же реестр, что `ctx.slots`, плюс `entries(key)`), минимальный `ctx.get('locale')` (`{active: "en", locale: "en"}`, `subscribe`), `ctx.inject(deps, callback)` (callback только когда резолвятся все зависимости — опциональные `betterSidebar`/`sidebarRight`/`webTerminals` не монтируются) и диагностика `window.__DSHR_LAST_ERROR__` (`{message, stack}`); conformance n=2 (`dsh-remote@1.1.2`). Подмножество сервисов и остаток C-full — ниже. |

## Открыто

| Приоритет | Долг | Комментарий |
|---|---|---|
| Deferred | C-full: полный design-system и SlotMap-семантика | [ADR-040](adr/040-react-client-half.md) (Proposed): C-lite PoC выпущен в раунде 79 (React-компоненты слотов, host-hooks/`ctx.get('sessions')`, частичный шим `@deepseek-ai/dsh-client-ui-primitives`, реальный плагин в conformance), а раунд 80 добавил сервисы моста (`ctx.get('slots')`/`locale`, `ctx.inject`, диагностика) и второй conformance-плагин. Чего нет: полного design-system `@deepseek-ai/dsh-client-ui-*` (в шиме только 5 компонентов), SlotMap-семантики (`ownerProps`/framework-hooks, cardinality/scope/chain/hoc), React-контекста/состояния шелла и композиции между плагинами, host-маршрутов DSH-плагинов, выбора версии React и настоящего `@dshr/cordis-compat@1`-шима. Решение по полному тиру C — за владельцем, D не рекомендуется. |
| P2 | Общая модульная таблица шелла — React и частичный design-system шим | `dsh.client.host_modules` отдаёт React 18.3.1 и jsx-шимы одним immutable-артефактом; с раунда 79 таблица расширена частичным шимом `@deepseek-ai/dsh-client-ui-primitives` (`Button`, `StateDot`, `Tooltip`, `Icon`, `Spinner`). Полного design-system `@deepseek-ai/dsh-client-ui-*`, React-контекста шелла и композиции между плагинами нет; каждый iframe исполняет рантайм в своём JS-реалме (один артефакт/одна загрузка по сети, а не общий JS-контекст) — расширение до контракта DSH = C-full [ADR-040](adr/040-react-client-half.md). |
| P2 | Host-маршруты DSH-плагинов не поддерживаются | Реальные плагины (`@kt11/dsh-deepseek-balance@0.2.1`, `dsh-remote@1.1.2`) в smoke рендерят карточки/слоты, но их data-запросы идут на host-маршруты DSH (`/api/...`), которых в dshr нет: popup balance показывает ветку «查询失败», а dsh-remote полагается только на опциональные сервисы. Рендер и интерактивность не ломаются, но полноценная работа Data-плагинов требует host-маршрутов — часть C-full [ADR-040](adr/040-react-client-half.md). |
| P2 | `wide` в host-state всегда `true` | Шелл не транслирует фактическую ширину сайдбара: в `host-state` и props компонента `wide` — константа `true`. Для адаптивных компонентов это означает «широкий режим всегда»; честная ширина — часть C-full. |
| P2 | Conformance-набор — n=2 | В smoke два реальных DSH-плагина: `@kt11/dsh-deepseek-balance@0.2.1` и `dsh-remote@1.1.2` (оба MIT, vendored fixtures); остальные клиентские половины smoke — синтетические фикстуры. План M5b 5.19 — 8–12 реальных плагинов в nightly CI — не закрыт. |
| P2 | CSP строже eval-based бандлов | Плагин `@michengai/dsh-archive-manager` 1.3.6 использует `new Function`/eval и не работает под текущим CSP (`script-src` без `'unsafe-eval'`); ослабление сознательно не включено. Вариант — opt-in-флаг/профиль `unsafe-eval` с отдельным предупреждением и аудитом; пока это документированное ограничение conformance-набора, а не часть n=2. |
| P2 | Падение `@michengai/dsh-skills-manager` на инициализации бандла | Версия 1.4.8 падает при инициализации (`Object.assign` над `undefined`) — нужен триаж исходников плагина: понять, какого сервиса/поля не хватает мосту, и либо добавить шим, либо записать несовместимость. В conformance не входит, набор остаётся n=2. |
| P2 | Мостовые сервисы — подмножество референсных | `ctx.get('sessions')` не отдаёт проекции, `sessionStats` и т.п.; `ctx.get('slots')` — реестр с `entries(key)`; `locale` минимальный (`{active: "en", locale: "en"}`, без `register`). Достаточно для мягкой деградации референсных плагинов, но не полный набор сервисов DSH — расширение входит в C-full [ADR-040](adr/040-react-client-half.md). |
| P2 | `dsh.client.external` не поддерживается | Внешние модули не подключаются: бандл самодостаточен, приносит модули сам через `dsh.client.runtime` или берёт их из общего рантайма хоста через `dsh.client.host_modules`. |
| P2 | Контракты `dsh.client.runtime` и `dsh.client.host_modules` экспериментальны | Classic script (не ESM), рантайм исполняется в каждом кадре, лимит 8 МиБ, отдельной подписи нет; после тира B PoC у половины два пути — свой runtime или общий host-рантайм; контракты могут поменяться до C-full. |
| P2 | CSP opt-in-кадров открывает origin сервера для скриптов | Единственное новое сетевое разрешение — `script-src ... <origin>` для `/v1/runtime/*`; `connect-src`/`img-src`/`style-src` не расширены, кадры без `host_modules` полностью network-free. Компромисс осознанный; host-header-инъекция в CSP закрыта валидацией `request_origin`. |
| P2 | Версия React закреплена за сервером | `HOST_RUNTIME_VERSION = 18.3.1`; у плагина нет выбора версии, смена версии — релиз сервера (одна версия на страницу — следствие iframe-изоляции). C-lite (раунд 79) ограничение не меняет: React-режим работает на той же закреплённой версии. Совмещение версий шелла и плагинов — открытый вопрос C-full [ADR-040](adr/040-react-client-half.md). |
| Deferred | Windows sandbox и сеть | Отложено по решению пользователя: нужен реальный Windows-хост. По умолчанию fail-closed; `DSHR_WINDOWS_SANDBOX` — opt-in, его CI-шаг помечен `continue-on-error` (AppContainer setup на hosted runner'ах зависает; прогон виден в summary, но не блокирует trunk). Enforcement сети на Windows — `Partial`. |
| P2 | Notarization | Тулинг есть (`scripts/notarize-app.sh`), нужны Apple Developer ID, сертификат и app-specific password; без них распространяется ad-hoc подпись. |
| P2 | GHCR-бутылки и brew-прогон | OCI-publisher и формула есть, push/pull подтверждены против mock и в Actions; нет tap-репозитория и учётных данных пакетов, brew-прогон на реальном GHCR не делался. |
| P2 | Центральная база отзывов plugin market | Отзывы серверные и общие для всех клиентов одного инстанса (`<home>/plugin-reviews.json`), но центральной/федеративной базы между инстансами нет. |
| Done | Экран настроек провайдера в Web UI | Диалог Settings: маршрут из известных, модель, `base_url`; сохранение пишет user-слой `config.yaml` (0600, атомарно) и просит перезапуск. Ключи в UI не принимаются — они читаются из окружения/`desktop.env`. |
| Done | Визуальный diff в Web UI | Карточка `tool_start` показывает `-old`/`+new` для `edit`, добавленные строки для `write` и тело патча для `apply_patch` (сервер передаёт input инструмента); полный файловый diff с контекстом остаётся в TUI. |
| Done | i18n остатка CLI | Закрыто: TUI/CLI, справка (`plugin npm`, `sessions import`), рантайм-сообщения `sessions`/`plugin`/`doctor`/`run` и префикс ошибок локализованы; тексты `DshrError`/`ToolError` намеренно остаются en, `--json` не зависит от языка. |
