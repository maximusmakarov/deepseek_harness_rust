# Клиентская половина: паритет с DSH 0.2.2-rc.2

Оценка по методике Replica (`replica-diff`): матрица `client-half-features.csv`
против нашего моста iframe. Скоринг считает must=3, should=2, could=1, partial —
половина, `skip` и наши собственные фичи не входят.

Матрица: [client-half-features.csv](client-half-features.csv)

## Итог

- **Parity: 96.1 / 100**
- must-haves: **12 из 12** — по правилам Replica это «shippable» (все must сделаны,
  score 80+, открытых S1/S2 нет)
- should: 2 partial (см. ниже), could: 1 done
- сознательно вне скоупа (`skip`): React-компоненты, инъекция в DOM шелла,
  внешние модули (`dsh.client.external`) — следствие ADR-013 §5, а не недоделка

Что осталось из should (не блокирует):

| Фича | Состояние | Чем отличается |
|---|---|---|
| Activation ordering by inject | partial | `immediately`-половины монтируются в порядке зависимостей `inject`; ленивая половина активируется по требованию |
| Bundles built for the reference module table | partial | шим `__ModuleLoader__.load({id, factory})` исполняет `factory` и `apply(ctx)`; React из таблицы модулей недоступен |

## Чем это подтверждено

| Область | Доказательство |
|---|---|
| discovery | `crates/cordis/src/client.rs`: `discover_client_half`, тесты на entry/exports/escape; серверный тест `/v1/plugins/clients` |
| security | канонизация пути, confinement пакета, тест traversal; capability-ассеты проверяются на чужой/просроченный cap (серверный тест) |
| slots | `tests/bridge_harness.mjs` (порядок, группы `inject`, `list()`), smoke: вкладки и переключение слотов |
| lifecycle | harness: `activate`/`deactivate`/`dispose`, `effect` cleanup; smoke: eager-кадр переживает закрытие, горячая перезагрузка панели |
| theme | harness: `onTheme`; smoke: переключение темы доходит до кадра |
| actions | harness: round-trip `action-result`; smoke: `close`, `reload`, `compose`, `notify`, неизвестное действие |
| loading | серверные тесты inline и capability-ассетов; smoke: многофайловый бандл и module-table бандл рендерятся |
| layout | smoke: `resize` выставляет высоту кадра |

Команды:

```sh
cargo test -p dshr-cordis          # bridge harness + assets + manager
cargo test -p dshr-server --test plugins_live
python3 scripts/plugin-smoke.py    # 25 проверок в headless Chrome
python3 parity.py docs/replica/client-half-features.csv
```

## Что бы сделал следующий раунд

1. Полный граф `inject` и для ленивых половин (сейчас порядок только у eager).
2. Общий стол модулей между половинками, если появится запрос на React-совместимость
   (против ADR-013 §5, потребует пересмотра решения).
3. Conformance-набор из реальных npm-пакетов вместо фикстур, когда такие появятся в
   экосистеме DSH.
