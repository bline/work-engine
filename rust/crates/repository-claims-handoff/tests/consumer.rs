//! Consumer-tool tests against the real `work-engine-server claims-stdio` process and a
//! temporary, clearly SYNTHETIC claims registry (not a production publication).
use claim_evidence::{
    RepositoryApplication,
    codec::{canonical_json, parse_json},
    sha256_bytes,
};
use repository_claims_handoff::*;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
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

use repository_claims_handoff::consumer::*;

fn consumer(f: &Fixture, page: usize, calls: usize) -> ClaimsConsumer {
    ClaimsConsumer::open(
        &f.server,
        &Options {
            page_bytes: page,
            deadline: Duration::from_secs(30),
        },
        calls,
    )
    .unwrap()
}
fn args(r: &ExactRef) -> Value {
    json!({"claim_id":r.claim_id,"revision_id":r.revision_id,"revision_sha256":r.revision_sha256})
}
fn with(mut base: Value, extra: Value) -> Value {
    for (k, v) in extra.as_object().unwrap() {
        base[k] = v.clone();
    }
    base
}
/// Reassemble every page of one tool result through the consumer.
fn drain(c: &mut ClaimsConsumer, tool: &str, mut base: Value, page: usize) -> (String, Value) {
    let mut text = String::new();
    let mut first = Value::Null;
    loop {
        let r = c
            .call(
                tool,
                &with(base.clone(), json!({"offset":text.len(),"max_bytes":page})),
                None,
            )
            .unwrap();

        assert!(r["text"].as_str().unwrap().len() <= page);
        text.push_str(r["text"].as_str().unwrap());
        if first.is_null() {
            first = r.clone();
            if tool == TOOL_DISCOVER {
                // Paging a discovery result names the digest it is paging.
                base = with(base, json!({"blob_sha256": first["blob_sha256"]}));
            }
        }
        if !r["has_more"].as_bool().unwrap() {
            assert_eq!(r["total_bytes"].as_u64().unwrap() as usize, text.len());
            assert_eq!(
                sha256_bytes(text.as_bytes()),
                r["blob_sha256"].as_str().unwrap()
            );
            return (text, first);
        }
    }
}

#[test]
fn discover_resolve_evidence_page_exactly_and_keep_native_provenance() {
    let _serial = serial();
    let f = Fixture::new();
    let before = inventory(&f.base);
    let mut c = consumer(&f, 512, 64);
    let (found, first) = drain(&mut c, TOOL_DISCOVER, json!({"query":"assertion"}), 700);
    assert_eq!(first["applicability"], "not_assessed");
    assert!(found.contains(&f.v1.revision_id) || found.contains(&f.v2.revision_id));
    // Exact historical revision with a newer head: not replaced by latest.
    let (proj, page) = drain(&mut c, TOOL_RESOLVE, args(&f.v1), 900);
    assert_eq!(page["owner"]["revision_id"], f.v1.revision_id.as_str());
    assert_eq!(page["owner"]["current_head"], false);
    assert_eq!(
        page["owner"]["head_revision_ids"],
        json!([f.v2.revision_id])
    );
    assert!(proj.contains("Historical sentence."));
    assert!(!proj.contains("Newer sentence."));
    let listed = page["owner"]["listed_evidence_sha256"].as_array().unwrap();
    assert!(listed.iter().any(|e| e == f.evidence.as_str()));
    // Same exact bytes as resolve_handoff reports.
    let r = f.resolve(&f.v1, 4096).unwrap();
    assert_eq!(
        r.refs[0].projection_sha256,
        page["blob_sha256"].as_str().unwrap()
    );
    // Large evidence pages through the bound and reassembles to its digest.
    let (ev, evp) = drain(
        &mut c,
        TOOL_EVIDENCE,
        with(args(&f.v1), json!({"sha256":f.evidence})),
        1000,
    );
    assert!(ev.len() > 3000);
    assert_eq!(evp["owner"]["integrity_sha256_verified"], true);
    assert_eq!(sha256_bytes(ev.as_bytes()), f.evidence);
    // Currency is the owner's result, not coerced.
    let cur = c
        .call(
            TOOL_CURRENCY,
            &with(args(&f.v1), json!({"sha256":f.evidence})),
            None,
        )
        .unwrap();
    assert_eq!(cur["authority"], "native_owner_currency_result");
    assert_eq!(cur["applicability"], "not_assessed");
    assert!(cur["owner_result"].is_object());
    assert_eq!(
        before,
        inventory(&f.base),
        "reads must not mutate registry or artifacts"
    );
}

