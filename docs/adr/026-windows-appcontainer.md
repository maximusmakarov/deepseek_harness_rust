# 026. Windows sandbox: AppContainer через helper

- **Статус:** Accepted
- **Дата:** 2026-11-13

## Статус: experimental / opt-in

AppContainer реализован, но setup зависает на GitHub Windows runner; по умолчанию Windows остаётся fail-closed, включение — `DSHR_WINDOWS_SANDBOX=1`. Диагностический тест исключён из CI, чтобы не блокировать job.

## Контекст

Windows оставался fail-closed: `bash`/jobs не работали. Полноценный AppContainer FFI (security capabilities, ACL, CreateProcess) — большой и рискованный код; писать его без Windows-раннера в цикле разработки опасно.

## Решение

1. **Helper-процесс:** `dshr __sandbox-exec` на Windows запускает целевой процесс через `sandboxrs-windows` 0.1.1 (pin exact): он пробует Windows Sandbox API, затем AppContainer, иначе fail-closed.
2. **Права:** workspace — read-write; temp — read-write; `SystemRoot`, `ProgramFiles`, `ProgramData`, `.cargo`, `.rustup` — read-only. `read-only`-режим использует пустой temp как базу, а workspace добавляет read-only.
3. **Изоляция:** AppContainer без capabilities; сеть не гарантируется, поэтому enforcement помечен `Partial`.
4. **Fail-closed:** если sandboxrs не может поднять ни один backend, helper завершается с кодом 125; команда не выполняется.
5. **MSRV:** зависимость требует Rust 1.90; workspace MSRV поднят с 1.89 до 1.90.

## Последствия

- Windows shell и jobs работают в AppContainer, а не отказывают.
- Появляется сторонняя зависимость в security-пути; версия закреплена, ADR фиксирует границы. При обновлении — прогонять escape suite на Windows CI.
- Сеть на Windows не является гарантией; это отражено в `Partial`.

## Альтернативы

- Собственный AppContainer FFI — отклонено: объём и риск без локального Windows.
- Job Objects без ACL — отклонено: не даёт файловой изоляции.
- Оставить fail-closed — отклонено: основной сценарий на Windows не работал.
