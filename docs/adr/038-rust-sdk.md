# 038. Typed Rust HTTP/SSE SDK

- **Статус:** Accepted
- **Дата:** 2026-11-13

## Решение

Крейт `dshr-sdk` с `Client`: health, sessions, turn, cancel, approve и SSE-поток событий. Ошибки типизированы (`SdkError`), HTTP-таймаут 30 с. Тест поднимает реальный `dshr-server` с mock-провайдером.

## Границы

Только HTTP/SSE API; JSON-RPC SDK и Leptos UI не входят.
