# 031. Browser/CUA: headless Chrome

- **Статус:** Accepted
- **Дата:** 2026-11-13

## Контекст

Нужны навигация, скриншоты и действия на страницах. Playwright требует Node; писать CDP с нуля — большой объём. Chrome уже есть на целевых машинах.

## Решение

1. **`chromiumoxide`** управляет headless Chrome по CDP; браузер стартует лениво и живёт до закрытия.
2. **Инструменты:** `browser_open`, `browser_screenshot`, `browser_click`, `browser_type`, `browser_eval`, `browser_close`.
3. **Запуск:** `--headless=new --no-sandbox --disable-gpu --disable-dev-shm-usage`; путь к Chrome из `CHROME`/`CHROME_PATH` или стандартных мест.
4. **Скриншоты:** PNG в `.dshr/screenshots` или указанный путь; ToolOutcome.files.
5. **Границы:** нет координатных кликов, drag&drop, вкладок, загрузки файлов, auto-restart; SSRF не фильтруется.

## Последствия

- CUA работает без Node и без отдельного сервера.
- Chrome sandbox отключён ради CI/root; страница недоверенная, но процесс браузера — нет.
- Инструменты требуют approval; `browser_eval` — произвольный JS.

## Альтернативы

- Playwright — отклонено: Node-зависимость.
- Свой CDP-клиент — отложено: chromiumoxide уже покрывает протокол.
- WebDriver — отклонено: дополнительный драйвер.
