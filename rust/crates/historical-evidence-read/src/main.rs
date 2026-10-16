//! Local read-only adapter over the legacy checkpoint, gate and review owners.
use historical_evidence_wire::{
    Candidate, Finding, Gate, GateCheck, MAX_FRAME, Observation, RUN_ID, ReplacementLink, Reply,
    Request, ReviewEpisode, ReviewRevision, SourceRead, Subject,
};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

type Result<T> = std::result::Result<T, String>;

static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);
static ACTIVE_OWNER_PGID: AtomicI32 = AtomicI32::new(0);

extern "C" fn shutdown_signal(_signal: libc::c_int) {
    SHUTTING_DOWN.store(true, Ordering::SeqCst);
    let group = ACTIVE_OWNER_PGID.load(Ordering::SeqCst);
    if group > 0 {
        unsafe {
            libc::kill(-group, libc::SIGKILL);
        }
    }
}

fn install_shutdown_handlers() -> Result<()> {
    let mut action: libc::sigaction = unsafe { std::mem::zeroed() };
    action.sa_sigaction = shutdown_signal as usize;
    action.sa_flags = 0;
    unsafe {
        libc::sigemptyset(&mut action.sa_mask);
    }
    for signal in [libc::SIGTERM, libc::SIGINT] {
        if unsafe { libc::sigaction(signal, &action, std::ptr::null_mut()) } != 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
    }
    if unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) } != 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(())
}

fn reap_owner_descendants(group: i32) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let mut status = 0;
        let result = unsafe { libc::waitpid(-group, &mut status, libc::WNOHANG) };
        if result > 0 {
            continue;
        }
        if result == -1 {
            match std::io::Error::last_os_error().raw_os_error() {
                Some(libc::EINTR) => continue,
                Some(libc::ECHILD) => return Ok(()),
                _ => return Err("unable to reap owner descendants".into()),
            }
        }
        if Instant::now() >= deadline {
            unsafe { libc::kill(-group, libc::SIGKILL) };
            return Err("owner descendants exceeded shutdown reap deadline".into());
        }
        thread::sleep(Duration::from_millis(10));
    }
}

