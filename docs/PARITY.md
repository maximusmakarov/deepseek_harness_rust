# Паритет с DSH Desktop 0.2.2-rc.2

Матрица пользовательских сценариев оригинала и `dshr`. Источники: reference-клон
`dsh-desktop@233b76c` (`docs/user-guide.md`, `docs/architecture.md`), установленный
DSH 0.2.2-rc.2 и собственные ADR. Статусы: ✅ есть, ⚠️ частично, ❌ нет,
⏸ сознательно отложено.

## Desktop

| Сценарий оригинала | dshr | Статус |
|---|---|---|
| Первый запуск, профиль по умолчанию, настройка | авто-создание home/README/desktop-profile + нативный provider setup (маршрут/модель/ключ, `desktop.env 0600`); wizard'а бандлов нет | ✅ |
| Запуск с папкой (`dsh-desktop <folder>`, drag-and-drop) | `dshr-desktop <folder>` задаёт cwd сервера; drag-and-drop нет | ⚠️ |
| Профили: список в трее, переключение с рестартом, last-known-good | CLI `--profile`/`DSH_PROFILE`; трей: список, New profile…, переключение пишет `desktop-profile` и перезапускает shell | ✅ |
| Профиль как композиция бандлов/зависимостей/патчей | config-слои + Cordis-sidecar; композиции бандлов нет | ⚠️ |
| Режимы окна (compatibility/extended/enhanced), материалы macOS | Compatibility/Enhanced (Overlay title bar) и Material (macOS vibrancy) через трей; extended-рамки и Linux-режимы нет | ⚠️ |
| Локальный web-порт: random/фиксированный, LAN-подтверждение | случайный порт или `DSHR_DESKTOP_PORT`; LAN-тумблера нет | ⚠️ |
| Трей: show/hide, profile, terminal, updates, quit; закрытие окна = hide | трей: show/hide, профили, браузер, терминал, диагностика, quit; close-to-tray | ⚠️ |
| Plugin market | UI-панель Plugins (установленные tools + поиск npm), `plugin npm search/info`, установка через `npm add`; отдельного каталога/маркета нет | ⚠️ |
| Плагины: `dsh plugin --profile add/remove/update`, рестарт | `dshr plugin npm add/remove/update/list` + привязка к `--profile` (profile.yaml, отдельные packages); UI-менеджера и market нет | ⚠️ |
| Терминал из трея с приватными shim'ами | открывает системный терминал; приватных shim'ов нет | ⚠️ |
| Обновления: фоновая проверка, канал, installer, Linux → download page | проверка, уведомление, скачивание macOS zip с проверкой SHA256SUMS, распаковка и reveal; тихой установки/каналов нет | ⚠️ |
| Recovery/safe mode: plugin management, rollback, switch profile, diagnostics | `--safe-mode` (пропуск MCP/Cordis/WASM), трей Restart in safe mode, диагностика; recovery-окна/rollback нет | ⚠️ |
| `--export-diagnostics`: zip с логами, crash dumps, system-info | `--check`; экспорт диагностики — в работе | ⚠️ |
| Платформы: macOS/Windows installer, Linux deb/AppImage/tar.gz | macOS `.app` (ad-hoc), 5 tar.gz; Windows/Linux installer'ов нет | ⚠️ |
| Автообновление/подпись/notarization | ad-hoc подпись `.app`; notarization нет | ⚠️ |

## Host и инструменты

| Область | dshr | Статус |
|---|---|---|
| LLM-провайдеры, tool-loop, сессии, resume/fork/rewind, compaction | DeepSeek + OpenAI-совместимые | ✅ |
| Инструменты fs/shell/search/todo/present/web | есть | ✅ |
| `apply_patch` (DSH-формат) | есть, multi-file add/update/delete/move | ✅ |
| LLM-заголовки сессий | `run` генерирует заголовок провайдером (DSH-промпт), fallback — первая строка; server/TUI пока нет | ⚠️ |
| Импорт сессий DSH v4 | `sessions import` (zstd JSONL → наша модель, title/tool calls) | ✅ |
| `ask_user` | TUI-модалка и Web-диалог через observer; headless/ACP fail-closed | ✅ |
| persistent bash, diff-view | нет | ❌ |
| MCP: stdio/HTTP, tools/resources/prompts/sampling, OAuth refresh | есть; elicitation нет | ⚠️ |
| ACP v1: сессии, permissions, fs, terminal, image/audio/resource, MCP | есть; optional methods (`session/list`, modes) нет | ⚠️ |
| Плагины: WASM core/component, Ed25519, Cordis-sidecar | есть; hot reload, grants/audit, client half нет | ⚠️ |
| Плагин-маркет/экосистема (fabric/market) | нет | ❌ |
| Субагенты, teams, workflows, schedules, jobs (durable), LSP, Office, CUA | есть | ✅ |
| Windows sandbox/network | fail-closed, AppContainer не работает | ⏸ |
| i18n ru/en | TUI/CLI частично; ошибки en | ⚠️ |
| OTel, SDK, rate-limit, OpenAPI | есть | ✅ |

## Дистрибутивы и CI

| Пункт | dshr | Статус |
|---|---|---|
| Бинарные архивы 5 таргетов, SHA256SUMS, Ed25519, provenance | есть | ✅ |
| Homebrew: бинарная формула | есть; ghcr-бутылки нет | ⚠️ |
| Reproducible builds | архивы детерминированы (mtime/uid/gid/sort); бинарь побайтово не воспроизводим | ⚠️ |
| Desktop в release CI | macOS `.app` zip | ✅ |
| Windows/Linux desktop packages | нет | ❌ |

## Приоритеты паритета

P0 (закрыто 14.11): трей и close-to-tray, профили/создание профиля, фиксированный порт,
диагностика, `apply_patch`, `ask_user`, базовая проверка обновлений и first-run home.
Осталось: тихая установка/замена приложения и каналы обновлений.
P1: setup/onboarding, запуск с папкой, обновления, npm/Cordis install/update,
плагин-маркет, elicitation, Windows sandbox.
P2: режимы окна/материалы, recovery-окна, reproducible builds, ghcr-бутылки.

Каждый срез: код + тест/сборка + обновление этой матрицы.
