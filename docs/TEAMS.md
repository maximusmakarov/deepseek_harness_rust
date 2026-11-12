# Agent teams

Team — YAML-файл с задачами-DAG и числом воркеров. Воркеры — субагенты; они атомарно захватывают готовые задачи с общей доски, поэтому одна задача не выполняется дважды.

Каталоги: `~/.dshr/teams` и `<cwd>/.dshr/teams`.

```yaml
name: release
workers: 3
tasks:
  - id: inspect
    title: Inspect changes
    prompt: Inspect the repository and list risks.
  - id: tests
    title: Run tests
    prompt: Run the test suite and report failures.
  - id: report
    title: Write report
    prompt: |
      Combine the inspection and test results into a release report.
      Inspection: {{inspect.output}}
      Tests: {{tests.output}}
    depends_on: [inspect, tests]
```

Запуск:

```sh
dshr run --danger "use team release with input 'HEAD~1'"
```

Как это работает:

1. Задача готова, когда все её `depends_on` в статусе `done`.
2. `claim_next` под мьютексом переводит задачу в `in_progress` и закрепляет за воркером.
3. Воркер получает prompt задачи + выводы зависимостей + вход команды.
4. Успех → `done` с результатом; ошибка → `failed`, зависимые остаются заблокированными.
5. Воркеры работают конкурентно (до 8); отчёт содержит статусы, владельцев и результаты.

Ограничения M7: доска живёт в памяти запуска, задачи нельзя добавлять во время выполнения, бюджеты воркеров не заданы, merge/PR не автоматизируются.