struct BoundSocket {
    path: PathBuf,
    device: u64,
    inode: u64,
}
impl BoundSocket {
    fn new(path: PathBuf) -> Result<Self> {
        let info = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
        if !info.file_type().is_socket() {
            return Err("bound path is not a socket".into());
        }
        Ok(Self {
            path,
            device: info.dev(),
            inode: info.ino(),
        })
    }
}
impl Drop for BoundSocket {
    fn drop(&mut self) {
        if fs::symlink_metadata(&self.path)
            .is_ok_and(|info| info.dev() == self.device && info.ino() == self.inode)
        {
            let _ = fs::remove_file(&self.path);
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckpointDescriptor {
    kind: String,
    attempt: u8,
    r#ref: String,
    oid: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GateDescriptor {
    attempt: u8,
    checkpoint_ref: String,
    checkpoint_oid: String,
    capture_path: String,
    capture_line: u32,
    capture_sha256: String,
    source_kind: String,
    receipt_sha256: String,
    call_id: Option<String>,
    event_id: Option<String>,
    output_line: Option<u32>,
    output_sha256: Option<String>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReviewDescriptor {
    label: String,
    key: String,
    head: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Grant {
    schema_version: u8,
    profile: String,
    grant_id: String,
    repository: String,
    run_id: String,
    slice_number: u8,
    issued_at: String,
    expires_at: String,
    issuer_provenance: String,
    peer_uid: u32,
    audience: String,
    fields: Vec<String>,
    checkpoints: Vec<CheckpointDescriptor>,
    gates: Vec<GateDescriptor>,
    reviews: Vec<ReviewDescriptor>,
}

fn read_grant(path: &Path) -> Result<(Grant, Vec<u8>)> {
    if !path.is_absolute() {
        return Err("grant path must be absolute".into());
    }
    let parent = path.parent().ok_or("grant has no containing directory")?;
    let dir = fs::symlink_metadata(parent).map_err(|e| e.to_string())?;
    let uid = unsafe { libc::geteuid() };
    if !dir.is_dir()
        || dir.file_type().is_symlink()
        || dir.uid() != uid
        || dir.mode() & 0o777 != 0o700
    {
        return Err("grant directory owner or mode is invalid".into());
    }
    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    let file: File = options.open(path).map_err(|e| e.to_string())?;
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    if !metadata.is_file()
        || metadata.uid() != uid
        || metadata.mode() & 0o777 != 0o600
        || metadata.len() > 64 * 1024
    {
        return Err("opened grant file owner, mode or size is invalid".into());
    }
    let mut bytes = Vec::new();
    file.take(64 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 64 * 1024 {
        return Err("grant file grew beyond read limit".into());
    }
    let grant: Grant = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if grant.schema_version != 1
        || grant.profile != "historical-s5-local-inspection-v1"
        || grant.run_id != RUN_ID
        || grant.slice_number != 6
        || grant.peer_uid != uid
        || grant.audience != "local_terminal_inspection"
        || grant.grant_id.is_empty()
        || grant.issuer_provenance.is_empty()
        || grant.issued_at.is_empty()
        || grant.expires_at.is_empty()
        || grant.fields
            != [
                "checkpoint",
                "gate",
                "metric_discrepancy",
                "review_summary",
                "source_limit",
            ]
        || grant.checkpoints.len() != 6
        || grant.gates.len() != 5
        || grant.reviews.len() != 3
    {
        return Err("grant does not admit the fixed S5 read contract".into());
    }
    let repository = Path::new(&grant.repository);
    if !repository.is_absolute()
        || repository.canonicalize().map_err(|e| e.to_string())? != repository
    {
        return Err("grant repository identity is invalid".into());
    }
    for (index, checkpoint) in grant.checkpoints.iter().enumerate() {
        let kind = if index == 5 {
            "accepted".to_string()
        } else {
            format!("candidate-{}", index + 1)
        };
        let suffix = if index == 5 {
            "accepted".to_string()
        } else {
            kind.clone()
        };
        if checkpoint.kind != kind
            || checkpoint.attempt != (index.min(4) + 1) as u8
            || checkpoint.r#ref != format!("refs/work-engine/checkpoints/{RUN_ID}/slice-6/{suffix}")
            || checkpoint.oid.len() != 40
            || !checkpoint.oid.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err("grant checkpoint set is invalid".into());
        }
    }
    for (index, gate) in grant.gates.iter().enumerate() {
        let cp = &grant.checkpoints[index];
        if gate.attempt != cp.attempt
            || gate.checkpoint_ref != cp.r#ref
            || gate.checkpoint_oid != cp.oid
            || !Path::new(&gate.capture_path).is_absolute()
            || gate.capture_line == 0
            || gate.capture_line > 10_000
            || gate.capture_sha256.len() != 64
            || gate.receipt_sha256.len() != 64
            || (index < 4
                && (gate.source_kind != "slice_builder_runner_output"
                    || gate.call_id.is_none()
                    || gate.event_id.is_some()
                    || gate.output_line.is_some()
                    || gate.output_sha256.is_some()))
            || (index == 4
                && (gate.source_kind != "coordinator_constructed_receipt"
                    || gate.event_id.is_none()
                    || gate.call_id.is_none()
                    || gate.output_line.is_none()
                    || gate
                        .output_sha256
                        .as_ref()
                        .is_none_or(|digest| digest.len() != 64)))
        {
            return Err("grant gate pairing is invalid".into());
        }
    }
    let expected_reviews = [
        (
            "original-generic",
            "independent-review-episode-v1/512fb086b3df879ed552fe59a9513d423b5dfa63905576db73466eea5dc7912c",
        ),
        (
            "instruction",
            "independent-review-episode-v1/b40803c2de2d5698e7ce571505ca2dfa0407d77827cdd377857638da7cd0f539",
        ),
        (
            "replacement-generic",
            "independent-review-episode-v1/8c55f4e88ef8f28cff64f97cd2c8f49b37427a6b5fbab5a75afd9135814148b6",
        ),
    ];
    for (review, (label, key)) in grant.reviews.iter().zip(expected_reviews) {
        if review.label != label || review.key != key || review.head.len() != 40 {
            return Err("grant review case or head is invalid".into());
        }
    }
    Ok((grant, bytes))
}

const OWNER_DEADLINE: Duration = Duration::from_secs(20);
const STDERR_LIMIT: usize = 64 * 1024;

fn bounded_pipe(mut pipe: impl Read, limit: usize) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        let count = pipe.read(&mut chunk).map_err(|e| e.to_string())?;
        if count == 0 {
            return Ok(output);
        }
        if output.len() + count > limit {
            return Err("owner output byte limit exceeded".into());
        }
        output.extend_from_slice(&chunk[..count]);
    }
}

fn owner_call(
    repository: &Path,
    script: &str,
    args: &[String],
    deadline: Instant,
) -> Result<Value> {
    if SHUTTING_DOWN.load(Ordering::SeqCst) {
        return Err("host shutting down".into());
    }
    if Instant::now() >= deadline {
        return Err("owner read deadline exceeded".into());
    }
    let path = repository.join(script);
    let mut command = Command::new("python3");
    command
        .arg(path)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("GIT_NO_REPLACE_OBJECTS", "1");
    // A stalled owner can launch descendants. Reap the whole isolated process
    // group on deadline or byte-limit failure.
    unsafe {
        command.pre_exec(|| {
            if libc::setpgid(0, 0) == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = command.spawn().map_err(|e| e.to_string())?;
    let pid = child.id() as i32;
    ACTIVE_OWNER_PGID.store(pid, Ordering::SeqCst);
    let (sender, receiver) = mpsc::channel();
    let pipes: [(usize, Box<dyn Read + Send>, usize); 2] = [
        (
            0,
            Box::new(child.stdout.take().ok_or("owner stdout unavailable")?),
            MAX_FRAME,
        ),
        (
            1,
            Box::new(child.stderr.take().ok_or("owner stderr unavailable")?),
            STDERR_LIMIT,
        ),
    ];
    let mut handles = Vec::new();
    for (kind, pipe, limit) in pipes {
        let sender = sender.clone();
        handles.push(thread::spawn(move || {
            let _ = sender.send((kind, bounded_pipe(pipe, limit)));
        }));
    }
    drop(sender);
    let owner_end = Instant::now()
        .checked_add(OWNER_DEADLINE)
        .ok_or("deadline overflow")?;
    let mut output = [None, None];
    let mut status = None;
    let mut error = None;
    while status.is_none() || output.iter().any(Option::is_none) {
        if SHUTTING_DOWN.load(Ordering::SeqCst) {
            error = Some("host shutting down".into());
            break;
        }
        if Instant::now() >= owner_end || Instant::now() >= deadline {
            error = Some("owner read deadline exceeded".to_string());
            break;
        }
        match receiver.recv_timeout(Duration::from_millis(10)) {
            Ok((kind, result)) => match result {
                Ok(bytes) => output[kind] = Some(bytes),
                Err(reason) => {
                    error = Some(reason);
                    break;
                }
            },
            Err(mpsc::RecvTimeoutError::Disconnected) if output.iter().any(Option::is_none) => {
                error = Some("owner output stream disconnected".into());
                break;
            }
            Err(_) => (),
        }
        if status.is_none() {
            match child.try_wait() {
                Ok(next) => status = next,
                Err(failure) => {
                    error = Some(failure.to_string());
                    break;
                }
            }
        }
    }
    // A successful direct child may still have descendants in its group.
    unsafe {
        libc::kill(-pid, libc::SIGKILL);
    }
    let final_status = child.wait();
    for handle in handles {
        let _ = handle.join();
    }
    let descendants = reap_owner_descendants(pid);
    ACTIVE_OWNER_PGID.store(0, Ordering::SeqCst);
    let final_status = final_status.map_err(|e| e.to_string())?;
    descendants?;
    if SHUTTING_DOWN.load(Ordering::SeqCst) {
        return Err("host shutting down".into());
    }
    if let Some(reason) = error {
        return Err(format!("{script}: {reason}"));
    }
    if !final_status.success() {
        return Err(format!("{} refused read (exit {})", script, final_status));
    }
    serde_json::from_slice(output[0].as_ref().ok_or("owner stdout missing")?)
        .map_err(|e| format!("{} returned invalid JSON: {e}", script))
}

fn string(value: &Value, key: &str) -> Result<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("owner projection missing {key}"))
}
fn array<'a>(value: &'a Value, key: &str) -> Result<&'a Vec<Value>> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("owner projection missing {key}"))
}

fn hash_regular_file(identity: &mut Sha256, path: &Path, limit: u64) -> Result<()> {
    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    let mut file = options.open(path).map_err(|e| e.to_string())?;
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.len() > limit {
        return Err("owner source file type or size invalid".into());
    }
    let mut read_total = 0u64;
    let mut chunk = [0u8; 8192];
    loop {
        let count = file.read(&mut chunk).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        read_total += count as u64;
        if read_total > limit {
            return Err("owner source file grew beyond limit".into());
        }
        identity.update(&chunk[..count]);
    }
    Ok(())
}

fn source_identity(repository: &Path, grant_bytes: &[u8]) -> Result<String> {
    let cp_script = "skills/slice-checkpoint/scripts/checkpoint.py";
    let gate_script = "skills/slice-builder/scripts/legacy_gate_inspect.py";
    let review_script = "skills/independent-review-state/scripts/historical_inspect.py";
    let review_owner_script = "skills/independent-review-state/scripts/independent_review_state.py";
    let durable_script = "skills/durable-state/scripts/durable_state.py";
    let mut identity = Sha256::new();
    identity.update(grant_bytes);
    for script in [
        cp_script,
        gate_script,
        review_script,
        review_owner_script,
        durable_script,
    ] {
        hash_regular_file(&mut identity, &repository.join(script), 1024 * 1024)?;
    }
    hash_regular_file(
        &mut identity,
        &std::env::current_exe().map_err(|e| e.to_string())?,
        64 * 1024 * 1024,
    )?;
    Ok(format!("{:x}", identity.finalize()))
}

fn observe(grant: &Grant, grant_path: &Path, grant_bytes: &[u8]) -> Result<Observation> {
    let deadline = Instant::now()
        .checked_add(Duration::from_secs(30))
        .ok_or("deadline overflow")?;
    let (_, current_grant_bytes) = read_grant(grant_path)?;
    if current_grant_bytes != grant_bytes {
        return Err("inspection grant changed during host lifetime".into());
    }
    let repository = Path::new(&grant.repository);
    let cp_script = "skills/slice-checkpoint/scripts/checkpoint.py";
    let gate_script = "skills/slice-builder/scripts/legacy_gate_inspect.py";
    let review_script = "skills/independent-review-state/scripts/historical_inspect.py";
    let source_identity_sha256 = source_identity(repository, grant_bytes)?;
    let mut candidates = Vec::new();
    let mut source_reads = Vec::new();
    for (index, descriptor) in grant.checkpoints.iter().enumerate() {
        let checkpoint = owner_call(
            repository,
            cp_script,
            &[
                "--repository".into(),
                grant.repository.clone(),
                "--inspect-ref".into(),
                descriptor.r#ref.clone(),
                "--expected-oid".into(),
                descriptor.oid.clone(),
            ],
            deadline,
        )?;
        if string(&checkpoint, "commit_oid")? != descriptor.oid
            || string(&checkpoint, "ref")? != descriptor.r#ref
        {
            return Err("checkpoint owner returned a different exact subject".into());
        }
        let metadata = checkpoint
            .get("metadata")
            .ok_or("checkpoint metadata missing")?;
        let gate_digest = string(metadata, "gate_receipt_digest")?;
        let gate = if index < 5 {
            let checkpoint_json = serde_json::to_string(&checkpoint).map_err(|e| e.to_string())?;
            let gate_owner = owner_call(
                repository,
                gate_script,
                &[
                    "--inspection-grant".into(),
                    grant_path.to_string_lossy().to_string(),
                    "--attempt".into(),
                    (index + 1).to_string(),
                    "--checkpoint-json".into(),
                    checkpoint_json,
                ],
                deadline,
            )?;
            if string(&gate_owner, "receipt_sha256")? != gate_digest {
                return Err("gate digest changed across owners".into());
            }
            let checks = array(&gate_owner, "checks")?
                .iter()
                .map(|value| {
                    Ok(GateCheck {
                        identity: string(value, "identity")?,
                        outcome: string(value, "outcome")?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            let gate = Gate {
                source_kind: string(&gate_owner, "source_kind")?,
                receipt_sha256: gate_digest.clone(),
                capture_sha256: string(&gate_owner, "capture_sha256")?,
                corroborating_output_sha256: gate_owner["corroborating_output_sha256"]
                    .as_str()
                    .map(str::to_string),
                status: string(&gate_owner, "status")?,
                check_count: checks.len(),
                passed_count: gate_owner["passed_count"]
                    .as_u64()
                    .ok_or("gate passed count missing")? as usize,
                checks,
                limit: string(&gate_owner, "source_limit")?,
            };
            if gate.passed_count > gate.check_count
                || gate_owner["check_count"].as_u64() != Some(gate.check_count as u64)
                || gate.passed_count
                    != gate
                        .checks
                        .iter()
                        .filter(|check| check.outcome == "passed")
                        .count()
                || gate.corroborating_output_sha256 != grant.gates[index].output_sha256
                || gate.source_kind != grant.gates[index].source_kind
            {
                return Err("gate check counts do not match owner outcomes".into());
            }
            source_reads.push(SourceRead {
                owner: "slice-builder".into(),
                identity: format!("{}@{}", descriptor.r#ref, gate.receipt_sha256),
                integrity: "capture_and_checkpoint_digest_verified".into(),
                completeness: "complete_for_trusted_capture".into(),
                confidentiality: "allowlisted_summary".into(),
                limit: gate.limit.clone(),
            });
            Some(gate)
        } else {
            None
        };
        source_reads.push(SourceRead {
            owner: "slice-checkpoint".into(),
            identity: format!("{}@{}", descriptor.r#ref, descriptor.oid),
            integrity: "validated_commit_ref_and_metadata".into(),
            completeness: "complete_for_exact_checkpoint".into(),
            confidentiality: "allowlisted_metadata".into(),
            limit: "checkpoint is not gate execution attestation".into(),
        });
        candidates.push(Candidate {
            kind: descriptor.kind.clone(),
            attempt: descriptor.attempt,
            checkpoint_ref: descriptor.r#ref.clone(),
            commit_oid: descriptor.oid.clone(),
            tree_oid: string(&checkpoint, "tree_oid")?,
            parent_oid: string(&checkpoint, "parent_commit_oid")?,
            gate_digest,
            gate,
            integrity: string(&checkpoint, "integrity")?,
        });
    }
    let review = owner_call(
        repository,
        review_script,
        &[
            "--inspection-grant".into(),
            grant_path.to_string_lossy().to_string(),
        ],
        deadline,
    )?;
    if string(&review, "grant_id")? != grant.grant_id {
        return Err("review owner used a different grant".into());
    }
    let mut reviews = Vec::new();
    for (index, value) in array(&review, "reviews")?.iter().enumerate() {
        let descriptor = grant
            .reviews
            .get(index)
            .ok_or("unadmitted review returned")?;
        if string(value, "key")? != descriptor.key || string(value, "head")? != descriptor.head {
            return Err("review owner returned an unadmitted head".into());
        }
        let revisions = array(value, "revisions")?
            .iter()
            .map(|item| {
                let subjects = array(item, "subject")?
                    .iter()
                    .map(|subject| {
                        Ok(Subject {
                            owner: string(subject, "owner")?,
                            reference: string(subject, "reference")?,
                            revision: string(subject, "revision")?,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                let findings = array(item, "finding_states")?
                    .iter()
                    .map(|finding| {
                        Ok(Finding {
                            id: string(finding, "id")?,
                            status: string(finding, "status")?,
                            severity: string(finding, "severity")?,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                Ok(ReviewRevision {
                    revision: string(item, "revision")?,
                    predecessor_revision: item["predecessor_revision"].as_str().map(str::to_string),
                    payload_sha256: string(item, "payload_sha256")?,
                    status: string(item, "status")?,
                    phase: string(item, "phase")?,
                    subjects,
                    findings,
                    authority_reference: string(&item["authority_source"], "reference")?,
                    authority_manifest_sha256: string(item, "authority_manifest_sha256")?,
                    historical_grant_id: string(item, "historical_grant_id")?,
                    writer_generation: item["writer_generation"]
                        .as_u64()
                        .ok_or("writer generation missing")?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        source_reads.push(SourceRead {
            owner: "independent-review-state".into(),
            identity: format!("{}@{}", descriptor.key, descriptor.head),
            integrity: "envelope_history_and_state_validated".into(),
            completeness: "linked_history_to_null_predecessor".into(),
            confidentiality: "allowlisted_review_summary".into(),
            limit: "historical writer manifests are not this read grant; reported is not retired"
                .into(),
        });
        reviews.push(ReviewEpisode {
            label: descriptor.label.clone(),
            key: descriptor.key.clone(),
            head: descriptor.head.clone(),
            revisions,
        });
    }
    let replacement_links = array(&review, "replacement_links")?
        .iter()
        .map(|value| {
            Ok(ReplacementLink {
                replacement_key: string(value, "replacement_key")?,
                replacement_revision: string(value, "replacement_revision")?,
                original_key: string(value, "original_key")?,
                original_reported_revision: string(value, "original_reported_revision")?,
                authority_reference: string(value, "authority_reference")?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if source_identity(repository, grant_bytes)? != source_identity_sha256 {
        return Err("owner or host source changed during read".into());
    }
    if read_grant(grant_path)?.1 != grant_bytes {
        return Err("inspection grant changed during owner read".into());
    }
    let read_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    let observation = Observation {
        protocol_version: 1, projection_version: 1, operation: historical_evidence_wire::OPERATION.into(),
        run_id: RUN_ID.into(), slice_number: 6, grant_id: grant.grant_id.clone(), cutoff: grant.issued_at.clone(),
        read_at: format!("unix:{read_at}"), source_identity_sha256,
        candidates, reviews, replacement_links, source_reads,
        gaps: vec![
            "The UI evidence map reports historical stage_total=8, but that metrics row is not admitted by this read; candidate 5 receipt has five checks.".into(),
            "Provider attempts, quota and scheduler history have no public read contract in this increment.".into(),
            "Gate captures bind producer output to checkpoints; they do not independently attest execution.".into(),
            "This case is a bounded historical investigation, not full UI-0 causal coverage.".into(),
        ],
    };
    observation.validate().map_err(str::to_string)?;
    Ok(observation)
}

fn peer_uid(stream: &UnixStream) -> Result<u32> {
    use std::os::fd::AsRawFd;
    let mut credentials = libc::ucred {
        pid: 0,
        uid: 0,
        gid: 0,
    };
    let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    let result = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&mut credentials as *mut libc::ucred).cast(),
            &mut len,
        )
    };
    if result != 0 || len as usize != std::mem::size_of::<libc::ucred>() {
        return Err("unable to authenticate local peer".into());
    }
    Ok(credentials.uid)
}

fn read_before(stream: &mut UnixStream, bytes: &mut [u8], deadline: Instant) -> Result<()> {
    let mut filled = 0;
    while filled < bytes.len() {
        if SHUTTING_DOWN.load(Ordering::SeqCst) {
            return Err("host shutting down".into());
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err("request frame deadline exceeded".into());
        }
        stream
            .set_read_timeout(Some(remaining.min(Duration::from_secs(1))))
            .map_err(|e| e.to_string())?;
        match stream.read(&mut bytes[filled..]) {
            Ok(0) => return Err("request frame ended early".into()),
            Ok(count) => filled += count,
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                ) => {}
            Err(error) => return Err(error.to_string()),
        }
    }
    Ok(())
}

fn write_before(stream: &mut UnixStream, bytes: &[u8], deadline: Instant) -> Result<()> {
    let mut sent = 0;
    while sent < bytes.len() {
        if SHUTTING_DOWN.load(Ordering::SeqCst) {
            return Err("host shutting down".into());
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err("response frame deadline exceeded".into());
        }
        stream
            .set_write_timeout(Some(remaining.min(Duration::from_secs(1))))
            .map_err(|e| e.to_string())?;
        match stream.write(&bytes[sent..]) {
            Ok(0) => return Err("response frame peer closed".into()),
            Ok(count) => sent += count,
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                ) => {}
            Err(error) => return Err(error.to_string()),
        }
    }
    Ok(())
}

fn serve_one(mut stream: UnixStream, grant: &Grant, path: &Path, bytes: &[u8]) -> Result<()> {
    let request_deadline = Instant::now()
        .checked_add(Duration::from_secs(5))
        .ok_or("deadline overflow")?;
    if peer_uid(&stream)? != grant.peer_uid {
        return Err("peer UID is not admitted".into());
    }
    let mut len_bytes = [0u8; 4];
    read_before(&mut stream, &mut len_bytes, request_deadline)?;
    let len = u32::from_be_bytes(len_bytes) as usize;
    if len == 0 || len > MAX_FRAME {
        return Err("request frame limit exceeded".into());
    }
    let mut request_bytes = vec![0; len];
    read_before(&mut stream, &mut request_bytes, request_deadline)?;
    let reply = match serde_json::from_slice::<Request>(&request_bytes) {
        Ok(request)
            if request.validate().is_ok()
                && request
                    .review_key
                    .as_ref()
                    .is_none_or(|key| grant.reviews.iter().any(|review| &review.key == key)) =>
        {
            match observe(grant, path, bytes) {
                Ok(observation) => Reply::Observed { observation },
                Err(_) => Reply::Unavailable {
                    reason: "one or more admitted owner reads failed integrity or availability"
                        .into(),
                },
            }
        }
        _ => Reply::Refused {
            reason: "unsupported or unbounded historical request".into(),
        },
    };
    let response = serde_json::to_vec(&reply).map_err(|e| e.to_string())?;
    if response.len() > MAX_FRAME {
        return Err("response frame limit exceeded".into());
    }
    let response_deadline = Instant::now()
        .checked_add(Duration::from_secs(5))
        .ok_or("deadline overflow")?;
    write_before(
        &mut stream,
        &(response.len() as u32).to_be_bytes(),
        response_deadline,
    )?;
    write_before(&mut stream, &response, response_deadline)?;
    Ok(())
}

fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 4 || args[0] != "--inspection-grant" || args[2] != "--socket" {
        return Err("use --inspection-grant ABSOLUTE_PATH --socket ABSOLUTE_PATH".into());
    }
    let path = PathBuf::from(&args[1]);
    let socket = PathBuf::from(&args[3]);
    let (grant, bytes) = read_grant(&path)?;
    if !socket.is_absolute() || socket.parent() != path.parent() || socket.exists() {
        return Err("socket must be a new path in the private grant directory".into());
    }
    install_shutdown_handlers()?;
    let listener = UnixListener::bind(&socket).map_err(|e| e.to_string())?;
    let _bound_socket = BoundSocket::new(socket.clone())?;
    fs::set_permissions(&socket, fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())?;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    eprintln!(
        "historical-evidence-read: listening for admitted S5 reads on {}",
        socket.display()
    );
    while !SHUTTING_DOWN.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _)) => {
                if let Err(error) = serve_one(stream, &grant, &path, &bytes) {
                    eprintln!("historical-evidence-read: refused local read: {error}");
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(20));
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => (),
            Err(error) => return Err(format!("local connection failed: {error}")),
        }
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("historical-evidence-read: {error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stalled_and_oversized_owners_are_bounded_then_next_read_succeeds() {
        let directory = std::env::temp_dir().join(format!(
            "historical-owner-bounds-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&directory).unwrap();
        fs::write(
            directory.join("stalled.py"),
            "import subprocess, time\nsubprocess.Popen(['sleep', '30'])\ntime.sleep(30)\n",
        )
        .unwrap();
        fs::write(
            directory.join("large.py"),
            "import sys\nsys.stdout.write('x' * 200000)\n",
        )
        .unwrap();
        fs::write(
            directory.join("stderr.py"),
            "import sys\nsys.stderr.write('x' * 70000)\n",
        )
        .unwrap();
        fs::write(directory.join("good.py"), "print('{\"ready\":true}')\n").unwrap();
        let start = Instant::now();
        let stalled = owner_call(
            &directory,
            "stalled.py",
            &[],
            start + Duration::from_millis(300),
        );
        assert!(stalled.unwrap_err().contains("deadline"));
        assert!(start.elapsed() < Duration::from_secs(3));
        assert!(
            owner_call(
                &directory,
                "large.py",
                &[],
                Instant::now() + Duration::from_secs(3)
            )
            .unwrap_err()
            .contains("byte limit")
        );
        assert!(
            owner_call(
                &directory,
                "stderr.py",
                &[],
                Instant::now() + Duration::from_secs(3)
            )
            .unwrap_err()
            .contains("byte limit")
        );
        let good = owner_call(
            &directory,
            "good.py",
            &[],
            Instant::now() + Duration::from_secs(3),
        )
        .unwrap();
        assert_eq!(good["ready"], true);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn partial_frame_has_absolute_deadline_and_fresh_connection_can_read() {
        let (mut left, mut right) = UnixStream::pair().unwrap();
        let writer = thread::spawn(move || {
            let _ = right.write_all(b"a");
            thread::sleep(Duration::from_millis(200));
        });
        let mut bytes = [0u8; 4];
        let result = read_before(
            &mut left,
            &mut bytes,
            Instant::now() + Duration::from_millis(60),
        );
        assert!(result.unwrap_err().contains("deadline"));
        writer.join().unwrap();
        let (mut left, mut right) = UnixStream::pair().unwrap();
        right.write_all(b"test").unwrap();
        read_before(
            &mut left,
            &mut bytes,
            Instant::now() + Duration::from_secs(1),
        )
        .unwrap();
        assert_eq!(&bytes, b"test");
    }
}
