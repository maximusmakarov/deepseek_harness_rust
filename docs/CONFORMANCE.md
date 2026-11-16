# Conformance: реальные плагины DSH под sidecar dshr

Матрица снята 16.11.2026 прогоном [scripts/conformance.py](../scripts/conformance.py):
для каждого пакета создаётся чистый `<home>`, ставится `dshr plugin npm add`,
затем `dshr prompt` монтирует host-половину через sidecar, а из вывода
собираются имена инструментов (`cordis__<plugin>__*`) и команд
(`cmd__<plugin>__*`) и все предупреждения.

Node: pack `node@22.11.0` (кроме случая, отмеченного ниже). Установка идёт из
публичного registry, поэтому числа пакетов включают peer'ы и транзитивные
зависимости.

| Плагин | Установка | Пакетов | Инструменты | Команды | Итог |
|---|---|---|---|---|---|
| `dsh-remote@1.2.0` | ok | 59 | **21** (`rw_*`) | 3 (`remote`, `remote-forget-key`, `remote-ignore`) | работает |
| `dsh-mnemon@0.5.46` | ok | 54 | **3** (`mnemon_view_route`, `mnemon_view_action`, `mnemon_subagent_result`) | 1 (`mnemon`) | работает |
| `dsh-plugin-debugging@0.8.1` | ok | 36 | **8** (`dsh_diagnose_plugin_load`, `dsh_dump_config`, `dsh_hot_reload`, …) | — | работает |
| `dsh-session-superagent@0.10.12` | ok | 12 | — | — | нужен Node ≥ 22.15: плагин импортирует `zstdDecompressSync` из `node:zlib`; на node 22.11 sidecar подсказывает про Node, с Node 26 монтируется |
| `@kt11/dsh-deepseek-balance@0.2.1` | ok | 1 | — | — | host-половина без инструментов: отдаёт HTTP-маршруты (`/v1/plugins/...`), проверено в C1 |
| `@ruoyang/dsh-plugin-deepseek-balance@1.0.2` | ok | 1 | — | — | host-половина без инструментов |
| `dsh-genui@0.3.3` | ok | 122 | — | — | клиентская половина (UI), host-инструментов нет |
| `dsh-plugin-codegraph-panel@0.6.5` | ok | 2 | — | — | клиентская половина (панель), host-инструментов нет |
| `dsh-plugin-studio@0.3.1` | ok | 14 | — | — | клиентская половина (настройки), host-инструментов нет |
| `dsh-plugin-notify@0.1.1` | ok | 467 | **2** (`notify_user`, `user_activity`) | 1 (`notify`) | работает (большое дерево: ~6,5 мин на установку) |
| `dsh-plugin-market@1.3.0` | **ошибка** | 0 | — | — | апстрим-пакет содержит `workspace:^`/`link:` спеки: из registry не ставится (ошибка резолвера называет зависимость) |

## Что это значит

- **Четыре плагина с host-инструментами работают end-to-end** (`dsh-remote`,
  `dsh-mnemon`, `dsh-plugin-debugging`, `dsh-plugin-notify`): sidecar поднимает их половину, отдаёт
  инструменты агенту и исполняет их; `dsh-remote` дополнительно публикует
  команды, которые видны агенту (`cmd__dsh-remote__*`) и по HTTP
  (`/v1/commands`).
- **Четыре плагина — клиентские половины** без host-инструментов: их UI
  монтируется отдельным треком C-lite (React/слоты), в этой матрице проверяется
  только то, что host-половина не ломает запуск.
- **Один плагин требует более новый Node** (`dsh-session-superagent`): sidecar
  теперь подсказывает это в предупреждении («the plugin needs a newer Node»),
  а `dshr runtime install node@24 …` закрывает требование.
- **Один плагин не ставится по вине апстрима** (`dsh-plugin-market`): в
  опубликованном пакете остались `workspace:^` и `link:` спеки. Резолвер
  отказывает с внятной ошибкой, а не ставит мусор.

## Как повторить

```sh
# один раз: runtime pack (или DSHR_NODE с системным node)
dshr runtime install node@22.11.0 --from <tar.gz> --sha256 <hex>
# матрица
python3 scripts/conformance.py --dshr ./target/debug/dshr \
    --node-pack ~/.dshr/runtimes --out /tmp/conformance
```

Скрипт пишет `conformance.json` (машинный формат) и печатает таблицу; для
регулярного прогона его стоит вызывать в nightly-CI с `DSHR_NODE` вместо пака.
