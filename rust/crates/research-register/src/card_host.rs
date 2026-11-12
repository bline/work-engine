//! Bounded same-worker composition and exact-copy reconstruction.
use crate::{
    GitObjects, Outcome, PinnedSnapshot, card,
    card_admission::{self, Installation, InstallationData},
    card_wire::{self, CurrentCardRequest, Envelope},
    read_host::{self, HostProfile, ReadRequest},
    use_bundle::{self, revision},
};
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardRequest {
    pub source: ReadRequest,
    pub current: CurrentCardRequest,
    pub codec: String,
}
impl CardRequest {
    /// Scalar wire constraints are checked before retaining a request or touching host state.
    pub fn validate_scalars(&self) -> Result<(), String> {
        card_wire::hex(&self.current.request_sha256, 64)?;
        card_wire::hex(&self.source.commit, 40)?;
        card_wire::hex(&self.source.bundle.commit, 40)?;
        card_wire::hex(&self.source.bundle.blob, 40)?;
        card_wire::hex(&self.source.bundle.sha256, 64)?;
        if self.current.decision_id.is_empty()
            || self.current.decision_revision.is_empty()
            || self.source.register_path.is_empty()
            || self.source.bundle.path.is_empty()
            || self.source.repository.as_os_str().is_empty()
        {
            return Err("bad_request".into());
        }
        Ok(())
    }
    pub fn validate(&self) -> Result<(), String> {
        self.validate_scalars()?;
        if self.codec != card_wire::CODEC {
            return Err("unsupported_version".into());
        }
        Ok(())
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerInput {
    pub request: CardRequest,
    pub profile: HostProfile,
    pub producer: String,
    pub(crate) installation: Option<InstallationData>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectCopy {
    pub oid: String,
    pub kind: String,
    pub bytes: Vec<u8>,
}
/// Offline reconstruction basis. Its external digest must be obtained from the saved delivery.
/// A caller-supplied replacement digest is not scientific acceptance or authorization.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reconstruction {
    pub request: CardRequest,
    pub profile: HostProfile,
    pub producer: String,
    installation: Option<InstallationData>,
    pub objects: Vec<ObjectCopy>,
    pub card_bytes: Vec<u8>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Product {
    pub producer: String,
    pub outcome: String,
    pub commit: String,
    pub bundle: use_bundle::BlobPin,
    pub profile_sha256: String,
    pub envelope_utf8: String,
    pub reconstruction: Reconstruction,
}
fn read_failure(e: impl std::fmt::Display) -> String {
    let s = e.to_string();
    if s.contains("Limit") || s.contains("limit") {
        "resource_exceeded".into()
    } else if s.contains("ScopeUnavailable") {
        "scope_unavailable".into()
    } else {
        "unreadable".into()
    }
}
fn compose(
    objects: GitObjects,
    input: &WorkerInput,
    replay: bool,
) -> Result<Reconstruction, String> {
    input.request.validate()?;
    let r = &input.request.source;
    card_wire::hex(&r.commit, 40)?;
    let s = PinnedSnapshot::build(objects, &r.commit, &r.register_path).map_err(read_failure)?;
    let v = match crate::validate(&s).map_err(read_failure)? {
        Outcome::Valid(v) => v,
        Outcome::Invalid(_) => return Err("invalid_register".into()),
    };
    if v.records().decisions.len() > input.profile.read.decisions {
        return Err("resource_exceeded".into());
    }
    // Root-approved error precedence: establish the complete anchor batch first. A required
    // rename-interim gap cannot form an M1 target-blob proof, and still has its own refusal.
    let anchors = crate::anchors(&v, &s).map_err(read_failure)?;
    card::request_check(&v, &input.request.current)?;
    if anchors
        .entries
        .iter()
        .find(|a| a.decision_id == input.request.current.decision_id)
        .is_none_or(|a| a.class.is_none())
    {
        return Err("anchor_gap".into());
    }
    let b = use_bundle::validate_use_bundle(&s, &v, r.bundle.clone()).map_err(|e| match e {
        use_bundle::UseError::Unreadable(e) => read_failure(e),
        _ => "invalid_bundle".into(),
    })?;
    let installation = Installation {
        data: input.installation.clone(),
    };
    let admitted = card_admission::verify(&installation, &b, &input.request.current, s.objects())?;
    let mut copies = if replay {
        vec![]
    } else {
        s.objects().copied_objects().ok_or("scope_unavailable")?
    };
    copies.sort_by_key(|(id, _, _)| *id);
    let batch = card::AdmittedReadBatch {
        anchors,
        objects: copies.clone(),
    };
    let read_scope =
        revision(&serde_json::to_vec(&input.profile.read).map_err(|_| "scope_unavailable")?);
    let c = card::project_current(
        &s,
        &v,
        &b,
        &input.request.current,
        &admitted,
        &batch,
        &input.producer,
        &read_scope,
    )?;
    let card_bytes = Envelope::success(input.request.current.clone(), c).bytes()?;
    Ok(Reconstruction {
        request: input.request.clone(),
        profile: input.profile.clone(),
        producer: input.producer.clone(),
        installation: input.installation.clone(),
        objects: copies
            .into_iter()
            .map(|(id, k, bytes)| ObjectCopy {
                oid: id.to_string(),
                kind: crate::objects::kind_name(k).into(),
                bytes,
            })
            .collect(),
        card_bytes,
    })
}
pub fn worker(input: WorkerInput) -> Result<serde_json::Value, String> {
    #[cfg(not(feature = "synthetic-card-host"))]
    if input.installation.is_some() {
        return Err("authority_unavailable".into());
    }
    input.profile.validate().map_err(|_| "resource_exceeded")?;
    read_host::verify_controls(&input.profile).map_err(|_| "controls_unavailable")?;
    let r = &input.request.source;
    let scope = input
        .profile
        .repository_scope
        .as_ref()
        .ok_or("scope_unavailable")?;
    let repo = std::fs::canonicalize(&r.repository).map_err(read_failure)?;
    if std::fs::canonicalize(scope).map_err(read_failure)? != repo {
        return Err("scope_unavailable".into());
    }
    let objects =
        GitObjects::open_bounded(&repo, input.profile.read.clone()).map_err(read_failure)?;
    let basis = compose(objects, &input, false)?;
    let product = Product {
        producer: input.producer.clone(),
        outcome: "complete".into(),
        commit: r.commit.clone(),
        bundle: r.bundle.clone(),
        profile_sha256: revision(&serde_json::to_vec(&input.profile).map_err(|_| "bad_request")?),
        envelope_utf8: String::from_utf8(basis.card_bytes.clone()).map_err(|_| "bad_request")?,
        reconstruction: basis,
    };
    serde_json::to_value(product).map_err(|_| "bad_request".into())
}
/// Runs the actual existing Linux enclosure. The installation comes from host startup,
/// separate from the request. Production startup has no install route in this slice.
#[allow(clippy::too_many_arguments)]
pub fn run(
    executable: &Path,
    cgroup: &Path,
    request: CardRequest,
    profile: HostProfile,
    installation: &Installation,
    cancel: &AtomicBool,
    start: Instant,
) -> Result<Product, String> {
    run_with_clock(
        executable,
        cgroup,
        request,
        profile,
        installation,
        cancel,
        start,
        Instant::now,
    )
}
#[allow(clippy::too_many_arguments)]
fn run_with_clock(
    executable: &Path,
    cgroup: &Path,
    request: CardRequest,
    profile: HostProfile,
    installation: &Installation,
    cancel: &AtomicBool,
    start: Instant,
    release_time: impl FnOnce() -> Instant,
) -> Result<Product, String> {
    request.validate()?;
    #[cfg(not(feature = "synthetic-card-host"))]
    if installation.data.is_some() {
        return Err("authority_unavailable".into());
    }
    let producer = read_host::producer(executable).map_err(read_failure)?;
    let input = WorkerInput {
        request,
        profile,
        producer,
        installation: installation.data.clone(),
    };
    let encoded = serde_json::to_vec(&input).map_err(|_| "bad_request")?;
    #[cfg(target_os = "linux")]
    let value = read_host::run_payload(
        executable,
        cgroup,
        &input.profile,
        &input.producer,
        encoded,
        cancel,
        start,
    )
    .map_err(|e| host_failure(&e))?;
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (encoded, cgroup, cancel, start);
        return Err("controls_unavailable".into());
    }
    #[cfg(target_os = "linux")]
    {
        if value.get("outcome").and_then(|v| v.as_str()) == Some("refused")
            && value.get("producer").and_then(|v| v.as_str()) == Some(&input.producer)
        {
            return Err(value
                .get("reason")
                .and_then(|v| v.as_str())
                .filter(|s| card_wire::OUTCOMES.contains(s))
                .unwrap_or("unreadable")
                .into());
        }
        let product: Product = serde_json::from_value(value).map_err(|_| "delivery_unknown")?;
        let expected = revision(&serde_json::to_vec(&input.profile).map_err(|_| "bad_request")?);
        if product.producer != input.producer
            || product.outcome != "complete"
            || product.commit != input.request.source.commit
            || product.bundle != input.request.source.bundle
            || product.profile_sha256 != expected
            || product.reconstruction.card_bytes != product.envelope_utf8.as_bytes()
        {
            return Err("delivery_unknown".into());
        }
        let envelope = card_wire::decode(product.envelope_utf8.as_bytes())?;
        if envelope.outcome != "card"
            || envelope.request.as_ref() != Some(&input.request.current)
            || envelope.card.as_ref().is_none_or(|c| {
                c.producer_sha256 != input.producer
                    || c.bundle != input.request.source.bundle
                    || c.source.commit != input.request.source.commit
                    || c.coverage.read_scope_sha256
                        != revision(&serde_json::to_vec(&input.profile.read).expect("read profile"))
            })
        {
            return Err("delivery_unknown".into());
        }
        release(
            product,
            cancel,
            start,
            input.profile.wall_millis,
            release_time(),
        )
    }
}
fn host_failure(e: &str) -> String {
    if e.contains("Cancelled") {
        "cancelled"
    } else if e.contains("Limit") || e.contains("WorkerExit") {
        "resource_exceeded"
    } else if e.contains("ControlsUnavailable") {
        "controls_unavailable"
    } else {
        "delivery_unknown"
    }
    .into()
}
/// Rebuilds solely from frozen copies and the original producer/profile/admission evidence.
/// The expected digest must be pinned outside this basis.
pub fn reconstruct(basis: &Reconstruction, expected_sha256: &str) -> Result<Vec<u8>, String> {
    // Saved JSON and a caller-supplied digest establish no trusted installation.
    // Only the explicitly synthetic qualification build may replay synthetic admission.
    #[cfg(not(feature = "synthetic-card-host"))]
    if basis.installation.is_some() {
        return Err("authority_unavailable".into());
    }
    if revision(&serde_json::to_vec(basis).map_err(|_| "unreadable")?) != expected_sha256 {
        return Err("unreadable".into());
    }
    let mut copies = vec![];
    for o in &basis.objects {
        let id = gix::hash::ObjectId::from_hex(o.oid.as_bytes()).map_err(|_| "unreadable")?;
        let k = match o.kind.as_str() {
            "commit" => gix::objs::Kind::Commit,
            "tree" => gix::objs::Kind::Tree,
            "blob" => gix::objs::Kind::Blob,
            "tag" => gix::objs::Kind::Tag,
            _ => return Err("unreadable".into()),
        };
        copies.push((id, k, o.bytes.clone()));
    }
    let objects = GitObjects::from_copies(copies).map_err(read_failure)?;
    let input = WorkerInput {
        request: basis.request.clone(),
        profile: basis.profile.clone(),
        producer: basis.producer.clone(),
        installation: basis.installation.clone(),
    };
    let mut replay = compose(objects, &input, true)?;
    // Object descriptors are the retained, hash-verified closure used by the original batch.
    let mut env = card_wire::decode(&replay.card_bytes)?;
    env.card.as_mut().ok_or("unreadable")?.anchor.objects = basis
        .objects
        .iter()
        .map(|o| card_wire::Object {
            oid: o.oid.clone(),
            kind: o.kind.clone(),
            sha256: revision(&o.bytes),
            size: o.bytes.len() as u64,
        })
        .collect();
    replay.card_bytes = env.bytes()?;
    if replay.card_bytes != basis.card_bytes {
        return Err("unreadable".into());
    }
    Ok(replay.card_bytes)
}
/// Test-fixture export of a fully read source closure. It installs no authority and
/// returns no card. Only available with the explicitly synthetic qualification build.
#[cfg(feature = "synthetic-card-host")]
pub fn synthetic_source_copies(
    repository: &Path,
    commit: &str,
    register_path: &str,
) -> Result<Vec<ObjectCopy>, String> {
    let s = PinnedSnapshot::build(
        GitObjects::open_bounded(repository, Default::default()).map_err(read_failure)?,
        commit,
        register_path,
    )
    .map_err(read_failure)?;
    let v = match crate::validate(&s).map_err(read_failure)? {
        Outcome::Valid(v) => v,
        _ => return Err("invalid_register".into()),
    };
    crate::anchors(&v, &s).map_err(read_failure)?;
    let mut out = s
        .objects()
        .copied_objects()
        .ok_or("scope_unavailable")?
        .into_iter()
        .map(|(id, k, bytes)| ObjectCopy {
            oid: id.to_string(),
            kind: crate::objects::kind_name(k).into(),
            bytes,
        })
        .collect::<Vec<_>>();
    out.sort_by(|a, b| a.oid.cmp(&b.oid));
    Ok(out)
}

// Staged output is consumed and dropped on refusal. `now` is supplied only by the host;
// unit tests can deterministically cross the deadline after worker/validation completion.
fn release<T>(
    staged: T,
    cancel: &AtomicBool,
    start: Instant,
    wall_millis: u64,
    now: Instant,
) -> Result<T, String> {
    if cancel.load(Ordering::SeqCst) {
        return Err("cancelled".into());
    }
    if now.duration_since(start) >= std::time::Duration::from_millis(wall_millis) {
        return Err("resource_exceeded".into());
    }
    Ok(staged)
}
#[cfg(test)]
mod deadline_tests {
    use super::*;
    #[test]
    fn completed_staged_card_expires_at_release() {
        let start = Instant::now();
        let cancel = AtomicBool::new(false);
        let staged = b"validated card bytes".to_vec();
        assert!(
            release(
                staged.clone(),
                &cancel,
                start,
                100,
                start + std::time::Duration::from_millis(99)
            )
            .is_ok()
        );
        assert_eq!(
            release(
                staged,
                &cancel,
                start,
                100,
                start + std::time::Duration::from_millis(100)
            )
            .unwrap_err(),
            "resource_exceeded"
        );
    }
    #[test]
    #[cfg(all(feature = "synthetic-card-host", target_os = "linux"))]
    #[ignore = "requires delegated cgroup, frozen synthetic basis and built image"]
    fn actual_worker_completes_but_validation_release_expires() {
        let basis: Reconstruction = serde_json::from_slice(
            &std::fs::read(std::env::var("RESEARCH_CARD_TEST_BASIS").unwrap()).unwrap(),
        )
        .unwrap();
        let executable = std::env::var("RESEARCH_CARD_TEST_EXE").unwrap();
        let cgroup = std::env::var("RESEARCH_READ_CGROUP_ROOT").unwrap();
        let installation = Installation {
            data: basis.installation,
        };
        let cancel = AtomicBool::new(false);
        let start = Instant::now();
        let control = run_with_clock(
            Path::new(&executable),
            Path::new(&cgroup),
            basis.request.clone(),
            basis.profile.clone(),
            &installation,
            &cancel,
            start,
            Instant::now,
        )
        .unwrap();
        assert_eq!(control.outcome, "complete");
        let start = Instant::now();
        let reached_release = std::cell::Cell::new(false);
        let result = run_with_clock(
            Path::new(&executable),
            Path::new(&cgroup),
            basis.request,
            basis.profile.clone(),
            &installation,
            &cancel,
            start,
            || {
                // This seam is reached only after the real worker, framing, decoding and pin checks.
                reached_release.set(true);
                start + std::time::Duration::from_millis(basis.profile.wall_millis)
            },
        );
        assert!(
            reached_release.get(),
            "real worker and complete validation must finish in time"
        );
        assert_eq!(result.err().unwrap(), "resource_exceeded");
    }
}
