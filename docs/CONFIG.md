# Конфигурация

Слои: defaults < profile < user < patch. Схема: `dshr config show --schema`.

```yaml
model:
  provider: deepseek-account
  name: deepseek-flash
session:
  dir: ~/.dshr/sessions
  auto_title: true   # LLM-заголовок при первом prompt (run)
tools:
  output_limit_chars: 30000
ui:
  language: ru   # ru | en
plugins:
  trusted_keys: []           # Ed25519 public keys (32-byte hex)
  cordis:                    # JS/npm plugins через node-sidecar
    - name: demo
      entry: plugins/demo.mjs
      enabled: true
logging:
  level: info
  format: json
```

`ui.language` влияет на подсказки, статусы и approval-диалог TUI. Для CLI язык задаётся `--lang en|ru` или `DSHR_LANG`; локализованы верхнеуровневая справка, аргументы `run`/`sessions`, вывод `sessions`, `doctor`, `plugin`, `schedule`, `run`-сводка и `serve`. Часть справки по остальным командам и ошибки остаются английскими.

`plugins.trusted_keys` включает режим подписи WASM-плагинов. Пустой список — подписи не проверяются (подписанный плагин без ключей отклоняется); непустой список — каждый WASM-плагин обязан иметь `signature` в манифесте, которая проверяется по одному из ключей. Отсутствие подписи или чужой ключ означает отказ загрузки. Ключи и подписи создаются командами `dshr plugin keygen` и `dshr plugin sign` — см. [PLUGINS.md](PLUGINS.md).
