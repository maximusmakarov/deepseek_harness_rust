# Browser / CUA

Headless Chrome через `chromiumoxide`.

## Требование

Chrome/Chromium должен быть установлен. Поиск: `$CHROME`, `$CHROME_PATH`, затем стандартные пути macOS/Linux (`/Applications/Google Chrome.app/...`, `/usr/bin/google-chrome`, `/usr/bin/chromium`).

## Инструменты

| Инструмент | Аргументы | Действие |
|---|---|---|
| `browser_open` | `url` | перейти на http(s)/data/about |
| `browser_screenshot` | `path?` | PNG текущей страницы |
| `browser_click` | `selector` | клик по CSS-селектору |
| `browser_type` | `selector`, `text` | клик и ввод текста |
| `browser_eval` | `script` | выполнить JS, вернуть JSON |
| `browser_close` | — | закрыть браузер |

Скриншоты по умолчанию: `.dshr/screenshots/screenshot-<uuid>.png`; путь возвращается в `files`.

## Поведение

- Браузер стартует лениво при первом вызове и живёт до `browser_close`/конца процесса.
- Запуск: `--headless=new --no-sandbox --disable-gpu --disable-dev-shm-usage`; Chrome sandbox отключён (нужно для CI/root).
- Селекторы — CSS; клик/ввод через DOM, без координатной сетки.
- Ошибки Chrome возвращаются как tool error; автоматического перезапуска нет.

## Безопасность

- `browser_eval` выполняет произвольный JS, `browser_open` ходит в сеть (включая localhost) — инструменты требуют approval.
- Нет SSRF-фильтра: CUA по определению работает с произвольными URL.
- Скриншоты и страницы могут содержать секреты; не публикуйте их без проверки.
