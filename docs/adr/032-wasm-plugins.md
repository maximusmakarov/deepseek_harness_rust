# 032. WASM-плагины: wasmi и JSON ABI

- **Статус:** Accepted
- **Дата:** 2026-11-13

## Контекст

Нужен безопасный плагинный тир без Node/WASI. Полный component model/WIT на wasmtime — тяжёлая зависимость и отдельный toolchain; для M7 достаточно изолированных вычислительных инструментов.

## Решение

1. **Бэкенды:** wasmi 2.0 для legacy core-модулей (чистый Rust, fuel metering) и wasmtime 50 для component model/WIT.
2. **ABI:** legacy `dshr.wasm.v1` (`list_tools() -> i64`, `call_tool(i32,i32) -> i64`, опциональный `alloc`, `ptr<<32|len`) и `dshr:plugin@1.0.0` (WIT: `list-tools`/`call-tool` со строками).
3. **Изоляция:** core-модули без host-импортов; компоненты получают пустой WASI-контекст (без preopens/args/env/сети), fuel и лимит памяти.
4. **Манифест** `*.yaml` с `name`, `wasm`, `enabled`; каталоги `~/.dshr/plugins` и `.dshr/plugins`.
5. **Sidecar** — это MCP: `plugin add --command/--url`, без отдельного протокола.
6. **Границы:** fuel 10M, memory cap 64 MiB, инстанцирование на вызов, `sha256` манифеста; Ed25519-подпись (`dshr-wasm`, ключи доверия в `plugins.trusted_keys`); hot reload нет.

## Последствия

- Плагины не требуют wasmtime/WASI и работают на всех целевых ОС.
- Нет состояния между вызовами; для stateful-плагинов нужен sidecar.
- Hot reload и capability-grants отложены.
- Подпись не решает отзыв ключей и доверие к источнику плагина — это следующий слой.

## Альтернативы

- wasmtime + WIT — отложено: размер и toolchain.
- Полноценный WASI — отклонено: даёт файловый/сетевой доступ без capability-модели.
- Отдельный sidecar-протокол — отклонено: MCP уже покрывает.
