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
| Профиль как композиция бандлов/зависимостей/патчей | config-слои + Cordis-sidecar; композиции бандлов нет | ⚠️ |
| Режимы окна (compatibility/extended/enhanced), материалы macOS | Compatibility/Enhanced/Extended: overlay title bar на macOS, frameless+кастомный titlebar на Linux/Windows; Material (macOS vibrancy) | ⚠️ |
| Локальный web-порт: random/фиксированный, LAN-подтверждение | трей Network: local/LAN с предупреждением, фиксированный/случайный порт; как в оригинале — без отдельного LAN-диалога подтверждения | ✅ |
| Трей: show/hide, profile, terminal, updates, quit; закрытие окна = hide | трей: show/hide, профили, браузер, терминал, диагностика, quit; close-to-tray | ⚠️ |
| Web UI: тема | light/dark через кнопку Theme, localStorage и `?theme=light` | ✅ |
| Plugin market | UI-панель Plugins (установленные tools + поиск npm), `plugin npm search/info`, установка через `npm add`; отдельного каталога/маркета нет | ⚠️ |
| Плагины: `dsh plugin --profile add/remove/update`, рестарт | `dshr plugin npm add/remove/update/list/enable/disable` + привязка к `--profile`; UI-менеджера и market нет | ⚠️ |
| Терминал из трея с приватными shim'ами | открывает системный терминал; приватных shim'ов нет | ⚠️ |
| Обновления: фоновая проверка, канал, installer, Linux → download page | проверка, уведомление, каналы Stable/Beta, скачивание macOS zip с проверкой SHA256SUMS, распаковка/reveal, установка через helper (macOS); notarization нет | ⚠️ |
| Recovery/safe mode: plugin management, rollback, switch profile, diagnostics | при сбое старта — recovery-окно (Retry/Safe mode/Diagnostics/Config/Disable+Enable всех plugins/по-плагинные Cordis/профили/Quit); safe-mode, last-known-good + Rollback profile | ⚠️ |
| `--export-diagnostics`: zip с логами, crash dumps, system-info | `--check`, `--export-diagnostics` (zip: логи, конфиг, system-info) | ✅ |
| Платформы: macOS/Windows installer, Linux deb/AppImage/tar.gz | macOS `.app` (ad-hoc), 5 tar.gz, Linux/Windows desktop-архивы в release CI; installer'ов (deb/NSIS/DMG) нет | ⚠️ |
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
| Плагины: WASM core/component, Ed25519, Cordis-sidecar | есть; hot reload, grants/audit, client half нет | ⚠️ |
| Плагин-маркет/экосистема (fabric/market) | CLI `plugin npm search/info` + UI-панель Plugins; установка через CLI, отдельного каталога нет | ⚠️ |
| Субагенты, teams, workflows, schedules, jobs (durable), LSP, Office, CUA | есть | ✅ |
| Windows sandbox/network | AppContainer backend есть, включается `DSHR_WINDOWS_SANDBOX`; fail-closed; CI-шаг гоняет ignored-тест | ⏸ |
| i18n ru/en | TUI/CLI частично; ошибки en | ⚠️ |
| OTel, SDK, rate-limit, OpenAPI | есть | ✅ |

## Дистрибутивы и CI

| Пункт | dshr | Статус |
|---|---|---|
| Бинарные архивы 5 таргетов, SHA256SUMS, Ed25519, provenance | есть | ✅ |
| Homebrew: бинарная формула | есть; ghcr-бутылки нет | ⚠️ |
| Reproducible builds | архивы детерминированы; бинарь воспроизводим при том же toolchain с remap/SOURCE_DATE_EPOCH (проверено локально); cross-Linux может отличаться | ✅ |
| Desktop в release CI | macOS `.app` zip; Linux/Windows desktop-архивы добавлены в workflow (прогон на теге не выполнялся) | ⚠️ |
| Windows/Linux desktop packages | Linux tar.gz и Windows zip собираются release-джобами; installer'ов нет, джобы не прогонялись | ⚠️ |

## Приоритеты паритета

P0 закрыто: трей/close-to-tray, профили, фиксированный порт/LAN, диагностика,
`apply_patch`, `ask_user` (включая ACP), обновления (проверка/скачивание/установка macOS),
recovery-меню и first-run home. Осталось: notarization и каналы обновлений.
P1: setup/onboarding и drag-and-drop папки, полный plugin market/catalog,
Windows sandbox (нужен Windows-раннер), GUI plugin manager.
P2: Linux-режимы/кастомный titlebar, recovery-окно вместо нативных диалогов,
ghcr-бутылки, прогон desktop CI на теге.

Каждый срез: код + тест/сборка + обновление этой матрицы.