#[test]
fn boundary_failures_fail_closed_without_fallback() {
    let _serial = serial();
    let f = Fixture::new();
    let mut c = consumer(&f, 4096, 64);
    let ok = args(&f.v1);
    let code = |c: &mut ClaimsConsumer, t: &str, a: Value, u: Option<Instant>| {
        c.call(t, &a, u).unwrap_err().code()
    };
    use std::time::Instant;
    assert_eq!(
        code(&mut c, "workspace_exec", json!({}), None),
        "unknown_tool"
    );
    assert_eq!(
        code(
            &mut c,
            TOOL_RESOLVE,
            with(ok.clone(), json!({"profile":"/etc/x"})),
            None
        ),
        "invalid_arguments"
    );
    assert_eq!(
        code(
            &mut c,
            TOOL_RESOLVE,
            with(ok.clone(), json!({"server":"/bin/sh"})),
            None
        ),
        "invalid_arguments"
    );
    assert_eq!(
        code(
            &mut c,
            TOOL_DISCOVER,
            json!({"query":"x".repeat(3000)}),
            None
        ),
        "invalid_arguments"
    );
    assert_eq!(
        code(&mut c, TOOL_DISCOVER, json!({"query":"a","limit":21}), None),
        "invalid_arguments"
    );
    assert_eq!(
        code(
            &mut c,
            TOOL_RESOLVE,
            with(ok.clone(), json!({"max_bytes":8193})),
            None
        ),
        "invalid_arguments"
    );
    // Wrong revision digest: the owner refuses; the consumer does not fall back.
    let mut wrong = ok.clone();
    wrong["revision_sha256"] = json!("0".repeat(64));
    assert!(c.call(TOOL_RESOLVE, &wrong, None).is_err());
    // Missing revision id never becomes latest.
    let mut gone = ok.clone();
    gone["revision_id"] = json!("no-such-revision");
    assert!(c.call(TOOL_RESOLVE, &gone, None).is_err());
    // Unlisted evidence.
    let other = with(ok.clone(), json!({"sha256":"f".repeat(64)}));
    assert_eq!(
        code(&mut c, TOOL_EVIDENCE, other.clone(), None),
        "evidence_not_listed"
    );
    assert_eq!(
        code(&mut c, TOOL_CURRENCY, other, None),
        "evidence_not_listed"
    );
    // Expired deadline refuses before any read.
    assert_eq!(
        code(&mut c, TOOL_RESOLVE, ok.clone(), Some(Instant::now())),
        "deadline_expired"
    );
    // Offset outside content.
    assert_eq!(
        code(
            &mut c,
            TOOL_RESOLVE,
            with(ok.clone(), json!({"offset":99_000_000})),
            None
        ),
        "invalid_arguments"
    );
}

#[test]
fn call_budget_is_finite_and_untrusted_pins_refuse_before_spawn() {
    let _serial = serial();
    let f = Fixture::new();
    let mut c = consumer(&f, 4096, 2);
    c.call(TOOL_RESOLVE, &args(&f.v1), None).unwrap();
    c.call(TOOL_RESOLVE, &args(&f.v1), None).unwrap();
    assert_eq!(
        c.call(TOOL_RESOLVE, &args(&f.v1), None).unwrap_err().code(),
        "budget_exhausted"
    );
    assert_eq!(c.counts().calls, 2);
    let mut bad = f.server.clone();
    bad.executable_sha256 = "0".repeat(64);
    assert_eq!(
        ClaimsConsumer::open(&bad, &Options::default(), 2)
            .err()
            .unwrap()
            .code(),
        "server_untrusted"
    );
    let mut bad = f.server.clone();
    bad.read_profile_sha256 = "0".repeat(64);
    assert_eq!(
        ClaimsConsumer::open(&bad, &Options::default(), 2)
            .err()
            .unwrap()
            .code(),
        "profile_untrusted"
    );
}

#[test]
fn reliance_decision_is_explicit_exact_canonical_and_grants_nothing() {
    let f = Fixture::new();
    let d = RelianceDecision::new(
        &f.v1,
        "needs_more_evidence",
        "newer head exists; evidence not re-read",
    )
    .unwrap();
    let line = d.line().unwrap();
    assert!(line.contains("\"grant\":\"none\""));
    assert!(line.contains("\"authoritative_reliance_record\":\"not_written\""));
    let text = format!("done reading\n{line}\nbye");
    let parsed = RelianceDecision::extract(&text).unwrap();
    assert_eq!(parsed, d);
    parsed.check_subject(std::slice::from_ref(&f.v1)).unwrap();
    assert!(
        parsed.check_subject(std::slice::from_ref(&f.v2)).is_err(),
        "subject must be an exact selected revision"
    );
    // Reading is not a decision; two decisions are ambiguous; tampering refuses.
    assert!(RelianceDecision::extract("I read it and it looks fine").is_err());
    assert!(RelianceDecision::extract(&format!("{line}\n{line}")).is_err());
    let body = line.strip_prefix(DECISION_MARKER).unwrap();
    assert!(
        RelianceDecision::parse(&body.replace("needs_more_evidence", "relied_forever")).is_err()
    );
    assert!(RelianceDecision::parse(&body.replace("\"none\"", "\"all\"")).is_err());
    assert!(RelianceDecision::new(&f.v1, "relied_for_task", "").is_err());
}
