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
| Плагины: WASM core/component, Ed25519, Cordis-sidecar | Cordis: audit-log, fs-гранты, hot reload (watcher + endpoint) и клиентская половина (sandboxed iframe, несколько слотов по `name`/`order`, `inject`/`effect`-lifecycle, capability-ассеты для многофайловых бандлов, host-действия, перезагрузка открытой панели); Replica-паритет клиентской половины 98.0/100 (must 12/12); WASM: hot reload (fingerprint watcher и `/v1/plugins/reload`) | ✅ |
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
P2: прогон brew/desktop CI на теге, React-слоты клиентских плагинов
(сейчас мост, ADR-013 §5), приватные terminal shim'ы.

## Проверка (14.11.2026)

- `cargo build --release --workspace --locked` и `cargo deny check` — чисто.
- `cargo test --workspace`: 335 passed + 3 ignored; desktop 21 passed (итого 356).
- `scripts/check-reproducible.sh`: два независимых билда дали одинаковый sha256
  (`abdca627…`).
- `package-desktop.sh` + `package-dmg.sh`: `.app` проходит `codesign --verify`,
  DMG собран (UDZO); установочный/архивный dry-run проверен ранее.
- Headless Chrome: light-тема, версия, Plugins; `/v1/tools`, `/v1/plugins/catalog`,
  `/v1/plugins/ratings` отвечают.
- Браузерный e2e (`scripts/web-e2e.py`, stdlib CDP): «New session» → промпт →
  модалка approval для `bash` → Allow → выполнение → стрим ответа → карточка
  инструмента → файл реально записан → сессия в журнале; 9/9 проверок, джоба
  `web-e2e` в CI.
- Linux/Windows desktop: [run 43591784883](https://github.com/maximusmakarov/deepseek_harness_rust/actions/runs/43591784883)
  — обе сборки и упаковка успешны; `.deb` проверен `dpkg-deb --info`.
- Cordis hot reload проверен на живом `serve`: правка entry перезагружает плагин
  (генерация 1→2), добавление/удаление плагина в `profile.yaml` стартует/останавливает
  sidecar, `POST /v1/plugins/reload` отвечает, `GET /v1/plugins/status` отражает
  состояние; `/v1/plugins/installed` следует за супервизором.
- Клиентская половина: `python3 scripts/plugin-smoke.py` — 28 проверок в headless
  Chrome: вкладки слотов и переключение, resize, live-тема, `ctx.action`
  (`close`/`reload`/`compose`/`notify` и отказ для неизвестного), eager-половина
  переживает закрытие панели, многофайловый capability-бандл и module-table бандл
  рендерятся, открытая панель подхватывает новый бандл после reload.
- Replica-оценка клиентской половины (`replica-diff`): 98.0/100, must 12/12 —
  `docs/replica/client-half-parity.md`.
- Формализация: `./formal/run_tlc.sh` — 3 модели / 5 конфигов TLC без нарушений
  (`CordisSupervisor` safety+liveness, `ClientAssets`, `ApprovalGate` safety+liveness);
  модели и абстракции — `docs/formal/README.md`.
- Каталог тест-кейсов `docs/testing/test-cases.md`: ~120 кейсов с ID, уровнями,
  привязкой к инвариантам TLA+ и статусами; baseline покрытия —
  `cargo llvm-cov --workspace --summary-only`: 72.9% строк (66.9% функций).

## Итог 60 раундов

Закрыты: desktop (трей, профили/композиция бандлов, порт/LAN, папка/drag-and-drop,
терминал с шимами, обновления stable/beta + установка, recovery-окно с профилями
и плагинами, режимы окна/материал, тема), plugins (npm/Cordis install/update/
enable/disable, Web-менеджер, featured-каталог, оценки, Cordis audit/fs-гранты),
host (`ask_user` в TUI/Web/ACP, `apply_patch`, LLM-заголовки везде, elicitation
stdio/HTTP/GET-SSE, DSH import, persistent bash, TUI diff), дистрибутивы
(детерминированные архивы, reproducible-бинарь, bottles, DMG, Linux/Windows
desktop-джобы, notarization-тулинг).

## Известные блокеры

- **GHCR-бутылки:** нет tap-репозитория/учётных данных пакетов; бутылки публикуются
  как GitHub release-ассеты.
- **Windows sandbox:** нужен реальный Windows-хост; локально проверяется только
  компиляция, ignored-тест вынесен в Windows CI.
- **Notarization:** нужны Apple Developer ID, сертификат и app-specific password.
- **Desktop CI/brew:** release-джобы выполняются только на теге; Linux/Windows
  desktop-сборка и упаковка отдельно проверены в Actions, локально — dry-run.

Каждый срез: код + тест/сборка + обновление этой матрицы.
