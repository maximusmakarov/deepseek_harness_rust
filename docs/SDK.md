# Rust SDK

`dshr-sdk` — типизированный клиент HTTP/SSE API.

```rust
use dshr_sdk::Client;

let client = Client::new("http://127.0.0.1:4317", "token");
let health = client.health().await?;
let session = client.create_session().await?;
client.start_turn(&session.id, "hello").await?;

let mut events = client.events(session.id.clone());
while let Some(event) = events.next().await {
    println!("{:?}", event?);
}
```

Методы: `health`, `list_sessions`, `create_session`, `session`, `start_turn`, `cancel`, `approve`, `events` (SSE stream). Ошибки — `SdkError` (`Http`, `Status`, `Json`). Таймаут HTTP — 30 с.
