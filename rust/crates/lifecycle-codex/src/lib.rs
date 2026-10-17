//! Pinned, private Codex app-server protocol boundary for the bound native profile.
//! A parsed notification is provider evidence, never lifecycle authority.

mod protocol;
mod transport;

pub use protocol::{
    NATIVE_BINARY_SHA256, NATIVE_USER_AGENT_VERSION, NativeEvent, NativeEventKind,
    NativeProtocolError, NativeToolCall, NativeTurnTerminal, PROFILE_ID,
    SCHEMA_THREAD_START_SHA256, SCHEMA_TOOL_CALL_SHA256, parse_event, read_state_changed_matches,
    user_agent_matches,
};
pub use transport::{NativeHarness, NativeHarnessError};
