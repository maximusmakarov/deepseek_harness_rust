# Desktop

## Tauri (реализовано)

`crates/desktop` — тонкая Tauri v2-оболочка над `dshr serve`. Она не добавляет
второй протокол и не отдаёт нативные API в страницу: процесс поднимает loopback
сервер со свежим bearer-токеном, ждёт `/v1/health` и открывает Leptos UI в системном
webview. Токен подставляется в URL (`?token=…`) и сохраняется в `localStorage`.

```sh
cargo build -p dshr-desktop --release
./target/release/dshr-desktop
# headless-проверка без окна:
./target/release/dshr-desktop --check
```

`DSHR_BIN` задаёт путь к `dshr`, иначе берётся соседний бинарь или `PATH`.
Серверный процесс завершается вместе с окном.

Сборка `.app` (macOS):

```sh
scripts/package-desktop.sh 0.1.0
# dist/dshr.app — ad-hoc подпись, внутри оба бинаря
./dist/dshr.app/Contents/MacOS/dshr-desktop --check
```

`--check` проверяет запуск sidecar-сервера без окна. Трей и автообновление пока не
настроены — это следующий срез desktop.

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
