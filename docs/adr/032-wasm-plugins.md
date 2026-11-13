# 032. WASM-плагины: wasmi и JSON ABI

- **Статус:** Accepted
- **Дата:** 2026-11-13

## Контекст

Нужен безопасный плагинный тир без Node/WASI. Полный component model/WIT на wasmtime — тяжёлая зависимость и отдельный toolchain; для M7 достаточно изолированных вычислительных инструментов.

## Решение

1. **wasmi 2.0** — чистый Rust-интерпретатор, fuel metering, без JIT.
2. **ABI `dshr.wasm.v1`:** `list_tools() -> i64`, `call_tool(i32,i32) -> i64`, опциональный `alloc`; JSON в памяти гостя, результат `ptr<<32|len`.
3. **Без host-импортов:** нет WASI, файлов, сети, времени; только вычисления.
4. **Манифест** `*.yaml` с `name`, `wasm`, `enabled`; каталоги `~/.dshr/plugins` и `.dshr/plugins`.
5. **Sidecar** — это MCP: `plugin add --command/--url`, без отдельного протокола.
6. **Границы:** fuel 10M, memory cap 64 MiB, инстанцирование на вызов, `sha256` манифеста; Ed25519-подпись (`dshr-wasm`, ключи доверия в `plugins.trusted_keys`); hot reload нет.

## Последствия

- Плагины не требуют wasmtime/WASI и работают на всех целевых ОС.
- Нет состояния между вызовами; для stateful-плагинов нужен sidecar.
- WIT/component model, hot reload и capability-grants отложены.
- Подпись не решает отзыв ключей и доверие к источнику плагина — это следующий слой.

## Альтернативы

- wasmtime + WIT — отложено: размер и toolchain.
- Полноценный WASI — отклонено: даёт файловый/сетевой доступ без capability-модели.
- Отдельный sidecar-протокол — отклонено: MCP уже покрывает.
