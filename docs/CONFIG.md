# Конфигурация

Слои: defaults < profile < user < patch. Схема: `dshr config show --schema`.

```yaml
model:
  provider: deepseek-account
  name: deepseek-flash
session:
  dir: ~/.dshr/sessions
tools:
  output_limit_chars: 30000
ui:
  language: ru   # ru | en
logging:
  level: info
  format: json
```

`ui.language` влияет на подсказки, статусы и approval-диалог TUI. Для CLI язык задаётся `--lang en|ru` или `DSHR_LANG`; локализованы верхнеуровневая справка, аргументы `run`/`sessions`, вывод `sessions`, `doctor`, `plugin`, `schedule`, `run`-сводка и `serve`. Часть справки по остальным командам и ошибки остаются английскими.
