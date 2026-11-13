//! Installed-record custody and conditional trusted operator composition.
//!
//! Checksums and owner labels are data. Only the dedicated externally admitted
//! operator startup can create the private consequence used by trusted owners.
//! Actual source/protection qualification remains external; C1 is unavailable.
use crate::{HostError, HostStartup, current_uid, valid_sha};
use retained_task_workflow::research_admission::{
    ChangeNotice, ResearchAdopt, ResearchKey, research_digest,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::{
        fd::AsRawFd,
        unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    },
    path::{Component, Path, PathBuf},
    sync::Arc,
};
const MAX_METADATA: usize = 128 * 1024;
const MAX_RETAINED: usize = 8 * 1024 * 1024;
// Linux host only; O_NOFOLLOW and O_DIRECTORY apply to the actual opened object.
const O_NOFOLLOW: i32 = 0x20000;
const O_DIRECTORY: i32 = 0x10000;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstallationSelection {
    version: u32,
    root: PathBuf,
    installation: Pin,
    authority_admission: Pin,
    configuration: Pin,
    operations: Pin,
    operation_admission: Pin,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Pin {
    name: String,
    sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum Role {
    Preparer,
    ResearchNotice,
    DecisionNotice,
    Adopter,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Approval {
    git_pin: research_register::use_bundle::BlobPin,
    role: String,
    issuer: String,
    event: Pin,
    source: Pin,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Authority {
    version: u32,
    kind: String,
    id: String,
    key: ResearchKey,
    purpose: String,
    decision_id: String,
    decision_revision: String,
    delegation: String,
    disclosure_scope: Pin,
    source_bundle: Pin,
    approvals: Vec<Approval>,
    producer_roles: BTreeSet<Role>,
    selectors: BTreeSet<String>,
    artifact_root: PathBuf,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AdmissionRecord {
    version: u32,
    kind: String,
    subject_digest: String,
    owner_source: Pin,
    caller_evidence: Pin,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Configuration {
    version: u32,
    kind: String,
    installation_digest: String,
    authority_admission_digest: String,
    /// Complete serialized HostStartup, removing ONLY the outer research
    /// installation selection. All current/new security settings compare by default.
    upstream_startup: Value,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OperationManifest {
    version: u32,
    kind: String,
    id: String,
    installation_digest: String,
    configuration_digest: String,
    intents: Vec<Intent>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Intent {
    operation: String,
    role: Role,
    key: ResearchKey,
    payload_digest: String,
    payload: Pin,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Prepare {
    key: ResearchKey,
    expected_revision: Option<u64>,
    basis_digest: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Notice {
    key: ResearchKey,
    notice_id: String,
    notice: ChangeNotice,
}
type Scope = research_register::card_admission::DisclosureScope;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ApprovalEvent {
    issuer: String,
    role: String,
    delegation: String,
    purpose: String,
    decision_id: String,
    decision_revision: String,
    request_sha256: String,
    bundle_sha256: String,
    records: BTreeSet<String>,
    scope_sha256: String,
}

fn invalid<T>() -> Result<T, HostError> {
    #[cfg(test)]
    if std::env::var_os("M4CB_DIAG").is_some() {
        eprintln!(
            "record rejection: {}",
            std::backtrace::Backtrace::force_capture()
        );
    }
    Err(HostError::Configuration)
}
fn sha(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
}
fn bounded_text(s: &str, limit: usize) -> bool {
    !s.is_empty() && s.len() <= limit && !s.contains('\0')
}
fn key_valid(k: &ResearchKey) -> bool {
    id(&k.task_id) && id(&k.subject_id) && valid_sha(&k.request_digest)
}
fn parse<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, HostError> {
    if bytes.is_empty() || bytes.len() > MAX_METADATA {
        return invalid();
    }
    let v = lifecycle_wire::parse_strict_json(bytes).map_err(|_| HostError::Configuration)?;
    serde_json::from_value(v).map_err(|_| HostError::Configuration)
}
fn header(version: u32, kind: &str, expected: &str) -> Result<(), HostError> {
    if version != 1 || kind != expected {
        return invalid();
    }
    Ok(())
}

/// One opened root and bounded single-component opens through its descriptor.
/// Bytes are retained once, not reopened from a caller path inside a transaction.
struct Custody {
    root: File,
    root_path: PathBuf,
    bytes: BTreeMap<String, (String, Vec<u8>)>,
    total: usize,
}
impl Custody {
    fn open(path: &Path) -> Result<Self, HostError> {
        if !path.is_absolute() || fs::canonicalize(path)? != path {
            return invalid();
        }
        let root = OpenOptions::new()
            .read(true)
            .custom_flags(O_NOFOLLOW | O_DIRECTORY)
            .open(path)?;
        let meta = root.metadata()?;
        if !meta.is_dir()
            || meta.uid() != current_uid()?
            || meta.permissions().mode() & 0o777 != 0o700
        {
            return invalid();
        }
        Ok(Self {
            root,
            root_path: path.to_path_buf(),
            bytes: BTreeMap::new(),
            total: 0,
        })
    }
    fn read(&mut self, pin: &Pin) -> Result<Vec<u8>, HostError> {
        let p = Path::new(&pin.name);
        if !id(&pin.name)
            || !valid_sha(&pin.sha256)
            || p.components().count() != 1
            || !matches!(p.components().next(), Some(Component::Normal(_)))
        {
            return invalid();
        }
        if let Some((digest, bytes)) = self.bytes.get(&pin.name) {
            if digest != &pin.sha256 {
                return invalid();
            }
            return Ok(bytes.clone());
        }
        if self.bytes.len() >= 128 {
            return invalid();
        }
        let anchor = PathBuf::from(format!("/proc/self/fd/{}", self.root.as_raw_fd()));
        let f = OpenOptions::new()
            .read(true)
            .custom_flags(O_NOFOLLOW)
            .open(anchor.join(&pin.name))?;
        let before = f.metadata()?;
        if !before.is_file()
            || before.uid() != current_uid()?
            || before.nlink() != 1
            || before.permissions().mode() & 0o777 != 0o600
            || before.len() > MAX_METADATA as u64
        {
            return invalid();
        }
        let mut bytes = Vec::new();
        (&f).take(MAX_METADATA as u64 + 1).read_to_end(&mut bytes)?;
        let after = f.metadata()?;
        if bytes.is_empty()
            || bytes.len() > MAX_METADATA
            || before.len() != bytes.len() as u64
            || before.len() != after.len()
            || before.mtime() != after.mtime()
            || before.mtime_nsec() != after.mtime_nsec()
            || before.ctime() != after.ctime()
            || before.ctime_nsec() != after.ctime_nsec()
            || sha(&bytes) != pin.sha256
        {
            return invalid();
        }
        self.total = self
            .total
            .checked_add(bytes.len())
            .ok_or(HostError::Configuration)?;
        if self.total > MAX_RETAINED {
            return invalid();
        }
        self.bytes
            .insert(pin.name.clone(), (pin.sha256.clone(), bytes.clone()));
        Ok(bytes)
    }
    fn record<T: DeserializeOwned>(&mut self, pin: &Pin) -> Result<T, HostError> {
        parse(&self.read(pin)?)
    }
}

const READINESS_PREFIX: &str = "native-readiness-v1-";
#[cfg(all(test, feature = "controlled-proof"))]
pub(crate) static READINESS_TEST_FAULTS: std::sync::Mutex<BTreeMap<PathBuf, String>> =
    std::sync::Mutex::new(BTreeMap::new());
#[cfg(all(test, feature = "controlled-proof"))]
fn readiness_test_fault(root: &Path, cut: &str) -> bool {
    let reached = READINESS_TEST_FAULTS
        .lock()
        .unwrap()
        .get(root)
        .is_some_and(|v| v == cut);
    if reached {
        fs::write(root.join("readiness-fault-reached"), cut).unwrap();
        eprintln!("readiness fault reached={cut}");
    }
    reached
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadinessReference {
    kind: String,
    member: String,
    sha256: String,
    length: usize,
}
#[derive(Clone)]
struct RetainedReadiness {
    reference: ReadinessReference,
    file: Arc<File>,
    bytes: Arc<Vec<u8>>,
}
impl Custody {
    fn check_root(&self) -> Result<(), HostError> {
        let held = self.root.metadata()?;
        let current = fs::symlink_metadata(&self.root_path)?;
        if !current.is_dir()
            || current.file_type().is_symlink()
            || current.dev() != held.dev()
            || current.ino() != held.ino()
            || current.uid() != current_uid()?
            || current.permissions().mode() & 0o777 != 0o700
            || fs::canonicalize(&self.root_path)? != self.root_path
        {
            return invalid();
        }
        Ok(())
    }
    fn readiness_path(&self, member: &str) -> PathBuf {
        PathBuf::from(format!("/proc/self/fd/{}", self.root.as_raw_fd())).join(member)
    }
    // Historical data readback has no live-child, ticket or authority ingress.
    fn read_readiness(&self, reference: &ReadinessReference) -> Result<(File, Vec<u8>), HostError> {
        self.check_root()?;
        if reference.kind != "native-readiness-observation-v1"
            || !valid_sha(&reference.sha256)
            || reference.member != format!("{READINESS_PREFIX}{}.json", reference.sha256)
            || reference.length == 0
            || reference.length > MAX_METADATA
        {
            return invalid();
        }
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(O_NOFOLLOW)
            .open(self.readiness_path(&reference.member))?;
        let before = file.metadata()?;
        if !before.is_file()
            || before.uid() != current_uid()?
            || before.nlink() != 1
            || before.permissions().mode() & 0o777 != 0o600
            || before.len() != reference.length as u64
        {
            return invalid();
        }
        let mut bytes = Vec::new();
        (&file)
            .take(MAX_METADATA as u64 + 1)
            .read_to_end(&mut bytes)?;
        let after = file.metadata()?;
        let named = fs::symlink_metadata(self.readiness_path(&reference.member))?;
        if bytes.len() != reference.length
            || sha(&bytes) != reference.sha256
            || before.len() != after.len()
            || before.ctime() != after.ctime()
            || before.ctime_nsec() != after.ctime_nsec()
            || named.dev() != after.dev()
            || named.ino() != after.ino()
            || named.file_type().is_symlink()
        {
            return invalid();
        }
        let record: ReadinessRecord = parse(&bytes)?;
        if record.version != 1
            || record.kind != reference.kind
            || record.phase != "held_readiness_observed"
        {
            return invalid();
        }
        self.check_root()?;
        Ok((file, bytes))
    }
    fn retain_readiness(&self, bytes: Vec<u8>) -> Result<RetainedReadiness, HostError> {
        if bytes.is_empty() || bytes.len() > MAX_METADATA {
            return invalid();
        }
        self.check_root()?;
        let digest = sha(&bytes);
        let reference = ReadinessReference {
            kind: "native-readiness-observation-v1".into(),
            member: format!("{READINESS_PREFIX}{digest}.json"),
            sha256: digest,
            length: bytes.len(),
        };
        #[cfg(all(test, feature = "controlled-proof"))]
        if readiness_test_fault(&self.root_path, "before-create") {
            return invalid();
        }
        match OpenOptions::new()
            .write(true)
            .read(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(O_NOFOLLOW)
            .open(self.readiness_path(&reference.member))
        {
            Ok(mut file) => {
                let m = file.metadata()?;
                if !m.is_file()
                    || m.uid() != current_uid()?
                    || m.nlink() != 1
                    || m.permissions().mode() & 0o777 != 0o600
                {
                    return invalid();
                }
                #[cfg(all(test, feature = "controlled-proof"))]
                if readiness_test_fault(&self.root_path, "partial-write") {
                    file.write_all(&bytes[..bytes.len() / 2])?;
                    return invalid();
                }
                file.write_all(&bytes)?;
                file.sync_all()?;
                #[cfg(all(test, feature = "controlled-proof"))]
                if readiness_test_fault(&self.root_path, "before-directory-sync") {
                    return invalid();
                }
                self.root.sync_all()?;
                #[cfg(all(test, feature = "controlled-proof"))]
                if readiness_test_fault(&self.root_path, "postpublication-tamper") {
                    file.set_len(1)?;
                }
                let (readback, actual) = self.read_readiness(&reference)?;
                let read_meta = readback.metadata()?;
                let held = file.metadata()?;
                if actual != bytes || held.dev() != read_meta.dev() || held.ino() != read_meta.ino()
                {
                    return invalid();
                }
                Ok(RetainedReadiness {
                    reference,
                    file: Arc::new(file),
                    bytes: Arc::new(bytes),
                })
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let (file, actual) = self.read_readiness(&reference)?;
                if actual != bytes {
                    return invalid();
                }
                file.sync_all()?;
                self.root.sync_all()?;
                self.check_root()?;
                Ok(RetainedReadiness {
                    reference,
                    file: Arc::new(file),
                    bytes: Arc::new(bytes),
                })
            }
            Err(error) => Err(error.into()),
        }
    }
    fn check_readiness(&self, retained: &RetainedReadiness) -> Result<(), HostError> {
        let (current, bytes) = self.read_readiness(&retained.reference)?;
        let a = current.metadata()?;
        let b = retained.file.metadata()?;
        if a.dev() != b.dev() || a.ino() != b.ino() || bytes != *retained.bytes {
            return invalid();
        }
        Ok(())
    }
    fn exclude_readiness(&self, native: &lifecycle_service::NativeConfig) -> Result<(), HostError> {
        self.check_root()?;
        let mut count = 0;
        let mut total = 0usize;
        for entry in fs::read_dir(self.readiness_path(""))? {
            let entry = entry?;
            if !entry
                .file_name()
                .to_string_lossy()
                .starts_with(READINESS_PREFIX)
            {
                continue;
            }
            count += 1;
            if count > 128 {
                return invalid();
            }
            let file = OpenOptions::new()
                .read(true)
                .custom_flags(O_NOFOLLOW)
                .open(entry.path())?;
            let m = file.metadata()?;
            if !m.is_file()
                || m.uid() != current_uid()?
                || m.nlink() != 1
                || m.permissions().mode() & 0o777 != 0o600
                || m.len() > MAX_METADATA as u64
            {
                return invalid();
            }
            let mut bytes = Vec::new();
            (&file)
                .take(MAX_METADATA as u64 + 1)
                .read_to_end(&mut bytes)?;
            total = total
                .checked_add(bytes.len())
                .ok_or(HostError::Configuration)?;
            if total > MAX_RETAINED || bytes.len() > MAX_METADATA {
                return invalid();
            }
            if !bytes.is_empty()
                && native
                    .snapshot
                    .members
                    .iter()
                    .any(|member| member.bytes.windows(bytes.len()).any(|w| w == bytes))
            {
                return invalid();
            }
        }
        self.check_root()
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadinessRecord {
    version: u32,
    kind: String,
    phase: String,
    qualification: String,
    startup_digest: String,
    selection_digest: String,
    startup_path: PathBuf,
    selection_path: PathBuf,
    operator_source_path: PathBuf,
    operator_source_digest: String,
    association: Value,
    host_pid: u32,
    host_starttime: String,
    root_device: u64,
    root_inode: u64,
    subject: String,
    context: String,
    native_readiness: Value,
    initial_observation: Value,
    trust_boundary: String,
}

fn upstream(startup: &HostStartup) -> Result<Value, HostError> {
    let mut value = serde_json::to_value(startup)?;
    if let Some(research) = value.get_mut("research").and_then(Value::as_object_mut) {
        research.remove("installation");
        research.remove("controlled_installation");
    }
    Ok(value)
}
fn validate_approvals(
    c: &mut Custody,
    a: &Authority,
) -> Result<research_register::card_admission::ValidatedInstallationRecords, HostError> {
    if a.approvals.len() != 2 {
        return invalid();
    }
    let scope: Scope = c.record(&a.disclosure_scope)?;
    if scope.decision_ids.len() > 256
        || scope.finding_ids.len() > 256
        || !scope.decision_ids.contains(&a.decision_id)
        || scope
            .decision_ids
            .iter()
            .chain(&scope.finding_ids)
            .any(|s| !bounded_text(s, 128))
    {
        return invalid();
    }
    let _ = scope.cross_decision_metadata;
    c.read(&a.source_bundle)?;
    let mut roles = BTreeSet::new();
    for approval in &a.approvals {
        if !bounded_text(&approval.issuer, 128) || !roles.insert(approval.role.as_str()) {
            return invalid();
        }
        if approval.git_pin.sha256 != approval.event.sha256 {
            return invalid();
        }
        c.read(&approval.source)?; // Retained source bytes, explicitly NOT authentication.
        let event: ApprovalEvent = c.record(&approval.event)?;
        if event.role != approval.role
            || event.issuer != approval.issuer
            || event.purpose != a.purpose
            || event.delegation != a.delegation
            || event.decision_id != a.decision_id
            || event.decision_revision != a.decision_revision
            || event.request_sha256 != a.key.request_digest
            || event.bundle_sha256 != a.source_bundle.sha256
            || event.scope_sha256 != sha(&serde_json::to_vec(&scope)?)
            || event.records.is_empty()
            || event.records.len() > 256
            || event.records.iter().any(|d| !valid_sha(d))
        {
            return invalid();
        }
    }
    if roles != BTreeSet::from(["qualification", "applicability"]) {
        return invalid();
    }
    let receipts = a
        .approvals
        .iter()
        .map(|approval| {
            Ok((
                approval.role.clone(),
                approval.issuer.clone(),
                approval.git_pin.clone(),
                c.read(&approval.event)?,
            ))
        })
        .collect::<Result<Vec<_>, HostError>>()?;
    research_register::card_admission::validate_installation_records(
        a.delegation.clone(),
        a.purpose.clone(),
        scope,
        receipts,
    )
    .map_err(|_| HostError::Configuration)
}
fn validate_admission(
    c: &mut Custody,
    pin: &Pin,
    kind: &str,
    subject: &str,
) -> Result<(), HostError> {
    let r: AdmissionRecord = c.record(pin)?;
    header(r.version, &r.kind, kind)?;
    if r.subject_digest != subject {
        return invalid();
    }
    c.read(&r.owner_source)?;
    c.read(&r.caller_evidence)?;
    Ok(())
}
fn payload_intent(op: &str, payload: Value) -> Result<(Role, ResearchKey, String), HostError> {
    match op {
        "research.prepare.v1" => {
            let p: Prepare = serde_json::from_value(payload)?;
            if !key_valid(&p.key) || !valid_sha(&p.basis_digest) {
                return invalid();
            }
            Ok((
                Role::Preparer,
                p.key.clone(),
                research_digest("prepare", &p)?,
            ))
        }
        "research.adopt.v1" => {
            let p: ResearchAdopt = serde_json::from_value(payload)?;
            if !key_valid(&p.key)
                || !valid_sha(&p.prepared_basis_digest)
                || p.expected_preparation_revision == 0
                || p.submit.task_id != p.key.task_id
                || p.submit.subject_id != p.key.subject_id
            {
                return invalid();
            }
            Ok((Role::Adopter, p.key.clone(), p.payload_digest()?))
        }
        "research.invalidate.v1" => {
            let p: Notice = serde_json::from_value(payload)?;
            if !key_valid(&p.key)
                || !id(&p.notice_id)
                || !id(&p.notice.selector)
                || !valid_sha(&p.notice.source_revision)
                || p.notice.reason.is_empty()
                || p.notice.reason.len() > 8192
                || p.notice.reason.contains('\0')
            {
                return invalid();
            }
            let role = if p.notice.selector.starts_with("research.") {
                Role::ResearchNotice
            } else if p.notice.selector.starts_with("decision.") {
                Role::DecisionNotice
            } else {
                return invalid();
            };
            let digest = research_digest("notice", &(&p.key, &p.notice_id, &p.notice))?;
            Ok((role, p.key, digest))
        }
        _ => invalid(),
    }
}

/// Checked DATA only. There is no conversion from this type to Installation,
/// ResearchProducer or VerifiedBasis. A later O1 disposition must establish that
/// distinct boundary; C1 must separately establish actual production delivery.
#[derive(Clone)]
struct Staged {
    checked: research_register::card_admission::ValidatedInstallationRecords,
    custody: Arc<Custody>,
    manifest: OperationManifest,
    authority: Authority,
    association: retained_task_workflow::research_admission::ResearchAdmissionAssociation,
}
impl Staged {
    fn exact_intent(&self, operation: &str, payload: Value) -> Result<(), HostError> {
        let (role, key, digest) = payload_intent(operation, payload)?;
        if !self.manifest.intents.iter().any(|i| {
            i.operation == operation && i.role == role && i.key == key && i.payload_digest == digest
        }) {
            return invalid();
        }
        Ok(())
    }
}
fn stage(startup: &HostStartup, selected: &InstallationSelection) -> Result<Staged, HostError> {
    if selected.version != 1 {
        return invalid();
    }
    let n = startup.research.as_ref().ok_or(HostError::Configuration)?;
    if n.controlled_fixture.is_some() {
        return invalid();
    }
    let mut c = Custody::open(&selected.root)?;
    // No protection claim is inferred from this necessary path exclusion.
    let native = startup
        .lifecycle
        .native
        .as_ref()
        .ok_or(HostError::Configuration)?;
    let native_root = fs::canonicalize(&native.root)?;
    if selected.root.starts_with(&native_root) || native_root.starts_with(&selected.root) {
        return invalid();
    }
    let a: Authority = c.record(&selected.installation)?;
    header(a.version, &a.kind, "research-authority-installation-v1")?;
    if !id(&a.id)
        || !key_valid(&a.key)
        || a.key.task_id != n.task_id
        || a.key.subject_id != n.subject_id
        || !bounded_text(&a.purpose, 8192)
        || !id(&a.decision_id)
        || !bounded_text(&a.decision_revision, 8192)
        || !bounded_text(&a.delegation, 8192)
        || a.artifact_root != selected.root
        || a.producer_roles.is_empty()
        || a.selectors.is_empty()
        || a.selectors.len() > 256
        || a.selectors
            .iter()
            .any(|s| !id(s) || !(s.starts_with("research.") || s.starts_with("decision.")))
    {
        return invalid();
    }
    let checked = validate_approvals(&mut c, &a)?;
    validate_admission(
        &mut c,
        &selected.authority_admission,
        "research-authority-admission-v1",
        &selected.installation.sha256,
    )?;
    let configuration: Configuration = c.record(&selected.configuration)?;
    header(
        configuration.version,
        &configuration.kind,
        "research-admission-configuration-v1",
    )?;
    if configuration.installation_digest != selected.installation.sha256
        || configuration.authority_admission_digest != selected.authority_admission.sha256
        || configuration.upstream_startup != upstream(startup)?
    {
        return invalid();
    }
    let manifest: OperationManifest = c.record(&selected.operations)?;
    header(
        manifest.version,
        &manifest.kind,
        "research-operation-manifest-v1",
    )?;
    if !id(&manifest.id)
        || manifest.installation_digest != selected.installation.sha256
        || manifest.configuration_digest != selected.configuration.sha256
        || manifest.intents.is_empty()
        || manifest.intents.len() > 16
    {
        return invalid();
    }
    let mut intents = BTreeSet::new();
    for i in &manifest.intents {
        if i.key != a.key
            || !a.producer_roles.contains(&i.role)
            || !valid_sha(&i.payload_digest)
            || !intents.insert((i.operation.clone(), i.payload_digest.clone()))
        {
            return invalid();
        }
        let payload: Value = c.record(&i.payload)?;
        let (role, key, digest) = payload_intent(&i.operation, payload.clone())?;
        if role != i.role || key != i.key || digest != i.payload_digest {
            return invalid();
        }
        if i.operation == "research.invalidate.v1" {
            let notice: Notice = serde_json::from_value(payload)?;
            if !a.selectors.contains(&notice.notice.selector) {
                return invalid();
            }
        }
        // Notice-only intents deliberately have no dependency on a checked basis.
    }
    validate_admission(
        &mut c,
        &selected.operation_admission,
        "research-operation-admission-v1",
        &selected.operations.sha256,
    )?;
    let staged = Staged {
        checked,
        custody: Arc::new(Custody {
            root: c.root.try_clone()?,
            root_path: c.root_path.clone(),
            bytes: c.bytes.clone(),
            total: c.total,
        }),
        manifest,
        authority: a,
        association: {
            let admission: AdmissionRecord = c.record(&selected.operation_admission)?;
            retained_task_workflow::research_admission::ResearchAdmissionAssociation {
                qualification: "controlled_only".into(),
                producer_role: "".into(),
                operation: "research.invalidate.v1".into(),
                startup_digest: sha(&serde_json::to_vec(startup)?),
                installation_digest: selected.installation.sha256.clone(),
                configuration_digest: selected.configuration.sha256.clone(),
                operation_manifest_digest: selected.operations.sha256.clone(),
                authority_admission_digest: selected.authority_admission.sha256.clone(),
                operation_admission_digest: selected.operation_admission.sha256.clone(),
                caller_evidence_digest: admission.caller_evidence.sha256,
                payload_digest: "".into(),
                selection_digest: None,
                operator_source_digest: None,
                protection_evidence_digest: None,
                native_realization_digest: None,
                current_host_identity_digest: None,
                currentness_evidence: None,
            }
        },
    };
    for i in &staged.manifest.intents {
        staged.exact_intent(&i.operation, c.record(&i.payload)?)?;
    }
    // A snapshot may disclose approved research/card bytes, never the protected
    // installation/approval/source records themselves. This is only a necessary
    // static check; actual tools, mounts and descriptor exclusion stay unestablished.
    c.exclude_readiness(native)?;
    for member in &native.snapshot.members {
        if c.bytes
            .values()
            .any(|(_, bytes)| member.bytes.windows(bytes.len()).any(|w| w == bytes))
        {
            return invalid();
        }
    }
    Ok(staged)
}

pub(crate) fn validate_records(startup: &HostStartup) -> Result<(), HostError> {
    let Some(selected) = startup
        .research
        .as_ref()
        .and_then(|n| n.installation.as_ref())
    else {
        return Ok(());
    };
    let _staged = stage(startup, selected)?;
    // Validation is data checking. Only dedicated trusted operator composition
    // can admit these records; ordinary entries refuse before opening stores.
    Ok(())
}

/// Pure record-level replacement proof. Authentication of R_O2 belongs to the
/// new externally authorized startup; this value only proves bounded data change.
struct ValidatedInstalledRestartReplacement;
pub(crate) fn normalize_installed_restart(
    old: &HostStartup,
    current: &HostStartup,
    normalized: &mut HostStartup,
) -> Result<(), HostError> {
    let old_selection = old.research.as_ref().and_then(|r| r.installation.as_ref());
    let current_selection = current
        .research
        .as_ref()
        .and_then(|r| r.installation.as_ref());
    match (old_selection, current_selection) {
        (None, None) => Ok(()),
        (Some(prior), Some(next)) => {
            let _proof = validate_installed_restart_replacement(old, current, prior, next)?;
            let normalized_selection = normalized
                .research
                .as_mut()
                .and_then(|r| r.installation.as_mut())
                .ok_or(HostError::Configuration)?;
            normalized_selection.configuration = prior.configuration.clone();
            normalized_selection.operations = prior.operations.clone();
            normalized_selection.operation_admission = prior.operation_admission.clone();
            Ok(())
        }
        _ => invalid(),
    }
}
fn validate_installed_restart_replacement(
    old: &HostStartup,
    current: &HostStartup,
    prior: &InstallationSelection,
    next: &InstallationSelection,
) -> Result<ValidatedInstalledRestartReplacement, HostError> {
    // Both stages independently require complete C/upstream(S) equality. The
    // semantic normalization below never participates in those comparisons.
    let before = stage(old, prior)?;
    let after = stage(current, next)?;
    if prior.version != next.version
        || prior.root != next.root
        || prior.installation != next.installation
        || prior.authority_admission != next.authority_admission
        || serde_json::to_value(&before.authority)? != serde_json::to_value(&after.authority)?
        || serde_json::to_value(&before.manifest.intents)?
            != serde_json::to_value(&after.manifest.intents)?
        || before.manifest.installation_digest != after.manifest.installation_digest
        || (prior.configuration != next.configuration
            && prior.configuration.name == next.configuration.name)
        || (prior.operations != next.operations
            && (prior.operations.name == next.operations.name
                || before.manifest.id == after.manifest.id))
        || (prior.operation_admission != next.operation_admission
            && prior.operation_admission.name == next.operation_admission.name)
    {
        return invalid();
    }
    // Exact I/R_I and every transitive immutable upstream artifact stay retained.
    // Replaced O/R_O retain their old source/caller history; new source/caller
    // data are admitted only by the separately authenticated v2 successor entry.
    for (name, (digest, bytes)) in &before.custody.bytes {
        let actual = fs::read(prior.root.join(name))?;
        if sha(&actual) != *digest || actual != *bytes {
            return invalid();
        }
    }
    Ok(ValidatedInstalledRestartReplacement)
}

/// Private installed scope; only admitted startup composition or explicit
/// engineering qualification can obtain it.
pub(crate) struct InstalledResearch {
    staged: Staged,
    _authority: research_register::card_admission::Installation,
    admitted: Option<AdmittedResearchStartup>,
}
#[cfg(feature = "controlled-proof")]
pub(crate) fn load_controlled(
    startup: &HostStartup,
) -> Result<Option<InstalledResearch>, HostError> {
    startup
        .research
        .as_ref()
        .and_then(|n| n.controlled_installation.as_ref())
        .map(|selected| {
            stage(startup, selected).map(|staged| InstalledResearch {
                staged,
                _authority: research_register::card_admission::Installation::unavailable(),
                admitted: None,
            })
        })
        .transpose()
}
impl InstalledResearch {
    pub(crate) fn register(
        &self,
        store: &retained_task_workflow::TaskStore,
    ) -> Result<(), HostError> {
        store.register_key_from_admitted_startup(
            &self.staged.authority.key,
            &self.staged.authority.selectors,
        )?;
        Ok(())
    }
    pub(crate) fn dispatch(
        &self,
        store: &retained_task_workflow::TaskStore,
        request: &Value,
    ) -> Result<Value, HostError> {
        use retained_task_workflow::research_admission::ProducerRole;
        let operation = request["op"].as_str().ok_or(HostError::Configuration)?;
        self.staged
            .exact_intent(operation, request["payload"].clone())?;
        // C1 cannot be manufactured by an installed manifest, even in this
        // engineering route. Only notices can use these scoped producers.
        if operation != "research.invalidate.v1" {
            return Err(HostError::Task(
                retained_task_workflow::TaskError::Unavailable,
            ));
        }
        let payload: Notice = serde_json::from_value(request["payload"].clone())?;
        let (role, _, digest) = payload_intent(operation, request["payload"].clone())?;
        if let Some(receipt) =
            store.read_research_notice(&payload.key, &payload.notice_id, &digest)?
        {
            let qualification = receipt
                .admission
                .as_ref()
                .map(|a| a.qualification.as_str())
                .unwrap_or("controlled_only");
            return Ok(
                serde_json::json!({"status":"invalidated","receipt":receipt,"qualification":qualification}),
            );
        }
        let currentness = self
            .admitted
            .as_ref()
            .map(|admitted| {
                let observation = admitted.observe_current()?;
                let bytes = serde_json::to_vec(&serde_json::json!({
                    "version":1,"kind":"native-currentness-v1",
                    "observation":observation,"readiness":admitted.readiness.reference,"qualification":admitted.qualification,
                    "startup_digest":admitted.startup_digest,
                    "selection_digest":admitted.selection_digest,
                    "native_realization_digest":admitted.native_realization_digest,
                    "host_pid":admitted.caller_pid,"host_starttime":admitted.caller_starttime,
                    "operation":operation,"payload_digest":digest
                }))?;
                Ok::<_, HostError>(
                    retained_task_workflow::research_admission::RetainedArtifact {
                        id: "native-currentness-v1".into(),
                        digest: sha(&bytes),
                        bytes,
                    },
                )
            })
            .transpose()?;
        let role = match role {
            Role::ResearchNotice => ProducerRole::ResearchNotice,
            Role::DecisionNotice => ProducerRole::DecisionNotice,
            _ => return invalid(),
        };
        let mut association = self.staged.association.clone();
        association.currentness_evidence = currentness;
        association.payload_digest = digest;
        association.producer_role = match role {
            ProducerRole::ResearchNotice => "research_notice",
            ProducerRole::DecisionNotice => "decision_notice",
            _ => return invalid(),
        }
        .into();
        let producer =
            store.install_notice_from_admitted_startup(&payload.key, role, association)?;
        let receipt = store.invalidate_research(
            &payload.key,
            &payload.notice_id,
            &payload.notice,
            &producer,
        )?;
        Ok(
            serde_json::json!({"status":"invalidated","receipt":receipt,"qualification":self.staged.association.qualification}),
        )
    }
}

/// Data ingress is private. Authentication is an external precondition of the
/// dedicated trusted operator invocation, never a property of these JSON fields.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OperatorInput {
    version: u32,
    startup_digest: String,
    selection_digest: String,
    installation_digest: String,
    configuration_digest: String,
    operation_manifest_digest: String,
    installation_approval_source: Pin,
    operation_approval_source: Pin,
    authority_admission_digest: String,
    operation_admission_digest: String,
    observer_policy: String,
    protected_root: PathBuf,
}

#[derive(Clone)]
pub(crate) struct AuthorizedResearchStartup {
    records: Staged,
    identity: crate::selected_startup::SelectedIdentity,
    input_digest: String,
    input_path: PathBuf,
    host_pid: u32,
    host_starttime: String,
    controlled: bool,
}
impl AuthorizedResearchStartup {
    pub(crate) fn protection_spec(
        &self,
        startup: &HostStartup,
    ) -> Result<lifecycle_service::ProtectedSurfaceSpec, HostError> {
        let root = startup
            .research
            .as_ref()
            .and_then(|r| r.installation.as_ref())
            .ok_or(HostError::Configuration)?
            .root
            .clone();
        let retained = self.records.custody.root.metadata()?;
        let current = fs::symlink_metadata(&root)?;
        if retained.dev() != current.dev()
            || retained.ino() != current.ino()
            || !current.is_dir()
            || current.file_type().is_symlink()
        {
            return invalid();
        }
        // The selected parent is a mutable identity/protection anchor. Its
        // selection/config members separately bind exact immutable bytes.
        let immutable_files = self
            .records
            .custody
            .bytes
            .iter()
            .map(|(name, (digest, _))| (root.join(name), digest.clone()))
            .chain(std::iter::once((
                self.input_path.clone(),
                self.input_digest.clone(),
            )))
            .chain([
                (
                    self.identity.startup_path.clone(),
                    self.identity.startup_digest.clone(),
                ),
                (
                    self.identity.selection_path.clone(),
                    self.identity.selection_digest.clone(),
                ),
            ])
            .collect();
        let parent = startup
            .task_store_path
            .parent()
            .ok_or(HostError::Configuration)?;
        Ok(lifecycle_service::ProtectedSurfaceSpec {
            identity_anchors: vec![parent.to_path_buf()],
            roots: vec![
                root,
                startup.task_store_path.clone(),
                startup.lifecycle.store_root.clone(),
                startup.lifecycle.socket_path.clone(),
                parent.join("startup.lock"),
                parent.join("startup-intent.json"),
                parent.join("startup-receipt.json"),
            ],
            future_members: vec![
                startup.task_store_path.with_extension("owner.lock"),
                PathBuf::from(format!("{}-wal", startup.task_store_path.display())),
                PathBuf::from(format!("{}-shm", startup.task_store_path.display())),
                startup.socket_path.clone(),
            ],
            immutable_files,
        })
    }
    #[cfg(feature = "controlled-proof")]
    pub(crate) fn controlled(&self) -> bool {
        self.controlled
    }
    pub(crate) fn admit(
        &self,
        pending: &lifecycle_service::native_startup::PendingNativeStartup,
        caller: &crate::HostIdentity,
    ) -> Result<AdmittedResearchStartup, HostError> {
        if self.host_pid != caller.pid
            || self.host_starttime != caller.linux_starttime_ticks
            || self.host_pid != std::process::id()
            || process_starttime(self.host_pid)? != self.host_starttime
            || pending.observation().controlled_only() != self.controlled
        {
            return invalid();
        }
        pending
            .guard()
            .check_current()
            .map_err(|_| HostError::Configuration)?;
        let evidence = pending.evidence();
        let startup: HostStartup = serde_json::from_slice(&self.identity.startup_bytes)?;
        let native = startup
            .lifecycle
            .native
            .as_ref()
            .ok_or(HostError::Configuration)?;
        if !pending
            .observation()
            .matches_protected_surface(&self.protection_spec(&startup)?)
            || evidence["thread_ack"]["thread"]["id"] != evidence["thread"]
            || evidence["thread_ack"]["thread"]["ephemeral"] != false
            || !pending.has_pinned_protocol()
            || evidence["thread_ack_sequence"]
                .as_u64()
                .is_none_or(|n| n == 0)
            || evidence["native_configuration_digest"] != sha(&serde_json::to_vec(native)?)
            || evidence["session_id"] != native.session_id
            || !evidence["transport"]
                .as_str()
                .is_some_and(|s| s.starts_with(&format!("transport:{}:", native.session_id)))
            || evidence["thread"].as_str().is_none_or(str::is_empty)
        {
            return invalid();
        }
        #[cfg(all(test, feature = "controlled-proof"))]
        OBSERVED_NATIVE_PIDS
            .lock()
            .unwrap()
            .push(pending.observation().native_pid());
        let root = self.records.custody.root.metadata()?;
        let mut retained_bytes = serde_json::to_vec(&ReadinessRecord {
            version:1, kind:"native-readiness-observation-v1".into(), phase:"held_readiness_observed".into(),
            qualification:if self.controlled {"controlled_only"} else {"owner_admitted_startup"}.into(),
            startup_digest:self.identity.startup_digest.clone(),selection_digest:self.identity.selection_digest.clone(),
            startup_path:self.identity.startup_path.clone(), selection_path:self.identity.selection_path.clone(),
            operator_source_path:self.input_path.clone(), operator_source_digest:self.input_digest.clone(),
            association:serde_json::to_value(&self.records.association)?,
            host_pid:self.host_pid, host_starttime:self.host_starttime.clone(), root_device:root.dev(), root_inode:root.ino(),
            subject:startup.lifecycle.subject_id.clone(), context:startup.lifecycle.context_generation.clone(),
            native_readiness:evidence.clone(), initial_observation:pending.observation().evidence().clone(),
            trust_boundary:"owned-harness observation; trusted installer/host/kernel; excludes hostile same-UID operator; no installation, activation or continuous-liveness assertion".into(),
        })?;
        retained_bytes.push(b'\n');
        if retained_bytes.len() > MAX_METADATA
            || native.snapshot.members.iter().any(|member| {
                member
                    .bytes
                    .windows(retained_bytes.len())
                    .any(|w| w == retained_bytes)
            })
        {
            return invalid();
        }
        let readiness = self.records.custody.retain_readiness(retained_bytes)?;
        pending
            .guard()
            .check_current()
            .map_err(|_| HostError::Configuration)?;
        self.records.custody.check_readiness(&readiness)?;
        Ok(AdmittedResearchStartup {
            readiness,
            records: self.records.clone(),
            guard: pending.guard(),
            startup_digest: self.identity.startup_digest.clone(),
            selection_digest: self.identity.selection_digest.clone(),
            caller_pid: self.host_pid,
            caller_starttime: self.host_starttime.clone(),
            installation_digest: self.records.association.installation_digest.clone(),
            configuration_digest: self.records.association.configuration_digest.clone(),
            operation_manifest_digest: self.records.association.operation_manifest_digest.clone(),
            operator_source_digest: self.input_digest.clone(),
            protection_evidence_digest: sha(&serde_json::to_vec(pending.observation().evidence())?),
            native_realization_digest: sha(&serde_json::to_vec(evidence)?),
            qualification: if self.controlled {
                "controlled_only"
            } else {
                "owner_admitted_startup"
            }
            .into(),
        })
    }
}
/// Only trusted startup composition constructs this consequence. It cannot be
/// deserialized, supplied by a native tool or derived from an RPC request.
/// Its authority assumes the actual Codex operator has externally authenticated
/// the owner's exact approvals and the independent observer's evidence.
#[derive(Clone)]
pub(crate) struct AdmittedResearchStartup {
    readiness: RetainedReadiness,
    records: Staged,
    guard: lifecycle_service::native_startup::NativeCurrentnessGuard,
    startup_digest: String,
    selection_digest: String,
    caller_pid: u32,
    caller_starttime: String,
    installation_digest: String,
    configuration_digest: String,
    operation_manifest_digest: String,
    operator_source_digest: String,
    protection_evidence_digest: String,
    native_realization_digest: String,
    qualification: String,
}
impl AdmittedResearchStartup {
    pub(crate) fn check_current(&self) -> Result<(), HostError> {
        self.observe_current().map(|_| ())
    }
    fn observe_current(&self) -> Result<Value, HostError> {
        if self.caller_pid != std::process::id()
            || self.caller_starttime != process_starttime(self.caller_pid)?
        {
            return invalid();
        }
        self.records.custody.check_readiness(&self.readiness)?;
        self.guard
            .observe_current()
            .map_err(|_| HostError::Configuration)
    }
}
fn process_starttime(pid: u32) -> Result<String, HostError> {
    let raw = fs::read_to_string(format!("/proc/{pid}/stat"))?;
    let tail = raw.rsplit_once(')').ok_or(HostError::Configuration)?.1;
    if matches!(tail.split_whitespace().next(), Some("Z" | "X")) {
        return invalid();
    }
    tail.split_whitespace()
        .nth(19)
        .map(str::to_owned)
        .ok_or(HostError::Configuration)
}
/// Called ONLY by the dedicated external-operator selected-startup route, after
/// exact selection/config verification and before opening stores. The call is
/// the trusted outer admission boundary; the file is merely its retained input.
#[cfg(all(test, feature = "controlled-proof"))]
pub(crate) static CONTROLLED_OPERATOR_INPUTS: std::sync::Mutex<BTreeSet<PathBuf>> =
    std::sync::Mutex::new(BTreeSet::new());
#[cfg(all(test, feature = "controlled-proof"))]
pub(crate) static OBSERVED_NATIVE_PIDS: std::sync::Mutex<Vec<u32>> =
    std::sync::Mutex::new(Vec::new());
pub(crate) fn admit_external_operator(
    startup: &HostStartup,
    selected: &crate::selected_startup::SelectedIdentity,
    input_path: &Path,
) -> Result<AuthorizedResearchStartup, HostError> {
    // This named ingress is reserved for the actual external operator. A raw
    // input field cannot select controlled evidence or elevate its classification.
    #[cfg(all(test, feature = "controlled-proof"))]
    let controlled = CONTROLLED_OPERATOR_INPUTS
        .lock()
        .unwrap()
        .contains(input_path);
    #[cfg(not(all(test, feature = "controlled-proof")))]
    let controlled = false;
    admit_operator_input(startup, selected, input_path, controlled)
}
fn admit_operator_input(
    startup: &HostStartup,
    selected: &crate::selected_startup::SelectedIdentity,
    input_path: &Path,
    controlled: bool,
) -> Result<AuthorizedResearchStartup, HostError> {
    if !controlled {
        let native = startup
            .lifecycle
            .native
            .as_ref()
            .ok_or(HostError::Configuration)?;
        if cfg!(feature = "controlled-proof")
            || !startup.control.is_native()
            || native.provider_route != crate::NativeProviderRoute::OpenaiChatgptAccessLeaseV1
            || native
                .chatgpt_access_source
                .as_ref()
                .and_then(|s| s.isolation.as_ref())
                .is_none()
        {
            return invalid();
        }
    }

    if selected.startup_bytes.is_empty()
        || selected.selection_bytes.is_empty()
        || sha(&selected.startup_bytes) != selected.startup_digest
        || sha(&selected.selection_bytes) != selected.selection_digest
    {
        return invalid();
    }
    let installed = startup
        .research
        .as_ref()
        .and_then(|n| n.installation.as_ref())
        .ok_or(HostError::Configuration)?;
    let checked = stage(startup, installed)?;
    if input_path.parent() != Some(installed.root.as_path()) {
        return invalid();
    }
    if !input_path.is_absolute() || fs::canonicalize(input_path)? != input_path {
        return invalid();
    }
    let name = input_path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or(HostError::Configuration)?;
    if !id(name) || input_path.as_os_str().len() > 8192 {
        return invalid();
    }
    // Descriptor-bound source acquisition: do not validate and reopen this path.
    let root = Custody::open(&installed.root)?;
    let f = OpenOptions::new()
        .read(true)
        .custom_flags(O_NOFOLLOW)
        .open(PathBuf::from(format!("/proc/self/fd/{}", root.root.as_raw_fd())).join(name))?;
    let before = f.metadata()?;
    if !before.is_file()
        || before.uid() != current_uid()?
        || before.permissions().mode() & 0o777 != 0o600
        || before.nlink() != 1
        || before.len() > MAX_METADATA as u64
    {
        return invalid();
    }
    let mut bytes = Vec::new();
    (&f).take(MAX_METADATA as u64 + 1).read_to_end(&mut bytes)?;
    let after = f.metadata()?;
    if before.len() != bytes.len() as u64
        || before.len() != after.len()
        || before.ctime() != after.ctime()
        || before.ctime_nsec() != after.ctime_nsec()
    {
        return invalid();
    }
    let input: OperatorInput = parse(&bytes)?;
    if input.version != 2
        || input.startup_digest != selected.startup_digest
        || input.selection_digest != selected.selection_digest
        || input.installation_digest != installed.installation.sha256
        || input.configuration_digest != installed.configuration.sha256
        || input.operation_manifest_digest != installed.operations.sha256
        || input.authority_admission_digest != installed.authority_admission.sha256
        || input.operation_admission_digest != installed.operation_admission.sha256
        || input.observer_policy != "owned-native-protected-surface-v1"
        || input.protected_root != installed.root
    {
        return invalid();
    }
    let mut custody = root;
    let authority_admission: AdmissionRecord = custody.record(&installed.authority_admission)?;
    let operation_admission: AdmissionRecord = custody.record(&installed.operation_admission)?;
    if authority_admission.owner_source != input.installation_approval_source
        || operation_admission.owner_source != input.operation_approval_source
    {
        return invalid();
    }
    custody.read(&input.installation_approval_source)?;
    custody.read(&input.operation_approval_source)?;
    let native = startup
        .lifecycle
        .native
        .as_ref()
        .ok_or(HostError::Configuration)?;
    for member in &native.snapshot.members {
        if member
            .bytes
            .windows(selected.startup_bytes.len())
            .any(|w| w == selected.startup_bytes)
            || member
                .bytes
                .windows(selected.selection_bytes.len())
                .any(|w| w == selected.selection_bytes)
            || member.bytes.windows(bytes.len()).any(|w| w == bytes)
            || custody
                .bytes
                .values()
                .any(|(_, b)| member.bytes.windows(b.len()).any(|w| w == b))
        {
            return invalid();
        }
    }
    Ok(AuthorizedResearchStartup {
        records: checked,
        identity: selected.clone(),
        input_digest: sha(&bytes),
        input_path: input_path.to_path_buf(),
        host_pid: std::process::id(),
        host_starttime: process_starttime(std::process::id())?,
        controlled,
    })
}
pub(crate) fn load_admitted(
    startup: &HostStartup,
    admission: &AdmittedResearchStartup,
    identity: &crate::selected_startup::SelectedIdentity,
    caller: &crate::HostIdentity,
) -> Result<InstalledResearch, HostError> {
    let selected = startup
        .research
        .as_ref()
        .and_then(|n| n.installation.as_ref())
        .ok_or(HostError::Configuration)?;
    if admission.startup_digest != identity.startup_digest
        || admission.selection_digest != identity.selection_digest
        || admission.caller_pid != std::process::id()
        || admission.caller_starttime != process_starttime(std::process::id())?
        || admission.installation_digest != selected.installation.sha256
        || admission.configuration_digest != selected.configuration.sha256
        || admission.operation_manifest_digest != selected.operations.sha256
    {
        return invalid();
    }
    if caller.pid != admission.caller_pid
        || caller.linux_starttime_ticks != admission.caller_starttime
    {
        return invalid();
    }
    admission.check_current()?;
    let mut staged = admission.records.clone();
    let authority = research_register::card_admission::Installation::install_from_admitted_startup(
        staged.checked.clone(),
    );
    staged.association.qualification = admission.qualification.clone();
    staged.association.current_host_identity_digest = Some(sha(&serde_json::to_vec(caller)?));
    staged.association.startup_digest = admission.startup_digest.clone();
    staged.association.selection_digest = Some(admission.selection_digest.clone());
    staged.association.operator_source_digest = Some(admission.operator_source_digest.clone());
    staged.association.protection_evidence_digest =
        Some(admission.protection_evidence_digest.clone());
    staged.association.native_realization_digest =
        Some(admission.native_realization_digest.clone());
    Ok(InstalledResearch {
        staged,
        _authority: authority,
        admitted: Some(admission.clone()),
    })
}
/// No evidence value is derived for ordinary startup entries.
pub(crate) fn refuse_unadmitted(startup: &HostStartup) -> Result<(), HostError> {
    if startup
        .research
        .as_ref()
        .is_some_and(|n| n.installation.is_some())
    {
        return Err(HostError::Task(
            retained_task_workflow::TaskError::Unavailable,
        ));
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::research_admission::ResearchNomination;
    use serde_json::json;
    use std::os::unix::fs::symlink;

    #[test]
    fn readiness_custody_is_durable_exact_bounded_and_non_authoritative() {
        let dir = tempfile::tempdir().unwrap();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let custody = Custody::open(dir.path()).unwrap();
        let mut record = ReadinessRecord {
            version: 1,
            kind: "native-readiness-observation-v1".into(),
            phase: "held_readiness_observed".into(),
            qualification: "controlled_only".into(),
            startup_digest: "1".repeat(64),
            selection_digest: "2".repeat(64),
            startup_path: PathBuf::from("/startup"),
            selection_path: PathBuf::from("/selection"),
            operator_source_path: PathBuf::from("/operator"),
            operator_source_digest: "3".repeat(64),
            association: json!({}),
            host_pid: std::process::id(),
            host_starttime: "test-only".into(),
            root_device: 0,
            root_inode: 0,
            subject: "subject:test".into(),
            context: "context:test".into(),
            native_readiness: json!({"thread_request":{"actual":"bytes"}}),
            initial_observation: json!({"controlled_only":true}),
            trust_boundary: "synthetic storage control".into(),
        };
        let mut bytes = serde_json::to_vec(&record).unwrap();
        bytes.push(b'\n');
        let held = custody.retain_readiness(bytes.clone()).unwrap();
        custody.check_readiness(&held).unwrap();
        let repeated = custody.retain_readiness(bytes.clone()).unwrap();
        assert_eq!(
            held.file.metadata().unwrap().ino(),
            repeated.file.metadata().unwrap().ino()
        );
        assert_eq!(custody.read_readiness(&held.reference).unwrap().1, bytes);
        assert!(
            custody
                .retain_readiness(vec![b'x'; MAX_METADATA + 1])
                .is_err()
        );
        // Exact stored limits include JSON expansion and the trailing newline.
        let base = serde_json::to_vec(&record).unwrap().len() + 1;
        record
            .trust_boundary
            .push_str(&"x".repeat(MAX_METADATA - base));
        let mut at_limit = serde_json::to_vec(&record).unwrap();
        at_limit.push(b'\n');
        assert_eq!(at_limit.len(), MAX_METADATA);
        custody.retain_readiness(at_limit.clone()).unwrap();
        record.trust_boundary.push('x');
        let mut overflow = serde_json::to_vec(&record).unwrap();
        overflow.push(b'\n');
        assert!(custody.retain_readiness(overflow).is_err());
        let path = dir.path().join(&held.reference.member);
        fs::write(&path, b"partial").unwrap();
        assert!(custody.read_readiness(&held.reference).is_err());
        assert!(custody.retain_readiness(bytes).is_err());
        assert_eq!(fs::read(path).unwrap(), b"partial");
    }

    fn put(root: &Path, name: &str, bytes: &[u8]) -> Pin {
        fs::write(root.join(name), bytes).unwrap();
        fs::set_permissions(root.join(name), fs::Permissions::from_mode(0o600)).unwrap();
        Pin {
            name: name.into(),
            sha256: sha(bytes),
        }
    }
    fn record(root: &Path, name: &str, value: &Value) -> Pin {
        put(root, name, &serde_json::to_vec(value).unwrap())
    }
    fn fixture(dir: &Path) -> (HostStartup, InstallationSelection, Value) {
        let mut startup =
            crate::tests::admission_host(dir, &dir.join("unused.sock"), false).startup;
        for grant in &mut startup.lifecycle.grants {
            grant.expires_wall_ms = 9_007_199_254_740_991;
        }
        let native = dir.join("native");
        fs::create_dir(&native).unwrap();
        startup.lifecycle.native = Some(serde_json::from_value(json!({
            "executable": "/bin/false", "root": native, "simulator_base_url": "http://127.0.0.1:1",
            "session_id": "synthetic-session", "invocation_id":"synthetic-invocation", "attempt_id":"synthetic-attempt",
            "prompt":"synthetic", "turn_grant_ref":"turn", "turn_grant_revision":1,
            "snapshot_grant_ref":"snapshot", "snapshot_grant_revision":1,
            "snapshot":{"id":"snapshot-0","source_revision":"synthetic","members":[{"id":"visible","sha256":sha(b"visible"),"bytes":b"visible"}]}
        })).unwrap());
        attach_records(dir, startup)
    }
    pub(crate) fn attach_records(
        dir: &Path,
        mut startup: HostStartup,
    ) -> (HostStartup, InstallationSelection, Value) {
        // Installed records use the strict JSON safe-integer contract.
        for grant in &mut startup.lifecycle.grants {
            grant.expires_wall_ms = 9_007_199_254_740_991;
        }
        let root = dir.join("protected");
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        startup.research = Some(ResearchNomination {
            task_id: "task-0".into(),
            subject_id: startup.lifecycle.subject_id.clone(),
            controlled_fixture: None,
            installation: None,
            controlled_installation: None,
        });
        let key = json!({"task_id":"task-0","subject_id":startup.lifecycle.subject_id,"request_digest":sha(b"request")});
        let source = put(
            &root,
            "source",
            b"explicitly synthetic owner source; not authenticated",
        );
        let caller = put(
            &root,
            "caller",
            b"synthetic installer caller; not actual native protection",
        );
        let bundle = put(&root, "bundle", b"synthetic exact use bundle");
        let scope_value =
            json!({"decision_ids":["decision-1"],"finding_ids":[],"cross_decision_metadata":false});
        let scope = record(&root, "scope", &scope_value);
        let typed_scope: Scope = serde_json::from_value(scope_value).unwrap();
        let mut approvals = vec![];
        for role in ["qualification", "applicability"] {
            let event = record(
                &root,
                role,
                &json!({
                    "issuer":"synthetic-owner", "role":role, "delegation":"synthetic-delegation",
                    "purpose":"audit", "decision_id":"decision-1", "decision_revision":sha(b"decision"),
                    "request_sha256":sha(b"request"), "bundle_sha256":bundle.sha256,
                    "records":[sha(b"request")], "scope_sha256":sha(&serde_json::to_vec(&typed_scope).unwrap())
                }),
            );
            approvals.push(
                json!({"role":role,"issuer":"synthetic-owner","event":event,"source":source,"git_pin":{"commit":"a".repeat(40),"blob":"b".repeat(40),"path":format!("approvals/{role}.json"),"sha256":event.sha256}}),
            );
        }
        let installation = record(
            &root,
            "installation",
            &json!({
                "version":1,"kind":"research-authority-installation-v1","id":"authority-1","key":key,
                "purpose":"audit","decision_id":"decision-1","decision_revision":sha(b"decision"),
                "delegation":"synthetic-delegation","disclosure_scope":scope,"source_bundle":bundle,
                "approvals":approvals,"producer_roles":["preparer","research_notice","decision_notice","adopter"],
                "selectors":["research.source","decision.target"],"artifact_root":root
            }),
        );
        let authority_admission = record(
            &root,
            "authority-admission",
            &json!({
                "version":1,"kind":"research-authority-admission-v1","subject_digest":installation.sha256,
                "owner_source":source,"caller_evidence":caller
            }),
        );
        let configuration = record(
            &root,
            "configuration",
            &json!({
                "version":1,"kind":"research-admission-configuration-v1","installation_digest":installation.sha256,
                "authority_admission_digest":authority_admission.sha256,"upstream_startup":upstream(&startup).unwrap()
            }),
        );
        // R2: notice-only admission has no B and no synthetic delivery claim.
        let payload = json!({"key":key,"notice_id":"notice-1","notice":{
            "selector":"research.source","source_revision":sha(b"new-source"),"reason":"new source revision"
        }});
        let (_, _, digest) = payload_intent("research.invalidate.v1", payload.clone()).unwrap();
        let pin = record(&root, "notice-payload", &payload);
        let operations = record(
            &root,
            "operations",
            &json!({
                "version":1,"kind":"research-operation-manifest-v1","id":"operations-1",
                "installation_digest":installation.sha256,"configuration_digest":configuration.sha256,
                "intents":[{"operation":"research.invalidate.v1","role":"research_notice","key":key,
                    "payload_digest":digest,"payload":pin}]
            }),
        );
        let operation_admission = record(
            &root,
            "operation-admission",
            &json!({
                "version":1,"kind":"research-operation-admission-v1","subject_digest":operations.sha256,
                "owner_source":source,"caller_evidence":caller
            }),
        );
        let selected = InstallationSelection {
            version: 1,
            root,
            installation,
            authority_admission,
            configuration,
            operations,
            operation_admission,
        };
        startup.research.as_mut().unwrap().installation = Some(selected.clone());
        (startup, selected, payload)
    }
    pub(crate) fn authorization(
        startup: &HostStartup,
        selection_bytes: &[u8],
        startup_bytes: &[u8],
    ) -> PathBuf {
        authorization_named(startup, selection_bytes, startup_bytes, "operator-input")
    }
    pub(crate) fn authorization_named(
        startup: &HostStartup,
        selection_bytes: &[u8],
        startup_bytes: &[u8],
        name: &str,
    ) -> PathBuf {
        let selected = startup
            .research
            .as_ref()
            .unwrap()
            .installation
            .as_ref()
            .unwrap();
        let authority: AdmissionRecord =
            parse(&fs::read(selected.root.join(&selected.authority_admission.name)).unwrap())
                .unwrap();
        let operation: AdmissionRecord =
            parse(&fs::read(selected.root.join(&selected.operation_admission.name)).unwrap())
                .unwrap();
        let input = record(
            &selected.root,
            name,
            &json!({"version":2,"startup_digest":sha(startup_bytes),"selection_digest":sha(selection_bytes),"installation_digest":selected.installation.sha256,"configuration_digest":selected.configuration.sha256,"operation_manifest_digest":selected.operations.sha256,"installation_approval_source":authority.owner_source,"operation_approval_source":operation.owner_source,"authority_admission_digest":selected.authority_admission.sha256,"operation_admission_digest":selected.operation_admission.sha256,"observer_policy":"owned-native-protected-surface-v1","protected_root":selected.root}),
        );
        selected.root.join(input.name)
    }
    pub(crate) fn refreshed_records(startup: &mut HostStartup) {
        refreshed_records_named(startup, "successor");
    }
    pub(crate) fn refreshed_records_named(startup: &mut HostStartup, suffix: &str) {
        let mut selected = startup
            .research
            .as_ref()
            .unwrap()
            .installation
            .clone()
            .unwrap();
        let root = selected.root.clone();
        let mut configuration: Value =
            parse(&fs::read(root.join(&selected.configuration.name)).unwrap()).unwrap();
        configuration["upstream_startup"] = upstream(startup).unwrap();
        selected.configuration = record(&root, &format!("configuration-{suffix}"), &configuration);
        let mut operations: Value =
            parse(&fs::read(root.join(&selected.operations.name)).unwrap()).unwrap();
        operations["id"] = json!(format!("operations-{suffix}"));
        operations["configuration_digest"] = json!(selected.configuration.sha256);
        selected.operations = record(&root, &format!("operations-{suffix}"), &operations);
        let mut admission: Value =
            parse(&fs::read(root.join(&selected.operation_admission.name)).unwrap()).unwrap();
        admission["subject_digest"] = json!(selected.operations.sha256);
        selected.operation_admission =
            record(&root, &format!("operation-admission-{suffix}"), &admission);
        startup.research.as_mut().unwrap().installation = Some(selected);
    }
    // Every candidate gets new immutable members; predecessor records are never
    // rewritten merely to manufacture a negative successor.
    pub(crate) fn restart_negative(
        startup: &mut HostStartup,
        predecessor: &HostStartup,
        case: &str,
    ) {
        let prior = predecessor
            .research
            .as_ref()
            .unwrap()
            .installation
            .clone()
            .unwrap();
        if case == "changed-prompt" {
            startup
                .lifecycle
                .native
                .as_mut()
                .unwrap()
                .prompt
                .push_str(" changed");
        }
        refreshed_records_named(startup, case);
        let mut selected = startup
            .research
            .as_ref()
            .unwrap()
            .installation
            .clone()
            .unwrap();
        let root = selected.root.clone();
        match case {
            "changed-prompt" => {}
            "stale-current-C" => rewrite(&root, &mut selected.configuration, |v| {
                v["upstream_startup"]["answer_authority_ref"] = json!("stale-current-C");
            }),
            "changed-intent" => rewrite(&root, &mut selected.operations, |v| {
                let mut payload: Value = parse(
                    &fs::read(root.join(v["intents"][0]["payload"]["name"].as_str().unwrap()))
                        .unwrap(),
                )
                .unwrap();
                payload["notice"]["reason"] = json!("changed finite authorized intent");
                let pin = record(&root, "changed-intent-payload", &payload);
                let (_, _, digest) = payload_intent("research.invalidate.v1", payload).unwrap();
                v["intents"][0]["payload"] = serde_json::to_value(pin).unwrap();
                v["intents"][0]["payload_digest"] = json!(digest);
            }),
            "missing-owner-source" => rewrite(&root, &mut selected.operation_admission, |v| {
                v["owner_source"]["name"] = json!("absent-successor-owner-source");
            }),
            "old-operation-link" => {
                selected.operations = prior.operations;
                selected.operation_admission = prior.operation_admission;
            }
            _ => panic!("unknown restart negative {case}"),
        }
        if case == "stale-current-C" {
            rewrite(&root, &mut selected.operations, |v| {
                v["configuration_digest"] = json!(selected.configuration.sha256);
            });
        }
        if case != "old-operation-link" {
            rewrite(&root, &mut selected.operation_admission, |v| {
                v["subject_digest"] = json!(selected.operations.sha256);
            });
        }
        startup.research.as_mut().unwrap().installation = Some(selected);
    }
    pub(crate) fn rebind_records(startup: &mut HostStartup, selected: &mut InstallationSelection) {
        let root = selected.root.clone();
        rewrite(&root, &mut selected.authority_admission, |v| {
            v["subject_digest"] = json!(selected.installation.sha256)
        });
        rewrite(&root, &mut selected.configuration, |v| {
            v["installation_digest"] = json!(selected.installation.sha256);
            v["authority_admission_digest"] = json!(selected.authority_admission.sha256);
            v["upstream_startup"] = upstream(startup).unwrap();
        });
        rewrite(&root, &mut selected.operations, |v| {
            v["installation_digest"] = json!(selected.installation.sha256);
            v["configuration_digest"] = json!(selected.configuration.sha256);
        });
        rewrite(&root, &mut selected.operation_admission, |v| {
            v["subject_digest"] = json!(selected.operations.sha256)
        });
        startup.research.as_mut().unwrap().installation = Some(selected.clone());
    }
    pub(crate) fn rebind_configuration(startup: &mut HostStartup) {
        let mut selected = startup
            .research
            .as_ref()
            .unwrap()
            .installation
            .clone()
            .unwrap();
        rebind_records(startup, &mut selected);
    }
    pub(crate) fn malformed_approval_pin(startup: &mut HostStartup, field: &str, value: Value) {
        let mut selected = startup
            .research
            .as_ref()
            .unwrap()
            .installation
            .clone()
            .unwrap();
        rewrite(&selected.root, &mut selected.installation, |v| {
            v["approvals"][0]["git_pin"][field] = value
        });
        rebind_records(startup, &mut selected);
    }
    pub(crate) fn append_notice_intent(
        startup: &mut HostStartup,
        payload: &Value,
        notice_id: &str,
    ) -> Value {
        let mut selected = startup
            .research
            .as_ref()
            .unwrap()
            .installation
            .clone()
            .unwrap();
        let mut notice = payload.clone();
        notice["notice_id"] = json!(notice_id);
        let (role, key, digest) = payload_intent("research.invalidate.v1", notice.clone()).unwrap();
        let _ = role;
        let pin = record(&selected.root, notice_id, &notice);
        rewrite(&selected.root, &mut selected.operations, |v| {
            v["intents"].as_array_mut().unwrap().push(json!({"operation":"research.invalidate.v1","role":"research_notice","key":key,"payload_digest":digest,"payload":pin}))
        });
        rebind_records(startup, &mut selected);
        notice
    }
    pub(crate) fn append_history_intents(
        startup: &mut HostStartup,
        basis: &retained_task_workflow::research_admission::BasisBundle,
    ) {
        let mut selected = startup
            .research
            .as_ref()
            .unwrap()
            .installation
            .clone()
            .unwrap();
        let digest = research_digest("basis", basis).unwrap();
        let preparation = json!({"key":basis.key,"expected_revision":null,"basis_digest":digest});
        let adoption = json!({"submit":basis.submit,"key":basis.key,"expected_preparation_revision":1,"prepared_basis_digest":digest});
        record(
            &selected.root,
            "controlled-history-basis",
            &serde_json::to_value(basis).unwrap(),
        );
        for (name, op, payload) in [
            (
                "controlled-history-prepare",
                "research.prepare.v1",
                preparation,
            ),
            ("controlled-history-adopt", "research.adopt.v1", adoption),
        ] {
            let (role, key, digest) = payload_intent(op, payload.clone()).unwrap();
            let pin = record(&selected.root, name, &payload);
            rewrite(&selected.root, &mut selected.operations, |v| {
                v["intents"].as_array_mut().unwrap().push(json!({"operation":op,"role":role,"key":key,"payload_digest":digest,"payload":pin}))
            });
        }
        rebind_records(startup, &mut selected);
    }
    pub(crate) fn malformed_record(startup: &mut HostStartup, case: &str) {
        let mut selected = startup
            .research
            .as_ref()
            .unwrap()
            .installation
            .clone()
            .unwrap();
        let root = selected.root.clone();
        match case {
            "reference-alias" => {
                rewrite(&root, &mut selected.installation, |v| {
                    // Same member name with conflicting digest is a concrete
                    // invalid reference, without a self-hash fixed point.
                    v["approvals"][1]["source"]["sha256"] = json!("0".repeat(64));
                });
            }
            "reference-order" => {
                rewrite(&root, &mut selected.authority_admission, |v| {
                    // An R_I cannot refer forward to O instead of exact I.
                    v["subject_digest"] = json!(selected.operations.sha256);
                });
            }
            "missing-member" | "member-bound" | "member-name" => {
                rewrite(&root, &mut selected.installation, |v| match case {
                    "missing-member" => {
                        v["approvals"][0]["source"]["name"] = json!("absent-owner-source")
                    }
                    "member-name" => v["approvals"][0]["source"]["name"] = json!("a".repeat(257)),
                    "member-bound" => {
                        let bytes = vec![b'x'; MAX_METADATA + 1];
                        let pin = put(&root, "oversized-source", &bytes);
                        v["approvals"][0]["source"] = serde_json::to_value(pin).unwrap();
                    }
                    _ => unreachable!(),
                });
            }
            "selector-family" => rewrite(&root, &mut selected.installation, |v| {
                v["selectors"] = json!(["unapproved.selector"])
            }),
            "protected-root-overlap" => {
                startup.lifecycle.native.as_mut().unwrap().root = root.clone();
            }
            "controlled-mix" => {
                startup.research.as_mut().unwrap().controlled_fixture = Some(root.join("source"));
            }
            "installation-kind" => rewrite(&root, &mut selected.installation, |v| {
                v["kind"] = json!("future")
            }),
            "installation-version" => rewrite(&root, &mut selected.installation, |v| {
                v["version"] = json!(2)
            }),
            "duplicate-role" => rewrite(&root, &mut selected.installation, |v| {
                v["approvals"][1] = v["approvals"][0].clone()
            }),
            "issuer" => rewrite(&root, &mut selected.installation, |v| {
                v["approvals"][0]["issuer"] = json!("other")
            }),
            "delegation" => rewrite(&root, &mut selected.installation, |v| {
                v["delegation"] = json!("other")
            }),
            "purpose" => rewrite(&root, &mut selected.installation, |v| {
                v["purpose"] = json!("other")
            }),
            "roles-empty" => rewrite(&root, &mut selected.installation, |v| {
                v["producer_roles"] = json!([])
            }),
            "artifact-root" => rewrite(&root, &mut selected.installation, |v| {
                v["artifact_root"] = json!(root.parent().unwrap())
            }),
            "approval-json" | "duplicate-json" => {
                let mut installation: Value =
                    parse(&fs::read(root.join(&selected.installation.name)).unwrap()).unwrap();
                let bytes: &[u8] = if case == "approval-json" {
                    b"not-json"
                } else {
                    br#"{"issuer":"a","issuer":"b"}"#
                };
                let pin = put(&root, "qualification", bytes);
                installation["approvals"][0]["event"] = serde_json::to_value(&pin).unwrap();
                installation["approvals"][0]["git_pin"]["sha256"] = json!(pin.sha256);
                selected.installation = record(&root, "installation", &installation);
            }
            "source-digest" => rewrite(&root, &mut selected.installation, |v| {
                v["approvals"][0]["source"]["sha256"] = json!(sha(b"wrong"))
            }),
            "source-path" => rewrite(&root, &mut selected.installation, |v| {
                v["approvals"][0]["source"]["name"] = json!("../source")
            }),
            "authority-kind" => rewrite(&root, &mut selected.authority_admission, |v| {
                v["kind"] = json!("other")
            }),
            "intent-role" => rewrite(&root, &mut selected.operations, |v| {
                v["intents"][0]["role"] = json!("adopter")
            }),
            "intent-key" => rewrite(&root, &mut selected.operations, |v| {
                v["intents"][0]["key"]["task_id"] = json!("other")
            }),
            "intent-payload" => rewrite(&root, &mut selected.operations, |v| {
                v["intents"][0]["payload_digest"] = json!(sha(b"other"))
            }),
            "intent-duplicate" => rewrite(&root, &mut selected.operations, |v| {
                v["intents"] = json!([v["intents"][0].clone(), v["intents"][0].clone()])
            }),
            "intent-bound" => rewrite(&root, &mut selected.operations, |v| {
                v["intents"] = json!(vec![v["intents"][0].clone(); 17])
            }),
            "operation-kind" => rewrite(&root, &mut selected.operations, |v| {
                v["kind"] = json!("other")
            }),
            "operation-admission-kind" => rewrite(&root, &mut selected.operation_admission, |v| {
                v["kind"] = json!("other")
            }),
            "configuration-kind" | "current-C" | "operation-configuration" => {}
            _ => panic!("unknown malformed case {case}"),
        }
        rebind_records(startup, &mut selected);
        match case {
            "reference-order" => rewrite(&root, &mut selected.authority_admission, |v| {
                v["subject_digest"] = json!(selected.operations.sha256);
            }),
            "configuration-kind" | "current-C" => {
                rewrite(&root, &mut selected.configuration, |v| {
                    if case == "configuration-kind" {
                        v["kind"] = json!("final-startup");
                    } else {
                        v["upstream_startup"]["answer_authority_ref"] = json!("different");
                    }
                });
                rewrite(&root, &mut selected.operations, |v| {
                    v["configuration_digest"] = json!(selected.configuration.sha256)
                });
                rewrite(&root, &mut selected.operation_admission, |v| {
                    v["subject_digest"] = json!(selected.operations.sha256)
                });
            }
            "operation-configuration" => {
                rewrite(&root, &mut selected.operations, |v| {
                    v["configuration_digest"] = json!(sha(b"future-C"))
                });
                rewrite(&root, &mut selected.operation_admission, |v| {
                    v["subject_digest"] = json!(selected.operations.sha256)
                });
            }
            _ => {}
        }
        startup.research.as_mut().unwrap().installation = Some(selected);
    }
    fn rewrite(root: &Path, pin: &mut Pin, edit: impl FnOnce(&mut Value)) {
        let mut v: Value =
            serde_json::from_slice(&fs::read(root.join(&pin.name)).unwrap()).unwrap();
        edit(&mut v);
        *pin = record(root, &pin.name, &v);
    }
    #[test]
    fn acyclic_notice_records_check_but_cannot_admit_or_mutate() {
        let dir = tempfile::tempdir().unwrap();
        let (startup, selected, payload) = fixture(dir.path());
        let staged = stage(&startup, &selected).unwrap();
        staged
            .exact_intent("research.invalidate.v1", payload.clone())
            .unwrap();
        assert!(matches!(
            refuse_unadmitted(&startup),
            Err(HostError::Task(
                retained_task_workflow::TaskError::Unavailable
            ))
        ));
        let store = retained_task_workflow::TaskStore::open(
            &dir.path().join("other.sqlite"),
            "owner".into(),
        )
        .unwrap();
        assert!(crate::research_admission::install(&startup, &store, None).is_err());
        // A failed staged installation has not nominated/guarded this task.
        assert!(
            store
                .submit(&serde_json::from_value(crate::tests::admission_submit()).unwrap())
                .is_ok()
        );
        for (field, value) in [
            ("notice_id", json!("notice-2")),
            (
                "key",
                json!({"task_id":"other","subject_id":"subject-0","request_digest":sha(b"request")}),
            ),
        ] {
            let mut changed = payload.clone();
            changed[field] = value;
            assert!(
                staged
                    .exact_intent("research.invalidate.v1", changed)
                    .is_err()
            );
        }
        let mut changed = payload;
        changed["notice"]["reason"] = json!("changed");
        assert!(
            staged
                .exact_intent("research.invalidate.v1", changed)
                .is_err()
        );
        assert!(staged.exact_intent("research.prepare.v1", json!({"key":staged.manifest.intents[0].key,"expected_revision":null,"basis_digest":sha(b"basis")})).is_err());
    }
    #[cfg(feature = "controlled-proof")]
    #[test]
    fn controlled_installed_notice_uses_exact_intent_and_retains_original_association() {
        let dir = tempfile::tempdir().unwrap();
        let (mut startup, selected, payload) = fixture(dir.path());
        startup.research.as_mut().unwrap().installation = None;
        startup.research.as_mut().unwrap().controlled_installation = Some(selected.clone());
        let mut host =
            crate::tests::admission_host(dir.path(), &dir.path().join("unused.sock"), false);
        host.startup = startup;
        host.research = std::sync::OnceLock::from(
            crate::research_admission::install(&host.startup, &host.store, None).unwrap(),
        );
        let response = crate::dispatch(
            &host,
            json!({"op":"research.invalidate.v1","payload":payload}),
        );
        assert_eq!(response["status"], "invalidated", "{response}");
        assert_eq!(response["qualification"], "controlled_only");
        assert_eq!(
            response["receipt"]["admission"]["installation_digest"],
            selected.installation.sha256
        );
        assert_eq!(
            response["receipt"]["admission"]["startup_digest"],
            sha(&serde_json::to_vec(&host.startup).unwrap())
        );
        assert_eq!(
            response["receipt"]["admission"]["producer_role"],
            "research_notice"
        );
        let retry = crate::dispatch(
            &host,
            json!({"op":"research.invalidate.v1","payload":payload}),
        );
        assert_eq!(retry, response);
        let key: ResearchKey = serde_json::from_value(payload["key"].clone()).unwrap();
        let head = host.store.read_research_preparation(&key).unwrap().unwrap();
        assert!(!head.ready);
        assert!(head.basis_digest.is_none());
        assert_eq!(head.revision, 1);
        let mut changed = payload.clone();
        changed["notice"]["reason"] = json!("changed");
        assert!(
            crate::dispatch(
                &host,
                json!({"op":"research.invalidate.v1","payload":changed})
            )
            .get("code")
            .is_some()
        );
        assert_eq!(
            host.store.read_research_preparation(&key).unwrap().unwrap(),
            head
        );
        let prepare = crate::dispatch(
            &host,
            json!({"op":"research.prepare.v1","payload":{"key":key,"expected_revision":1,"basis_digest":sha(b"complete:true")}}),
        );
        assert!(prepare.get("code").is_some());
        drop(host);
        let reopened = retained_task_workflow::TaskStore::open(
            &dir.path().join("tasks.sqlite"),
            "human-operator".into(),
        )
        .unwrap();
        assert_eq!(
            reopened.read_research_preparation(&key).unwrap().unwrap(),
            head
        );
    }
    #[test]
    #[ignore = "requires the separately built default executable"]
    fn actual_default_executable_refuses_ordinary_installed_startup_without_mutation() {
        let binary =
            std::env::var_os("M4CB_DEFAULT_BINARY").expect("explicit default build executable");
        let dir = tempfile::tempdir().unwrap();
        let (startup, _, _) = fixture(dir.path());
        let before = fs::read(&startup.task_store_path).unwrap();
        let path = dir.path().join("startup.json");
        fs::write(&path, serde_json::to_vec(&startup).unwrap()).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        let reply = std::process::Command::new(binary)
            .args(["serve", "--startup"])
            .arg(&path)
            .output()
            .unwrap();
        assert!(!reply.status.success());
        assert!(
            String::from_utf8_lossy(&reply.stderr).contains("unavailable"),
            "{}",
            String::from_utf8_lossy(&reply.stderr)
        );
        assert_eq!(fs::read(&startup.task_store_path).unwrap(), before);
        assert!(!startup.lifecycle.store_root.exists());
        assert!(!startup.socket_path.exists());
    }
    #[test]
    fn installed_restart_proves_bounded_replacement_and_full_current_configuration() {
        let dir = tempfile::tempdir().unwrap();
        let (old, prior, _) = fixture(dir.path());
        let original_files = fs::read_dir(&prior.root)
            .unwrap()
            .map(|e| {
                let p = e.unwrap().path();
                let b = fs::read(&p).unwrap();
                (p, b)
            })
            .collect::<Vec<_>>();
        let mut next = old.clone();
        let native = next.lifecycle.native.as_mut().unwrap();
        native.session_id = "synthetic-successor".into();
        native.invocation_id = "invocation-successor".into();
        native.attempt_id = "attempt-successor".into();
        native.root = dir.path().join("native-successor");
        fs::create_dir(&native.root).unwrap();
        // A refreshed native identity with old C is independently invalid.
        assert!(stage(&next, &prior).is_err());
        let mut replacement = prior.clone();
        let mut configuration: Value =
            parse(&fs::read(prior.root.join(&prior.configuration.name)).unwrap()).unwrap();
        configuration["upstream_startup"] = upstream(&next).unwrap();
        replacement.configuration = record(&prior.root, "configuration-successor", &configuration);
        let mut operations: Value =
            parse(&fs::read(prior.root.join(&prior.operations.name)).unwrap()).unwrap();
        operations["id"] = json!("operations-successor");
        operations["configuration_digest"] = json!(replacement.configuration.sha256);
        replacement.operations = record(&prior.root, "operations-successor", &operations);
        let mut admission: Value =
            parse(&fs::read(prior.root.join(&prior.operation_admission.name)).unwrap()).unwrap();
        admission["subject_digest"] = json!(replacement.operations.sha256);
        replacement.operation_admission =
            record(&prior.root, "operation-admission-successor", &admission);
        next.research.as_mut().unwrap().installation = Some(replacement.clone());
        let mut normalized = next.clone();
        normalized.lifecycle.native = old.lifecycle.native.clone();
        normalize_installed_restart(&old, &next, &mut normalized).unwrap();
        assert_eq!(
            serde_json::to_value(&old).unwrap(),
            serde_json::to_value(normalized).unwrap()
        );
        let mut stale = next.clone();
        stale
            .research
            .as_mut()
            .unwrap()
            .installation
            .as_mut()
            .unwrap()
            .configuration = prior.configuration.clone();
        assert!(normalize_installed_restart(&old, &stale, &mut stale.clone()).is_err());
        let mut changed = next.clone();
        changed.lifecycle.native.as_mut().unwrap().prompt = "different prompt".into();
        assert!(
            normalize_installed_restart(&old, &changed, &mut changed.clone()).is_err(),
            "new C must reflect complete current S"
        );
        let mut changed_selection = replacement.clone();
        rewrite(&prior.root, &mut changed_selection.operations, |v| {
            v["intents"][0]["role"] = json!("decision_notice")
        });
        changed.research.as_mut().unwrap().installation = Some(changed_selection);
        assert!(normalize_installed_restart(&old, &changed, &mut changed.clone()).is_err());
        // Old C/O/admission/source history was never rewritten by replacement.
        for (path, bytes) in original_files {
            assert_eq!(fs::read(path).unwrap(), bytes);
        }
    }

    #[test]
    fn all_upstream_settings_and_record_kinds_compare() {
        let dir = tempfile::tempdir().unwrap();
        let (startup, selected, _) = fixture(dir.path());
        for path in [
            "provider_identity",
            "approved_snapshot_ref",
            "answer_authority_ref",
            "source_sha256",
        ] {
            let mut v = serde_json::to_value(&startup).unwrap();
            v[path] = json!("changed");
            let changed: HostStartup = serde_json::from_value(v).unwrap();
            assert!(stage(&changed, &selected).is_err(), "{path}");
        }
        let mut changed = startup.clone();
        changed.lifecycle.max_executions += 1;
        assert!(stage(&changed, &selected).is_err());
        let mut bad = selected.clone();
        rewrite(&bad.root, &mut bad.configuration, |v| {
            v["kind"] = json!("final-startup-v1")
        });
        assert!(stage(&startup, &bad).is_err());
    }
    #[test]
    fn exact_operation_role_digest_scope_and_capacity_refuse() {
        for variant in 0..7 {
            let dir = tempfile::tempdir().unwrap();
            let (startup, mut selected, _) = fixture(dir.path());
            rewrite(
                &selected.root,
                &mut selected.operations,
                |v| match variant {
                    0 => v["intents"][0]["role"] = json!("adopter"),
                    1 => v["intents"][0]["payload_digest"] = json!(sha(b"changed")),
                    2 => v["intents"][0]["operation"] = json!("research.install.v1"),
                    3 => v["intents"][0]["key"]["task_id"] = json!("other"),
                    4 => {
                        let i = v["intents"][0].clone();
                        v["intents"] = json!(vec![i; 17]);
                    }
                    5 => v["installation_digest"] = json!(sha(b"future")),
                    _ => {
                        let i = v["intents"][0].clone();
                        v["intents"] = json!([i.clone(), i]);
                    }
                },
            );
            assert!(stage(&startup, &selected).is_err(), "variant {variant}");
        }
    }
    #[test]
    fn missing_forged_and_changed_source_bytes_refuse() {
        for name in ["source", "caller", "qualification", "applicability"] {
            let dir = tempfile::tempdir().unwrap();
            let (startup, selected, _) = fixture(dir.path());
            fs::write(selected.root.join(name), b"forged owner/issuer/caller").unwrap();
            assert!(stage(&startup, &selected).is_err(), "{name}");
        }
        let dir = tempfile::tempdir().unwrap();
        let (startup, selected, _) = fixture(dir.path());
        fs::remove_file(selected.root.join("source")).unwrap();
        assert!(stage(&startup, &selected).is_err());
    }
    #[test]
    fn strict_bounded_json_and_pins_refuse() {
        assert!(parse::<Value>(br#"{"a":1,"a":1}"#).is_err());
        assert!(parse::<Value>(&vec![b' '; MAX_METADATA + 1]).is_err());
        assert!(parse::<InstallationSelection>(br#"{"version":2,"extra":true}"#).is_err());
        let dir = tempfile::tempdir().unwrap();
        let (_, selected, _) = fixture(dir.path());
        let mut c = Custody::open(&selected.root).unwrap();
        for name in ["../source", "/etc/passwd", ".", "..", "bad/name"] {
            assert!(
                c.read(&Pin {
                    name: name.into(),
                    sha256: sha(b"x")
                })
                .is_err()
            );
        }
        let huge = put(&selected.root, "oversize", &vec![b'x'; MAX_METADATA + 1]);
        assert!(c.read(&huge).is_err());
    }
    #[test]
    fn descriptor_custody_rejects_links_aliases_and_retains_open_root() {
        let dir = tempfile::tempdir().unwrap();
        let (_, selected, _) = fixture(dir.path());
        let mut c = Custody::open(&selected.root).unwrap();
        let source = put(&selected.root, "original", b"original");
        fs::hard_link(selected.root.join("original"), selected.root.join("alias")).unwrap();
        assert!(c.read(&source).is_err());
        fs::remove_file(selected.root.join("alias")).unwrap();
        symlink(selected.root.join("original"), selected.root.join("link")).unwrap();
        assert!(
            c.read(&Pin {
                name: "link".into(),
                sha256: source.sha256.clone()
            })
            .is_err()
        );
        let moved = dir.path().join("moved");
        fs::rename(&selected.root, &moved).unwrap();
        fs::create_dir(&selected.root).unwrap();
        put(&selected.root, "original", b"replacement");
        assert_eq!(c.read(&source).unwrap(), b"original");
        let linkroot = dir.path().join("linkroot");
        symlink(&moved, &linkroot).unwrap();
        assert!(Custody::open(&linkroot).is_err());
    }
    #[test]
    fn native_overlap_snapshot_disclosure_and_controlled_mixing_refuse() {
        let dir = tempfile::tempdir().unwrap();
        let (mut startup, selected, _) = fixture(dir.path());
        startup.research.as_mut().unwrap().controlled_fixture = Some(dir.path().join("fixture"));
        assert!(stage(&startup, &selected).is_err());
        assert!(crate::research_admission::validate_nomination(&startup).is_err());
        startup.research.as_mut().unwrap().controlled_fixture = None;
        startup.lifecycle.native.as_mut().unwrap().root = dir.path().to_path_buf();
        assert!(stage(&startup, &selected).is_err());
        let dir = tempfile::tempdir().unwrap();
        let (mut startup, mut selected, _) = fixture(dir.path());
        startup.lifecycle.native.as_mut().unwrap().snapshot.members[0].bytes =
            fs::read(selected.root.join("source")).unwrap();
        rewrite(&selected.root, &mut selected.configuration, |v| {
            v["upstream_startup"] = upstream(&startup).unwrap()
        });
        // Rebind only downstream operations; source disclosure remains invalid.
        rewrite(&selected.root, &mut selected.operations, |v| {
            v["configuration_digest"] = json!(selected.configuration.sha256)
        });
        rewrite(&selected.root, &mut selected.operation_admission, |v| {
            v["subject_digest"] = json!(selected.operations.sha256)
        });
        assert!(stage(&startup, &selected).is_err());
    }
}
