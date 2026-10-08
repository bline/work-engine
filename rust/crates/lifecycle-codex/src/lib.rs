//! Pinned, private Codex app-server protocol boundary for the S5 native profile.
//! A parsed notification is provider evidence, never lifecycle authority.

mod protocol;
mod transport;

pub use protocol::{
    NATIVE_BINARY_SHA256, NativeEvent, NativeEventKind, NativeProtocolError, NativeToolCall,
    NativeTurnTerminal, PROFILE_ID, SCHEMA_THREAD_START_SHA256, SCHEMA_TOOL_CALL_SHA256,
    parse_event,
};
pub use transport::{NativeHarness, NativeHarnessError};
