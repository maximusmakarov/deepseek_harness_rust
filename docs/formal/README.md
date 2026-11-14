# Формальные модели (TLA+)

Модели описывают конкурентные протоколы, где обычные unit-тесты проверяют лишь
несколько сценариев. Каждая модель — пара `.tla` + `.cfg`; TLC проверяет
инварианты (safety) и, где осмысленно, темпоральные свойства (liveness).

```sh
./formal/run_tlc.sh            # все пары, локальный JDK 17+ и tla2tools.jar
TLA2TOOLS_JAR=/path/tla2tools.jar ./formal/run_tlc.sh
```

`run_tlc.sh` ищет `tla2tools.jar` в `formal/tools/`, `$TLA2TOOLS_JAR` и нескольких
стандартных местах; `JAVA_HOME` или `/opt/homebrew/opt/openjdk@21` задают JDK.

## Модели

| Модель | Что формализует | Конфиги | Инварианты |
|---|---|---|---|
| `CordisSupervisor.tla` | жизненный цикл sidecar'ов и hot-swap реестра инструментов (`CordisManager`, `ToolRegistry`) | safety (`AllowReload=TRUE`, `AllowFailure=TRUE`), liveness (`AllowReload=FALSE`, `AllowFailure=FALSE`) | `RegisteredImpliesStarted`, `StoppedHasNoTools`, `RunningGenerationMatches`, `GenerationSound`, `InflightSound`; liveness `<>[] enabled ~> <>[] running`, `<>[] disabled ~> <>[] stopped` |
| `ClientAssets.tla` | capability-билеты для ассетов клиентской половины | `MaxRequests=3`, `MaxTick=4`, `TTL=2` | `ServedIssued`, `ServedConfined`, `ServedWithinTTL`, `OutsideNeverAllowed`, `UnissuedNeverAllowed`, `LateNeverAllowed`, `OneTicketPerCapability`, `IssuedExactness` |
| `ApprovalGate.tla` | approval-gate агентского цикла | safety (`Policy="ask"`), liveness (каждое pending-решение рано или поздно принято) | `NoUnapprovedExecution`, `ExecutedNotDenied`, `ApprovalDecisionDisjoint`, `PendingUndecided`, `FinishedTurnClean`; liveness `PendingEventuallyDecided` |

## Абстракции и что не моделируется

- `CordisSupervisor`: набор инструментов плагина — один атомарный элемент реестра
  (`registered[p]`); сам реестр — множество таких элементов. `MaxGen` — бюджет
  состояний, а не ограничение реализации (u64-генерации не переполняются).
  Релоад-шторм и перезапуск процесса не моделируются: liveness проверяется при
  стабильном конфиге и выключенном триггере reload.
- `ClientAssets`: один плагин; `InsidePaths`/`OutsidePaths` — абстракция
  принадлежности файла пакету. Криптография билета не моделируется (билет —
  непрозрачный идентификатор; TTL и scope — моделируются).
- `ApprovalGate`: инструменты — атомарные действия без аргументов; политика
  `never` соответствует `DangerFull`-режиму, `ask` — approvals в UI.

## Результаты

Все пять пар проходят TLC без нарушений (`./formal/run_tlc.sh` → `all models
checked`). Предупреждения TLC о совпадении fingerprint'ов в больших прогонах —
артефакт 64-битных хешей, а не ошибка модели.

Найденные моделью дефекты формализации (исправлены в моделях, не в коде):
`Start` без снятия stale-инструментов нарушал `RegisteredImpliesStarted`;
`Stop` при включённом желании и `WF(Stop)` делали недостижимой liveness;
`Cancel` без очистки `pending` нарушал `FinishedTurnClean`.

## Связь с тестами

Каждое действие модели имеет тест-кейс в
[`docs/testing/test-cases.md`](../testing/test-cases.md) (секции `SUP`, `REG`,
`CLI`, `AGT`). Модель — источник сценариев, тест — исполняемая проверка.
