#![forbid(unsafe_code)]
//! Core primitives shared by every `dshr` crate.
//!
//! This crate intentionally has almost no dependencies: identifiers, the error
//! model, and secret redaction are needed by every other component, including
//! the ones that must stay small and auditable.

pub mod error;
pub mod ids;
pub mod redact;

pub use error::{DshrError, ErrorKind, Result};
pub use ids::{JobId, SessionId, StepId, ToolCallId, TurnId};
pub use redact::redact;
