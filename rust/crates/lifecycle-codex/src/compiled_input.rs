//! Checked foreign transport observations. Construction is confined to an owned harness.
use crate::NativeEvent;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{File, OpenOptions},
    io::Read,
    os::unix::{
        fs::{MetadataExt, OpenOptionsExt},
        io::{AsRawFd, FromRawFd},
    },
    path::{Path, PathBuf},
};

pub const METADATA_LIMIT: usize = 128 * 1024;
pub const PAYLOAD_LIMIT: usize = 8 * 1024 * 1024;
const RECORD_LIMIT: usize = 128;
const SEND_LIMIT: usize = 32;

#[derive(Debug, thiserror::Error)]
#[error("native input unavailable, malformed, mismatched or outside bounded custody")]
pub struct NativeInputError;
type Result<T> = std::result::Result<T, NativeInputError>;
fn bad<T>() -> Result<T> {
    Err(NativeInputError)
}
pub(crate) fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[derive(Debug)]
#[cfg_attr(feature = "native-input-proof", derive(Clone))]
pub struct CheckedNativePayload {
    pub inference_call_id: String,
    pub physical_send: u64,
    pub representation: String,
    pub original_sha256: String,
    pub original_codec: String,
    pub bytes: Vec<u8>,
}
/// No Deserialize or caller constructor: this is an acquisition, not a receipt.
#[derive(Debug)]
#[cfg_attr(feature = "native-input-proof", derive(Clone))]
pub struct AcquiredNativeInput {
    pub(crate) transport_session: String,
    pub(crate) metadata: Vec<u8>,
    pub(crate) terminal: NativeEvent,
    pub(crate) payloads: Vec<CheckedNativePayload>,
    pub(crate) eligible: bool,
    pub(crate) outcome: &'static str,
    pub(crate) causal: Option<crate::causal::AcquiredNativeCausal>,
}
impl AcquiredNativeInput {
    pub fn causal(&self) -> Option<&crate::causal::AcquiredNativeCausal> {
        self.causal.as_ref()
    }
    pub fn transport_session(&self) -> &str {
        &self.transport_session
    }
    pub fn metadata(&self) -> &[u8] {
        &self.metadata
    }
    pub fn terminal(&self) -> &NativeEvent {
        &self.terminal
    }
    pub fn outcome(&self) -> &str {
        self.outcome
    }
    pub fn eligible(&self) -> bool {
        self.eligible
    }
    pub fn payloads(&self) -> &[CheckedNativePayload] {
        &self.payloads
    }
}

