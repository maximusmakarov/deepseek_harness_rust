# Журнал раундов паритета (1–63)

Цель: довести `dshr` до паритета с DSH 0.2.2-rc.2 / DSH Desktop.
Каждый раунд — код/документация + проверка. Итоговая сверка: 308 root-тестов
(+3 ignored) и 21 desktop-тест = **329**, `clippy`/`fmt`/`deny` чисто,
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
- Windows: runtime-тест AppContainer (запись внутри/снаружи workspace) добавлен
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

## Что осталось

- React-слоты клиентских плагинов: сейчас мост iframe и один слот `panel`
  (ADR-013 §5), полного набора DSH-слотов нет.
- WASM hot reload (Cordis hot reload есть).
- GHCR-хостинг бутылок — нет tap/учётных данных (сейчас GitHub Releases).
- Windows sandbox на реальном Windows — CI-шаг есть, локальной проверки нет.

Подробная матрица и блокеры: [PARITY.md](PARITY.md).
