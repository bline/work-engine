use claim_evidence::{OpenFiles, RepositoryReadStartup, RepositoryReader, sha256_bytes};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
struct Fixture {
    _temp: tempfile::TempDir,
    base: PathBuf,
    root: PathBuf,
    startup: PathBuf,
    sha: String,
}
impl Fixture {
    fn new(repository: &str) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().to_path_buf();
        let root = base.join("root");
        let source = base.join("source.json");
        let config = base.join("config.json");
        let startup = base.join("startup.json");
        let source_value = json!({"schema_version":3,"owner":"trusted-fixture-issuer","reference":"fixture:authority","revision":"1","freshness":"pinned","profile":"repository-claims-v1","repository_id":repository,"actors":["producer"],"decision_scopes":["scope"],"grant_ids":["grant"]});
        fs::write(&source, serde_json::to_vec(&source_value).unwrap()).unwrap();
        let source_sha = sha256_bytes(&fs::read(&source).unwrap());
        let config_value = json!({"schema_version":3,"profile":"repository-claims-v1","root_id":"fixture-root","repository_id":repository,"grants":[{"schema_version":1,"grant_id":"grant","actor":"producer","profile":"repository-fact-v1","permissions":["create_claim","publish_revision","record_reliance"],"decision_scope":"scope","authority_reference":{"owner":"trusted-fixture-issuer","reference":"fixture:authority","revision":"1","freshness":"pinned","integrity_sha256":source_sha,"status":"verified"}}]});
        fs::write(&config, serde_json::to_vec(&config_value).unwrap()).unwrap();
        let config_sha = sha256_bytes(&fs::read(&config).unwrap());
        let startup_value = json!({"schema_version":1,"root":root,"config":config,"source":source,"expected_config_sha256":config_sha,"expected_source_sha256":source_sha,"repository_id":repository,"decision_scope":"scope","actor":"producer","grant_id":"grant","allowed_actions":["create_claim","publish_revision","record_reliance"]});
        fs::write(&startup, serde_json::to_vec(&startup_value).unwrap()).unwrap();
        let sha = sha256_bytes(&fs::read(&startup).unwrap());
        Self {
            _temp: temp,
            base,
            root,
            startup,
            sha,
        }
    }
    fn call(&self, action: &str, request: Option<&Value>) -> Result<Value, String> {
        call(&self.startup, &self.sha, action, request)
    }
    fn init(&self) {
        assert_eq!(
            self.call("initialize", None).unwrap()["status"],
            "initialized"
        );
    }
    fn reader(&self) -> RepositoryReader {
        let v: Value = serde_json::from_slice(&fs::read(&self.startup).unwrap()).unwrap();
        RepositoryReader::open(RepositoryReadStartup {
            files: OpenFiles {
                root: self.root.clone(),
                config: PathBuf::from(v["config"].as_str().unwrap()),
                source: PathBuf::from(v["source"].as_str().unwrap()),
                expected_config_sha256: v["expected_config_sha256"].as_str().unwrap().into(),
                expected_source_sha256: v["expected_source_sha256"].as_str().unwrap().into(),
            },
            repository_id: v["repository_id"].as_str().unwrap().into(),
            decision_scope: "scope".into(),
        })
        .unwrap()
    }
}
fn call(startup: &Path, sha: &str, action: &str, request: Option<&Value>) -> Result<Value, String> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_work-engine-claims-host"))
        .arg(action)
        .arg(startup)
        .arg(sha)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    if let Some(request) = request {
        child
            .stdin
            .take()
            .unwrap()
            .write_all(&serde_json::to_vec(request).unwrap())
            .unwrap();
    }
    let output = child.wait_with_output().unwrap();
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into());
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("{e}: {}", String::from_utf8_lossy(&output.stdout)))
}
fn reference() -> Value {
    json!({"owner":"capture","reference":"fixture:evidence","revision":"1","integrity_sha256":"0".repeat(64),"freshness":"exact","status":"unavailable"})
}
fn payload(repo: &str, key: &str) -> Value {
    json!({"proposition":"The source declares this bounded assertion.","support_qualification":"attributed fixture judgment","assumptions":[],"limitations":["Synthetic source; unobserved dependencies remain unknown."],"confidence":{"label":"bounded"},"evidence_references":[reference()],"sensitivity_references":[],"evidence_mode":"capture","judgment_kind":"repository_fact","decision_scope":"scope","profile_payload":{"repository_id":repo,"assertion_key":key,"assertion_scope":"bounded assertion about symbol"},"reopening_conditions":["new source evidence"],"tombstone":false})
}
fn create(repo: &str, key: &str, op: &str) -> Value {
    json!({"schema_version":1,"operation_id":op,"action":"create_claim","profile":"repository-fact-v1","expected_state":null,"payload":{"subject":{"namespace":format!("repository-fact:{repo}"),"subject_kind":"repository_fact","stable_subject_id":key,"evidence_baseline":reference(),"content_set":["src/symbol.rs"]},"statement_identity":key,"initial_revision":payload(repo,key)}})
}
fn revise(create: &Value, receipt: &Value, op: &str, proposition: &str) -> Value {
    let mut payload = create["payload"]["initial_revision"].clone();
    payload["proposition"] = json!(proposition);
    json!({"schema_version":1,"operation_id":op,"action":"publish_revision","profile":"repository-fact-v1","expected_state":receipt["result"]["record"]["id"],"payload":{"claim_id":receipt["result"]["record"]["claim_id"],"revision":payload}})
}
fn resolve(receipt: &Value) -> Value {
    json!({"action":"resolve","claim_id":receipt["result"]["record"]["claim_id"],"revision_id":receipt["result"]["record"]["id"],"revision_sha256":receipt["result"]["sha256"]})
}
fn snapshot(path: &Path) -> BTreeMap<PathBuf, (u32, u64, u64, Vec<u8>)> {
    fn visit(base: &Path, path: &Path, out: &mut BTreeMap<PathBuf, (u32, u64, u64, Vec<u8>)>) {
        let m = fs::symlink_metadata(path).unwrap();
        out.insert(
            path.strip_prefix(base).unwrap().into(),
            (
                m.permissions().mode(),
                m.dev(),
                m.ino(),
                if m.is_file() {
                    fs::read(path).unwrap()
                } else {
                    Vec::new()
                },
            ),
        );
        if m.is_dir() {
            for entry in fs::read_dir(path).unwrap() {
                visit(base, &entry.unwrap().path(), out);
            }
        }
    }
    let mut out = BTreeMap::new();
    visit(path, path, &mut out);
    out
}
#[test]
fn actual_host_fresh_process_revision_history_reliance_and_replay() {
    let f = Fixture::new("repo-a");
    f.init();
    let request = create("repo-a", "assertion-a", "create-a");
    let first = f.call("execute", Some(&request)).unwrap();
    assert_eq!(first["status"], "applied", "{first}");
    assert_eq!(
        f.call("execute", Some(&request)).unwrap()["status"],
        "replayed"
    );
    let mut changed = request.clone();
    changed["payload"]["initial_revision"]["proposition"] = json!("changed");
    assert_eq!(
        f.call("execute", Some(&changed)).unwrap()["status"],
        "no_effect"
    );
    let next = revise(&request, &first, "revise-a", "new attributed judgment");
    let second = f.call("execute", Some(&next)).unwrap();
    assert_eq!(second["status"], "applied", "{second}");
    assert_eq!(
        f.call("execute", Some(&request)).unwrap()["status"],
        "replayed"
    );
    let old = f.call("read", Some(&resolve(&first))).unwrap();
    assert_eq!(old["items"][0]["current_head"], false);
    assert_eq!(
        old["items"][0]["unresolved_references"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let reliance = json!({"schema_version":1,"operation_id":"rely-old","action":"record_reliance","profile":"repository-fact-v1","expected_state":null,"payload":{"consumer":"second-session","consumer_revision":"turn-2","decision_scope":"scope","claim_revision_id":first["result"]["record"]["id"],"claim_revision_sha256":first["result"]["sha256"],"state":"active","predecessor_reliance":null}});
    let relied = f.call("execute", Some(&reliance)).unwrap();
    assert_eq!(relied["status"], "applied", "{relied}");
    assert_eq!(
        f.call("execute", Some(&reliance)).unwrap()["status"],
        "replayed"
    );
    let old = f.call("read", Some(&resolve(&first))).unwrap();
    assert_eq!(
        old["items"][0]["reliances"][0]["attributed_producer"],
        "producer"
    );
    assert_eq!(
        f.call("inspect", Some(&reliance)).unwrap()["status"],
        "committed"
    );
    let conflicting = revise(&request, &first, "competing", "competing");
    assert_eq!(
        f.call("execute", Some(&conflicting)).unwrap()["status"],
        "no_effect"
    );
    let mut immutable = revise(&request, &second, "scope-change", "scope drift");
    immutable["payload"]["revision"]["profile_payload"]["assertion_scope"] = json!("new scope");
    assert_eq!(
        f.call("execute", Some(&immutable)).unwrap()["status"],
        "no_effect"
    );
    let before = snapshot(&f.base);
    let reader = f.reader();
    let item = &first["result"]["record"];
    for _ in 0..3 {
        reader
            .resolve(
                item["claim_id"].as_str().unwrap(),
                item["id"].as_str().unwrap(),
                first["result"]["sha256"].as_str().unwrap(),
            )
            .unwrap();
        reader.discover("", 1, None).unwrap();
    }
    drop(reader);
    assert_eq!(before, snapshot(&f.base));
}
#[test]
fn identities_scope_and_capabilities_are_not_client_selected() {
    let a = Fixture::new("repo-a");
    let b = Fixture::new("repo-b");
    a.init();
    b.init();
    let ra = a
        .call("execute", Some(&create("repo-a", "assertion-a", "a")))
        .unwrap();
    let rb = b
        .call("execute", Some(&create("repo-b", "assertion-a", "a")))
        .unwrap();
    let rc = a
        .call("execute", Some(&create("repo-a", "assertion-b", "b")))
        .unwrap();
    assert_ne!(
        ra["result"]["record"]["claim_id"],
        rb["result"]["record"]["claim_id"]
    );
    assert_ne!(
        ra["result"]["record"]["claim_id"],
        rc["result"]["record"]["claim_id"]
    );
    for key in [
        "root",
        "actor",
        "grant_id",
        "authority_file",
        "decision_scope",
    ] {
        let mut request = create("repo-a", "forged", key);
        request[key] = json!("attacker");
        assert_eq!(
            a.call("execute", Some(&request)).unwrap()["status"],
            "no_effect"
        );
    }
    for (path, value) in [("repository_id", "other"), ("assertion_key", "other")] {
        let mut request = create("repo-a", "assertion-forge", path);
        request["payload"]["initial_revision"]["profile_payload"][path] = json!(value);
        assert_eq!(
            a.call("execute", Some(&request)).unwrap()["status"],
            "no_effect"
        );
    }
    let mut request = create("repo-a", "scope-forge", "wrong-scope");
    request["payload"]["initial_revision"]["decision_scope"] = json!("other");
    assert_eq!(
        a.call("execute", Some(&request)).unwrap()["status"],
        "no_effect"
    );
    let mut request = create("repo-a", "profile-forge", "wrong-profile");
    request["profile"] = json!("revision-bound-review-finding-v1");
    assert_eq!(
        a.call("execute", Some(&request)).unwrap()["status"],
        "no_effect"
    );
    // Cursor binds query and snapshot; pages preserve explicit scope.
    let reader = a.reader();
    let page = reader.discover("", 1, None).unwrap();
    let cursor = claim_evidence::codec::field(&page, "next_cursor").unwrap();
    assert_ne!(cursor, &claim_evidence::codec::JsValue::Null);
    reader.discover("", 1, Some(cursor)).unwrap();
    assert!(reader.discover("changed", 1, Some(cursor)).is_err());
    a.call("execute", Some(&create("repo-a", "assertion-c", "c")))
        .unwrap();
    assert!(reader.discover("", 1, Some(cursor)).is_err());
    // Pinned source changes are rejected, without grant auto-admission.
    fs::write(a.base.join("source.json"), b"{}").unwrap();
    assert!(
        a.call("execute", Some(&create("repo-a", "x", "d")))
            .is_err()
    );
}
#[test]
fn reader_missing_invalid_wal_recovery_and_readable_modes_never_mutate() {
    let f = Fixture::new("repo-a");
    let before = snapshot(&f.base);
    assert!(
        f.call(
            "read",
            Some(&json!({"action":"discover","query":"","limit":1,"cursor":null}))
        )
        .is_err()
    );
    assert_eq!(before, snapshot(&f.base));
    f.init();
    let req = create("repo-a", "assertion-a", "create");
    let receipt = f.call("execute", Some(&req)).unwrap();
    fs::set_permissions(&f.root, fs::Permissions::from_mode(0o755)).unwrap();
    for e in fs::read_dir(&f.root).unwrap() {
        fs::set_permissions(e.unwrap().path(), fs::Permissions::from_mode(0o644)).unwrap();
    }
    let before = snapshot(&f.base);
    f.call("read", Some(&resolve(&receipt))).unwrap();
    assert_eq!(before, snapshot(&f.base));
    for suffix in ["-wal", "-shm", "-journal"] {
        let p = f.root.join(format!("claim-evidence.sqlite3{suffix}"));
        fs::write(&p, b"recovery marker").unwrap();
        let before = snapshot(&f.base);
        assert!(f.call("read", Some(&resolve(&receipt))).is_err());
        assert_eq!(before, snapshot(&f.base));
        fs::remove_file(p).unwrap();
    }
    let db = f.root.join("claim-evidence.sqlite3");
    let original = fs::read(&db).unwrap();
    let mut bytes = original.clone();
    bytes[18] = 2;
    bytes[19] = 2;
    fs::write(&db, &bytes).unwrap();
    let before = snapshot(&f.base);
    assert!(f.call("read", Some(&resolve(&receipt))).is_err());
    assert_eq!(before, snapshot(&f.base));
    fs::write(&db, &original).unwrap();
    let marker = f.root.join("claim-evidence-root.json");
    let original = fs::read(&marker).unwrap();
    fs::write(&marker, b"{}").unwrap();
    let before = snapshot(&f.base);
    assert!(f.call("read", Some(&resolve(&receipt))).is_err());
    assert_eq!(before, snapshot(&f.base));
    fs::write(&marker, original).unwrap();
}
#[test]
fn concurrent_successors_have_one_winner_and_readers_are_coherent_or_fail() {
    let f = Fixture::new("repo-a");
    f.init();
    let request = create("repo-a", "assertion-a", "create");
    let first = f.call("execute", Some(&request)).unwrap();
    let one = revise(&request, &first, "revise-one", "one");
    let two = revise(&request, &first, "revise-two", "two");
    let (r1, r2) = std::thread::scope(|s| {
        let a = s.spawn(|| f.call("execute", Some(&one)));
        let b = s.spawn(|| f.call("execute", Some(&two)));
        for _ in 0..10 {
            if let Ok(read) = f.call("read", Some(&resolve(&first))) {
                let item = &read["items"][0];
                assert_eq!(item["revision_sha256"], first["result"]["sha256"]);
                assert_eq!(item["head_revision_ids"].as_array().unwrap().len(), 1);
            }
        }
        (a.join().unwrap(), b.join().unwrap())
    });
    let applied = [&r1, &r2]
        .iter()
        .filter(|v| matches!(v,Ok(value) if value["status"]=="applied"))
        .count();
    assert_eq!(applied, 1, "{r1:?} {r2:?}");
    let read = f.call("read", Some(&resolve(&first))).unwrap();
    assert_eq!(read["items"][0]["current_head"], false);
}
#[test]
fn wrong_trusted_selection_refuses_before_initialization() {
    for key in ["actor", "grant_id", "decision_scope", "repository_id"] {
        let f = Fixture::new("repo-a");
        let mut startup: Value = serde_json::from_slice(&fs::read(&f.startup).unwrap()).unwrap();
        startup[key] = json!("not-admitted");
        let bytes = serde_json::to_vec(&startup).unwrap();
        fs::write(&f.startup, &bytes).unwrap();
        let sha = sha256_bytes(&bytes);
        let before = snapshot(&f.base);
        assert!(call(&f.startup, &sha, "initialize", None).is_err());
        assert_eq!(before, snapshot(&f.base));
        assert!(!f.root.exists());
    }
}
#[test]
fn native_review_root_remains_restricted_and_repository_reader_rejects_it() {
    struct Port;
    struct Lease;
    impl claim_evidence::FindingLease for Lease {}
    impl claim_evidence::FindingAdmissionPort for Port {
        fn acquire<'a>(
            &'a self,
            _: &claim_evidence::AdmissionBinding,
        ) -> claim_evidence::ClaimResult<Box<dyn claim_evidence::FindingLease + 'a>> {
            Ok(Box::new(Lease))
        }
    }
    let f = Fixture::new("repo-a");
    let mut source: Value =
        serde_json::from_slice(&fs::read(f.base.join("source.json")).unwrap()).unwrap();
    source["schema_version"] = json!(1);
    source["profile"] = json!("native-review-claims-v1");
    source.as_object_mut().unwrap().remove("repository_id");
    let bytes = serde_json::to_vec(&source).unwrap();
    fs::write(f.base.join("source.json"), &bytes).unwrap();
    let source_sha = sha256_bytes(&bytes);
    let mut config: Value =
        serde_json::from_slice(&fs::read(f.base.join("config.json")).unwrap()).unwrap();
    config["schema_version"] = json!(1);
    config["profile"] = json!("native-review-claims-v1");
    config.as_object_mut().unwrap().remove("repository_id");
    config["grants"][0]["profile"] = json!("revision-bound-review-finding-v1");
    config["grants"][0]["authority_reference"]["integrity_sha256"] = json!(source_sha);
    let bytes = serde_json::to_vec(&config).unwrap();
    fs::write(f.base.join("config.json"), &bytes).unwrap();
    let config_sha = sha256_bytes(&bytes);
    let files = OpenFiles {
        root: f.root.clone(),
        config: f.base.join("config.json"),
        source: f.base.join("source.json"),
        expected_config_sha256: config_sha,
        expected_source_sha256: source_sha,
    };
    let legacy = claim_evidence::ClaimsApplication::initialize(
        claim_evidence::BootstrapFiles {
            root: files.root.clone(),
            config: files.config.clone(),
            source: files.source.clone(),
            expected_config_sha256: files.expected_config_sha256.clone(),
            expected_source_sha256: files.expected_source_sha256.clone(),
        },
        Port,
    )
    .unwrap();
    drop(legacy);
    let before = snapshot(&f.base);
    assert!(
        RepositoryReader::open(RepositoryReadStartup {
            files,
            repository_id: "repo-a".into(),
            decision_scope: "scope".into()
        })
        .is_err()
    );
    assert_eq!(before, snapshot(&f.base));
    let repo = Fixture::new("repo-a");
    repo.init();
    let v: Value = serde_json::from_slice(&fs::read(&repo.startup).unwrap()).unwrap();
    let files = OpenFiles {
        root: repo.root.clone(),
        config: repo.base.join("config.json"),
        source: repo.base.join("source.json"),
        expected_config_sha256: v["expected_config_sha256"].as_str().unwrap().into(),
        expected_source_sha256: v["expected_source_sha256"].as_str().unwrap().into(),
    };
    let before = snapshot(&repo.base);
    assert!(claim_evidence::ClaimsApplication::open(files, Port).is_err());
    assert_eq!(before, snapshot(&repo.base));
}
#[cfg(feature = "test-faults")]
fn paused_process(f: &Fixture, request: &Value, cut: &str) -> (std::process::Child, PathBuf) {
    let signals = f.base.join(format!("fault-{cut}"));
    fs::create_dir(&signals).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_work-engine-claims-host"))
        .arg("execute")
        .arg(&f.startup)
        .arg(&f.sha)
        .env("CLAIM_EVIDENCE_FAULT_CUT", cut)
        .env("CLAIM_EVIDENCE_FAULT_DIR", &signals)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(request).unwrap())
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !signals.join(format!("{cut}.ready")).exists() {
        if let Some(status) = child.try_wait().unwrap() {
            panic!("child exited early {status}");
        }
        assert!(
            std::time::Instant::now() < deadline,
            "fault boundary timeout"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    (child, signals)
}
#[cfg(feature = "test-faults")]
#[test]
fn actual_host_crash_reconciles_entered_unknown_without_blind_retry() {
    for (cut, expected) in [
        ("repository_before_commit", "no_effect"),
        ("repository_after_commit", "committed"),
    ] {
        let f = Fixture::new("repo-a");
        f.init();
        let request = create("repo-a", "assertion-a", "create");
        let (mut child, _) = paused_process(&f, &request, cut);
        child.kill().unwrap();
        child.wait().unwrap();
        // A pre-commit crash leaves an entered locator and possibly a hot journal.
        // The read-only facade never performs recovery; the admitted host may.
        if cut == "repository_before_commit" {
            let before = snapshot(&f.base);
            let _ = f.call(
                "read",
                Some(&json!({"action":"discover","query":"","limit":1,"cursor":null})),
            );
            assert_eq!(before, snapshot(&f.base));
        }
        let repeated = f.call("execute", Some(&request)).unwrap();
        assert!(
            repeated["status"] == "outcome_unknown" || repeated["status"] == "committed",
            "{repeated}"
        );
        let reconciled = f.call("inspect", Some(&request)).unwrap();
        assert_eq!(reconciled["status"], expected, "{reconciled}");
        let repeated = f.call("execute", Some(&request)).unwrap();
        assert_eq!(
            repeated["status"],
            if cut == "repository_before_commit" {
                "applied"
            } else {
                "replayed"
            },
            "{repeated}"
        );
    }
}
#[cfg(feature = "test-faults")]
#[test]
fn lease_retains_trusted_startup_binding_through_commit() {
    let f = Fixture::new("repo-a");
    f.init();
    let request = create("repo-a", "assertion-a", "create");
    let (child, signals) = paused_process(&f, &request, "repository_before_commit");
    let original = fs::read(&f.startup).unwrap();
    let mut changed: Value = serde_json::from_slice(&original).unwrap();
    changed["actor"] = json!("attacker");
    fs::write(&f.startup, serde_json::to_vec(&changed).unwrap()).unwrap();
    fs::write(signals.join("repository_before_commit.release"), b"release").unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["status"], "no_effect", "{result}");
    fs::write(&f.startup, original).unwrap();
    let discovered = f
        .call(
            "read",
            Some(&json!({"action":"discover","query":"","limit":100,"cursor":null})),
        )
        .unwrap();
    assert_eq!(discovered["items"].as_array().unwrap().len(), 0);
}
#[test]
fn canonical_admission_tamper_is_rejected_without_reader_repairs() {
    let f = Fixture::new("repo-a");
    f.init();
    let request = create("repo-a", "assertion-a", "create");
    let first = f.call("execute", Some(&request)).unwrap();
    let output = f.call("read", Some(&resolve(&first))).unwrap();
    let admitted = &output["items"][0]["admitted_operation"];
    assert_eq!(admitted["binding"]["repository_id"], "repo-a");
    assert_eq!(admitted["request"], request);
    let db = rusqlite::Connection::open(f.root.join("claim-evidence.sqlite3")).unwrap();
    db.execute(
        "UPDATE repository_admission_bindings SET binding_json='{}'",
        [],
    )
    .unwrap();
    drop(db);
    let before = snapshot(&f.base);
    assert!(f.call("read", Some(&resolve(&first))).is_err());
    assert_eq!(before, snapshot(&f.base));
}
#[test]
fn hot_journal_child() {
    let Ok(path) = std::env::var("RF1_HOT_JOURNAL_DATABASE") else {
        return;
    };
    let ready = std::env::var("RF1_HOT_JOURNAL_READY").unwrap();
    let conn = rusqlite::Connection::open(path).unwrap();
    conn.execute_batch("PRAGMA cache_size=1; PRAGMA synchronous=FULL; BEGIN IMMEDIATE; CREATE TABLE uncommitted_spill(payload BLOB); INSERT INTO uncommitted_spill VALUES(zeroblob(262144));").unwrap();
    fs::write(ready, b"ready").unwrap();
    std::thread::sleep(std::time::Duration::from_secs(60));
    panic!("parent failed to kill hot journal fixture");
}
#[test]
fn actual_hot_journal_is_refused_without_mutation_then_writer_recovers() {
    let f = Fixture::new("repo-a");
    f.init();
    let request = create("repo-a", "assertion-a", "create");
    let first = f.call("execute", Some(&request)).unwrap();
    let ready = f.base.join("hot.ready");
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "hot_journal_child"])
        .env(
            "RF1_HOT_JOURNAL_DATABASE",
            f.root.join("claim-evidence.sqlite3"),
        )
        .env("RF1_HOT_JOURNAL_READY", &ready)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !ready.exists() {
        if let Some(status) = child.try_wait().unwrap() {
            panic!("hot journal child exited early {status}");
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    child.kill().unwrap();
    child.wait().unwrap();
    let journal = fs::read(f.root.join("claim-evidence.sqlite3-journal")).unwrap();
    assert_eq!(
        &journal[..8],
        &[0xd9, 0xd5, 0x05, 0xf9, 0x20, 0xa1, 0x63, 0xd7]
    );
    let before = snapshot(&f.base);
    assert!(f.call("read", Some(&resolve(&first))).is_err());
    assert_eq!(before, snapshot(&f.base));
    let inspected = f.call("inspect", Some(&request)).unwrap();
    assert_eq!(inspected["status"], "committed", "{inspected}");
    assert!(!f.root.join("claim-evidence.sqlite3-journal").exists());
    let before = snapshot(&f.base);
    f.call("read", Some(&resolve(&first))).unwrap();
    assert_eq!(before, snapshot(&f.base));
}

fn rewrite_startup(f: &mut Fixture, startup: &Value) {
    let bytes = serde_json::to_vec(startup).unwrap();
    fs::write(&f.startup, &bytes).unwrap();
    f.sha = sha256_bytes(&bytes);
}

#[test]
fn successor_cannot_cross_another_grants_decision_scope() {
    let mut f = Fixture::new("repo-a");
    let source_path = f.base.join("source.json");
    let config_path = f.base.join("config.json");
    let mut source: Value = serde_json::from_slice(&fs::read(&source_path).unwrap()).unwrap();
    source["actors"] = json!(["producer", "producer-b"]);
    source["decision_scopes"] = json!(["scope", "scope-b"]);
    source["grant_ids"] = json!(["grant", "grant-b"]);
    fs::write(&source_path, serde_json::to_vec(&source).unwrap()).unwrap();
    let source_sha = sha256_bytes(&fs::read(&source_path).unwrap());
    let mut config: Value = serde_json::from_slice(&fs::read(&config_path).unwrap()).unwrap();
    config["grants"][0]["authority_reference"]["integrity_sha256"] = json!(source_sha);
    let mut grant_b = config["grants"][0].clone();
    grant_b["grant_id"] = json!("grant-b");
    grant_b["actor"] = json!("producer-b");
    grant_b["decision_scope"] = json!("scope-b");
    config["grants"].as_array_mut().unwrap().push(grant_b);
    fs::write(&config_path, serde_json::to_vec(&config).unwrap()).unwrap();
    let mut startup: Value = serde_json::from_slice(&fs::read(&f.startup).unwrap()).unwrap();
    startup["expected_source_sha256"] = json!(source_sha);
    startup["expected_config_sha256"] = json!(sha256_bytes(&fs::read(&config_path).unwrap()));
    rewrite_startup(&mut f, &startup);
    let mut startup_b = startup.clone();
    startup_b["actor"] = json!("producer-b");
    startup_b["grant_id"] = json!("grant-b");
    startup_b["decision_scope"] = json!("scope-b");
    let b_path = f.base.join("startup-b.json");
    let b_bytes = serde_json::to_vec(&startup_b).unwrap();
    fs::write(&b_path, &b_bytes).unwrap();
    let b_sha = sha256_bytes(&b_bytes);
    f.init();
    let request = create("repo-a", "assertion-a", "scope-a-create");
    let first = f.call("execute", Some(&request)).unwrap();
    assert_eq!(first["status"], "applied", "{first}");
    let before = reconstructed_input(&f);
    let mut cross = revise(&request, &first, "scope-b-cross", "B attempts to replace A");
    cross["payload"]["revision"]["decision_scope"] = json!("scope-b");
    let refused = call(&b_path, &b_sha, "execute", Some(&cross)).unwrap();
    assert_eq!(refused["status"], "no_effect", "{refused}");
    assert!(
        refused["error"]
            .as_str()
            .unwrap()
            .contains("predecessor decision scope")
    );
    assert_eq!(
        before,
        reconstructed_input(&f),
        "rejected successor changed canonical input"
    );
    let discovered = f
        .call(
            "read",
            Some(&json!({"action":"discover","query":"","limit":100,"cursor":null})),
        )
        .unwrap();
    assert_eq!(discovered["items"].as_array().unwrap().len(), 1);
    assert_eq!(
        discovered["items"][0]["revision"]["id"],
        first["result"]["record"]["id"]
    );
    assert_eq!(discovered["items"][0]["current_head"], true);
    let authorized = f
        .call(
            "execute",
            Some(&revise(
                &request,
                &first,
                "scope-a-successor",
                "Authorized A successor",
            )),
        )
        .unwrap();
    assert_eq!(authorized["status"], "applied", "{authorized}");
    let resolved = f.call("read", Some(&resolve(&authorized))).unwrap();
    assert_eq!(resolved["items"][0]["current_head"], true);
}

/// Reconstruct the documented input from SQL and pinned bootstrap bytes in a
/// separate coherent read, using the canonical owner's codec rather than serde hashing.
fn reconstructed_input(f: &Fixture) -> (String, String) {
    use claim_evidence::codec::{JsValue, digest, parse_json};
    let startup: Value = serde_json::from_slice(&fs::read(&f.startup).unwrap()).unwrap();
    let config: Value =
        serde_json::from_slice(&fs::read(f.base.join("config.json")).unwrap()).unwrap();
    let conn = rusqlite::Connection::open_with_flags(
        f.root.join("claim-evidence.sqlite3"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    conn.execute_batch("BEGIN").unwrap();
    let state: String = conn
        .query_row(
            "SELECT store_json FROM canonical_claim_state WHERE singleton=1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let state = parse_json(&state).unwrap();
    let mut statement = conn.prepare("SELECT operation_id,binding_json,request_json FROM repository_admission_bindings ORDER BY operation_id").unwrap();
    let rows = statement
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })
        .unwrap();
    let admissions = rows
        .map(|r| {
            let (id, binding, request) = r.unwrap();
            JsValue::object([
                ("operation_id", JsValue::text(&id)),
                ("binding", parse_json(&binding).unwrap()),
                ("request", parse_json(&request).unwrap()),
            ])
        })
        .collect();
    let full = JsValue::object([
        (
            "root_id",
            JsValue::text(config["root_id"].as_str().unwrap()),
        ),
        ("profile", JsValue::text("repository-claims-v1")),
        (
            "bootstrap_config_sha256",
            JsValue::text(startup["expected_config_sha256"].as_str().unwrap()),
        ),
        (
            "bootstrap_source_sha256",
            JsValue::text(startup["expected_source_sha256"].as_str().unwrap()),
        ),
        ("canonical_state", state.clone()),
        ("admissions", JsValue::Array(admissions)),
    ]);
    let result = (digest(&full).unwrap(), digest(&state).unwrap());
    conn.execute_batch("ROLLBACK").unwrap();
    result
}

#[test]
fn projection_input_digest_binds_cursor_admissions_and_distinct_root_identity() {
    let a = Fixture::new("repo-a");
    let mut b = Fixture::new("repo-a");
    let config_path = b.base.join("config.json");
    let mut config: Value = serde_json::from_slice(&fs::read(&config_path).unwrap()).unwrap();
    config["root_id"] = json!("distinct-root");
    fs::write(&config_path, serde_json::to_vec(&config).unwrap()).unwrap();
    let mut startup: Value = serde_json::from_slice(&fs::read(&b.startup).unwrap()).unwrap();
    startup["expected_config_sha256"] = json!(sha256_bytes(&fs::read(&config_path).unwrap()));
    rewrite_startup(&mut b, &startup);
    a.init();
    b.init();
    let mut inputs = Vec::new();
    for f in [&a, &b] {
        let first = f
            .call(
                "execute",
                Some(&create("repo-a", "assertion-a", "create-a")),
            )
            .unwrap();
        assert_eq!(first["status"], "applied");
        let second = f
            .call(
                "execute",
                Some(&create("repo-a", "assertion-b", "create-b")),
            )
            .unwrap();
        assert_eq!(second["status"], "applied");
        let page = f
            .call(
                "read",
                Some(&json!({"action":"discover","query":"","limit":1,"cursor":null})),
            )
            .unwrap();
        let resolved = f.call("read", Some(&resolve(&first))).unwrap();
        let (full, state) = reconstructed_input(f);
        assert_eq!(page["projection"]["canonical_input_sha256"], full);
        assert_eq!(page["next_cursor"]["canonical_input_sha256"], full);
        assert_eq!(page["projection"]["canonical_state_sha256"], state);
        assert_eq!(resolved["projection"]["canonical_input_sha256"], full);
        assert_eq!(resolved["projection"]["canonical_state_sha256"], state);
        assert_ne!(full, state);
        inputs.push((full, state));
    }
    assert_eq!(
        inputs[0].1, inputs[1].1,
        "canonical states should be byte-identical across the two roots"
    );
    assert_ne!(
        inputs[0].0, inputs[1].0,
        "full input must retain distinct root/bootstrap/admission provenance"
    );
}
