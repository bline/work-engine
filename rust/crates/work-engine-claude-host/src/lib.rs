//! Claude Host core: runs Claude Code headless as a bounded analysis turn with
//! a relayed tool surface, and holds exact request evidence before dispatch.
//! Protocol-independent: it does not speak to `lifecycle-service` yet.

pub mod capture;
pub mod driver;
pub mod fixture;
pub mod http;
pub mod proxy;
pub mod relay;
pub mod snapshot;
pub mod spec;
pub mod sse;
