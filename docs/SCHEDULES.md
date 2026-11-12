# Schedules

Расписания — durable-записи в `~/.dshr/schedules.yaml`. Cron-выражения в **UTC** и **с секундами** (6 полей, как в крейте `cron`): `0 */5 * * * *` — каждые 5 минут.

```sh
dshr schedule add --name nightly --cron "0 0 2 * * *" --prompt "Run the nightly checks and summarize"
dshr schedule list
dshr schedule disable nightly
dshr schedule remove nightly
```

Проверить и выполнить должное один раз (удобно для cron/systemd):

```sh
dshr schedule run-due
# или на фиксированный момент:
dshr schedule run-due --now 2030-01-01T00:00:00Z
```

Автономный цикл:

```sh
dshr schedule daemon --interval-secs 30
```

Поведение:

- `run-due` берёт записи с `next_run <= now`, создаёт новую сессию (или открывает `--session`), выполняет prompt и сдвигает `next_run`.
- Заголовок сессии — `schedule: <id>`; сессии видны в `sessions list`.
- Автономный запуск не спрашивает approval: используется `AutoApprover` и `WorkspaceWrite` + sandbox. `--danger` снимает ограничения — применять осознанно.
- Ошибка шага не блокирует следующие запуски: `next_run` всё равно сдвигается, ошибка печатается.
- Два демона одновременно не координируются; для продакшена запускайте один экземпляр или используйте `run-due` из системного cron.
