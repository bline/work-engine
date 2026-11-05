//! Immutable CBM observations for repository claims. A verified artifact hash
//! establishes artifact integrity, never proposition truth or applicability.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::{fd::AsRawFd, unix::fs::OpenOptionsExt},
    path::{Component, Path, PathBuf},
    time::{Duration, Instant},
};

const LIMIT: usize = 16 * 1024 * 1024;
const MAX_PAGES: usize = 64;
const SNAPSHOT_TOOLS: &[&str] = &[
    "query_graph",
    "search_graph",
    "trace_path",
    "get_code_snippet",
    "search_code",
    "check_index_coverage",
];
mod json_input;
pub use json_input::parse_json;
mod runtime;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("CBM evidence refused: {0}")]
    Invalid(&'static str),
    #[error("CBM evidence I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("CBM evidence JSON: {0}")]
    Json(#[from] serde_json::Error),
}
type Result<T> = std::result::Result<T, Error>;

fn require(ok: bool, message: &'static str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(Error::Invalid(message))
    }
}

fn wire_size(value: &impl Serialize) -> Result<usize> {
    struct Counter(usize);
    impl Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > LIMIT.saturating_sub(self.0) {
                return Err(std::io::Error::other("capture exceeds capacity"));
            }
            self.0 += bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut counter = Counter(0);
    serde_json::to_writer(&mut counter, value)?;
    Ok(counter.0)
}
pub fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn is_sha(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

pub fn executable_digest(path: &Path) -> Result<String> {
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    let metadata = file.metadata()?;
    require(
        metadata.is_file() && metadata.len() <= 1024 * 1024 * 1024,
        "invalid executable file",
    )?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

/// Selected by the trusted capture launcher, independently of tool arguments.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub repository_id: String,
    pub repository: PathBuf,
    pub project: String,
    pub executable: PathBuf,
    pub executable_sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolRequest {
    pub tool: String,
    pub arguments: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureRequest {
    pub source_paths: Vec<String>,
    pub tools: Vec<ToolRequest>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceAnchor {
    pub path: String,
    pub content: String,
    pub sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub request: ToolRequest,
    /// Exact UTF-8 stdout from the pinned executable's CLI MCP envelope.
    pub response: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capture {
    schema_version: u8,
    repository_id: String,
    project: String,
    executable_sha256: String,
    initialization: String,
    project_binding: ProjectBinding,
    store_generation: String,
    anchors: Vec<SourceAnchor>,
    observations: Vec<Observation>,
    limitations: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectBinding {
    repository_path: PathBuf,
    before: String,
    index_status: String,
    after: String,
}

#[derive(Clone, Debug)]
pub struct Artifact {
    capture: Capture,
    bytes: Vec<u8>,
    sha256: String,
}

impl Artifact {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn sha256(&self) -> &str {
        &self.sha256
    }
    pub fn capture(&self) -> &Capture {
        &self.capture
    }
    pub fn reference(&self) -> Value {
        json!({"owner":"repository-evidence-cbm", "reference":format!("cbm-capture@{}",self.sha256),
            "revision":self.sha256,"integrity_sha256":self.sha256,
            "freshness":"immutable observation; source currency and applicability not assessed", "status":"verified"})
    }
}

impl Capture {
    pub fn store_generation(&self) -> &str {
        &self.store_generation
    }
    pub fn repository_id(&self) -> &str {
        &self.repository_id
    }
    pub fn anchors(&self) -> &[SourceAnchor] {
        &self.anchors
    }
    pub fn observations(&self) -> &[Observation] {
        &self.observations
    }
    pub fn limitations(&self) -> &[String] {
        &self.limitations
    }
    pub fn documentation_status(&self) -> Result<Value> {
        let mut pages = 0usize;
        let mut rows = 0u64;
        let mut complete = true;
        let mut expected_cursor: Option<String> = None;
        let mut expected_total = None;
        let mut group_rows = 0u64;
        for observation in &self.observations {
            if observation.request.tool != "detect_changes" {
                continue;
            }
            let (_, data) = observed_data(observation)?;
            if observation
                .request
                .arguments
                .get("reference_cursor")
                .is_none()
            {
                if expected_cursor.is_some() {
                    complete = false;
                }
                expected_total = data["total"].as_u64();
                group_rows = 0;
            } else {
                require(
                    observation.request.arguments["reference_cursor"].as_str()
                        == expected_cursor.as_deref(),
                    "missing or reordered doc page",
                )?;
                require(
                    data["total"].as_u64() == expected_total,
                    "doc totals changed",
                )?;
            }
            let returned = data["returned"]
                .as_u64()
                .ok_or(Error::Invalid("missing doc row count"))?;
            pages += 1;
            rows += returned;
            group_rows += returned;
            if data["has_more"] == true {
                expected_cursor = Some(
                    data["next_cursor"]
                        .as_str()
                        .filter(|v| !v.is_empty())
                        .ok_or(Error::Invalid("missing doc continuation"))?
                        .into(),
                );
            } else {
                expected_cursor = None;
                if data["total_relation"] != "eq" || data["truncated"] != false {
                    complete = false;
                } else {
                    require(
                        expected_total == Some(group_rows),
                        "incomplete doc row coverage",
                    )?;
                }
            }
        }
        if expected_cursor.is_some() {
            complete = false;
        }
        Ok(
            json!({"pages":pages,"rows":rows,"pagination":if pages==0 {"not_requested"} else if complete {"complete"} else {"partial"},"coverage":"indexed_references_only","semantics":"referenced_targets_for_review"}),
        )
    }
}

fn source(root: &Path, relative: &str) -> Result<SourceAnchor> {
    let path = Path::new(relative);
    require(
        !relative.is_empty() && path.components().all(|c| matches!(c, Component::Normal(_))),
        "invalid source path",
    )?;
    let resolved = root.join(path).canonicalize()?;
    require(
        resolved.starts_with(root) && resolved.is_file(),
        "source escapes repository",
    )?;
    let bytes = read_within(root, &resolved)?;
    let content = String::from_utf8(bytes).map_err(|_| Error::Invalid("source is not UTF-8"))?;
    Ok(SourceAnchor {
        path: relative.into(),
        sha256: sha256(content.as_bytes()),
        content,
    })
}

fn read_within(root: &Path, path: &Path) -> Result<Vec<u8>> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    require(file.metadata()?.is_file(), "not a regular evidence file")?;
    let actual = fs::read_link(format!("/proc/self/fd/{}", file.as_raw_fd()))?;
    require(
        actual.starts_with(root),
        "evidence file escaped its selected root",
    )?;
    let mut bytes = Vec::new();
    file.take((LIMIT + 1) as u64).read_to_end(&mut bytes)?;
    require(bytes.len() <= LIMIT, "evidence file exceeds capacity")?;
    Ok(bytes)
}

fn normalize(binding: &Binding, request: &ToolRequest) -> Result<ToolRequest> {
    let mut result = request.clone();
    let args = result
        .arguments
        .as_object_mut()
        .ok_or(Error::Invalid("tool arguments are not an object"))?;
    require(
        args.get("project")
            .is_none_or(|v| v.as_str() == Some(&binding.project)),
        "second CBM project",
    )?;
    require(
        args.get("format").is_none_or(|v| v == "json"),
        "JSON format required",
    )?;
    args.insert("project".into(), json!(binding.project));
    args.insert("format".into(), json!("json"));
    if SNAPSHOT_TOOLS.contains(&result.tool.as_str()) {
        require(
            args.get("snapshot").is_none_or(|v| v == true),
            "snapshot required",
        )?;
        args.insert("snapshot".into(), json!(true));
        require(
            result.tool != "trace_path" || args.get("mode") != Some(&json!("cross_service")),
            "multi-project tracing is unsupported",
        )?;
    } else {
        require(
            result.tool == "detect_changes" && args.get("scope") == Some(&json!("doc_references")),
            "unsupported evidence tool",
        )?;
        require(
            !args.contains_key("snapshot"),
            "doc references have their own publication binding",
        )?;
        require(
            !args.contains_key("reference_cursor"),
            "a documentation capture must start at the first page",
        )?;
    }
    Ok(result)
}

fn payload(raw: &str) -> Result<Value> {
    let raw_envelope: Value = parse_json(raw.as_bytes())?;
    require(raw_envelope.get("error").is_none(), "CBM RPC error")?;
    let envelope = raw_envelope.get("result").unwrap_or(&raw_envelope);
    require(
        envelope.get("isError").is_none_or(|v| v == false) && envelope.get("error").is_none(),
        "CBM tool error",
    )?;
    let content = envelope
        .get("content")
        .and_then(Value::as_array)
        .ok_or(Error::Invalid("missing MCP content"))?;
    require(
        content.len() == 1 && content[0]["type"] == "text",
        "ambiguous MCP content",
    )?;
    let text: Value = parse_json(
        content[0]["text"]
            .as_str()
            .ok_or(Error::Invalid("missing MCP text"))?
            .as_bytes(),
    )?;
    if let Some(value) = envelope.get("structuredContent") {
        require(value == &text, "conflicting MCP representations")?;
    }
    Ok(text)
}

fn observed_data(observation: &Observation) -> Result<(String, Value)> {
    let value = payload(&observation.response)?;
    let generation = value
        .get("store_generation")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or(Error::Invalid("missing publication identity"))?
        .to_owned();
    if SNAPSHOT_TOOLS.contains(&observation.request.tool.as_str()) {
        require(
            value["snapshot_consistent"] == true,
            "unbound graph observation",
        )?;
        let data = value
            .get("data")
            .filter(|v| v.is_object())
            .ok_or(Error::Invalid("missing JSON snapshot data"))?;
        Ok((generation, data.clone()))
    } else {
        require(
            value["scope"] == "doc_references"
                && value["semantics"] == "referenced_targets_for_review"
                && value["coverage"] == "indexed_references_only",
            "wrong documentation evidence semantics",
        )?;
        let references = value["references"]
            .as_array()
            .ok_or(Error::Invalid("missing doc references"))?;
        require(
            value["returned"].as_u64() == Some(references.len() as u64)
                && value["has_more"].is_boolean(),
            "invalid doc page",
        )?;
        Ok((generation, value))
    }
}

fn partial(value: &Value) -> bool {
    match value {
        Value::Object(map) => map.iter().any(|(k, v)| {
            ((k == "has_more" || k == "truncated" || k.ends_with("_has_more")) && v == true)
                || partial(v)
        }),
        Value::Array(items) => items.iter().any(partial),
        _ => false,
    }
}

fn validate(capture: &Capture) -> Result<()> {
    let initialized = parse_json(capture.initialization.as_bytes())?;
    require(
        initialized["result"]["serverInfo"]["name"] == "codebase-memory-mcp"
            && initialized["result"]["protocolVersion"] == "2024-11-05",
        "invalid capture initialization",
    )?;
    validate_project_binding(
        &capture.project_binding,
        &capture.project,
        &capture.store_generation,
    )?;
    require(
        capture.schema_version == 1
            && !capture.repository_id.is_empty()
            && !capture.project.is_empty()
            && is_sha(&capture.executable_sha256),
        "invalid capture binding",
    )?;
    require(
        !capture.anchors.is_empty() && !capture.observations.is_empty(),
        "empty evidence capture",
    )?;
    let mut paths = BTreeSet::new();
    for anchor in &capture.anchors {
        require(
            !anchor.path.is_empty()
                && Path::new(&anchor.path)
                    .components()
                    .all(|c| matches!(c, Component::Normal(_))),
            "invalid stored source path",
        )?;
        require(
            paths.insert(&anchor.path) && sha256(anchor.content.as_bytes()) == anchor.sha256,
            "source anchor integrity",
        )?;
    }
    for observation in &capture.observations {
        require(
            observation.request.arguments["project"] == capture.project
                && observation.request.arguments["format"] == "json",
            "capture project mismatch",
        )?;
        if SNAPSHOT_TOOLS.contains(&observation.request.tool.as_str()) {
            require(
                observation.request.arguments["snapshot"] == true,
                "snapshot request missing",
            )?;
        } else {
            require(
                observation.request.tool == "detect_changes"
                    && observation.request.arguments["scope"] == "doc_references",
                "unsupported capture tool",
            )?;
        }
        let (generation, _) = observed_data(observation)?;
        require(
            generation == capture.store_generation,
            "mixed graph publications",
        )?;
    }
    require(
        !capture.limitations.is_empty(),
        "missing evidence limitations",
    )?;
    capture.documentation_status()?;
    Ok(())
}

fn validate_project_binding(
    binding: &ProjectBinding,
    project: &str,
    generation: &str,
) -> Result<()> {
    require(
        binding.repository_path.is_absolute(),
        "invalid captured repository path",
    )?;
    for raw in [&binding.before, &binding.after] {
        let snapshot = payload(raw)?;
        require(
            snapshot["snapshot_consistent"] == true && snapshot["store_generation"] == generation,
            "project binding publication changed",
        )?;
    }
    let status = payload(&binding.index_status)?;
    require(
        status["project"] == project
            && status["root_path"].as_str().map(PathBuf::from).as_ref()
                == Some(&binding.repository_path),
        "CBM project is bound to another repository",
    )
}

/// Reads only; a mismatch yields a new-capture requirement, never a claim edit.
pub fn source_currency(artifact: &Artifact, repository: &Path) -> Result<Value> {
    let root = repository.canonicalize()?;
    let states: Vec<_> = artifact
        .capture
        .anchors
        .iter()
        .map(|anchor| {
            let state = match source(&root, &anchor.path) {
                Ok(current) if current.sha256 == anchor.sha256 => "bytes_match",
                Ok(_) => "bytes_changed",
                Err(_) => "unavailable",
            };
            json!({"path":anchor.path,"state":state,"captured_sha256":anchor.sha256})
        })
        .collect();
    Ok(
        json!({"anchors":states,"semantics":"selected anchor byte equality only","applicability":"not_assessed"}),
    )
}

pub fn verify(bytes: &[u8], expected_sha256: &str, repository_id: &str) -> Result<Artifact> {
    require(
        bytes.len() <= LIMIT && is_sha(expected_sha256) && sha256(bytes) == expected_sha256,
        "capture digest mismatch",
    )?;
    let capture: Capture = serde_json::from_value(parse_json(bytes)?)?;
    validate(&capture)?;
    require(
        capture.repository_id == repository_id,
        "capture scope mismatch",
    )?;
    Ok(Artifact {
        capture,
        bytes: bytes.to_vec(),
        sha256: expected_sha256.into(),
    })
}

/// Content-addressed, create-only custody. Replaying identical bytes is safe.
pub fn publish_artifact(root: &Path, artifact: &Artifact) -> Result<PathBuf> {
    require(
        root.is_absolute() && root.canonicalize()? == root && root.is_dir(),
        "artifact root must already exist and be canonical",
    )?;
    let path = root.join(format!("{}.json", artifact.sha256));
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| Error::Invalid("invalid publication clock"))?
        .as_nanos();
    let temporary = root.join(format!(
        ".capture-{}-{nonce}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temporary)?;
    struct Staging {
        path: PathBuf,
        armed: bool,
    }
    impl Drop for Staging {
        fn drop(&mut self) {
            if self.armed {
                let _ = fs::remove_file(&self.path);
            }
        }
    }
    let mut staging = Staging {
        path: temporary,
        armed: true,
    };
    file.write_all(&artifact.bytes)?;
    file.sync_all()?;
    drop(file);
    match fs::hard_link(&staging.path, &path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            require(
                read_within(root, &path)? == artifact.bytes,
                "artifact custody conflict",
            )?;
        }
        Err(error) => return Err(error.into()),
    }
    fs::remove_file(&staging.path)?;
    staging.armed = false;
    File::open(root)?.sync_all()?;
    Ok(path)
}

pub fn read_artifact(root: &Path, digest: &str, repository_id: &str) -> Result<Artifact> {
    require(is_sha(digest), "invalid capture digest")?;
    require(
        root.is_absolute() && root.canonicalize()? == root,
        "artifact root is redirected",
    )?;
    verify(
        &read_within(root, &root.join(format!("{digest}.json")))?,
        digest,
        repository_id,
    )
}

pub fn capture(binding: &Binding, request: &CaptureRequest) -> Result<Artifact> {
    let capture_deadline = Instant::now() + Duration::from_secs(300);
    require(
        !binding.repository_id.is_empty()
            && !binding.project.is_empty()
            && is_sha(&binding.executable_sha256),
        "invalid trusted capture binding",
    )?;
    require(
        binding.repository.is_absolute()
            && binding.repository.canonicalize()? == binding.repository,
        "repository is redirected",
    )?;
    require(
        binding.executable.is_absolute()
            && binding.executable.canonicalize()? == binding.executable,
        "executable is redirected",
    )?;
    require(
        !request.source_paths.is_empty()
            && request.source_paths.len() <= 4096
            && !request.tools.is_empty()
            && request.tools.len() <= 32,
        "empty or oversized capture request",
    )?;
    let mut stored = wire_size(binding)? + 1024;
    let mut seen = BTreeSet::new();
    let mut anchors = Vec::new();
    for path in &request.source_paths {
        require(
            Instant::now() < capture_deadline,
            "capture deadline exceeded",
        )?;
        require(seen.insert(path), "duplicate source path")?;
        let anchor = source(&binding.repository, path)?;
        stored += wire_size(&anchor)?;
        require(stored <= LIMIT, "capture exceeds capacity")?;
        anchors.push(anchor);
    }
    let mut session = runtime::Session::start(binding, capture_deadline)?;
    // Fence the unwrapped index metadata between two publication observations;
    // every later observation must retain that same publication identity.
    let probe = ToolRequest {
        tool: "query_graph".into(),
        arguments: json!({"project":binding.project,"format":"json","snapshot":true,"query":"MATCH (n:Function) RETURN n.name LIMIT 1"}),
    };
    let before = session.observe(&probe)?;
    let status = session.observe(&ToolRequest {
        tool: "index_status".into(),
        arguments: json!({"project":binding.project,"format":"json","diagnostics":"none"}),
    })?;
    let after = session.observe(&probe)?;
    let expected_generation = payload(&before)?["store_generation"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or(Error::Invalid("missing project publication"))?
        .to_owned();
    let project_binding = ProjectBinding {
        repository_path: binding.repository.clone(),
        before,
        index_status: status,
        after,
    };
    validate_project_binding(&project_binding, &binding.project, &expected_generation)?;
    stored += wire_size(&project_binding)? + wire_size(&session.initialization())?;
    require(stored <= LIMIT, "capture exceeds capacity")?;
    let mut observations = Vec::new();
    let mut generation = Some(expected_generation);
    let mut limitations=BTreeSet::from([
        "Observations are bounded evidence; proposition truth, sufficiency and applicability are not assessed.".to_owned(),
        "Graph publication does not pin live filesystem bytes; only the listed source anchors were captured before and after the observations.".to_owned(),
        "Repository bindings and executable pins are supplied by the trusted launcher; the artifact is not independent process attestation.".to_owned(),
    ]);
    for initial in &request.tools {
        let mut current = normalize(binding, initial)?;
        let mut cursors = BTreeSet::new();
        for page in 0..MAX_PAGES {
            require(
                Instant::now() < capture_deadline,
                "capture deadline exceeded",
            )?;
            let response = session.observe(&current)?;
            let observation = Observation {
                request: current.clone(),
                response,
            };
            stored += wire_size(&observation)?;
            require(stored <= LIMIT, "capture exceeds capacity")?;
            let (actual, data) = observed_data(&observation)?;
            require(
                generation.as_ref().is_none_or(|g| g == &actual),
                "graph publication changed; restart capture",
            )?;
            generation = Some(actual);
            observations.push(observation);
            if current.tool != "detect_changes" {
                if partial(&data) {
                    limitations.insert(format!(
                        "{} has partial or bounded results; no exhaustive coverage claim.",
                        current.tool
                    ));
                }
                break;
            }
            limitations.insert("Documentation references cover indexed review candidates only, including unresolved/unavailable rows; they are not semantic impact or removed historical references.".into());
            if data["has_more"] != true {
                break;
            }
            if page + 1 == MAX_PAGES {
                limitations.insert(
                    "Documentation reference pagination is incomplete at the capture page limit."
                        .into(),
                );
                break;
            }
            let cursor = data["next_cursor"]
                .as_str()
                .filter(|c| !c.is_empty())
                .ok_or(Error::Invalid("missing doc continuation"))?;
            require(
                cursors.insert(cursor.to_owned()),
                "doc cursor did not advance",
            )?;
            current.arguments["reference_cursor"] = json!(cursor);
        }
    }
    for anchor in &anchors {
        require(
            source(&binding.repository, &anchor.path)?.sha256 == anchor.sha256,
            "source bytes changed during capture",
        )?;
    }
    let initialization = session.initialization().to_owned();
    session.finish()?;
    require(
        executable_digest(&binding.executable)? == binding.executable_sha256,
        "CBM executable changed during capture",
    )?;
    let capture = Capture {
        schema_version: 1,
        repository_id: binding.repository_id.clone(),
        project: binding.project.clone(),
        executable_sha256: binding.executable_sha256.clone(),
        initialization,
        project_binding,
        store_generation: generation.ok_or(Error::Invalid("no observations"))?,
        anchors,
        observations,
        limitations: limitations.into_iter().collect(),
    };
    validate(&capture)?;
    capture.documentation_status()?;
    wire_size(&capture)?;
    let bytes = serde_json::to_vec(&capture)?;
    require(bytes.len() <= LIMIT, "capture exceeds capacity")?;
    Ok(Artifact {
        sha256: sha256(&bytes),
        bytes,
        capture,
    })
}
