# Референсные исходники: DSH Desktop

Локальная копия используется только как **источник архитектурных фактов и поведения**, не как зависимость сборки.

| Параметр | Значение |
|---|---|
| Репозиторий | https://github.com/anywhere-labs/dsh-desktop |
| Ветка | `master` |
| Зафиксированный коммит | `233b76c65d2a83a777a96419d798cb5582acb8f7` |
| Дата клонирования | 12.11.2026 |
| Лицензия | MIT (файл `LICENSE` в клоне) |
| Продукт | DSH Desktop 2.0.18 (`dsh-plugin-desktop`) |
| Upstream-пин | `deepseek-harness` `66491b05e3f05ad35bcdd4f6cd7ed26041d9e3db` = 0.2.2-rc.1 (`upstream.json`, канал beta) |
| Установленный DSH | 0.2.2-rc.2 (Desktop, macOS) — на один rc новее пина |

## Что склонировано

Клон разреженный (`--filter=blob:none --sparse`), 36 МБ вместо ~380 МБ:

- `docs/` — архитектура, гайды, FAQ;
- `dsh-plugin-desktop/` — стабильный desktop-плагин (Electron host);
- `dsh-desktop-next/`, `dsh-plugin-desktop-beta/` — следующие каналы;
- `dsh-community-fabric/`, `dsh-community-market/` — экосистема плагинов;
- `patches/`, `scripts/`, `.agents/`, `.github/`;
- корневые `package.json`, `upstream.json`, `README*.md`, `AGENTS.md`, `LICENSE`.

Исключено: `vendor/` (346 МБ предсобранных рантаймов) и submodule `deepseek-harness/` (пустой указатель; upstream доступен отдельно).

## Ключевые факты для `dshr`

1. **Desktop — тонкая оболочка, а не второй harness.** Electron main запускает официальный DSH Host, Host поднимает HTTP/WS Web carrier на loopback, renderer — обычный Web UI. Отдельного renderer-IPC для плагинов нет, Electron API в страницу не отдаётся.
2. **Всё — плагин.** Desktop-возможности (окно, трей, профили, терминал, обновления) — Cordis-плагины рядом с upstream; сторонние плагины подключаются тем же механизмом.
3. **Границы generation.** Смена профиля/режима уничтожает текущую generation (сервисы, окна, subprocess-хендлы) и поднимает новую; кэшировать объекты между generation нельзя.
4. **Carrier — единственный канал.** Web UI общается с Host по HTTP/WS; desktop не добавляет второй протокол. Это же решение подходит `dshr`: Tauri-оболочка над `dshr serve` на loopback.
5. **Платформенный seam.** Различия macOS/Windows/Linux собраны в `ElectronPlatformStrategy`, выбираемый один раз при старте.
6. **Профили и pnpm.** `desktopProfiles.current` — единственный источник имени/каталога профиля; `desktopPnpm` и `runPlugin()` работают в рамках generation.

## Как обновлять

```sh
git -C reference/dsh-desktop fetch --depth 1 origin master
git -C reference/dsh-desktop checkout FETCH_HEAD
git -C reference/dsh-desktop rev-parse HEAD   # обновить таблицу выше
```

После обновления — сверить `upstream.json` и перечитать `docs/architecture.md`; расхождения с `docs/ARCHITECTURE.md` проекта фиксировать в ADR.

## Ограничения использования

- Код референса не копируется в `dshr`; переносятся только идеи и контракты (MIT это позволяет, но проект остаётся самостоятельной реализацией). Целевые улучшения и метрики — [docs/IMPROVEMENTS.md](../docs/IMPROVEMENTS.md).
- Submodule `deepseek-harness` не инициализирован: upstream-код читается из установленного пакета `0.2.2-rc.2` и официальной документации.
- `reference/` исключён из будущего git-репозитория проекта (вложенный `.git`).
