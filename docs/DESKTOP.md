# Desktop

## Tauri (реализовано)

`crates/desktop` — тонкая Tauri v2-оболочка над `dshr serve`. Она не добавляет
второй протокол и не отдаёт нативные API в страницу: процесс поднимает loopback
сервер со свежим bearer-токеном, ждёт `/v1/health` и открывает Leptos UI в системном
webview. Токен подставляется в URL (`?token=…`) и сохраняется в `localStorage`.

```sh
CARGO_TARGET_DIR=target cargo build --release --manifest-path crates/desktop/Cargo.toml
./target/release/dshr-desktop
# headless-проверка без окна:
./target/release/dshr-desktop --check
```

Крейт вынесен из workspace (как `crates/web`), чтобы Tauri/WebKit не попадали в
Linux-CI; на macOS его тесты запускаются отдельным шагом CI.

`DSHR_BIN` задаёт путь к `dshr`, иначе берётся соседний бинарь или `PATH`;
`DSH_HOME` задаёт home, `DSHR_DESKTOP_PORT` — фиксированный порт (0 = случайный),
а первый позиционный аргумент — папка workspace (cwd сервера). Drag-and-drop не поддержан.
Закрытие окна скрывает его в трей, а Quit завершает сервер.

Трей: Show/Hide, Open in browser, Open terminal, Open config folder, Restart in
safe mode / Restart normally,
New profile…, Export diagnostics…, Check for updates…, Profile (список
`default` + `<home>/profiles/*`, переключение через `desktop-profile` и рестарт
shell) и Quit. Окно: трей Window → Compatibility/Enhanced и Material (macOS vibrancy, прозрачность).
Настройки хранятся в `<home>/desktop-window.json`; смена перезапускает shell.

Safe mode пишет маркер `<home>/desktop-safe-mode` и передаёт `--safe-mode` серверу
(пропускает MCP/Cordis/WASM-плагины); «Restart normally» убирает маркер.

Первый запуск создаёт home, `desktop-profile=default` и README. Если провайдер не
настроен в окружении, при старте открывается нативный setup (провайдер, модель,
API key, base URL для OpenAI-совместимого); настройки пишутся в `<home>/desktop.env`
с правами `0600` и передаются дочернему `dshr serve`. Tray → Provider setup… меняет
их и перезапускает shell. Ошибка старта показывается нативным диалогом. `--export-diagnostics` собирает `system-info.txt`,
`desktop.log`, вывод `dshr doctor` и упаковывает в `dshr-diagnostics-*.tar.gz`.

Сборка `.app` (macOS):

```sh
scripts/package-desktop.sh 0.1.0
# dist/dshr.app — ad-hoc подпись, внутри оба бинаря
./dist/dshr.app/Contents/MacOS/dshr-desktop --check
```

`--check` проверяет запуск sidecar-сервера без окна. Обновления: «Check for updates…» и фоновая проверка GitHub Releases; «Download update…»
скачивает macOS zip, сверяет SHA256SUMS, распаковывает в `<home>/updates/<version>` и
показывает в Finder. Тихая установка/замена приложения не выполняется — пользователь
заменяет `.app` сам (как при открытии DMG в оригинале). Recovery-окна и режимы окна
(compatibility/extended/enhanced) пока не реализованы — это следующий
срез desktop; матрица — [PARITY.md](PARITY.md).

## PWA

Web UI отдаёт PWA-манифест и иконки; установите его как отдельное приложение из браузера:

```sh
dshr serve --port 4317
# открыть http://127.0.0.1:4317, затем «Установить приложение» (Chrome/Edge)
```

Что есть:
- `manifest.webmanifest`, `display: standalone`;
- иконки 192/512;
- офлайн-режим не поддержан: UI работает только при запущенном `dshr serve`.
