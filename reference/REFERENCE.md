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
| Upstream-пин | `deepseek-harness` `39163dd30f703a79079e2d2446545cff3c453738` = `0.2.2-rc.3` (master, 13.11.2026) |
| Установленный DSH | 0.2.2-rc.2 (Desktop, macOS) — на один rc старше пина |

Официальный upstream вырос в отдельную оболочку: в `deepseek-harness` с
`0.2.2-rc.3` есть `apps/desktop` (Electron), `apps/desktop-host`,
`apps/desktop-publisher`, `apps/web`, `apps/cli`. Паритет по десктопу сверяется
с ним, а не только с community-плагином `dsh-plugin-desktop`; предыдущий пин
`66491b05` = `0.2.2-rc.1` устарел.

## Что склонировано

Клон community-плагина разреженный (`--filter=blob:none --sparse`), 36 МБ
вместо ~380 МБ:

- `docs/` — архитектура, гайды, FAQ;
- `dsh-plugin-desktop/` — стабильный desktop-плагин (Electron host);
- `dsh-desktop-next/`, `dsh-plugin-desktop-beta/` — следующие каналы;
- `dsh-community-fabric/`, `dsh-community-market/` — экосистема плагинов;
- `patches/`, `scripts/`, `.agents/`, `.github/`;
- корневые `package.json`, `upstream.json`, `README*.md`, `AGENTS.md`, `LICENSE`.

Исключено: `vendor/` (346 МБ предсобранных рантаймов) и submodule
`deepseek-harness/` — он читается из отдельного клона upstream.

### Официальный upstream

Второй клон — `deepseek-harness` `39163dd` (`0.2.2-rc.3`, полный, 251 МБ),
используется как источник фактов по клиентской половине и официальной
десктоп-оболочке:

- `apps/desktop/` — Electron-оболочка: `src/` (single instance, login-shell
  environment, welcome, recovery, updates, browser guests), `installer/`
  (NSIS, локали, uninstall-data), `packaging/`, `publishing/`,
  `windows-signing/`;
- `apps/desktop-host/` — приватный Desktop Host, `apps/desktop-publisher/` —
  публикация релизов, `apps/web/`, `apps/cli/`;
- `packages/client/*` — UI-половина (primitives, store, locale, слоты),
  `packages/host/*` — webserver, authentication-desktop, плагины;
- `packages/extensions/cordis-client-runner` — реестр слотов.

Рантайм официальной оболочки (bundled Node/pnpm, `app.asar`, runtime
descriptor) — артефакт Electron; из него переносится поведение, а не код.

## Ключевые факты для `dshr`

1. **Desktop — тонкая оболочка, а не второй harness.** Electron main запускает официальный DSH Host, Host поднимает HTTP/WS Web carrier на loopback, renderer — обычный Web UI. Отдельного renderer-IPC для плагинов нет, Electron API в страницу не отдаётся.
2. **Всё — плагин.** Desktop-возможности (окно, трей, профили, терминал, обновления) — Cordis-плагины рядом с upstream; сторонние плагины подключаются тем же механизмом.
3. **Границы generation.** Смена профиля/режима уничтожает текущую generation (сервисы, окна, subprocess-хендлы) и поднимает новую; кэшировать объекты между generation нельзя.
4. **Carrier — единственный канал.** Web UI общается с Host по HTTP/WS; desktop не добавляет второй протокол. Это же решение подходит `dshr`: Tauri-оболочка над `dshr serve` на loopback.
5. **Платформенный seam.** Различия macOS/Windows/Linux собраны в `ElectronPlatformStrategy`, выбираемый один раз при старте.
6. **Профили и pnpm.** `desktopProfiles.current` — единственный источник имени/каталога профиля; `desktopPnpm` и `runPlugin()` работают в рамках generation.
7. **Официальная оболочка — Electron + встроенный рантайм.** `apps/desktop`
   запускает Host в Electron RunAsNode, несёт собственную копию dsh и pnpm,
   поэтому shell и рантайм обновляются одной единицей. Наш аналог — Tauri +
   `dshr serve` из того же бандла: версии жёстко связаны, отдельного
   runtime pack нет.
8. **Владение профилем.** Официальная оболочка держит single-instance lock до
   чтения профиля и единолично владеет `$DSH_HOME/profiles/desktop`; наш
   аналог — `desktop.lock` в home и `--home` у дочернего сервера.
9. **Login-shell env.** GUI-запуск не наследует rc-файлы; оболочка один раз
   читает `env -0` логин-шелла и передаёт Host. У `dshr` это
   `crates/desktop/src/login_env.rs`.

## Как обновлять

```sh
# community-плагин (поведение Desktop-оболочки)
git -C reference/dsh-desktop fetch --depth 1 origin master
git -C reference/dsh-desktop checkout FETCH_HEAD
git -C reference/dsh-desktop rev-parse HEAD   # обновить таблицу выше

# официальный upstream (клиентская половина, apps/desktop)
git -C /tmp/dsh-upstream fetch --depth 1 origin master
git -C /tmp/dsh-upstream checkout FETCH_HEAD
git -C /tmp/dsh-upstream rev-parse HEAD       # обновить таблицу выше
```

После обновления — сверить `upstream.json` и перечитать `docs/architecture.md`;
расхождения с `docs/ARCHITECTURE.md` и [docs/PARITY.md](../docs/PARITY.md)
фиксировать в ADR. Клон живёт вне репозитория (`/tmp`), потому что upstream
тяжелее community-плагина и не нужен сборке.

## Ограничения использования

- Код референса не копируется в `dshr`; переносятся только идеи и контракты (MIT это позволяет, но проект остаётся самостоятельной реализацией). Целевые улучшения и метрики — [docs/IMPROVEMENTS.md](../docs/IMPROVEMENTS.md).
- Клон upstream читается только как источник фактов: из него не берутся
  npm-зависимости, сборочные скрипты и рантайм; `apps/desktop` — референс
  поведения, а не шаблон (у нас Tauri и нет Node/pnpm).
- `reference/` исключён из будущего git-репозитория проекта (вложенный `.git`).
