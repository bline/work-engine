//! Offline, tool-free Responses request and observation foundation.
//!
//! This crate does not send requests, acquire credentials, interpret domain output,
//! or assign lifecycle identity. A caller owns those decisions and transport.

mod journal;
mod request;
mod stream;

pub use journal::{AttemptJournal, AttemptRecord, AttemptState, JournalError};
pub use request::{assemble_request, AssembledRequest, Bounds, RequestError, ToolFreeProfile};
pub use stream::{
    ObservedTerminal, StreamError, StreamObservation, StreamObserver, TokenCounts,
    UsageObservation, UsageSemantics,
};
