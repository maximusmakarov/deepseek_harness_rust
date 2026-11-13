# Поиск по сессиям

ADR-016 задаёт гейт: по умолчанию — скан; индекс включается при превышении порога.

## Бенчмарк

Синтетические сессии (`crates/search/tests/benchmark.rs`, ignored-тест):

```sh
DSHR_BENCH_SESSIONS=1000 DSHR_BENCH_EVENTS=20 \
  cargo test -p dshr-search --test benchmark -- --ignored --nocapture
```

Результат на локальной машине (1 000 сессий, 20 000 событий):

```text
scan      121 ms
index     280 ms (построение)
fts        36 ms
```

p95 скана превышает 100 мс → гейт разрешает FTS5.

## Использование

```sh
# построить/перестроить индекс
dshr sessions reindex

# поиск по индексу
dshr sessions search "unique needle" --fts

# поиск сканом (по умолчанию)
dshr sessions search "unique needle"
```

Индекс: `<session.dir>/search.sqlite`, таблица FTS5 с `tokenize='trigram'`.
Короткие запросы (<3 символов) автоматически идут сканом.

## Ограничения

- Индекс не обновляется автоматически при `run`; после новых сессий нужен `sessions reindex`.
- Trigram не делает стемминг/prefix; это подстрочный поиск.
- Один индекс на store; конкурентная запись SQLite не координируется между процессами.
- Скан остаётся источником истины и дефолтом до явного `--fts`.
