# Участие в разработке `dshr`

## Требования

- Rust stable (edition 2024, MSRV 1.89; проект разрабатывается на 1.100).
- `rustup` ставит `rustfmt` и `clippy` из `rust-toolchain.toml`.
- Для интеграционных тестов ничего дополнительно не нужно: сеть и токены не используются.

## Локальная проверка

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Опционально, как в CI:

```sh
cargo install cargo-nextest --locked
cargo nextest run --workspace
cargo deny check
```

## Структура

```text
crates/core     идентификаторы, ошибки, редакция секретов
crates/config   слои конфигурации, схема, пути
crates/cli      бинарь dshr: doctor, config show
docs/           план, архитектура, ADR, проверки
```

Правила зависимостей: `cli -> config -> core`, обратных стрелок нет.

## Стиль

- `#![forbid(unsafe_code)]` во всех крейтах; FFI — только отдельным крейтом с обоснованием.
- Ошибки: `dshr_core::DshrError` в библиотеках, коды стабильны; паники — только для нарушенных инвариантов.
- Публичные типы документируются; поведение, влияющее на проводной формат, фиксируется ADR.
- Никаких секретов в коде, тестах, фикстурах и логах. Токены читаются из env и проходят через `dshr_core::redact`.
- Новый инструмент/провайдер — регистрация, а не правка ядра.

## Коммиты и PR

- Conventional commits: `feat:`, `fix:`, `docs:`, `test:`, `refactor:`, `chore:`.
- Один PR — одна логическая задача; описание содержит риск и способ проверки.
- Перед PR: fmt, clippy, test зелёные; для изменений контрактов — ADR.
- Опасные зоны (песочница, approvals, секреты, сеть) требуют отдельного ревью.

## Definition of Done

Код + тест + документация + observability (лог/метрика) + отсутствие `TODO` без issue.

## Лицензия

Отправляя PR, вы соглашаетесь с лицензией проекта MIT OR Apache-2.0.
