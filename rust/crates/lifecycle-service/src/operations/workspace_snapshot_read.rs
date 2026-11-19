use std::collections::BTreeMap;

use lifecycle_core::{ClockSample, SnapshotId, SubjectId};
use lifecycle_store::{NativeOperationEntry, SqliteLifecycleStore, StoreError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use work_engine_types::{CodecContract, IdValue};

const MAX_MEMBERS: usize = 128;
const MAX_MEMBER_BYTES: usize = 1_048_576;
const MAX_TOTAL_BYTES: usize = 16_777_216;
const MAX_RESULT_BYTES: u64 = 65_536;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TrustedSnapshot {
    pub id: String,
    pub source_revision: String,
    pub members: Vec<TrustedMember>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TrustedMember {
    pub id: String,
    pub sha256: String,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct PublishedSnapshot {
    pub id: SnapshotId,
    pub manifest_sha256: String,
    members: BTreeMap<String, (String, usize)>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadArguments {
    pub snapshot_id: String,
    pub member_id: String,
    pub offset: u64,
    pub length: u64,
}

/// A snapshot or member identifier outside the identifier grammar
/// (`work_engine_types::IdValue`: 1-128 bytes of ASCII alphanumerics and
/// `._:-`). The task owner records every read under that grammar
/// (`retained-task-workflow` `validate_effect_attribution`), so an identifier
/// outside it would be refused only after the native read had committed.
/// `IdValue` is the one shared definition: `retained-task-workflow` depends on
/// this crate, so this crate cannot use the owner's own check.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnapshotIdRefusal {
    SnapshotIdOutsideGrammar,
    MemberIdOutsideGrammar,
}

impl SnapshotIdRefusal {
    pub fn reason(self) -> &'static str {
        match self {
            Self::SnapshotIdOutsideGrammar => "snapshot-id-outside-identifier-grammar",
            Self::MemberIdOutsideGrammar => "member-id-outside-identifier-grammar",
        }
    }
}

impl std::fmt::Display for SnapshotIdRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.reason())
    }
}

/// The first identifier of `snapshot` outside the identifier grammar, for
/// every profile, checked at configuration before any native call.
pub fn snapshot_id_refusal(snapshot: &TrustedSnapshot) -> Option<SnapshotIdRefusal> {
    if IdValue::parse(snapshot.id.as_str()).is_err() {
        return Some(SnapshotIdRefusal::SnapshotIdOutsideGrammar);
    }
    snapshot
        .members
        .iter()
        .any(|m| IdValue::parse(m.id.as_str()).is_err())
        .then_some(SnapshotIdRefusal::MemberIdOutsideGrammar)
}

/// Call arguments carry the same grammar, so a model-supplied identifier
/// outside it is refused before any claim.
pub fn parse_arguments(value: Value) -> Result<ReadArguments, StoreError> {
    let args: ReadArguments = serde_json::from_value(value).map_err(|_| StoreError::Rejected)?;
    if IdValue::parse(args.snapshot_id.as_str()).is_err()
        || IdValue::parse(args.member_id.as_str()).is_err()
        || args.length == 0
        || args.length > MAX_RESULT_BYTES
        || args.offset.checked_add(args.length).is_none()
    {
        return Err(StoreError::Rejected);
    }
    Ok(args)
}

struct PreparedSnapshot {
    published: PublishedSnapshot,
    manifest: Vec<u8>,
}

pub(super) fn validate(snapshot: &TrustedSnapshot) -> Result<(), StoreError> {
    prepare(snapshot).map(|_| ())
}

fn prepare(snapshot: &TrustedSnapshot) -> Result<PreparedSnapshot, StoreError> {
    if snapshot_id_refusal(snapshot).is_some() {
        return Err(StoreError::Rejected);
    }
    let id = SnapshotId::parse(snapshot.id.clone()).map_err(|_| StoreError::Rejected)?;
    if snapshot.source_revision.is_empty()
        || snapshot.source_revision.len() > 256
        || snapshot.members.is_empty()
        || snapshot.members.len() > MAX_MEMBERS
    {
        return Err(StoreError::Rejected);
    }
    let mut total = 0usize;
    let mut previous: Option<&str> = None;
    let mut members = BTreeMap::new();
    let mut manifest_members = Vec::new();
    for member in &snapshot.members {
        if member.id.is_empty()
            || member.id.len() > 128
            || previous.is_some_and(|p| p >= member.id.as_str())
            || member.bytes.len() > MAX_MEMBER_BYTES
        {
            return Err(StoreError::Rejected);
        }
        previous = Some(&member.id);
        total = total
            .checked_add(member.bytes.len())
            .ok_or(StoreError::Rejected)?;
        if total > MAX_TOTAL_BYTES {
            return Err(StoreError::Rejected);
        }
        let digest = CodecContract::BinaryArtifactV1
            .digest_binary(&member.bytes)
            .map_err(|_| StoreError::Rejected)?
            .hex();
        if digest != member.sha256 {
            return Err(StoreError::Rejected);
        }
        members.insert(member.id.clone(), (digest.clone(), member.bytes.len()));
        manifest_members
            .push(serde_json::json!({"id":member.id,"sha256":digest,"length":member.bytes.len()}));
    }
    let manifest = serde_json::json!({"codec":"workspace-snapshot-manifest-v1","id":snapshot.id,
        "source_revision":snapshot.source_revision,"members":manifest_members});
    let bytes = serde_json::to_vec(&manifest).map_err(|_| StoreError::Unavailable)?;
    let manifest_sha256 = CodecContract::BinaryArtifactV1
        .digest_binary(&bytes)
        .map_err(|_| StoreError::Unavailable)?
        .hex();
    Ok(PreparedSnapshot {
        manifest: bytes,
        published: PublishedSnapshot {
            id,
            manifest_sha256,
            members,
        },
    })
}

