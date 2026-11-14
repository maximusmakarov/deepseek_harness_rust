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
| Web UI: тема | light/dark через кнопку Theme, localStorage и `?theme=light` | ✅ |
| Plugin market | Plugins UI: featured-каталог, поиск/инфо npm, Install, менеджер установленных, локальные оценки и отзывы с агрегатом ★/count; общей серверной базы отзывов нет | ⚠️ |
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
| Плагины: WASM core/component, Ed25519, Cordis-sidecar | есть; Cordis audit-log и fs-гранты есть; hot reload и client half нет | ⚠️ |
| Плагин-маркет/экосистема (fabric/market) | CLI `plugin npm search/info` + UI-панель Plugins; установка через CLI, отдельного каталога нет | ⚠️ |
| Субагенты, teams, workflows, schedules, jobs (durable), LSP, Office, CUA | есть | ✅ |
| Windows sandbox/network | AppContainer backend есть, включается `DSHR_WINDOWS_SANDBOX`; fail-closed; CI гоняет argv- и runtime-тест записи внутри/снаружи workspace | ⏸ |
| i18n ru/en | TUI/CLI частично; ошибки en | ⚠️ |
| OTel, SDK, rate-limit, OpenAPI | есть | ✅ |

## Дистрибутивы и CI

| Пункт | dshr | Статус |
|---|---|---|
| Бинарные архивы 5 таргетов, SHA256SUMS, Ed25519, provenance | есть | ✅ |
| Homebrew: бинарная формула | binary + бутылки; `bottles-ghcr` публикует OCI в GHCR и рендерит `dshr-ghcr.rb`; push не прогонялся (нет credentials) | ⚠️ |
| Reproducible builds | архивы детерминированы; бинарь воспроизводим при том же toolchain с remap/SOURCE_DATE_EPOCH (проверено локально); cross-Linux может отличаться | ✅ |
| Desktop в release CI | macOS `.app` zip; Linux/Windows desktop-архивы добавлены в workflow (прогон на теге не выполнялся) | ⚠️ |
| Windows/Linux desktop packages | Linux tar.gz и Windows zip собираются release-джобами; installer'ов нет, джобы не прогонялись | ⚠️ |

## Приоритеты паритета

P0 закрыто: трей/close-to-tray, профили, фиксированный порт/LAN, drag-and-drop папки,
диагностика, `apply_patch`, `ask_user` (включая ACP), каналы и установка обновлений,
recovery-окно с профилями/плагинами, first-run home. Осталось: прогон notarization
с секретами Apple.
P1: композиция профилей из бандлов, полный plugin market/catalog, GUI-менеджер плагинов,
верификация Windows sandbox на реальном Windows.
P2: ghcr-хостинг бутылок, прогон brew/desktop CI на теге, Cordis hot reload и grants,
приватные terminal shim'ы.

## Проверка (14.11.2026)

- `cargo build --release --workspace --locked` и `cargo deny check` — чисто.
- `cargo test --workspace`: 283 passed + 3 ignored; desktop 21 passed (итого 304).
- `scripts/check-reproducible.sh`: два независимых билда дали одинаковый sha256
  (`abdca627…`).
- `package-desktop.sh` + `package-dmg.sh`: `.app` проходит `codesign --verify`,
  DMG собран (UDZO); установочный/архивный dry-run проверен ранее.
- Headless Chrome: light-тема, версия, Plugins; `/v1/tools`, `/v1/plugins/catalog`,
  `/v1/plugins/ratings` отвечают.

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
- **Desktop CI/brew:** джобы выполняются только на теге; локально прогнан dry-run
  упаковки/установки.

Каждый срез: код + тест/сборка + обновление этой матрицы.
