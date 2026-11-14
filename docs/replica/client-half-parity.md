# Клиентская половина: паритет с DSH 0.2.2-rc.2

Оценка по методике Replica (`replica-diff`): матрица `client-half-features.csv`
против нашего моста iframe. Скоринг считает must=3, should=2, could=1, partial —
половина, `skip` и наши собственные фичи не входят.

Матрица: [client-half-features.csv](client-half-features.csv)

## Итог

- **Parity: 98.1 / 100** (53 из 54 баллов)
- must-haves: **13 из 13** — по правилам Replica это «shippable» (все must сделаны,
  score 80+, открытых S1/S2 нет)
- should: 6 done, 1 partial; could: 1 done в скоринге + 3 dshr-дополнения вне
  скоринга (deep link, plugin-shipped runtime и shared host module table)
- сознательно вне скоупа (`skip`): React-компоненты, инъекция в DOM шелла,
  внешние модули (`dsh.client.external`) — следствие ADR-013 §5, а не недоделка;
  путь к полной React-совместимости — [ADR-040](../adr/040-react-client-half.md)
  (Proposed: тир C после PoC тира B)

Арифметика по документированным весам (13 must, 7 should, 1 could; один should —
partial, то есть половина):

| Компонент | Баллы | Максимум |
|---|---|---|
| must done: 13 × 3 | 39 | 39 |
| should done: 6 × 2 | 12 | 12 |
| should partial: 1 × 2 × ½ | 1 | 2 |
| could done: 1 × 1 | 1 | 1 |
| **Итого** | **53** | **54** |

53 / 54 = 98.15 → **98.1**. Ровно 100 нет из-за единственного partial should:
module-table бандл исполняется, React от хоста теперь доступен opt-in
(`dsh.client.host_modules`, раунд 78), но полной таблицы модулей DSH
(design-system, React-контекст шелла) по-прежнему нет.

Строка раунда 77 `Plugin-shipped runtime/module table (bring your own React)` —
**dshr-дополнение** (`original=no`, как и deep link): enabler `dsh.client.runtime`
позволяет половине принести свою модульную таблицу и, например, собранный React.
В скоринг такие строки не входят, поэтому parity не меняется — те же 53 из 54.
Если бы строка считалась фичей оригинала, было бы 54/55 = 98.18 → 98.2; в матрице
она намеренно помечена `original=no`.

Строка раунда 78 `Shared host module table (React served by the host)` — тоже
**dshr-дополнение** (`original=no`): половина опционально берёт React и таблицу
модулей от хоста, host-страница грузит один версионированный immutable-артефакт
`/v1/runtime/react-18.3.1.js` вместо встраивания ~140 КБ в каждую страницу.
Скоринг от неё не меняется: must 13/13 (39 из 39), should 6 done (12 из 12) и
1 partial (1 из 2), could 1 done (1 из 1) — **53 из 54 = 98.15 → 98.1**.

Что осталось из should (не блокирует):

| Фича | Состояние | Чем отличается |
|---|---|---|
| Bundles built for the reference module table | partial | шим `__ModuleLoader__.load({id, factory})` исполняет `factory` и `apply(ctx)`; `require` резолвит модули из `dsh.client.runtime`, если половина их опубликовала (в PoC — React 18.3.1), или из общего рантайма хоста, если половина опционально объявила `dsh.client.host_modules` (React 18.3.1). Полной таблицы модулей DSH (design-system, React-контекст шелла) нет — это единственная незакрытая should-фича, поэтому score 98.1, а не 100 |

## Чем это подтверждено

| Область | Доказательство |
|---|---|
| discovery | `crates/cordis/src/client.rs`: `discover_client_half`, тесты на entry/exports/escape; серверный тест `/v1/plugins/clients` |
| security | канонизация пути, confinement пакета, тест traversal; capability-ассеты проверяются на чужой/просроченный cap (серверный тест) |
| slots | `crates/cordis/src/bridge.js`: таблица `SLOT_REGIONS` — 116 ключей DSH 0.2.2-rc.2 плюс legacy `panel`/`message.action`, 17 регионов, fallback `panel`; `tests/bridge_harness.mjs` (порядок, группы `inject`, `list()`, slot-режим); smoke: вкладки и переключение слотов |
| regions | eager registry-кадр на половину (`mode=registry`) и iframe `mode=slot` на регистрацию (`crates/web/src/main.rs`: `SlotRegion`/`SlotFrame`/`RegistryFrames`/`PanelRegion`); smoke: монтаж в каждый регион, fallback незнакомого имени, `context` повторяющихся регионов, закрытие диалога снимает его фреймы |
| lifecycle | harness: `activate`/`deactivate`/`dispose`, `effect` cleanup; smoke: eager-кадр переживает закрытие, горячая перезагрузка панели |
| theme | harness: `onTheme`; smoke: переключение темы доходит до всех фреймов |
| actions | harness: round-trip `action-result` с адресацией отправителю; smoke: `close`, `reload`, `compose`, `notify`, неизвестное действие |
| loading | серверные тесты inline и capability-ассетов; smoke: многофайловый бандл и module-table бандл рендерятся |
| runtime | `crates/cordis/src/client.rs`: `ClientHalf.runtime`/`runtime_path`, `host_page`/`host_page_assets` встраивают runtime до бандла, `requireShim` резолвит `window.__DSHR_MODULES__`; `crates/server/src/lib.rs`: `/host` читает и валидирует runtime (confined к пакету, ≤ 8 МиБ, `</script>` → 400); тесты: cordis +4, `plugins_live` +3, smoke +3 (discovery, runtime-before-bundle, require render); React 18.3.1 PoC (UMD в `runtime.js`, 143 197 байт) — рендер, `useState`, `ctx.onTheme` проверены вручную |
| shared runtime | `crates/cordis/src/client.rs`: `HOST_MODULES`, `HOST_RUNTIME_VERSION`, `host_runtime_url`, `PageRuntime` (порядок shared → plugin runtime → bundle); `crates/server/src/lib.rs`: `GET /v1/runtime/{file}` (immutable-кэш, 404 на неизвестную версию), `client_host_csp` (origin сервера только для opt-in кадров); `crates/server/assets/runtime/` + `scripts/check-runtime-assets.sh`; тесты: cordis +4, `plugins_live` +3, smoke +5 (discovery `host_modules`, immutable-артефакт, host-страница без встроенного React, React из общего рантайма, `useState`) |
| layout | smoke: `resize` выставляет высоту конкретного кадра (кламп 24–640 px) |

Команды:

```sh
cargo test -p dshr-cordis          # bridge harness (каталог слотов, регионы, slot-режим) + assets + runtime + shared host runtime + manager
cargo test -p dshr-server --test plugins_live
python3 scripts/plugin-smoke.py    # 53 проверки в headless Chrome
python3 parity.py docs/replica/client-half-features.csv
```

## Что бы сделал следующий раунд

1. Тир C по [ADR-040](../adr/040-react-client-half.md): React-остров в Leptos и
   SlotMap-подмножество — только если появится запрос на экосистему DSH UI;
   PoC тира B (общий React/таблица модулей от хоста) сделан в раунде 78.
2. Расширение host-таблицы до контракта DSH: design-system
   `@deepseek-ai/dsh-client-ui-*`, React-контекст шелла, композиция между
   плагинами (всё — тир C; D не рекомендуется: переписывание шелла).
3. Conformance-набор из реальных npm-пакетов вместо фикстур, когда такие появятся в
   экосистеме DSH.