pub fn publish(
    store: &mut SqliteLifecycleStore,
    subject: &SubjectId,
    snapshot: &TrustedSnapshot,
    clock: ClockSample,
) -> Result<PublishedSnapshot, StoreError> {
    // Validate the complete input, including later members, before staging
    // even the first artifact. The same owner serves startup preflight.
    let prepared = prepare(snapshot)?;
    for member in &snapshot.members {
        let expected = &prepared.published.members[&member.id].0;
        let staged = store.stage_artifact(subject.clone(), &member.bytes)?;
        let published = store.publish_artifact(staged, clock)?;
        if &published.digest_hex != expected || published.byte_length != member.bytes.len() {
            return Err(StoreError::Unavailable);
        }
    }
    let staged = store.stage_artifact(subject.clone(), &prepared.manifest)?;
    let published = store.publish_artifact(staged, clock)?;
    if published.digest_hex != prepared.published.manifest_sha256
        || published.byte_length != prepared.manifest.len()
    {
        return Err(StoreError::Unavailable);
    }
    Ok(prepared.published)
}

/// Exact result bytes come only from store-owned immutable artifact custody.
pub fn execute(
    store: &SqliteLifecycleStore,
    snapshot: &PublishedSnapshot,
    entry: &NativeOperationEntry,
) -> Result<Vec<u8>, StoreError> {
    if entry.snapshot() != &snapshot.id {
        return Err(StoreError::ClaimConflict);
    }
    let (digest, size) = snapshot
        .members
        .get(entry.member_id())
        .ok_or(StoreError::Rejected)?;
    let (offset, length) = entry.range();
    let end = offset.checked_add(length).ok_or(StoreError::Rejected)?;
    if length > MAX_RESULT_BYTES || end > u64::try_from(*size).map_err(|_| StoreError::Rejected)? {
        return Err(StoreError::Rejected);
    }
    let bytes = store
        .read_committed_artifact(digest)?
        .ok_or(StoreError::Unavailable)?;
    if bytes.len() != *size {
        return Err(StoreError::Unavailable);
    }
    Ok(bytes[offset as usize..end as usize].to_vec())
}

#[cfg(test)]
mod id_grammar_tests {
    use super::*;

    fn snapshot(id: &str, members: &[&str]) -> TrustedSnapshot {
        let bytes = b"member".to_vec();
        let sha256 = CodecContract::BinaryArtifactV1
            .digest_binary(&bytes)
            .unwrap()
            .hex();
        TrustedSnapshot {
            id: id.into(),
            source_revision: "source:test".into(),
            members: members
                .iter()
                .map(|m| TrustedMember {
                    id: (*m).into(),
                    sha256: sha256.clone(),
                    bytes: bytes.clone(),
                })
                .collect(),
        }
    }

    /// The live H2 member ids before and after the rename (027-h2-live).
    #[test]
    fn snapshot_ids_outside_the_grammar_refuse_with_a_named_reason() {
        let slash = snapshot("snapshot:h2-advice-r2", &["research/a4-request-card"]);
        assert_eq!(
            snapshot_id_refusal(&slash),
            Some(SnapshotIdRefusal::MemberIdOutsideGrammar)
        );
        assert_eq!(
            SnapshotIdRefusal::MemberIdOutsideGrammar.reason(),
            "member-id-outside-identifier-grammar"
        );
        assert!(matches!(validate(&slash), Err(StoreError::Rejected)));
        let bad_snapshot = snapshot("snapshot/h2", &["research:a4-request-card"]);
        assert_eq!(
            snapshot_id_refusal(&bad_snapshot),
            Some(SnapshotIdRefusal::SnapshotIdOutsideGrammar)
        );
        assert!(matches!(validate(&bad_snapshot), Err(StoreError::Rejected)));
        let colon = snapshot(
            "snapshot:h2-treatment-r2",
            &[
                "git-1a1cb443:skills:manual-orchestration:references:context-boundary.md",
                "research:a4-request-card",
                "skills:manual-orchestration:scripts:child_context.py",
            ],
        );
        assert_eq!(snapshot_id_refusal(&colon), None);
        assert!(validate(&colon).is_ok());
        for refused in ["", "a b", "a/b", "ä", &"m".repeat(129)] {
            assert_eq!(
                snapshot_id_refusal(&snapshot("snapshot:test", &[refused])),
                Some(SnapshotIdRefusal::MemberIdOutsideGrammar),
                "{refused:?}"
            );
        }
    }

