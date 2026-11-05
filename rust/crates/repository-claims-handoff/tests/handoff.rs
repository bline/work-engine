//! Acceptance tests against the real `work-engine-server claims-stdio` process and a
//! temporary (clearly synthetic) claims registry. Nothing here is a production
//! publication. The server binary comes from WORK_ENGINE_SERVER_BIN or, by default,
//! the sibling `work-engine-server` in this build's target directory.
use claim_evidence::{
    RepositoryApplication,
    codec::{canonical_json, digest, parse_json},
    sha256_bytes,
};
use repository_claims_handoff::*;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};
use work_engine_claims_host::TrustedAdmission;

/// Tests in this file write executable scripts and spawn processes; running them one at a
/// time avoids the ETXTBSY race where a concurrent fork inherits a script's write descriptor.
fn serial() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn private(path: &Path, value: &Value) -> String {
    let bytes = serde_json::to_vec(value).unwrap();
    fs::write(path, &bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    sha256_bytes(&bytes)
}
fn server_bin() -> PathBuf {
    if let Ok(p) = std::env::var("WORK_ENGINE_SERVER_BIN") {
        return p.into();
    }
    let exe = std::env::current_exe().unwrap();
    exe.parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("work-engine-server")
}
fn inventory(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, p: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        let m = fs::symlink_metadata(p).unwrap();
        out.insert(
            p.strip_prefix(root).unwrap().into(),
            if m.is_file() {
                fs::read(p).unwrap()
            } else {
                vec![]
            },
        );
        if m.is_dir() {
            for e in fs::read_dir(p).unwrap() {
                walk(root, &e.unwrap().path(), out);
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}
fn observation(tool: &str, data: Value) -> Value {
    let mut args = json!({"project":"project-a","format":"json"});
    args["snapshot"] = json!(true);
    let result = json!({"store_generation":"generation-a","snapshot_consistent":true,"data":data});
    let _ = tool;
    json!({"request":{"tool":tool,"arguments":args},"response":json!({"content":[{"type":"text","text":result.to_string()}],"structuredContent":result,"isError":false}).to_string()})
}
/// Synthetic CBM capture (same shape the evidence crate's own tests use).
fn capture(padding: usize) -> Vec<u8> {
    let note = format!("Posted entries are summed.\n{}", "x".repeat(padding));
    serde_json::to_vec(&json!({"schema_version":1,"repository_id":"repo-a","project":"project-a","executable_sha256":"a".repeat(64),"initialization":json!({"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05","serverInfo":{"name":"codebase-memory-mcp","version":"fixture"}}}).to_string(),"project_binding":{"repository_path":"/fixture/repository","before":observation("query_graph",json!({"rows":[]}))["response"],"index_status":json!({"content":[{"type":"text","text":json!({"project":"project-a","root_path":"/fixture/repository"}).to_string()}],"structuredContent":{"project":"project-a","root_path":"/fixture/repository"},"isError":false}).to_string(),"after":observation("query_graph",json!({"rows":[]}))["response"]},"store_generation":"generation-a","anchors":[{"path":"docs/notes.md","content":note,"sha256":sha256_bytes(note.as_bytes())}],"observations":[observation("get_code_snippet",json!({"content_verified":true,"source":"def post(): pass"}))],"limitations":["Bounded attributed observations; applicability not assessed."]})).unwrap()
}
struct Fixture {
    _temp: tempfile::TempDir,
    base: PathBuf,
    artifacts: PathBuf,
    profile: PathBuf,
    v1: ExactRef,
    v2: ExactRef,
    evidence: String,
    server: TrustedServer,
}
fn exact(receipt: &Value, evidence: &str) -> ExactRef {
    ExactRef {
        claim_id: receipt["result"]["record"]["claim_id"]
            .as_str()
            .unwrap()
            .into(),
        revision_id: receipt["result"]["record"]["id"].as_str().unwrap().into(),
        revision_sha256: receipt["result"]["sha256"].as_str().unwrap().into(),
        evidence_sha256: vec![evidence.into()],
    }
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let (root, artifacts, repo) = (
            base.join("root"),
            base.join("artifacts"),
            base.join("checkout"),
        );
        for d in [&artifacts, &repo] {
            fs::create_dir(d).unwrap();
        }
        let bytes = capture(3000);
        let artifact =
            repository_evidence_cbm::verify(&bytes, &sha256_bytes(&bytes), "repo-a").unwrap();
        repository_evidence_cbm::publish_artifact(&artifacts, &artifact).unwrap();
        let evidence = sha256_bytes(&bytes);
        let source = base.join("source.json");
        let config = base.join("config.json");
        let startup = base.join("startup.json");
        let profile = base.join("read.json");
        let source_sha = private(
            &source,
            &json!({"schema_version":3,"owner":"trusted-fixture-issuer","reference":"fixture:authority","revision":"1","freshness":"pinned","profile":"repository-claims-v1","repository_id":"repo-a","actors":["producer"],"decision_scopes":["scope"],"grant_ids":["grant"]}),
        );
        let config_sha = private(
            &config,
            &json!({"schema_version":3,"profile":"repository-claims-v1","root_id":"fixture-root","repository_id":"repo-a","grants":[{"schema_version":1,"grant_id":"grant","actor":"producer","profile":"repository-fact-v1","permissions":["create_claim","publish_revision","record_reliance"],"decision_scope":"scope","authority_reference":{"owner":"trusted-fixture-issuer","reference":"fixture:authority","revision":"1","freshness":"pinned","integrity_sha256":source_sha,"status":"verified"}}]}),
        );
        let read = json!({"schema_version":1,"root":root,"config":config,"source":source,"expected_config_sha256":config_sha,"expected_source_sha256":source_sha,"repository_id":"repo-a","decision_scope":"scope","evidence":{"artifacts":artifacts,"repository":repo,"project":"project-a"}});
        let mut producer = read.clone();
        producer.as_object_mut().unwrap().remove("evidence");
        producer["actor"] = json!("producer");
        producer["grant_id"] = json!("grant");
        producer["allowed_actions"] =
            json!(["create_claim", "publish_revision", "record_reliance"]);
        let sha = private(&startup, &producer);
        let port = TrustedAdmission::load(&startup, &sha).unwrap();
        let mut app =
            RepositoryApplication::initialize(port.startup().producer().unwrap(), port).unwrap();
        let reference = json!({"owner":"repository-evidence-cbm","reference":format!("cbm-capture@{evidence}"),"revision":evidence,"integrity_sha256":evidence,"freshness":"immutable observation","status":"verified"});
        let revision = |text: &str| json!({"proposition":text,"support_qualification":"attributed fixture judgment","assumptions":[],"limitations":["Synthetic fixture"],"confidence":{"label":"bounded"},"evidence_references":[reference],"sensitivity_references":[],"evidence_mode":"cbm_capture","judgment_kind":"repository_fact","decision_scope":"scope","profile_payload":{"repository_id":"repo-a","assertion_key":"assertion","assertion_scope":"bounded assertion about symbol"},"reopening_conditions":["new source evidence"],"tombstone":false});
        let mut run = |request: Value| -> Value {
            let r = app
                .execute(&parse_json(&request.to_string()).unwrap())
                .unwrap();
            serde_json::from_str(&canonical_json(&r).unwrap()).unwrap()
        };
        let first = run(
            json!({"schema_version":1,"operation_id":"create","action":"create_claim","profile":"repository-fact-v1","expected_state":null,"payload":{"subject":{"namespace":"repository-fact:repo-a","subject_kind":"repository_fact","stable_subject_id":"assertion","evidence_baseline":reference,"content_set":["src/symbol.rs"]},"statement_identity":"assertion","initial_revision":revision("Historical sentence.")}}),
        );
        let second = run(
            json!({"schema_version":1,"operation_id":"revise","action":"publish_revision","profile":"repository-fact-v1","expected_state":first["result"]["record"]["id"],"payload":{"claim_id":first["result"]["record"]["claim_id"],"revision":revision("Newer sentence.")}}),
        );
        let _reliance = run(
            json!({"schema_version":1,"operation_id":"rely","action":"record_reliance","profile":"repository-fact-v1","expected_state":null,"payload":{"consumer":"consumer-a","consumer_revision":"1","decision_scope":"scope","claim_revision_id":first["result"]["record"]["id"],"claim_revision_sha256":first["result"]["sha256"],"state":"active","predecessor_reliance":null}}),
        );
        drop(app);
        private(&profile, &read);
        let exe = server_bin();
        let server = TrustedServer {
            executable_sha256: file_sha256(&exe)
                .expect("server binary (set WORK_ENGINE_SERVER_BIN)"),
            executable: exe,
            read_profile_sha256: file_sha256(&profile).unwrap(),
            read_profile: profile.clone(),
        };
        Self {
            v1: exact(&first, &evidence),
            v2: exact(&second, &evidence),
            _temp: temp,
            base,
            artifacts,
            profile,
            evidence,
            server,
        }
    }
    fn handoff(&self, r: &ExactRef) -> Handoff {
        Handoff::new(
            "root:fixture",
            "explicit historical selection in a synthetic fixture",
            vec![r.clone()],
        )
        .unwrap()
    }
    fn resolve(&self, r: &ExactRef, page: usize) -> Result<Resolution, Error> {
        resolve_handoff(
            &self.handoff(r),
            &self.server,
            &Options {
                page_bytes: page,
                deadline: Duration::from_secs(30),
            },
        )
    }
}
fn checkpoint(h: &Handoff) -> String {
    let output = format!(
        "Host command success: true\nrustc 1.92.0\n{}\nBOUNDED_CODING_EDIT_AND_TEST_PASSED\n\n[stderr]\n",
        h.line().unwrap()
    );
    json!({"version":1,"successor_context":"context:new","fixture":true,"coding_result":{"type":"tool_result","id":"call-1","ok":true,"output":output},"coding_result_sha256":"0".repeat(64),"workspace_manifest_sha256":"1".repeat(64),"tool_executions":1,"unanswered_human":{"authority":"human","resolved":false}}).to_string()
}

#[test]
fn historical_revision_resolves_exactly_with_newer_head_and_framed_pages() {
    let _serial = serial();
    let f = Fixture::new();
    let before = inventory(&f.base);
    let old = f.resolve(&f.v1, 512).unwrap();
    let r = &old.refs[0];
    assert_eq!(r.revision_id, f.v1.revision_id);
    assert!(!r.current_head, "v1 has a newer head");
    assert_eq!(r.head_revision_ids, vec![f.v2.revision_id.clone()]);
    assert!(
        r.projection_pages > 1,
        "projection must be paged: {}",
        r.projection_bytes
    );
    assert_eq!(r.evidence[0].sha256, f.evidence);
    assert!(r.evidence[0].pages > 1);
    // Page size must not change the verified identity, digest or bytes.
    let whole = f.resolve(&f.v1, 32768).unwrap();
    assert_eq!(whole.refs[0].projection_pages, 1);
    assert_eq!(whole.refs[0].projection_sha256, r.projection_sha256);
    assert_eq!(
        whole.refs[0].evidence[0].total_bytes,
        r.evidence[0].total_bytes
    );
    let new = f.resolve(&f.v2, 4096).unwrap();
    assert!(new.refs[0].current_head);
    assert_ne!(new.refs[0].projection_sha256, r.projection_sha256);
    let report = old.report();
    assert_eq!(report["applicability"], "not_assessed");
    assert_eq!(
        before,
        inventory(&f.base),
        "reads must not mutate the registry or artifacts"
    );
}

#[test]
fn checkpoint_recovery_is_exact_and_refuses_missing_ambiguous_or_altered_lines() {
    let _serial = serial();
    let f = Fixture::new();
    let h = f.handoff(&f.v1);
    assert_eq!(extract_from_checkpoint(&checkpoint(&h)).unwrap(), h);
    assert!(h.line().unwrap().len() < MAX_HANDOFF_BYTES + MARKER.len());
    let missing = json!({"version":1,"coding_result":{"output":"no reference here"}}).to_string();
    assert_eq!(
        extract_from_checkpoint(&missing).unwrap_err().code(),
        "handoff_missing"
    );
    assert_eq!(
        extract_from_checkpoint("{}").unwrap_err().code(),
        "invalid_checkpoint"
    );
    let other = f.handoff(&f.v2);
    let two = json!({"coding_result":{"output":format!("{}\n{}", h.line().unwrap(), other.line().unwrap())}}).to_string();
    assert_eq!(
        extract_from_checkpoint(&two).unwrap_err().code(),
        "handoff_ambiguous"
    );
    // Identical duplicate text is one selection; a reference outside coding_result is not read.
    let dup = json!({"coding_result":{"a":h.line().unwrap(),"b":h.line().unwrap()}}).to_string();
    assert_eq!(extract_from_checkpoint(&dup).unwrap(), h);
    let outside = json!({"note":h.line().unwrap(),"coding_result":{"output":"none"}}).to_string();
    assert_eq!(
        extract_from_checkpoint(&outside).unwrap_err().code(),
        "handoff_missing"
    );
    // Altered, non-canonical, extra-field or unbounded handoffs are refused as handoffs.
    let body = h.canonical().unwrap();
    for bad in [
        body.replace("\"exact\"", "\"newest\""),
        body.replace("not_assessed", "applicable"),
        body.replace("{\"applicability", "{\"extra\":1,\"applicability"),
        format!(" {body}"),
        body.replace(&f.v1.revision_sha256, "ABC"),
    ] {
        assert_eq!(
            Handoff::parse(&bad).unwrap_err().code(),
            "invalid_handoff",
            "{bad}"
        );
    }
    let huge = checkpoint(&h) + &" ".repeat(MAX_CHECKPOINT_BYTES);
    assert_eq!(
        extract_from_checkpoint(&huge).unwrap_err().code(),
        "invalid_checkpoint"
    );
    assert!(Handoff::new("a", "b", vec![]).is_err());
    let mut too_many = vec![f.v1.clone(); 2];
    too_many[1].revision_id.push('x');
    assert!(Handoff::new("a", "b", too_many).is_ok());
    let mut dup = vec![f.v1.clone(); 2];
    dup[1].evidence_sha256.clear();
    assert!(
        Handoff::new("a", "b", dup).is_err(),
        "duplicate exact reference"
    );
}

#[test]
fn wrong_lost_or_tampered_references_refuse_without_fallback_or_mutation() {
    let _serial = serial();
    let f = Fixture::new();
    let before = inventory(&f.base);
    let flip = |s: &str| {
        let mut c: Vec<char> = s.chars().collect();
        c[0] = if c[0] == '0' { '1' } else { '0' };
        c.into_iter().collect::<String>()
    };
    let mut wrong_digest = f.v1.clone();
    wrong_digest.revision_sha256 = flip(&f.v1.revision_sha256);
    assert_eq!(
        f.resolve(&wrong_digest, 512).unwrap_err().code(),
        "owner_refused"
    );
    let mut crossed = f.v1.clone();
    crossed.revision_id = f.v2.revision_id.clone(); // no newest fallback, no silent repair
    assert_eq!(
        f.resolve(&crossed, 512).unwrap_err().code(),
        "owner_refused"
    );
    let mut unknown = f.v1.clone();
    unknown.claim_id = "claim-v1@".to_string() + &"f".repeat(64);
    assert_eq!(
        f.resolve(&unknown, 512).unwrap_err().code(),
        "owner_refused"
    );
    let mut unlisted = f.v1.clone();
    unlisted.evidence_sha256 = vec![flip(&f.evidence)];
    assert_eq!(
        f.resolve(&unlisted, 512).unwrap_err().code(),
        "evidence_not_listed"
    );
    // Untrusted pins refuse before any server starts.
    let mut s = f.server.clone();
    s.executable_sha256 = flip(&s.executable_sha256);
    assert_eq!(
        resolve_handoff(&f.handoff(&f.v1), &s, &Options::default())
            .unwrap_err()
            .code(),
        "server_untrusted"
    );
    let mut s = f.server.clone();
    s.read_profile_sha256 = flip(&s.read_profile_sha256);
    assert_eq!(
        resolve_handoff(&f.handoff(&f.v1), &s, &Options::default())
            .unwrap_err()
            .code(),
        "profile_untrusted"
    );
    let mut s = f.server.clone();
    s.executable = f.base.join("absent-server");
    assert_eq!(
        resolve_handoff(&f.handoff(&f.v1), &s, &Options::default())
            .unwrap_err()
            .code(),
        "server_unavailable"
    );
    assert_eq!(f.resolve(&f.v1, 3).unwrap_err().code(), "invalid_options");
    assert_eq!(before, inventory(&f.base));
    // Tampered then lost evidence artifact: the owner read refuses; claim/revision still resolve.
    let artifact = f.artifacts.join(format!("{}.json", f.evidence));
    let original = fs::read(&artifact).unwrap();
    let mut changed = original.clone();
    changed[10] ^= 1;
    fs::write(&artifact, &changed).unwrap();
    assert_eq!(f.resolve(&f.v1, 512).unwrap_err().code(), "owner_refused");
    fs::remove_file(&artifact).unwrap();
    assert_eq!(f.resolve(&f.v1, 512).unwrap_err().code(), "owner_refused");
    let mut no_evidence = f.v1.clone();
    no_evidence.evidence_sha256.clear();
    assert!(
        f.resolve(&no_evidence, 512).unwrap().refs[0]
            .evidence
            .is_empty()
    );
}

const PROXY: &str = r#"#!/usr/bin/python3
import hashlib, json, subprocess, sys
mode = "__MODE__"
p = subprocess.Popen(["__REAL__", "claims-stdio", "__PROFILE__"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
n = 0
def enc(v):
    return json.dumps(v, sort_keys=True, separators=(",", ":"), ensure_ascii=False) + "\n"
def replace_projection(o, text):
    o["canonical_projection_page"] = text
    o["total_bytes"] = len(text.encode())
    o["projection_sha256"] = hashlib.sha256(text.encode()).hexdigest()
    o["has_more"] = False
    o["next_offset"] = None
def mutate_projection(o):
    v = json.loads(o["canonical_projection_page"])
    it = v["items"][0]
    text = None
    if mode == "recode": pass
    elif mode == "subst": it["revision"]["proposition"] = "Altered sentence."
    elif mode == "noncanonical": text = json.dumps(v) + "\n"
    elif mode == "drop_head": del it["head_revision_ids"]
    elif mode == "null_head": it["head_revision_ids"] = None
    elif mode == "bad_head": it["current_head"] = not it["current_head"]
    elif mode == "drop_current": del it["current_head"]
    elif mode == "drop_admitted": del it["admitted_operation"]
    elif mode == "drop_authority": del it["authority"]
    elif mode == "drop_limitations": del v["limitations"]
    elif mode == "no_actor": del it["authority"]["actor"]
    elif mode == "no_creation_actor": del it["creation_authority"]["actor"]
    elif mode == "no_authority_reference_field": del it["authority"]["authority_reference"]["integrity_sha256"]
    elif mode == "creation_permissions_type": it["creation_authority"]["permissions"] = "x"
    elif mode == "no_binding_producer": del it["admitted_operation"]["binding"]["producer"]
    elif mode == "no_operation_field": del it["admitted_operation"]["operation"]["payload_sha256"]
    elif mode == "no_request_field": del it["admitted_operation"]["request"]["profile"]
    elif mode == "request_payload_type": it["admitted_operation"]["request"]["payload"] = "x"
    elif mode == "no_baseline_field": del it["claim"]["subject"]["evidence_baseline"]["owner"]
    elif mode == "rel_no_producer": del it["reliances"][0]["attributed_producer"]
    elif mode == "rel_no_actor": del it["reliances"][0]["authority"]["actor"]
    elif mode == "rel_authority_type": it["reliances"][0]["authority"]["permissions"] = [1]
    elif mode == "rel_no_binding_producer": del it["reliances"][0]["admitted_operation"]["binding"]["producer"]
    elif mode == "rel_no_request_field": del it["reliances"][0]["admitted_operation"]["request"]["payload"]
    elif mode == "reviewer_pair": del it["authority"]["actor"]; del it["admitted_operation"]["binding"]["producer"]
    elif mode == "request": v["projection"]["request_sha256"] = "0" * 64
    elif mode == "minimal":
        v = {"applicability": "not_assessed", "items": [{"claim": {"id": it["claim"]["id"]}, "revision": {"id": it["revision"]["id"], "claim_id": it["claim"]["id"]}, "revision_sha256": it["revision_sha256"]}]}
    replace_projection(o, text if text is not None else enc(v))
for line in sys.stdin:
    p.stdin.write(line); p.stdin.flush()
    out = p.stdout.readline()
    d = json.loads(out)
    o = d.get("result", {}).get("structuredContent", {}).get("owner_result")
    if o and ("canonical_projection_page" in o or "utf8" in o):
        n += 1
        key = "canonical_projection_page" if "canonical_projection_page" in o else "utf8"
        if mode == "flip" and n == 2: o[key] = "X" + o[key][1:]
        if mode == "gap" and n == 2: o["offset"] = o["offset"] + 1
        if mode == "short" and not o["has_more"] and n > 1: o[key] = o[key][:-1]
        if mode == "lie" and n == 1: o["has_more"] = False; o["next_offset"] = None
        if mode == "digest" and key == "canonical_projection_page": o["projection_sha256"] = "0" * 64
        if mode == "evnext" and key == "utf8" and not o["has_more"]: o["next_offset"] = 5
        if key == "canonical_projection_page" and mode in ("recode", "subst", "noncanonical", "drop_head", "null_head", "bad_head", "drop_current", "drop_admitted", "drop_authority", "drop_limitations", "request", "minimal", "no_actor", "no_creation_actor", "no_authority_reference_field", "creation_permissions_type", "no_binding_producer", "no_operation_field", "no_request_field", "request_payload_type", "no_baseline_field", "rel_no_producer", "rel_no_actor", "rel_authority_type", "rel_no_binding_producer", "rel_no_request_field", "reviewer_pair"):
            mutate_projection(o)
        out = json.dumps(d) + "\n"
    sys.stdout.write(out); sys.stdout.flush()
"#;
/// Fault injection between the real server and the verifier: every response is the real
/// server's, except one mutated according to MODE.
fn proxy(f: &Fixture, mode: &str) -> TrustedServer {
    let script = f.base.join(format!("proxy-{mode}.py"));
    let body = PROXY
        .replace("__MODE__", mode)
        .replace("__REAL__", &f.server.executable.display().to_string())
        .replace("__PROFILE__", &f.profile.display().to_string());
    fs::write(&script, body).unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
    TrustedServer {
        executable_sha256: file_sha256(&script).unwrap(),
        executable: script,
        read_profile: f.profile.clone(),
        read_profile_sha256: f.server.read_profile_sha256.clone(),
    }
}
#[test]
fn partial_or_corrupt_paged_projection_and_evidence_are_refused() {
    let _serial = serial();
    let f = Fixture::new();
    // Control: the proxy with no mutation reproduces the verified result.
    let control = proxy(&f, "none");
    let ok = resolve_handoff(
        &f.handoff(&f.v1),
        &control,
        &Options {
            page_bytes: 512,
            deadline: Duration::from_secs(30),
        },
    )
    .unwrap();
    assert!(ok.refs[0].projection_pages > 1 && ok.refs[0].evidence[0].pages > 1);
    for (mode, code) in [
        ("flip", "projection_corrupt"),
        ("gap", "projection_corrupt"),
        ("short", "projection_corrupt"),
        ("lie", "projection_corrupt"),
        ("digest", "owner_refused"),
    ] {
        let e = resolve_handoff(
            &f.handoff(&f.v1),
            &proxy(&f, mode),
            &Options {
                page_bytes: 512,
                deadline: Duration::from_secs(30),
            },
        )
        .unwrap_err();
        assert_eq!(e.code(), code, "{mode}: {e}");
    }
    // Evidence-only handoff cannot reach evidence corruption before projection: flip applies to
    // the second paged response overall, so verify the evidence path with a projection that fits one page.
    for mode in ["flip", "gap", "short"] {
        let e = resolve_handoff(
            &f.handoff(&f.v1),
            &proxy(&f, mode),
            &Options {
                page_bytes: 32768,
                deadline: Duration::from_secs(30),
            },
        )
        .unwrap_err();
        assert_eq!(e.code(), "evidence_corrupt", "{mode}: {e}");
    }
}

#[test]
fn cli_recovers_from_checkpoint_and_refuses_with_exit_status() {
    let _serial = serial();
    let f = Fixture::new();
    let cli = env!("CARGO_BIN_EXE_repository-claims-handoff");
    let line = Command::new(cli)
        .args([
            "emit",
            "--selected-by",
            "root:fixture",
            "--basis",
            "explicit historical selection",
            "--ref",
        ])
        .arg(format!(
            "{},{},{},{}",
            f.v1.claim_id, f.v1.revision_id, f.v1.revision_sha256, f.evidence
        ))
        .output()
        .unwrap();
    assert!(
        line.status.success(),
        "{}",
        String::from_utf8_lossy(&line.stdout)
    );
    let line = String::from_utf8(line.stdout).unwrap();
    let h = Handoff::parse(line.trim_end().strip_prefix(MARKER).unwrap()).unwrap();
    assert_eq!(
        h,
        f.handoff(&f.v1)
            .clone()
            .with_text("explicit historical selection")
    );
    let cp = f.base.join("checkpoint.json");
    fs::write(&cp, checkpoint(&h)).unwrap();
    let extracted = Command::new(cli).arg("extract").arg(&cp).output().unwrap();
    assert_eq!(String::from_utf8(extracted.stdout).unwrap(), line);
    let verify = |sha: &str| {
        Command::new(cli)
            .args(["verify", "--page-bytes", "512", "--checkpoint"])
            .arg(&cp)
            .arg("--server")
            .arg(&f.server.executable)
            .args(["--server-sha256", sha, "--profile"])
            .arg(&f.profile)
            .args(["--profile-sha256", &f.server.read_profile_sha256])
            .output()
            .unwrap()
    };
    let good = verify(&f.server.executable_sha256);
    assert!(
        good.status.success(),
        "{}",
        String::from_utf8_lossy(&good.stdout)
    );
    let report: Value = serde_json::from_slice(&good.stdout).unwrap();
    assert_eq!(report["status"], "resolved");
    assert_eq!(report["refs"][0]["current_head"], false);
    // The owner's continuity result carries the exact checkpoint string.
    let result = f.base.join("continuity-result.json");
    fs::write(
        &result,
        json!({"schema":"work-engine.pi-coding-continuity-result.v1","checkpoint":checkpoint(&h)})
            .to_string(),
    )
    .unwrap();
    let via_result = Command::new(cli)
        .args(["extract", "--continuity-result"])
        .arg(&result)
        .output()
        .unwrap();
    assert_eq!(String::from_utf8(via_result.stdout).unwrap(), line);
    fs::write(
        &result,
        json!({"schema":"other","checkpoint":checkpoint(&h)}).to_string(),
    )
    .unwrap();
    let wrong = Command::new(cli)
        .args(["extract", "--continuity-result"])
        .arg(&result)
        .output()
        .unwrap();
    assert_eq!(wrong.status.code(), Some(2));
    let bad = verify(&"0".repeat(64));
    assert_eq!(bad.status.code(), Some(2));
    let report: Value = serde_json::from_slice(&bad.stdout).unwrap();
    assert_eq!(
        (report["status"].as_str(), report["code"].as_str()),
        (Some("refused"), Some("server_untrusted"))
    );
    let none = f.base.join("no-handoff.json");
    fs::write(
        &none,
        json!({"coding_result":{"output":"nothing"}}).to_string(),
    )
    .unwrap();
    let missing = Command::new(cli)
        .arg("extract")
        .arg(&none)
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(2));
}
fn one_page(f: &Fixture, server: &TrustedServer, r: &ExactRef) -> Result<Resolution, Error> {
    let _ = f;
    resolve_handoff(
        &Handoff::new("root:fixture", "fault injection", vec![r.clone()]).unwrap(),
        server,
        &Options {
            page_bytes: 32768,
            deadline: Duration::from_secs(30),
        },
    )
}

#[test]
fn recomputed_digest_substitution_and_missing_native_metadata_are_refused() {
    let _serial = serial();
    let f = Fixture::new();
    // Control: re-encoding with a recomputed outer digest and no content change is accepted, so
    // each refusal below is attributable to the single mutation, not to the re-encoding.
    let control = one_page(&f, &proxy(&f, "recode"), &f.v1).unwrap();
    let real = f.resolve(&f.v1, 32768).unwrap();
    assert_eq!(
        control.refs[0].projection_sha256,
        real.refs[0].projection_sha256
    );
    for (mode, code) in [
        ("subst", "revision_mismatch"),
        ("noncanonical", "projection_corrupt"),
        ("drop_head", "revision_mismatch"),
        ("null_head", "revision_mismatch"),
        ("bad_head", "revision_mismatch"),
        ("drop_current", "revision_mismatch"),
        ("drop_admitted", "revision_mismatch"),
        ("drop_authority", "revision_mismatch"),
        ("drop_limitations", "revision_mismatch"),
        ("request", "revision_mismatch"),
        ("minimal", "revision_mismatch"),
        ("no_actor", "revision_mismatch"),
        ("no_creation_actor", "revision_mismatch"),
        ("no_authority_reference_field", "revision_mismatch"),
        ("creation_permissions_type", "revision_mismatch"),
        ("no_binding_producer", "revision_mismatch"),
        ("no_operation_field", "revision_mismatch"),
        ("no_request_field", "revision_mismatch"),
        ("request_payload_type", "revision_mismatch"),
        ("no_baseline_field", "revision_mismatch"),
        ("rel_no_producer", "revision_mismatch"),
        ("rel_no_actor", "revision_mismatch"),
        ("rel_authority_type", "revision_mismatch"),
        ("rel_no_binding_producer", "revision_mismatch"),
        ("rel_no_request_field", "revision_mismatch"),
        ("reviewer_pair", "revision_mismatch"),
    ] {
        let e = one_page(&f, &proxy(&f, mode), &f.v1).unwrap_err();
        assert_eq!(e.code(), code, "{mode}: {e}");
    }
    // Terminal evidence page with a non-null next_offset (CLAIMHANDOFF-004).
    let e = one_page(&f, &proxy(&f, "evnext"), &f.v1).unwrap_err();
    assert_eq!(e.code(), "evidence_corrupt", "evnext: {e}");
}

#[test]
fn report_retains_the_complete_native_owner_projection() {
    let _serial = serial();
    let f = Fixture::new();
    for (selected, other) in [(&f.v1, &f.v2), (&f.v2, &f.v1)] {
        let res = f.resolve(selected, 512).unwrap();
        let r = &res.refs[0];
        let report = res.report();
        let entry = &report["refs"][0];
        let p = &entry["owner_projection"];
        // The reported projection is the exact owner object: recanonicalising it with the
        // native codec reproduces the verified bytes and digest.
        let bytes = canonical_json(&parse_json(&p.to_string()).unwrap()).unwrap();
        assert_eq!(sha256_bytes(bytes.as_bytes()), r.projection_sha256);
        assert_eq!(bytes.len(), r.projection_bytes);
        assert_eq!(entry["projection_sha256"], r.projection_sha256);
        let item = &p["items"][0];
        let want = if selected.revision_id == f.v1.revision_id {
            "Historical sentence."
        } else {
            "Newer sentence."
        };
        assert_eq!(item["revision"]["proposition"], want);
        assert_eq!(item["revision"]["id"], selected.revision_id);
        assert_eq!(
            item["revision"]["limitations"],
            json!(["Synthetic fixture"])
        );
        assert_eq!(
            item["revision"]["evidence_references"][0]["integrity_sha256"],
            f.evidence
        );
        assert_eq!(item["claim"]["id"], selected.claim_id);
        assert_eq!(item["head_revision_ids"], json!([f.v2.revision_id]));
        assert_eq!(
            item["current_head"],
            selected.revision_id == f.v2.revision_id
        );
        assert_ne!(item["revision"]["id"], other.revision_id);
        assert!(item["admitted_operation"]["binding"].is_object());
        assert!(item["admitted_operation"]["request"].is_object());
        assert_eq!(
            item["authority"]["grant_id"],
            item["revision"]["authority_ref"]
        );
        assert_eq!(
            item["creation_authority"]["grant_id"],
            item["claim"]["authority_ref"]
        );
        assert!(item["unresolved_references"].is_array() && item["reliances"].is_array());
        assert_eq!(
            item["reliances"].as_array().unwrap().len(),
            usize::from(selected.revision_id == f.v1.revision_id),
            "fixture records one reliance on the historical revision"
        );
        assert_eq!(p["applicability"], "not_assessed");
        assert_eq!(p["source_currency"], "not_assessed");
        assert!(
            p["limitations"][0]
                .as_str()
                .unwrap()
                .contains("truth, applicability")
        );
        assert_eq!(p["projection"]["request_sha256"], {
            let input = json!({"claim_id":selected.claim_id,"revision_id":selected.revision_id,"revision_sha256":selected.revision_sha256}).to_string();
            digest(&parse_json(&input).unwrap()).unwrap()
        });
        assert_eq!(
            report["selection"]["authentication"],
            "unauthenticated_descriptive_data"
        );
        assert_eq!(report["selection"]["selected_by"], "root:fixture");
        assert_eq!(report["semantic_acceptance"], "not_assessed");
        assert_eq!(entry["verification"]["revision_digest_recomputed"], true);
    }
    // The CLI prints the same report, including the projection.
    let cli = env!("CARGO_BIN_EXE_repository-claims-handoff");
    let h = f.handoff(&f.v1);
    let cp = f.base.join("cp.json");
    fs::write(&cp, checkpoint(&h)).unwrap();
    let out = Command::new(cli)
        .args(["verify", "--checkpoint"])
        .arg(&cp)
        .arg("--server")
        .arg(&f.server.executable)
        .args(["--server-sha256", &f.server.executable_sha256, "--profile"])
        .arg(&f.profile)
        .args(["--profile-sha256", &f.server.read_profile_sha256])
        .output()
        .unwrap();
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        report["refs"][0]["owner_projection"]["items"][0]["revision"]["proposition"],
        "Historical sentence."
    );
    assert_eq!(report["schema"], "repository-claims-handoff-resolution-v2");
}

trait WithText {
    fn with_text(self, basis: &str) -> Self;
}
impl WithText for Handoff {
    fn with_text(mut self, basis: &str) -> Self {
        self.selection_basis = basis.into();
        self
    }
}
