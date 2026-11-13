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

`ui.language` влияет на подсказки, статусы и approval-диалог TUI. CLI-справка и сообщения остаются английскими; `run`/`sessions` вывод не локализован.
