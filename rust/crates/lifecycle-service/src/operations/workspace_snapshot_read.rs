use std::collections::BTreeMap;

use lifecycle_core::{ClockSample, SnapshotId, SubjectId};
use lifecycle_store::{NativeOperationEntry, SqliteLifecycleStore, StoreError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use work_engine_types::CodecContract;

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

pub fn parse_arguments(value: Value) -> Result<ReadArguments, StoreError> {
    let args: ReadArguments = serde_json::from_value(value).map_err(|_| StoreError::Rejected)?;
    if args.member_id.is_empty()
        || args.member_id.len() > 128
        || args.length == 0
        || args.length > MAX_RESULT_BYTES
        || args.offset.checked_add(args.length).is_none()
    {
        return Err(StoreError::Rejected);
    }
    Ok(args)
}

pub fn publish(
    store: &mut SqliteLifecycleStore,
    subject: &SubjectId,
    snapshot: &TrustedSnapshot,
    clock: ClockSample,
) -> Result<PublishedSnapshot, StoreError> {
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
        let staged = store.stage_artifact(subject.clone(), &member.bytes)?;
        let published = store.publish_artifact(staged, clock)?;
        if published.digest_hex != digest || published.byte_length != member.bytes.len() {
            return Err(StoreError::Unavailable);
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
    let staged = store.stage_artifact(subject.clone(), &bytes)?;
    let published = store.publish_artifact(staged, clock)?;
    if published.digest_hex != manifest_sha256 || published.byte_length != bytes.len() {
        return Err(StoreError::Unavailable);
    }
    Ok(PublishedSnapshot {
        id,
        manifest_sha256,
        members,
    })
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
