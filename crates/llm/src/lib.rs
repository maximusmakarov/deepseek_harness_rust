#![forbid(unsafe_code)]
//! Provider-neutral LLM vocabulary.
//!
//! Adapters translate a provider wire format into [`StreamChunk`]s; the rest of
//! the harness only ever sees this vocabulary. That keeps the agent loop, the
//! session log, and the UI independent of any single provider.

pub mod assemble;
pub mod error;
pub mod provider;
pub mod sse;
pub mod types;

pub use assemble::{AssistantOutput, BlockAssembler};
pub use error::LlmError;
pub use provider::{Capabilities, ChunkStream, LlmProvider};
pub use sse::{SseDecoder, SseEvent};
pub use types::{ContentBlock, Message, Request, Role, StopReason, StreamChunk, ToolDef, Usage};

pub type Result<T> = std::result::Result<T, LlmError>;
