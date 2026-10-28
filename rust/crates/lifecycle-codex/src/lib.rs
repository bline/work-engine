//! Pinned, private Codex app-server protocol boundary for the bound native profile.
//! A parsed notification is provider evidence, never lifecycle authority.

#[cfg(all(feature = "synthetic-access-test", not(debug_assertions)))]
compile_error!("synthetic-access-test is confined to non-release qualification builds");

mod isolation;
mod protocol;
mod transport;

pub use isolation::AccessIsolationConfig;
pub use lifecycle_core::{IsolationDiagnostic, IsolationPhase, IsolationReason};
pub use protocol::{
    NATIVE_BINARY_SHA256, NATIVE_USER_AGENT_VERSION, NativeEvent, NativeEventKind,
    NativeProtocolError, NativeToolCall, NativeTurnTerminal, PROFILE_ID,
    READ_STATE_CONSUMER_POLICY_ID, SCHEMA_THREAD_START_SHA256, SCHEMA_TOOL_CALL_SHA256,
    parse_event, read_state_changed_matches, user_agent_matches,
};
pub use transport::{
    ExternalChatgptLease, NativeAuthMode, NativeHarness, NativeHarnessError,
    load_external_chatgpt_lease, trusted_loopback_https_proxy,
};
