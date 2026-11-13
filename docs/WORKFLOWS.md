# Workflows

Workflow — YAML-файл с DAG шагов. Каждый шаг выполняется дочерним агентом; результат предыдущих доступен через `{{step_id.output}}`, вход — через `{{input}}`.

Каталоги поиска: `~/.dshr/workflows` и `<cwd>/.dshr/workflows`.

```yaml
name: review
description: Inspect a change, then summarize risks
steps:
  - id: inspect
    prompt: |
      Inspect the repository at {{input}}. List changed files and risks.
  - id: summarize
    prompt: |
      Summarize this inspection for a reviewer:
      {{inspect.output}}
    depends_on: [inspect]
    retries: 1
```

Запуск:

```sh
dshr run --danger "use workflow review with input '.'"
```

- Шаги выполняются в топологическом порядке; цикл или неизвестная зависимость — ошибка загрузки.
- `retries` (0–10) повторяет шаг при ошибке; после исчерпания ошибка содержит число попыток.
- Шаги — отдельные сессии субагентов; `workflow` не регистрируется в дочернем реестре, поэтому вложенность ограничена.
- `workflow` требует approval, как и `subagent`; в headless используйте `--yes`/`--danger`.