pub(crate) struct CaptureOwner {
    root: File,
    path: PathBuf,
    identity: (u64, u64),
}
fn identity(file: &File) -> Result<(u64, u64)> {
    let m = file.metadata().map_err(|_| NativeInputError)?;
    Ok((m.dev(), m.ino()))
}
pub(crate) fn open_child(parent: &File, name: &str, directory: bool) -> Result<File> {
    if name.is_empty() || name == "." || name == ".." || name.contains('/') || name.contains('\0') {
        return bad();
    }
    let name = std::ffi::CString::new(name).map_err(|_| NativeInputError)?;
    let flags = libc::O_RDONLY
        | libc::O_CLOEXEC
        | libc::O_NOFOLLOW
        | if directory {
            libc::O_DIRECTORY
        } else {
            libc::O_NONBLOCK
        };
    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 {
        return bad();
    }
    let file = unsafe { File::from_raw_fd(fd) };
    let m = file.metadata().map_err(|_| NativeInputError)?;
    if (directory && !m.is_dir()) || (!directory && (!m.is_file() || m.nlink() != 1)) {
        return bad();
    }
    Ok(file)
}
fn read_bounded(mut file: File, limit: usize) -> Result<Vec<u8>> {
    let before = file.metadata().map_err(|_| NativeInputError)?;
    if before.len() > limit as u64 {
        return bad();
    }
    let mut bytes = Vec::new();
    (&mut file)
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| NativeInputError)?;
    let after = file.metadata().map_err(|_| NativeInputError)?;
    if bytes.len() > limit
        || before.len() != after.len()
        || before.mtime() != after.mtime()
        || before.mtime_nsec() != after.mtime_nsec()
        || before.ctime() != after.ctime()
        || before.ctime_nsec() != after.ctime_nsec()
    {
        return bad();
    }
    Ok(bytes)
}
pub(crate) fn read_child_bounded(parent: &File, name: &str, limit: usize) -> Result<Vec<u8>> {
    let file = open_child(parent, name, false)?;
    let original = identity(&file)?;
    let bytes = read_bounded(file, limit)?;
    if identity(&open_child(parent, name, false)?)? != original {
        return bad();
    }
    Ok(bytes)
}
impl CaptureOwner {
    pub(crate) fn create(path: PathBuf) -> Result<Self> {
        std::fs::create_dir(&path).map_err(|_| NativeInputError)?;
        let root = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_DIRECTORY)
            .open(&path)
            .map_err(|_| NativeInputError)?;
        let identity = identity(&root)?;
        Ok(Self {
            root,
            path,
            identity,
        })
    }
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
    // Producer-owned trace layout is traversed through retained directory handles.
    fn bundles(
        &self,
        directory: &File,
        depth: usize,
        visited: &mut usize,
        found: &mut Vec<File>,
        anchors: &mut Vec<(File, String, (u64, u64))>,
    ) -> Result<()> {
        if depth > 6 {
            return bad();
        }
        for entry in std::fs::read_dir(format!("/proc/self/fd/{}", directory.as_raw_fd()))
            .map_err(|_| NativeInputError)?
        {
            *visited += 1;
            if *visited > 256 {
                return bad();
            }
            let entry = entry.map_err(|_| NativeInputError)?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| NativeInputError)?;
            if name == "strict-transport-input.jsonl" {
                found.push(directory.try_clone().map_err(|_| NativeInputError)?);
                continue;
            }
            if entry
                .file_type()
                .map_err(|_| NativeInputError)?
                .is_symlink()
            {
                return bad();
            }
            if entry.file_type().map_err(|_| NativeInputError)?.is_dir() && name != "payloads" {
                let child = open_child(directory, &name, true)?;
                anchors.push((
                    directory.try_clone().map_err(|_| NativeInputError)?,
                    name.clone(),
                    identity(&child)?,
                ));
                self.bundles(&child, depth + 1, visited, found, anchors)?;
            }
        }
        Ok(())
    }
    pub(crate) fn acquire(
        &self,
        session: &str,
        thread: &str,
        turn: &str,
        terminal: &NativeEvent,
    ) -> Result<AcquiredNativeInput> {
        // A diagnostic preserves owner/terminal attribution without granting eligibility.
        // Never attach bytes from an unsafe/replaced root to that diagnostic.
        if terminal.transport_session != session {
            return bad();
        }
        match self.acquire_checked(session, thread, turn, terminal) {
            Ok(observation) => Ok(observation),
            Err(_) => {
                let mut metadata = Vec::new();
                if self.check_root().is_ok() {
                    let mut dirs = Vec::new();
                    if self
                        .bundles(&self.root, 0, &mut 0, &mut dirs, &mut Vec::new())
                        .is_ok()
                    {
                        for dir in dirs {
                            if let Ok(bytes) = read_child_bounded(
                                &dir,
                                "strict-transport-input.jsonl",
                                METADATA_LIMIT - metadata.len(),
                            ) {
                                metadata.extend(bytes);
                            }
                        }
                    }
                    if self.check_root().is_err() {
                        metadata.clear();
                    }
                }
                Ok(AcquiredNativeInput {
                    transport_session: session.into(),
                    metadata,
                    terminal: terminal.clone(),
                    payloads: Vec::new(),
                    eligible: false,
                    outcome: "capture_unavailable_or_malformed",
                    causal: None,
                })
            }
        }
    }
    fn check_root(&self) -> Result<()> {
        if identity(&self.root)? != self.identity
            || identity(
                &OpenOptions::new()
                    .read(true)
                    .custom_flags(libc::O_NOFOLLOW | libc::O_DIRECTORY)
                    .open(&self.path)
                    .map_err(|_| NativeInputError)?,
            )? != self.identity
        {
            return bad();
        }
        Ok(())
    }
    fn acquire_checked(
        &self,
        session: &str,
        thread: &str,
        turn: &str,
        terminal: &NativeEvent,
    ) -> Result<AcquiredNativeInput> {
        if identity(&self.root)? != self.identity
            || identity(
                &OpenOptions::new()
                    .read(true)
                    .custom_flags(libc::O_NOFOLLOW | libc::O_DIRECTORY)
                    .open(&self.path)
                    .map_err(|_| NativeInputError)?,
            )? != self.identity
            || terminal.transport_session != session
        {
            return bad();
        }
        let t = lifecycle_wire::parse_strict_json(&terminal.raw).map_err(|_| NativeInputError)?;
        let completed = t["method"] == "turn/completed";
        let diagnostic = matches!(
            t["method"].as_str(),
            Some("error" | "turn/failed" | "turn/interrupted")
        );
        let observed_turn = if completed {
            &t["params"]["turn"]["id"]
        } else {
            &t["params"]["turnId"]
        };
        if (!completed && !diagnostic) || t["params"]["threadId"] != thread || observed_turn != turn
        {
            return bad();
        }
        let eligible_terminal = completed
            && t["params"]["turn"]["status"] == "completed"
            && t["params"]["turn"].get("error") == Some(&Value::Null);
        let mut bundles = Vec::new();
        let mut anchors = Vec::new();
        self.bundles(&self.root, 0, &mut 0, &mut bundles, &mut anchors)?;
        let mut metadata_total = 0;
        let mut selected = None;
        for dir in bundles {
            let bytes = read_child_bounded(
                &dir,
                "strict-transport-input.jsonl",
                METADATA_LIMIT - metadata_total,
            )?;
            metadata_total += bytes.len();
            // Validate producer framing before selecting any turn-specific spans.
            if !bytes.ends_with(b"\n") {
                return bad();
            }
            // A trace root can contain prior turns. Select only an exact envelope;
            // no valid row in a different bundle is transplanted into this one.
            let mut matching = Vec::new();
            let mut selected_rows = false;
            // Retain the exact selected spans, including their original delimiters.
            // Empty records remain visible to framing validation, never normalized away.
            for span in bytes.split_inclusive(|b| *b == b'\n') {
                let line = span.strip_suffix(b"\n").unwrap_or(span);
                if line.len() > 32 * 1024 {
                    return bad();
                }
                if line.is_empty() {
                    matching.extend_from_slice(span);
                    continue;
                }
                let row = lifecycle_wire::parse_strict_json(line).map_err(|_| NativeInputError)?;
                if row["thread_id"] == thread && row["codex_turn_id"] == turn {
                    selected_rows = true;
                    matching.extend_from_slice(span);
                }
            }
            if selected_rows {
                if selected.is_some() {
                    return bad();
                }
                selected = Some((dir, matching));
            }
        }
        let (dir, metadata) = selected.ok_or(NativeInputError)?;
        let decoded = decode_checked(&dir, &metadata, thread, turn);
        let eligible = eligible_terminal && decoded.as_ref().is_ok_and(|(_, complete)| *complete);
        let outcome = if !eligible_terminal {
            "native_terminal_ineligible"
        } else if !eligible {
            "capture_malformed_or_incomplete"
        } else {
            "checked_completed"
        };
        let payloads = decoded.map(|(payloads, _)| payloads).unwrap_or_default();
        let causal = if eligible {
            crate::causal::acquire(
                &dir,
                &metadata,
                payloads.iter().map(|p| p.bytes.len()).sum(),
                thread,
                turn,
            )
            .ok()
        } else {
            None
        };
        self.check_root()?;
        for (parent, name, original) in anchors {
            if identity(&open_child(&parent, &name, true)?)? != original {
                return bad();
            }
        }
        Ok(AcquiredNativeInput {
            transport_session: session.into(),
            metadata,
            terminal: terminal.clone(),
            payloads,
            eligible,
            outcome,
            causal,
        })
    }
}
pub(crate) fn fields(v: &Value, expected: &[&str]) -> Result<()> {
    let o = v.as_object().ok_or(NativeInputError)?;
    if o.len() != expected.len() || expected.iter().any(|k| !o.contains_key(*k)) {
        return bad();
    }
    Ok(())
}
#[cfg(test)]
fn decode(
    dir: &File,
    metadata: &[u8],
    thread: &str,
    turn: &str,
) -> Result<Vec<CheckedNativePayload>> {
    let (payloads, complete) = decode_checked(dir, metadata, thread, turn)?;
    if !complete {
        return bad();
    }
    Ok(payloads)
}
fn decode_checked(
    dir: &File,
    metadata: &[u8],
    thread: &str,
    turn: &str,
) -> Result<(Vec<CheckedNativePayload>, bool)> {
    if metadata.is_empty() || !metadata.ends_with(b"\n") {
        return bad();
    }
    let mut groups: BTreeMap<(String, u64), Vec<Value>> = BTreeMap::new();
    let mut payloads = Vec::new();
    let mut total = 0usize;
    let mut paths = BTreeSet::new();
    let mut inodes = BTreeSet::new();
    let mut previous = BTreeMap::new();
    let mut sends = Vec::new();
    for (index, line) in metadata[..metadata.len() - 1]
        .split(|b| *b == b'\n')
        .enumerate()
    {
        if index >= RECORD_LIMIT || line.len() > 32 * 1024 {
            return bad();
        }
        let row = lifecycle_wire::parse_strict_json(line).map_err(|_| NativeInputError)?;
        if row["kind"] != "strict_transport_input"
            || row["version"] != 1
            || row["thread_id"] != thread
            || row["codex_turn_id"] != turn
        {
            return bad();
        }
        let call = row["inference_call_id"]
            .as_str()
            .filter(|s| !s.is_empty() && s.len() <= 256)
            .ok_or(NativeInputError)?
            .to_owned();
        let send = row["physical_send"]
            .as_u64()
            .filter(|n| *n > 0)
            .ok_or(NativeInputError)?;
        let stage = row["stage"].as_str().ok_or(NativeInputError)?;
        let mut expected = vec![
            "kind",
            "version",
            "thread_id",
            "codex_turn_id",
            "inference_call_id",
            "physical_send",
            "stage",
            "payloads",
            "payload_limit",
        ];
        let transport = matches!(
            stage,
            "before_send"
                | "http_status:200"
                | "http_status:503"
                | "http_status:403"
                | "send_failed_unknown"
                | "sent"
        );
        if transport {
            expected.extend(["transport", "encoding", "self_contained"]);
            if !matches!(row["transport"].as_str(), Some("http" | "websocket"))
                || !matches!(row["encoding"].as_str(), Some("identity" | "zstd"))
                || row["self_contained"] != true
            {
                return bad();
            }
        } else {
            if !matches!(
                stage,
                "provider_created" | "provider_completed" | "provider_failed_or_unknown"
            ) || (stage != "provider_failed_or_unknown"
                && row["response_id"]
                    .as_str()
                    .is_none_or(|s| s.is_empty() || s.len() > 256))
            {
                return bad();
            }
            if stage == "provider_failed_or_unknown"
                && !row["response_id"].is_null()
                && row["response_id"]
                    .as_str()
                    .is_none_or(|s| s.is_empty() || s.len() > 256)
            {
                return bad();
            }
            expected.push("response_id");
        }
        fields(&row, &expected)?;
        let limit = row["payload_limit"]
            .as_u64()
            .filter(|n| (1..=PAYLOAD_LIMIT as u64).contains(n))
            .ok_or(NativeInputError)? as usize;
        let refs = row["payloads"].as_array().ok_or(NativeInputError)?;
        if stage == "before_send" {
            if sends.len() >= SEND_LIMIT || refs.is_empty() || refs.len() > 2 {
                return bad();
            }
            let last = previous.entry(call.clone()).or_insert(0);
            if send != *last + 1 {
                return bad();
            }
            *last = send;
            sends.push((call.clone(), send));
            let mut representations = BTreeMap::new();
            let mut combined = 0usize;
            let payload_dir = open_child(dir, "payloads", true)?;
            for reference in refs {
                fields(reference, &["representation", "path", "length", "sha256"])?;
                let role = reference["representation"]
                    .as_str()
                    .filter(|s| matches!(*s, "wire" | "decoded"))
                    .ok_or(NativeInputError)?;
                let path = reference["path"].as_str().ok_or(NativeInputError)?;
                let name = path.strip_prefix("payloads/").ok_or(NativeInputError)?;
                if !paths.insert(path.to_owned()) {
                    return bad();
                }
                let file = open_child(&payload_dir, name, false)?;
                if !inodes.insert(identity(&file)?) {
                    return bad();
                }
                let original = identity(&file)?;
                let bytes = read_bounded(file, limit.min(PAYLOAD_LIMIT - total))?;
                if identity(&open_child(&payload_dir, name, false)?)? != original {
                    return bad();
                }
                total = total.checked_add(bytes.len()).ok_or(NativeInputError)?;
                combined += bytes.len();
                if total > PAYLOAD_LIMIT
                    || combined > limit
                    || reference["length"] != bytes.len()
                    || reference["sha256"] != sha(&bytes)
                    || representations
                        .insert(role.to_owned(), bytes.clone())
                        .is_some()
                {
                    return bad();
                }
                payloads.push(CheckedNativePayload {
                    inference_call_id: call.clone(),
                    physical_send: send,
                    representation: role.into(),
                    original_sha256: sha(&bytes),
                    original_codec: if role == "wire" {
                        row["encoding"].as_str().unwrap().into()
                    } else {
                        "identity".into()
                    },
                    bytes,
                });
            }
            if identity(&open_child(dir, "payloads", true)?)? != identity(&payload_dir)? {
                return bad();
            }
            let wire = representations.get("wire").ok_or(NativeInputError)?;
            let decoded = if row["encoding"] == "zstd" {
                let mut decoded = Vec::new();
                let mut decoder = zstd::stream::read::Decoder::new(wire.as_slice())
                    .map_err(|_| NativeInputError)?;
                // Reject oversized frame windows before the decoder allocates them.
                decoder.window_log_max(23).map_err(|_| NativeInputError)?;
                decoder
                    .take(PAYLOAD_LIMIT as u64 + 1)
                    .read_to_end(&mut decoded)
                    .map_err(|_| NativeInputError)?;
                if decoded.len() > PAYLOAD_LIMIT || representations.get("decoded") != Some(&decoded)
                {
                    return bad();
                }
                decoded
            } else {
                if representations.len() != 1 {
                    return bad();
                }
                wire.clone()
            };
            let request =
                lifecycle_wire::parse_strict_json(&decoded).map_err(|_| NativeInputError)?;
            if !request.is_object()
                || request.get("input").and_then(Value::as_array).is_none()
                || request
                    .get("previous_response_id")
                    .is_some_and(|v| !v.is_null())
            {
                return bad();
            }
        } else if !refs.is_empty() {
            return bad();
        }
        groups.entry((call, send)).or_default().push(row);
    }
    if sends.is_empty() || groups.len() != sends.len() {
        return bad();
    }
    let mut accepted = false;
    let mut incomplete = false;
    for key in sends {
        let g = groups.get(&key).ok_or(NativeInputError)?;
        let stages = g
            .iter()
            .map(|r| r["stage"].as_str().unwrap())
            .collect::<Vec<_>>();
        let http = g[0]["transport"] == "http";
        if http
            && (stages
                == [
                    "before_send",
                    "send_failed_unknown",
                    "provider_failed_or_unknown",
                ]
                || stages
                    == [
                        "before_send",
                        "http_status:403",
                        "provider_failed_or_unknown",
                    ])
        {
            incomplete = true;
            continue;
        }
        if stages == ["before_send", "http_status:503"] && http {
            continue;
        }
        let expected = [
            "before_send",
            if http { "http_status:200" } else { "sent" },
            "provider_created",
            "provider_completed",
        ];
        // A supported provider stream failure preserves the independently checked
        // request representations, but can never qualify the observation.
        if stages.last() == Some(&"provider_failed_or_unknown") {
            let prefix = &stages[..stages.len() - 1];
            if (2..=3).contains(&prefix.len()) && prefix == &expected[..prefix.len()] {
                if prefix.len() == 3
                    && !g.last().unwrap()["response_id"].is_null()
                    && g[2]["response_id"] != g.last().unwrap()["response_id"]
                {
                    return bad();
                }
                incomplete = true;
                continue;
            }
            return bad();
        }
        if stages.len() > expected.len() || stages != expected[..stages.len()] {
            return bad();
        }
        if stages.len() < expected.len() {
            incomplete = true;
            continue;
        }
        if g[2]["response_id"] != g[3]["response_id"] {
            return bad();
        }
        if g[0]["transport"] != g[1]["transport"] || g[0]["payload_limit"] != g[1]["payload_limit"]
        {
            return bad();
        }
        accepted = true;
    }
    Ok((payloads, accepted && !incomplete))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn fixture() -> (tempfile::TempDir, CaptureOwner, Vec<Value>, NativeEvent) {
        let temp = tempfile::tempdir().unwrap();
        let owner = CaptureOwner::create(temp.path().join("capture")).unwrap();
        let bundle = owner.path().join("trace");
        std::fs::create_dir(&bundle).unwrap();
        std::fs::create_dir(bundle.join("payloads")).unwrap();
        let decoded = br#"{"input":[{"text":"synthetic complete input"}]}"#;
        let wire = zstd::stream::encode_all(decoded.as_slice(), 0).unwrap();
        std::fs::write(bundle.join("payloads/wire.bin"), &wire).unwrap();
        std::fs::write(bundle.join("payloads/decoded.bin"), decoded).unwrap();
        let mut rows = vec![
            json!({"kind":"strict_transport_input","version":1,"thread_id":"thread","codex_turn_id":"turn","inference_call_id":"inference","physical_send":1,"stage":"before_send","payload_limit":8388608,"transport":"http","encoding":"zstd","self_contained":true,"payloads":[{"representation":"wire","path":"payloads/wire.bin","length":wire.len(),"sha256":sha(&wire)},{"representation":"decoded","path":"payloads/decoded.bin","length":decoded.len(),"sha256":sha(decoded)}]}),
        ];
        let mut response = rows[0].clone();
        response["stage"] = json!("http_status:200");
        response["encoding"] = json!("identity");
        response["payloads"] = json!([]);
        rows.push(response);
        for stage in ["provider_created", "provider_completed"] {
            rows.push(json!({"kind":"strict_transport_input","version":1,"thread_id":"thread","codex_turn_id":"turn","inference_call_id":"inference","physical_send":1,"stage":stage,"payload_limit":8388608,"response_id":"response","payloads":[]}));
        }
        let terminal=crate::parse_event("transport".into(),1,serde_json::to_vec(&json!({"method":"turn/completed","params":{"threadId":"thread","turn":{"id":"turn","status":"completed","error":null}}})).unwrap()).unwrap();
        write(&owner, &rows);
        (temp, owner, rows, terminal)
    }
    fn write(owner: &CaptureOwner, rows: &[Value]) {
        std::fs::write(
            owner.path().join("trace/strict-transport-input.jsonl"),
            rows.iter().map(|r| format!("{r}\n")).collect::<String>(),
        )
        .unwrap();
    }
    fn assert_diagnostic_restart(observation: &AcquiredNativeInput) {
        use lifecycle_core::*;
        use lifecycle_store::{NativeInputPayload, NativeTurnRequest, SqliteLifecycleStore};
        let root = tempfile::tempdir().unwrap();
        let clock = ClockSample {
            wall: WallTimeMs::new(1),
            wait_budget: WaitBudgetMs::new(1),
        };
        let mut store = SqliteLifecycleStore::open(root.path(), "issuer".into()).unwrap();
        let subject = SubjectId::parse("subject").unwrap();
        let context = ContextGeneration::parse("context").unwrap();
        let build = BuildId::parse("build").unwrap();
        let proof = ProofRunId::parse("proof").unwrap();
        store
            .register_subject(
                subject.clone(),
                context.clone(),
                build.clone(),
                proof.clone(),
                clock,
            )
            .unwrap();
        let grant = GrantId::parse("grant").unwrap();
        store
            .install_trusted_grant(TrustedGrant {
                id: grant.clone(),
                issuer: "issuer".into(),
                principal: "principal".into(),
                subject: subject.clone(),
                context: context.clone(),
                build,
                proof_run: proof,
                scope: GrantScope::parse("native_turn").unwrap(),
                expires: AuthorityExpiresAt::new(WallTimeMs::new(100)),
                revision: Revision::new(1),
                revoked: false,
            })
            .unwrap();
        let digest = "a".repeat(64);
        let mut session = store
            .reserve_native_session(
                NativeSessionId::parse("session").unwrap(),
                subject,
                context,
                RuntimeIncarnation::parse("incarnation").unwrap(),
                &digest,
                &digest,
                &digest,
                SnapshotId::parse("snapshot").unwrap(),
                &digest,
                false,
                clock,
            )
            .unwrap();
        store
            .bind_native_thread(&mut session, ProviderThreadId::parse("thread").unwrap())
            .unwrap();
        // This pure acquisition/store probe needs no domain delivery or native send.
        let mut turn = store
            .claim_native_turn(
                &session,
                NativeTurnRequest {
                    invocation_id: "invocation".into(),
                    attempt: AttemptId::parse("attempt").unwrap(),
                    grant,
                    grant_revision: 1,
                    purpose: NativeInvocationPurpose::SemanticCompile,
                    prompt: b"diagnostic store probe".to_vec(),
                },
                "principal",
                clock,
            )
            .unwrap();
        store.mark_native_send_intent(&turn).unwrap();
        store
            .bind_native_turn(&mut turn, ProviderTurnId::parse("turn").unwrap())
            .unwrap();
        let terminal = observation.terminal();
        store
            .record_native_ingress(
                session.id(),
                observation.transport_session(),
                terminal.sequence,
                &terminal.raw,
                "qualified",
                Some("turn/completed"),
                Some("thread"),
                Some("turn"),
                None,
                clock,
            )
            .unwrap();
        let payloads = observation
            .payloads()
            .iter()
            .map(|p| NativeInputPayload {
                inference_call_id: p.inference_call_id.clone(),
                physical_send: p.physical_send,
                representation: p.representation.clone(),
                original_sha256: p.original_sha256.clone(),
                original_codec: p.original_codec.clone(),
                bytes: p.bytes.clone(),
            })
            .collect();
        store
            .retain_native_input(
                &session,
                &turn,
                observation.transport_session(),
                terminal.sequence,
                &terminal.raw,
                observation.metadata(),
                payloads,
                observation.eligible(),
                observation.outcome(),
                clock,
            )
            .unwrap();
        let original = store
            .native_input_observation("invocation")
            .unwrap()
            .unwrap();
        assert!(!original.eligible);
        assert_eq!(original.metadata, observation.metadata());
        assert_eq!(original.terminal_raw, terminal.raw);
        assert_eq!(original.payloads.len(), observation.payloads().len());
        for (a, b) in original.payloads.iter().zip(observation.payloads()) {
            assert_eq!(a.bytes, b.bytes);
            assert_eq!(a.original_sha256, b.original_sha256);
        }
        drop(store);
        let store = SqliteLifecycleStore::open(root.path(), "issuer".into()).unwrap();
        assert_eq!(
            store.native_input_observation("invocation").unwrap(),
            Some(original)
        );
    }
    #[test]
    fn provider_stream_failures_keep_checked_payloads_without_eligibility() {
        for created in [false, true] {
            let (_temp, owner, mut rows, terminal) = fixture();
            let expected = owner
                .acquire("transport", "thread", "turn", &terminal)
                .unwrap();
            rows.truncate(if created { 3 } else { 2 });
            rows.push(json!({"kind":"strict_transport_input","version":1,"thread_id":"thread","codex_turn_id":"turn","inference_call_id":"inference","physical_send":1,"stage":"provider_failed_or_unknown","payload_limit":8388608,"response_id":if created {json!("response")} else {Value::Null},"payloads":[]}));
            write(&owner, &rows);
            let actual = owner
                .acquire("transport", "thread", "turn", &terminal)
                .unwrap();
            assert!(!actual.eligible());
            assert_eq!(actual.payloads().len(), 2);
            for (a, b) in actual.payloads().iter().zip(expected.payloads()) {
                assert_eq!(a.bytes, b.bytes);
                assert_eq!(a.original_sha256, b.original_sha256);
            }
            assert_eq!(
                actual.metadata(),
                std::fs::read(owner.path().join("trace/strict-transport-input.jsonl")).unwrap()
            );
            assert_diagnostic_restart(&actual);
        }
    }
    #[test]
    fn original_framing_is_retained_and_never_repaired() {
        for (empty, foreign_final) in [(false, false), (true, false), (false, true)] {
            let (_temp, owner, _rows, terminal) = fixture();
            let path = owner.path().join("trace/strict-transport-input.jsonl");
            let mut bytes = std::fs::read(&path).unwrap();
            if foreign_final {
                let mut row = _rows.last().unwrap().clone();
                row["thread_id"] = json!("foreign-thread");
                row["codex_turn_id"] = json!("foreign-turn");
                bytes.extend_from_slice(&serde_json::to_vec(&row).unwrap());
            } else if empty {
                let position = bytes.iter().position(|b| *b == b'\n').unwrap();
                bytes.insert(position, b'\n');
            } else {
                assert_eq!(bytes.pop(), Some(b'\n'));
            }
            std::fs::write(path, &bytes).unwrap();
            let actual = owner
                .acquire("transport", "thread", "turn", &terminal)
                .unwrap();
            assert!(!actual.eligible());
            assert_eq!(actual.metadata(), bytes);
            assert!(actual.payloads().is_empty());
            assert_eq!(actual.terminal().raw, terminal.raw);
            assert_eq!(actual.terminal().sequence, terminal.sequence);
            assert_diagnostic_restart(&actual);
        }
    }
    #[test]
    fn complete_compression_requires_owned_terminal_and_location() {
        let (_temp, owner, rows, terminal) = fixture();
        let observation = owner
            .acquire("transport", "thread", "turn", &terminal)
            .unwrap();
        assert!(observation.eligible());
        assert_eq!(observation.payloads().len(), 2);
        for (session, thread, turn) in [
            ("other", "thread", "turn"),
            ("transport", "other", "turn"),
            ("transport", "thread", "other"),
        ] {
            assert!(
                owner
                    .acquire(session, thread, turn, &terminal)
                    .map_or(true, |o| !o.eligible())
            );
        }
        let mut failed = terminal.clone();
        failed.value["params"]["turn"]["status"] = json!("failed");
        failed.raw = serde_json::to_vec(&failed.value).unwrap();
        let observation = owner
            .acquire("transport", "thread", "turn", &failed)
            .unwrap();
        assert!(!observation.eligible());
        assert_eq!(observation.payloads().len(), 2);
        let mut absent = terminal.clone();
        absent.value["params"]["turn"]
            .as_object_mut()
            .unwrap()
            .remove("error");
        absent.raw = serde_json::to_vec(&absent.value).unwrap();
        assert!(
            !owner
                .acquire("transport", "thread", "turn", &absent)
                .unwrap()
                .eligible()
        );
        std::fs::rename(owner.path(), owner.path().with_extension("old")).unwrap();
        std::fs::create_dir(owner.path()).unwrap();
        write_copy(owner.path(), &rows);
        assert!(
            !owner
                .acquire("transport", "thread", "turn", &terminal)
                .unwrap()
                .eligible()
        );
    }
    fn write_copy(root: &Path, rows: &[Value]) {
        std::fs::create_dir(root.join("trace")).unwrap();
        std::fs::write(
            root.join("trace/strict-transport-input.jsonl"),
            rows.iter().map(|r| format!("{r}\n")).collect::<String>(),
        )
        .unwrap();
    }
    #[test]
    fn strict_ambiguity_paths_and_payload_bounds_refuse() {
        let (_temp, owner, rows, terminal) = fixture();
        let dir = open_child(&owner.root, "trace", true).unwrap();
        let original =
            std::fs::read(owner.path().join("trace/strict-transport-input.jsonl")).unwrap();
        for duplicate in [
            String::from_utf8(original.clone()).unwrap().replacen(
                "\"version\":1",
                "\"version\":1,\"version\":1",
                1,
            ),
            String::from_utf8(original.clone()).unwrap().replacen(
                "\"representation\":\"wire\"",
                "\"representation\":\"wire\",\"representation\":\"wire\"",
                1,
            ),
        ] {
            assert!(decode(&dir, duplicate.as_bytes(), "thread", "turn").is_err());
        }
        for mutation in ["length", "digest", "path", "limit", "codec", "stage"] {
            let mut bad = rows.clone();
            match mutation {
                "length" => bad[0]["payloads"][0]["length"] = json!(1),
                "digest" => bad[0]["payloads"][0]["sha256"] = json!("a".repeat(64)),
                "path" => bad[0]["payloads"][0]["path"] = json!("payloads/../wire.bin"),
                "limit" => bad[0]["payload_limit"] = json!(1),
                "codec" => bad[0]["encoding"] = json!("gzip"),
                _ => bad[3]["stage"] = json!("unknown_send"),
            };
            write(&owner, &bad);
            let raw =
                std::fs::read(owner.path().join("trace/strict-transport-input.jsonl")).unwrap();
            assert!(decode(&dir, &raw, "thread", "turn").is_err(), "{mutation}");
        }
        write(&owner, &rows);
        let wire = owner.path().join("trace/payloads/wire.bin");
        std::fs::remove_file(&wire).unwrap();
        std::os::unix::fs::symlink("decoded.bin", &wire).unwrap();
        assert!(decode(&dir, &original, "thread", "turn").is_err());
        std::fs::remove_file(&wire).unwrap();
        std::fs::hard_link(owner.path().join("trace/payloads/decoded.bin"), &wire).unwrap();
        assert!(decode(&dir, &original, "thread", "turn").is_err());
        std::fs::write(
            owner.path().join("trace/strict-transport-input.jsonl"),
            vec![b'x'; METADATA_LIMIT + 1],
        )
        .unwrap();
        assert!(
            !owner
                .acquire("transport", "thread", "turn", &terminal)
                .unwrap()
                .eligible()
        );
    }
    #[test]
    fn websocket_retry_and_diagnostic_capacity_are_distinct() {
        let (_temp, owner, rows, terminal) = fixture();
        let mut ws = rows.clone();
        ws[0]["transport"] = json!("websocket");
        ws[1]["transport"] = json!("websocket");
        ws[1]["stage"] = json!("sent");
        write(&owner, &ws);
        assert!(
            owner
                .acquire("transport", "thread", "turn", &terminal)
                .unwrap()
                .eligible()
        );
        let mut retry = rows.clone();
        retry[1]["stage"] = json!("http_status:503");
        retry.truncate(2);
        for mut row in rows.clone() {
            row["physical_send"] = json!(2);
            for reference in row["payloads"].as_array_mut().unwrap() {
                let old = reference["path"].as_str().unwrap().to_owned();
                let new = old.replace(".bin", "-retry.bin");
                std::fs::copy(
                    owner.path().join("trace").join(&old),
                    owner.path().join("trace").join(&new),
                )
                .unwrap();
                reference["path"] = json!(new);
            }
            retry.push(row);
        }
        write(&owner, &retry);
        assert!(
            owner
                .acquire("transport", "thread", "turn", &terminal)
                .unwrap()
                .eligible()
        );
        std::fs::write(
            owner.path().join("trace/strict-transport-input.jsonl"),
            b"{duplicate malformed\n",
        )
        .unwrap();
        let diagnostic = owner
            .acquire("transport", "thread", "turn", &terminal)
            .unwrap();
        assert!(!diagnostic.eligible());
        assert_eq!(diagnostic.metadata(), b"{duplicate malformed\n");
        assert!(diagnostic.payloads().is_empty());
        std::fs::write(
            owner.path().join("trace/strict-transport-input.jsonl"),
            vec![b'x'; METADATA_LIMIT + 1],
        )
        .unwrap();
        let diagnostic = owner
            .acquire("transport", "thread", "turn", &terminal)
            .unwrap();
        assert!(!diagnostic.eligible());
        assert!(diagnostic.metadata().len() <= METADATA_LIMIT);
    }
    #[test]
    fn decoded_excess_and_foreign_metadata_never_qualify() {
        let (_temp, owner, mut rows, terminal) = fixture();
        // Recomputed enclosing hashes do not cure a foreign thread/turn.
        rows[0]["thread_id"] = json!("foreign-thread");
        write(&owner, &rows);
        assert!(
            !owner
                .acquire("transport", "thread", "turn", &terminal)
                .unwrap()
                .eligible()
        );
        rows[0]["thread_id"] = json!("thread");
        let oversized = vec![b'x'; PAYLOAD_LIMIT + 1];
        let wire = zstd::stream::encode_all(oversized.as_slice(), 0).unwrap();
        std::fs::write(owner.path().join("trace/payloads/wire.bin"), &wire).unwrap();
        rows[0]["payloads"][0]["length"] = json!(wire.len());
        rows[0]["payloads"][0]["sha256"] = json!(sha(&wire));
        write(&owner, &rows);
        assert!(
            !owner
                .acquire("transport", "thread", "turn", &terminal)
                .unwrap()
                .eligible()
        );
    }
    #[test]
    fn malformed_compressed_stream_with_recomputed_hash_refuses() {
        let (_temp, owner, mut rows, _) = fixture();
        let wire = b"not zstd";
        std::fs::write(owner.path().join("trace/payloads/wire.bin"), wire).unwrap();
        rows[0]["payloads"][0]["length"] = json!(wire.len());
        rows[0]["payloads"][0]["sha256"] = json!(sha(wire));
        write(&owner, &rows);
        let raw = std::fs::read(owner.path().join("trace/strict-transport-input.jsonl")).unwrap();
        assert!(
            decode(
                &open_child(&owner.root, "trace", true).unwrap(),
                &raw,
                "thread",
                "turn"
            )
            .is_err()
        );
    }
}
