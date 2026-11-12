#![forbid(unsafe_code)]
//! Session log: durable events, projections, and model history.
//!
//! The log is the single source of truth. Everything the model sees is derived
//! from committed events with [`Session::derive_messages`].

pub mod events;
pub mod session;

pub use events::{SESSION_FORMAT_VERSION, SessionEvent, SessionHeader, TurnEndReason, now_rfc3339};
pub use session::{Session, SessionSummary};