    /// A model-supplied identifier outside the grammar is refused by the
    /// argument parser, which `execute_native_tool` runs before the claim
    /// (and the ingress classification runs before recording a call as
    /// qualified, which the claim requires).
    #[test]
    fn call_arguments_outside_the_grammar_refuse_before_claim() {
        let args = |snapshot: &str, member: &str| serde_json::json!({"snapshot_id":snapshot,"member_id":member,"offset":0,"length":8});
        assert!(parse_arguments(args("snapshot:h2", "research:a4-request-card")).is_ok());
        for (snapshot, member) in [
            ("snapshot:h2", "research/a4-request-card"),
            ("snapshot/h2", "research:a4-request-card"),
            ("snapshot:h2", ""),
            ("", "research:a4-request-card"),
        ] {
            assert!(
                matches!(
                    parse_arguments(args(snapshot, member)),
                    Err(StoreError::Rejected)
                ),
                "{snapshot:?} {member:?}"
            );
        }
    }

    /// Service configuration names the refusal before any native launch.
    #[test]
    fn service_configuration_names_the_snapshot_refusal() {
        let root = std::env::temp_dir();
        let config = |member: &str| -> crate::ServiceConfig {
            let bytes = b"member".to_vec();
            let sha = CodecContract::BinaryArtifactV1
                .digest_binary(&bytes)
                .unwrap()
                .hex();
            let semantic = serde_json::json!({"capture_id":"capture:test","compile_session_id":"session:compile",
                "verify_session_id":"session:verify","successor_verify_session_id":"session:successor",
                "compile_invocation_id":"invocation:compile","compile_attempt_id":"attempt:compile",
                "candidate_verify_invocation_id":"invocation:verify","candidate_verify_attempt_id":"attempt:verify",
                "successor_verify_invocation_id":"invocation:successor","successor_verify_attempt_id":"attempt:successor",
                "challenge_id":"challenge:test","challenge_text":"t","control_grant_ref":"grant:control",
                "control_grant_revision":1,"successor_control_grant_ref":"grant:next","successor_control_grant_revision":1});
            let snapshot = serde_json::json!({"id":"snapshot:test","source_revision":"source:test",
                "members":[{"id":member,"sha256":sha,"bytes":bytes}]});
            let native = serde_json::json!({"executable":"/never-executed","root":root.join("native"),
                "provider_route":"native-sim-proof-v1","chatgpt_access_source":null,
                "simulator_base_url":"http://127.0.0.1:1/v1","session_id":"session:test",
                "invocation_id":"invocation:test","attempt_id":"attempt:test","prompt":"p",
                "turn_grant_ref":"grant:turn","turn_grant_revision":1,"snapshot_grant_ref":"grant:snapshot",
                "snapshot_grant_revision":1,"same_native_session_required":false,"followup_turns":[],
                "snapshot":snapshot,"semantic":semantic,"semantic_contexts":[]});
            let principal = serde_json::json!({"uid":1000,"principal_ref":"principal:test","producer_ref":"producer:test"});
            serde_json::from_value(serde_json::json!({
                "profile":"retained-repository-analysis-v1","store_root":root.join("store"),
                "socket_path":root.join("service.sock"),"peer_socket_path":root.join("peer.sock"),
                "verifier_executable":null,"verifier_ledger_path":null,"proof_barrier_dir":null,
                "trusted_issuer":"issuer:test","principal":principal,"subject_id":"subject:test",
                "context_generation":"context:test","build_id":"build:test","proof_run_id":"proof:test",
                "grants":[],"max_executions":1,"max_wait_ms":1000,"native":native}))
            .unwrap()
        };
        let refused = crate::socket::validate_config(&config("research/a4"), true, true, false);
        assert!(
            matches!(
                refused,
                Err(crate::ServiceError::SnapshotIdentity(
                    SnapshotIdRefusal::MemberIdOutsideGrammar
                ))
            ),
            "{refused:?}"
        );
        assert_eq!(
            refused.unwrap_err().to_string(),
            "snapshot identifier refused: member-id-outside-identifier-grammar"
        );
        // A valid identifier passes this check; any later refusal is not it.
        assert!(!matches!(
            crate::socket::validate_config(&config("research:a4"), true, true, false),
            Err(crate::ServiceError::SnapshotIdentity(_))
        ));
    }
}
