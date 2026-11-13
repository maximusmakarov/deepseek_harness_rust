use std::pin::Pin;

use async_trait::async_trait;
use futures_util::Stream;
use tokio_util::sync::CancellationToken;

use crate::{LlmError, Request, StreamChunk};

/// Normalized chunk stream returned by every adapter.
pub type ChunkStream = Pin<Box<dyn Stream<Item = Result<StreamChunk, LlmError>> + Send>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capabilities {
    pub tools: bool,
    pub vision: bool,
    pub audio: bool,
    pub reasoning: bool,
}

impl Default for Capabilities {
    fn default() -> Self {
        Self {
            tools: true,
            vision: false,
            audio: false,
            reasoning: true,
        }
    }
}

/// A model provider.
///
/// Implementations are cheap to clone through `Arc` and are shared between
/// sessions. Cancellation is cooperative: adapters must stop reading and
/// return [`LlmError::Cancelled`] when the token fires.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn name(&self) -> &str;
    fn model(&self) -> &str;
    fn capabilities(&self) -> Capabilities;

    async fn stream(
        &self,
        request: Request,
        cancel: CancellationToken,
    ) -> Result<ChunkStream, LlmError>;
}
