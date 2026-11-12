#![forbid(unsafe_code)]
//! Deterministic replay for offline development and tests.
//!
//! A cassette stores the normalized chunk sequence of one model call. Replay
//! yields the same chunks without network access, which makes CLI and session
//! tests reproducible.

use std::net::SocketAddr;
use std::path::Path;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

use async_stream::stream;
use dshr_llm::{
    Capabilities, ChunkStream, LlmError, LlmProvider, Request, StopReason, StreamChunk,
};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cassette {
    pub model: String,
    pub chunks: Vec<StreamChunk>,
}

impl Cassette {
    #[must_use]
    pub fn new(model: impl Into<String>, chunks: Vec<StreamChunk>) -> Self {
        Self {
            model: model.into(),
            chunks,
        }
    }

    pub fn from_file(path: &Path) -> Result<Self, LlmError> {
        let text = std::fs::read_to_string(path)
            .map_err(|err| LlmError::Config(format!("read cassette {}: {err}", path.display())))?;
        let mut cassette: Self = serde_json::from_str(&text)
            .map_err(|err| LlmError::Config(format!("parse cassette {}: {err}", path.display())))?;
        cassette.normalize();
        Ok(cassette)
    }

    pub fn write_file(&self, path: &Path) -> Result<(), LlmError> {
        let text = serde_json::to_string_pretty(self)
            .map_err(|err| LlmError::Config(format!("serialize cassette: {err}")))?;
        std::fs::write(path, text)
            .map_err(|err| LlmError::Config(format!("write cassette {}: {err}", path.display())))
    }

    fn normalize(&mut self) {
        if !self
            .chunks
            .iter()
            .any(|chunk| matches!(chunk, StreamChunk::Done { .. }))
        {
            self.chunks.push(StreamChunk::Done {
                stop_reason: StopReason::EndTurn,
            });
        }
    }
}

pub struct ReplayProvider {
    cassette: Cassette,
}

impl ReplayProvider {
    #[must_use]
    pub fn new(cassette: Cassette) -> Self {
        Self { cassette }
    }

    pub fn from_file(path: &Path) -> Result<Self, LlmError> {
        Ok(Self::new(Cassette::from_file(path)?))
    }

    #[must_use]
    pub fn cassette(&self) -> &Cassette {
        &self.cassette
    }
}

#[async_trait::async_trait]
impl LlmProvider for ReplayProvider {
    fn name(&self) -> &str {
        "replay"
    }

    fn model(&self) -> &str {
        &self.cassette.model
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::default()
    }

    async fn stream(
        &self,
        _request: Request,
        cancel: CancellationToken,
    ) -> Result<ChunkStream, LlmError> {
        let chunks = self.cassette.chunks.clone();
        let stream = stream! {
            for chunk in chunks {
                if cancel.is_cancelled() {
                    yield Err(LlmError::Cancelled);
                    break;
                }
                yield Ok(chunk);
            }
        };
        Ok(Box::pin(stream))
    }
}

/// Minimal one-shot HTTP server that serves a fixed SSE body.
///
/// Intended for adapter tests: it removes the network from the HTTP path while
/// still exercising request construction, headers, and streaming decoding.
pub struct MockServer {
    addr: SocketAddr,
    handle: JoinHandle<()>,
}

impl MockServer {
    pub async fn start(body: impl Into<String>) -> std::io::Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let body = body.into();
        let handle = tokio::spawn(async move {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            let mut buffer = [0_u8; 8192];
            let _ = socket.read(&mut buffer).await;
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = socket.write_all(response.as_bytes()).await;
            let _ = socket.shutdown().await;
        });
        Ok(Self { addr, handle })
    }

    #[must_use]
    pub fn base_url(&self) -> String {
        format!("http://{}", self.addr)
    }

    pub fn abort(self) {
        self.handle.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dshr_llm::Usage;
    use futures_util::StreamExt;

    fn cassette() -> Cassette {
        Cassette::new(
            "deepseek-flash",
            vec![
                StreamChunk::Start {
                    model: "deepseek-flash".into(),
                },
                StreamChunk::TextDelta {
                    text: "pong".into(),
                },
                StreamChunk::Usage {
                    usage: Usage {
                        input_tokens: 2,
                        output_tokens: 1,
                        ..Usage::default()
                    },
                },
            ],
        )
    }

    #[tokio::test]
    async fn replay_yields_chunks_and_appends_done() {
        let mut cassette = cassette();
        cassette.normalize();
        let provider = ReplayProvider::new(cassette);
        let mut stream = provider
            .stream(Request::default(), CancellationToken::new())
            .await
            .expect("stream");
        let mut chunks = Vec::new();
        while let Some(chunk) = stream.next().await {
            chunks.push(chunk.expect("chunk"));
        }
        assert!(matches!(chunks[0], StreamChunk::Start { .. }));
        assert!(matches!(chunks[1], StreamChunk::TextDelta { .. }));
        assert!(matches!(chunks.last(), Some(StreamChunk::Done { .. })));
    }

    #[test]
    fn cassette_round_trips_through_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("cassette.json");
        cassette().write_file(&path).expect("write");
        let loaded = Cassette::from_file(&path).expect("read");
        assert_eq!(loaded.model, "deepseek-flash");
        assert!(matches!(
            loaded.chunks.last(),
            Some(StreamChunk::Done { .. })
        ));
    }

    #[tokio::test]
    async fn cancellation_stops_replay() {
        let provider = ReplayProvider::new(cassette());
        let cancel = CancellationToken::new();
        cancel.cancel();
        let mut stream = provider
            .stream(Request::default(), cancel)
            .await
            .expect("stream");
        let first = stream.next().await.expect("item").expect_err("cancelled");
        assert!(matches!(first, LlmError::Cancelled));
    }
}
