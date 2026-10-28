//! Fixed nonsecret attribution for the native isolation boundary.
//! Lifecycle Codex produces these values; durable owners and projections share
//! this pure vocabulary so no diagnostic can carry arbitrary runtime data.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IsolationPhase {
    BeforeLaunch,
    LaunchInvoked,
    AfterSpawn,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IsolationReason {
    TransportAuth,
    TransportProfile,
    TransportRoot,
    TransportAdmission,
    PathTopology,
    PolicyPin,
    PathDisjoint,
    SyntheticProfile,
    DirectoryTopology,
    DirectoryMetadata,
    TreeBound,
    TreeRead,
    TreeSymlink,
    TreeHardlink,
    TreeType,
    RootMount,
    HelperPin,
    PolicyDigest,
    BwrapPin,
    CaPin,
    ProxyConnect,
    ProxyPeer,
    ProxyCmdline,
    ProxyCwd,
    ProxyInterpreter,
    ProxyPolicyPin,
    ProxyMetadata,
    ProxyIdentity,
    HttpFixture,
    CompanionPin,
    ProxyEndpoint,
    TemporaryExists,
    TemporaryCreate,
    TemporaryMode,
    PipeSetup,
    FdSetup,
    LaunchInvoke,
    InfoTimeout,
    InfoPoll,
    InfoEof,
    InfoBound,
    InfoMalformed,
    ChildTopology,
    NamespaceIdentity,
    ProcessIdentity,
    Pidfd,
    DescriptorExposure,
    HiddenSource,
    MountedSocket,
    MountedBinary,
    MountReadonly,
    FinalLiveness,
    StatusObservation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IsolationDiagnostic {
    pub phase: IsolationPhase,
    pub reason: IsolationReason,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diagnostic_wire_rejects_unknown_codes_and_dynamic_fields() {
        let d = IsolationDiagnostic {
            phase: IsolationPhase::AfterSpawn,
            reason: IsolationReason::NamespaceIdentity,
        };
        let value = serde_json::to_value(d).unwrap();
        assert_eq!(
            value,
            serde_json::json!({"phase":"after_spawn","reason":"namespace_identity"})
        );
        for bad in [
            serde_json::json!({"phase":"after_spawn","reason":"/private/path"}),
            serde_json::json!({"phase":"after_spawn","reason":"namespace_identity","stderr":"secret"}),
            serde_json::json!({"phase":"unknown","reason":"namespace_identity"}),
        ] {
            assert!(serde_json::from_value::<IsolationDiagnostic>(bad).is_err());
        }
    }
}
