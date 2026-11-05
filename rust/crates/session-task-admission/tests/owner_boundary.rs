//! Actual private Git/SQLite fixture; no provider or native work.
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as b64};
use p256::{
    ecdsa::{Signature, SigningKey, signature::Signer},
    pkcs8::EncodePublicKey,
};
use serde_json::{Value, json};
use session_task_admission::{Ref, owner_boundary::*};
use sha2::{Digest, Sha256};
use std::{
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};
use time::macros::datetime;
fn hash(b: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(b.as_ref()))
}
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture {
    root: PathBuf,
    config: SyntheticConfig,
    raw: String,
    proposal: Ref,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
impl Fixture {
    fn new() -> Self {
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(format!(
            "owner-boundary-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir(&root).unwrap();
        assert!(
            Command::new("git")
                .args(["init", "-q"])
                .arg(&root)
                .status()
                .unwrap()
                .success()
        );
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .unwrap();
        let script = repo.join("skills/durable-state/scripts/codex_chatboard.py");
        let pin = |path: PathBuf| Pin {
            sha256: hash(std::fs::read(&path).unwrap()),
            path,
        };
        let program = Program {
            python: pin(PathBuf::from("/usr/bin/python3")),
            script: pin(script.clone()),
            dependencies: vec![
                pin(script.parent().unwrap().join("durable_state.py")),
                pin(script
                    .parent()
                    .unwrap()
                    .join("../contracts/codex-chatboard-kinds.v1.json")),
            ],
        };
        let fixture: Value = serde_json::from_str(include_str!("projection-fixture.json")).unwrap();
        let mut row: Value =
            serde_json::from_str(fixture["tasks"][0]["request_json"].as_str().unwrap()).unwrap();
        row["data"]["ask"] = "Synthetic owner decision".into();
        row["data"]["intent"]["objective"] = "Unicode café and λ".into();
        let data = root.join("data.json");
        std::fs::write(&data, serde_json::to_vec(&row["data"]).unwrap()).unwrap();
        let output = Command::new("/usr/bin/python3")
            .arg("-I")
            .arg(&script)
            .arg("--repository")
            .arg(&root)
            .args([
                "post",
                "--author",
                "fixture-actor",
                "--session-id",
                "11111111-1111-4111-8111-111111111111",
                "--topic",
                "fixture:task",
                "--body",
                "Synthetic only",
                "--kind",
                "request",
                "--to",
                "role:owner",
                "--message-id",
                "22222222-2222-4222-8222-222222222222",
                "--data-file",
            ])
            .arg(data)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let output = Command::new("/usr/bin/python3")
            .arg("-I")
            .arg(&script)
            .arg("--repository")
            .arg(&root)
            .args([
                "owner-boundary-io",
                "--command-json",
                r#"{"operation":"snapshot"}"#,
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let snap: Value = serde_json::from_slice(&output.stdout).unwrap();
        let raw = snap["rows"][0].as_str().unwrap().to_owned();
        let proposal: Ref =
            serde_json::from_value(row["data"]["intent"]["proposal"].clone()).unwrap();
        let key = SigningKey::from_slice(&[7; 32]).unwrap();
        let spki = key.verifying_key().to_public_key_der().unwrap();
        let fp = hash(spki.as_bytes());
        let policy = root.join("policy.json");
        std::fs::write(&policy,json!({"synthetic":true,"expires_at":"2026-11-04T00:00:00Z","owner":"owner","rp_id":"auth.example","origin":"https://auth.example","keys":{"key":{"spki_b64url":b64.encode(spki.as_bytes()),"fingerprint":fp,"cose_alg":-7}}}).to_string()).unwrap();
        std::fs::set_permissions(&policy, std::fs::Permissions::from_mode(0o600)).unwrap();
        let config = SyntheticConfig {
            database: root.join("registry.sqlite"),
            repository: root.clone(),
            board: program,
            policy: pin(policy),
            credential_id: "key".into(),
            key_fingerprint: fp,
            owner: "owner".into(),
            actor: "fixture-actor".into(),
            session_id: "33333333-3333-4333-8333-333333333333".into(),
            rp_id: "auth.example".into(),
            origin: "https://auth.example".into(),
            epoch: datetime!(2026-11-02 01:00 UTC),
            scoped_control: None,
        };
        let mut owner = SyntheticOwner::open_synthetic(config.clone()).unwrap();
        owner
            .register_proposal_fixture(&proposal, "live", None)
            .unwrap();
        owner.register_task_fixture(&raw, "live", None).unwrap();
        Self {
            root,
            config,
            raw,
            proposal,
        }
    }
    fn owner(&self) -> SyntheticOwner {
        SyntheticOwner::open_synthetic(self.config.clone()).unwrap()
    }
    fn request(&self, owner: &mut SyntheticOwner, n: u64, decision: &str) -> Consume {
        let selection = owner.project("fixture-task").unwrap().selection;
        let answer=json!({"v":1,"type":"work-engine.owner-answer","answer_id":format!("00000000-0000-4000-8000-{n:012x}"),"board_seq":1,"request_digest":hash(&self.raw),"decision":decision,"comment":if decision=="ask"{"Please clarify"}else{""},"owner":"owner","created_at":"2026-11-02T00:00:00Z","expires_at":"2026-11-03T00:00:00Z","rp_id":"auth.example"}).to_string();
        Consume {
            selection,
            signed: sign(answer),
        }
    }
    fn empty_effects(&self) {
        let db = rusqlite::Connection::open(&self.config.database).unwrap();
        for table in [
            "admission",
            "outbox",
            "capacity_claim",
            "scoped_claim",
            "pi_claim",
        ] {
            let n: i64 = db
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
                .unwrap();
            assert_eq!(n, 0, "{table}");
        }
    }
    fn post(&self, args: &[&str]) {
        let out = Command::new("/usr/bin/python3")
            .arg("-I")
            .arg(&self.config.board.script.path)
            .arg("--repository")
            .arg(&self.root)
            .args([
                "post",
                "--author",
                "other",
                "--session-id",
                "44444444-4444-4444-8444-444444444444",
                "--topic",
                "fixture:task",
                "--body",
                "other",
            ])
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}
fn sign(answer: String) -> SignedAnswer {
    let key = SigningKey::from_slice(&[7; 32]).unwrap();
    let client=json!({"type":"webauthn.get","challenge":b64.encode(Sha256::digest(answer.as_bytes())),"origin":"https://auth.example","crossOrigin":false}).to_string();
    let mut auth = Sha256::digest(b"auth.example").to_vec();
    auth.extend_from_slice(&[5, 0, 0, 0, 0]);
    let mut bytes = auth.clone();
    bytes.extend_from_slice(&Sha256::digest(client.as_bytes()));
    let sig: Signature = key.sign(&bytes);
    SignedAnswer {
        answer_json: answer,
        credential_id: "key".into(),
        authenticator_data: b64.encode(auth),
        client_data_json: b64.encode(client),
        signature: b64.encode(sig.to_der().as_bytes()),
    }
}
#[test]
fn all_dispositions_reopen_exact_history_and_no_execution() {
    for d in ["approve", "reject", "defer", "ask"] {
        let f = Fixture::new();
        let mut owner = f.owner();
        let p = owner.project("fixture-task").unwrap();
        assert_eq!(p.standing, Standing::Current);
        assert_eq!(p.request_json, f.raw);
        assert!(p.execution_admission.is_none());
        let request = f.request(&mut owner, 9003, d);
        assert!(matches!(owner.read_exact(&request), Err(Error::Unknown)));
        let first = owner.consume(&request).unwrap();
        assert!(!first.historical);
        if let Ok(output) = std::env::var("OWNER_BOUNDARY_FIXTURE_DIR") {
            std::fs::write(PathBuf::from(output).join(format!("{d}-fixture.json")),serde_json::to_vec_pretty(&json!({"synthetic":true,"live_authoritative_publisher":false,"projection":p,"consume_input":request,"decision":first,"admission":null,"native_effects":0})).unwrap()).unwrap();
        }
        drop(owner);
        let mut reopened = f.owner();
        let again = reopened.consume(&request).unwrap();
        assert_eq!(first.decision, again.decision);
        assert!(again.historical);
        assert!(again.execution_admission.is_none());
        assert_eq!(
            again.current_standing,
            if ["approve", "reject"].contains(&d) {
                Standing::Held
            } else {
                Standing::Current
            }
        );
        let mut changed = request.clone();
        changed.signed.signature.push('A');
        assert!(matches!(reopened.consume(&changed), Err(Error::Conflict)));
        reopened
            .register_task_fixture(&f.raw, "withdrawn", Some(1))
            .unwrap();
        let old = reopened.read_exact(&request).unwrap();
        assert_eq!(old.decision, first.decision);
        assert_eq!(old.current_standing, Standing::Revoked);
        f.empty_effects();
    }
}
#[test]
fn wrong_key_tamper_and_policy_construction_fail_closed() {
    let f = Fixture::new();
    let mut o = f.owner();
    let good = f.request(&mut o, 9004, "approve");
    for which in 0..3 {
        let mut r = good.clone();
        match which {
            0 => r.signed.signature = "AA".into(),
            1 => r.signed.answer_json.push(' '),
            _ => r.signed.credential_id = "wrong".into(),
        };
        assert!(matches!(o.consume(&r), Err(Error::Invalid)));
    }
    let mut config = f.config.clone();
    config.key_fingerprint = "0".repeat(64);
    assert!(SyntheticOwner::open_synthetic(config).is_err());
    let mut config = f.config.clone();
    config.epoch = datetime!(2026-11-05 00:00 UTC);
    assert!(SyntheticOwner::open_synthetic(config).is_err());
    std::fs::write(&f.config.policy.path, "{}").unwrap();
    assert!(SyntheticOwner::open_synthetic(f.config.clone()).is_err());
    std::fs::remove_file(&f.config.policy.path).unwrap();
    assert!(SyntheticOwner::open_synthetic(f.config.clone()).is_err());
    f.empty_effects();
}
#[test]
fn stale_superseded_registry_and_deadline_after_wait() {
    let f = Fixture::new();
    let mut o = f.owner();
    let r = f.request(&mut o, 9005, "approve");
    f.post(&[]);
    assert!(matches!(o.consume(&r), Err(Error::Conflict)));
    let r = f.request(&mut o, 9005, "approve");
    f.post(&["--supersedes", "1"]);
    assert!(matches!(o.consume(&r), Err(Error::Conflict)));
    assert_eq!(
        o.project("fixture-task").unwrap().standing,
        Standing::Replaced
    );
    let f = Fixture::new();
    let mut o = f.owner();
    let r = f.request(&mut o, 9006, "approve");
    o.register_proposal_fixture(&f.proposal, "withdrawn", Some(1))
        .unwrap();
    assert!(matches!(o.consume(&r), Err(Error::Held)));
    let f = Fixture::new();
    let mut cfg = f.config.clone();
    cfg.epoch = datetime!(2026-11-02 23:59:59.8 UTC);
    let mut o = SyntheticOwner::open_synthetic(cfg).unwrap();
    let r = f.request(&mut o, 9007, "approve");
    std::thread::sleep(std::time::Duration::from_millis(250));
    assert!(matches!(
        o.consume(&r),
        Err(Error::Held) | Err(Error::Invalid)
    ));
    assert_eq!(
        o.project("fixture-task").unwrap().standing,
        Standing::Expired
    );
    f.empty_effects();
}
#[test]
fn concurrent_exact_retries_and_competing_closes() {
    for exact in [true, false] {
        let f = Fixture::new();
        let mut o = f.owner();
        let a = f.request(&mut o, 9008, "approve");
        let mut b = a.clone();
        if !exact {
            let mut answer: Value = serde_json::from_str(&b.signed.answer_json).unwrap();
            answer["answer_id"] = "00000000-0000-4000-8000-000000009009".into();
            answer["decision"] = "reject".into();
            b.signed = sign(answer.to_string());
        }
        let mut other = f.owner();
        let t = std::thread::spawn(move || other.consume(&b));
        let one = o.consume(&a);
        let two = t.join().unwrap();
        if exact {
            assert_eq!(one.unwrap().decision, two.unwrap().decision);
        } else {
            assert_ne!(one.is_ok(), two.is_ok());
        }
        f.empty_effects();
    }
}

impl Fixture {
    fn snapshot(&self) -> Value {
        let out = Command::new("/usr/bin/python3")
            .arg("-I")
            .arg(&self.config.board.script.path)
            .arg("--repository")
            .arg(&self.root)
            .args([
                "owner-boundary-io",
                "--command-json",
                r#"{"operation":"snapshot"}"#,
            ])
            .output()
            .unwrap();
        assert!(out.status.success());
        serde_json::from_slice(&out.stdout).unwrap()
    }
    fn data_post(&self, kind: &str, data: Value, target: Option<u64>, id: &str) -> String {
        let path = self.root.join("post-data.json");
        std::fs::write(&path, data.to_string()).unwrap();
        let mut command = Command::new("/usr/bin/python3");
        command
            .arg("-I")
            .arg(&self.config.board.script.path)
            .arg("--repository")
            .arg(&self.root)
            .args([
                "post",
                "--author",
                "owner",
                "--session-id",
                "11111111-1111-4111-8111-111111111111",
                "--topic",
                "fixture:control",
                "--body",
                "Synthetic control",
                "--kind",
                kind,
                "--to",
                "session:11111111-1111-4111-8111-111111111111",
                "--message-id",
                id,
                "--data-file",
            ])
            .arg(path);
        if let Some(seq) = target {
            command.arg("--reply-to").arg(seq.to_string());
        }
        let out = command.output().unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let rows = self.snapshot();
        rows["rows"]
            .as_array()
            .unwrap()
            .last()
            .unwrap()
            .as_str()
            .unwrap()
            .into()
    }
    fn control(
        &self,
        profile: &Pin,
        state: &str,
        generation: i64,
        predecessor: Option<String>,
    ) -> String {
        let id = format!("00000000-0000-4000-8000-{:012x}", 10000 + generation * 2);
        let raw=self.data_post("request",json!({"ask":"Synthetic control","scoped_control":{"schema":"dot.scoped-control/v1","profile_sha256":profile.sha256,"generation":generation,"predecessor":predecessor,"state":state}}),None,&id);
        let row: Value = serde_json::from_str(&raw).unwrap();
        let seq = row["sequence"].as_u64().unwrap();
        let id = format!("00000000-0000-4000-8000-{:012x}", 10001 + generation * 2);
        let a=sign(json!({"v":1,"type":"work-engine.owner-answer","answer_id":id,"board_seq":seq,"request_digest":hash(&raw),"decision":"approve","comment":"","owner":"owner","created_at":"2026-11-02T00:00:00Z","expires_at":"2026-11-03T00:00:00Z","rp_id":"auth.example"}).to_string());
        self.data_post("reply",json!({"disposition":"answered","decision":"approve","comment":"","answer_id":id,"signed_answer":{"format":"work-engine.owner-answer/v1","answer_json":a.answer_json,"credential_id":a.credential_id,"authenticator_data":a.authenticator_data,"client_data_json":a.client_data_json,"signature":a.signature}}),Some(seq),&id);
        hash(raw)
    }
    fn scoped(&self) -> (SyntheticConfig, Pin, Pin) {
        use std::os::unix::fs::PermissionsExt;
        let mut owner = self.owner();
        let selection = owner.project("fixture-task").unwrap().selection;
        let reference = json!({"id":"fixture","revision":"r1","sha256":"a".repeat(64)});
        let profile = json!({"schema":"dot.scoped-profile/v1","id":"fixture-workspace","task":selection.task,"proposal":selection.proposal,"mode":reference,"configuration":reference,"actor":"fixture-actor","objective":"fixture-objective","session":"synthetic-session","turn_rule":"synthetic-only","entry_request_sha256":"a".repeat(64),"native_executable_sha256":"a".repeat(64),"target_generation":"synthetic","bridge_store_id":"synthetic","transport_sha256":"a".repeat(64),"root":self.root,"publication":"synthetic","effect":{"path":"unused","before_sha256":"a".repeat(64),"after":"unused","after_sha256":"a".repeat(64)},"parent":reference,"maximum":"5.00","reserve":"1.00","expires_at":"2026-11-03T00:00:00Z","trust":"synthetic"});
        let path = self.root.join("profile.json");
        std::fs::write(&path, profile.to_string()).unwrap();
        let pin = |path: PathBuf| Pin {
            sha256: hash(std::fs::read(&path).unwrap()),
            path,
        };
        let profile = pin(path);
        let mut board = self.config.board.clone();
        board.dependencies.push(board.script.clone());
        board.script =
            pin(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("examples/owner_boundary_snapshot.py"));
        let path = self.root.join("scoped-config.json");
        std::fs::write(&path,json!({"schema":"dot.admission-config/v2-owned-source","database":self.config.database,"sources":{"board":board,"board_repository":self.root,"key_policy":self.config.policy.path,"owner":"owner","rp_id":"auth.example","origin":"https://auth.example","actor":"fixture-actor","ledger":board,"budget_record":self.config.policy}}).to_string()).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        std::fs::set_permissions(
            &self.config.database,
            std::fs::Permissions::from_mode(0o600),
        )
        .unwrap();
        let config_pin = pin(path);
        let mut config = self.config.clone();
        config.scoped_control = Some((config_pin.clone(), profile.clone()));
        (config, config_pin, profile)
    }
}
#[test]
fn existing_signed_control_owner_revocation_and_historical_identity() {
    let f = Fixture::new();
    let (config, cp, pp) = f.scoped();
    let prior = f.control(&pp, "active", 1, None);
    let mut o = SyntheticOwner::open_synthetic(config).unwrap();
    let r = f.request(&mut o, 9010, "defer");
    assert_eq!(
        o.project("fixture-task").unwrap().standing,
        Standing::Current
    );
    let first = o.consume(&r).unwrap();
    f.control(&pp, "revoked", 2, Some(prior));
    let control = session_task_admission::scoped_pilot::ingest_control(
        cp,
        pp,
        datetime!(2026-11-02 01:00 UTC),
    )
    .unwrap();
    assert_eq!(control["state"], "revoked");
    let old = o.read_exact(&r).unwrap();
    assert_eq!(old.decision, first.decision);
    assert_eq!(old.current_standing, Standing::Revoked);
    let next = f.request(&mut o, 9011, "approve");
    assert!(matches!(o.consume(&next), Err(Error::Held)));
    f.empty_effects();
}
#[test]
fn real_sqlite_writer_wait_rechecks_deadline() {
    let f = Fixture::new();
    let mut cfg = f.config.clone();
    cfg.epoch = datetime!(2026-11-02 23:59:59 UTC);
    let mut o = SyntheticOwner::open_synthetic(cfg).unwrap();
    let r = f.request(&mut o, 9012, "approve");
    let lock = rusqlite::Connection::open(&f.config.database).unwrap();
    lock.execute_batch("BEGIN IMMEDIATE").unwrap();
    let t = std::thread::spawn(move || o.consume(&r));
    std::thread::sleep(std::time::Duration::from_millis(1100));
    lock.execute_batch("ROLLBACK").unwrap();
    assert!(matches!(
        t.join().unwrap(),
        Err(Error::Held) | Err(Error::Invalid)
    ));
    f.empty_effects();
}

// Invoked only by the process-crash test below. Backend settings are fixed by
// this executable; the environment selects its disposable private fixture root.
#[test]
fn owner_boundary_process_child() {
    let Ok(root) = std::env::var("OWNER_BOUNDARY_CHILD_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap();
    let script = repository.join("skills/durable-state/scripts/codex_chatboard.py");
    let pin = |path: PathBuf| Pin {
        sha256: hash(std::fs::read(&path).unwrap()),
        path,
    };
    let key = SigningKey::from_slice(&[7; 32]).unwrap();
    let spki = key.verifying_key().to_public_key_der().unwrap();
    let mut config = SyntheticConfig {
        database: root.join("registry.sqlite"),
        repository: root.clone(),
        board: Program {
            python: pin("/usr/bin/python3".into()),
            script: pin(script.clone()),
            dependencies: vec![
                pin(script.parent().unwrap().join("durable_state.py")),
                pin(script
                    .parent()
                    .unwrap()
                    .join("../contracts/codex-chatboard-kinds.v1.json")),
            ],
        },
        policy: pin(root.join("policy.json")),
        credential_id: "key".into(),
        key_fingerprint: hash(spki.as_bytes()),
        owner: "owner".into(),
        actor: "fixture-actor".into(),
        session_id: "33333333-3333-4333-8333-333333333333".into(),
        rp_id: "auth.example".into(),
        origin: "https://auth.example".into(),
        epoch: datetime!(2026-11-02 01:00 UTC),
        scoped_control: None,
    };
    let request: Consume =
        serde_json::from_slice(&std::fs::read(root.join("consume-input.json")).unwrap()).unwrap();
    let phase = std::env::var("OWNER_BOUNDARY_CRASH_PHASE").unwrap();
    if phase == "prepare" {
        config.board.python = pin(root.join("interleaving-python"));
    }
    let mut owner = SyntheticOwner::open_synthetic(config).unwrap();
    if phase == "writer" {
        std::fs::write(root.join("writer-started"), b"yes").unwrap();
        owner
            .register_task_fixture(
                &std::fs::read_to_string(root.join("raw-task.json")).unwrap(),
                "withdrawn",
                Some(1),
            )
            .unwrap();
        std::fs::write(root.join("writer-done"), b"yes").unwrap();
        return;
    }
    if phase == "before" {
        std::fs::write(root.join("child-ready"), b"before").unwrap();
        loop {
            std::thread::park();
        }
    }
    owner.consume(&request).unwrap();
    // The consumer deliberately never receives this process's answer ACK.
    std::fs::write(root.join("child-ready"), b"after").unwrap();
    loop {
        std::thread::park();
    }
}
#[test]
fn process_crash_lost_answer_ack_reopens_and_reconciles_exactly() {
    use std::{
        io::Read,
        time::{Duration, Instant},
    };
    for phase in ["before", "after"] {
        let f = Fixture::new();
        let mut owner = f.owner();
        let request = f.request(&mut owner, 9013, "approve");
        std::fs::write(
            f.root.join("consume-input.json"),
            serde_json::to_vec(&request).unwrap(),
        )
        .unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "owner_boundary_process_child", "--nocapture"])
            .env("OWNER_BOUNDARY_CHILD_ROOT", &f.root)
            .env("OWNER_BOUNDARY_CRASH_PHASE", phase)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let end = Instant::now() + Duration::from_secs(45);
        while !f.root.join("child-ready").is_file() && Instant::now() < end {
            if child.try_wait().unwrap().is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let ready = f.root.join("child-ready").is_file();
        let _ = child.kill();
        let status = child.wait().unwrap();
        let mut stderr = String::new();
        child
            .stderr
            .take()
            .unwrap()
            .read_to_string(&mut stderr)
            .unwrap();
        assert!(ready, "{status}: {stderr}");
        assert!(!status.success());
        drop(owner);
        let mut owner = f.owner();
        if phase == "before" {
            assert!(matches!(owner.read_exact(&request), Err(Error::Unknown)));
        } else {
            let historical = owner.read_exact(&request).unwrap();
            let retry = owner.consume(&request).unwrap();
            assert_eq!(historical.decision, retry.decision);
            assert!(retry.historical);
            let rows = f.snapshot();
            assert_eq!(rows["rows"].as_array().unwrap().len(), 2);
        }
        f.empty_effects();
    }
}

#[test]
fn registered_task_and_proposal_replacement_are_observed() {
    let f = Fixture::new();
    let mut o = f.owner();
    let request = f.request(&mut o, 9014, "approve");
    let mut next: Value = serde_json::from_str(&f.raw).unwrap();
    next["data"]["intent"]["revision"] = "r2".into();
    o.register_task_fixture(&next.to_string(), "live", Some(1))
        .unwrap();
    assert!(matches!(o.consume(&request), Err(Error::Held)));
    assert!(matches!(o.read_exact(&request), Err(Error::Unknown)));
    f.empty_effects();
    let f = Fixture::new();
    let mut o = f.owner();
    let request = f.request(&mut o, 9015, "approve");
    let mut proposal = f.proposal.clone();
    proposal.revision = "r2".into();
    o.register_proposal_fixture(&proposal, "live", Some(1))
        .unwrap();
    assert!(matches!(o.consume(&request), Err(Error::Held)));
    assert_eq!(
        o.project("fixture-task").unwrap().standing,
        Standing::Replaced
    );
    f.empty_effects();
}
#[test]
fn later_board_supersession_and_proof_expiry_preserve_history() {
    let f = Fixture::new();
    let mut o = f.owner();
    let request = f.request(&mut o, 9016, "approve");
    let decision = o.consume(&request).unwrap();
    f.post(&["--supersedes", "1"]);
    let old = o.read_exact(&request).unwrap();
    assert_eq!(old.decision, decision.decision);
    assert_eq!(old.current_standing, Standing::Replaced);
    let mut cfg = f.config.clone();
    cfg.epoch = datetime!(2026-11-03 00:00:01 UTC);
    let mut later = SyntheticOwner::open_synthetic(cfg).unwrap();
    let old = later.read_exact(&request).unwrap();
    assert_eq!(old.decision, decision.decision);
    assert!(old.historical);
    f.empty_effects();
    let f = Fixture::new();
    let mut o = f.owner();
    let mut request = f.request(&mut o, 9017, "approve");
    let mut answer: Value = serde_json::from_str(&request.signed.answer_json).unwrap();
    answer["expires_at"] = "2026-11-02T01:00:01Z".into();
    request.signed = sign(answer.to_string());
    let lock = rusqlite::Connection::open(&f.config.database).unwrap();
    lock.execute_batch("BEGIN IMMEDIATE").unwrap();
    let t = std::thread::spawn(move || o.consume(&request));
    std::thread::sleep(std::time::Duration::from_millis(1100));
    lock.execute_batch("ROLLBACK").unwrap();
    assert!(matches!(
        t.join().unwrap(),
        Err(Error::Invalid) | Err(Error::Held)
    ));
    f.empty_effects();
}

// A pinned fixture-only interpreter adapter. It invokes the real board owner;
// no production hook or caller-selected executable is added to the API.
impl Fixture {
    fn interleaving_config(&self, mode: &str) -> SyntheticConfig {
        use std::os::unix::fs::PermissionsExt;
        let wrapper = self.root.join("interleaving-python");
        let body = format!(
            r##"#!/usr/bin/python3
import sys,json,time,os
from pathlib import Path
script=sys.argv[2]
sys.path.insert(0,str(Path(script).parent))
import codex_chatboard as b
args=sys.argv[3:]
root=Path({root:?})
mode={mode:?}
command=json.loads(args[args.index('--command-json')+1])
if command['operation']=='conditional_reply':
 if mode=='race': b.post(root,**command['candidate'])
 if mode=='initial-read':
  revision=command['expected_revision']
  blob=root/'.git/objects'/revision[:2]/revision[2:]
  blob.rename(str(blob)+'.saved')
  os.mkfifo(blob)
  (root/'preparation-python').write_text(str(os.getpid()))
  (root/'original-deadline').write_text(str(command['deadline_monotonic']))
 result=b.owner_boundary_io(root,command)
 (root/'publication-returned').write_text('yes')
 print(json.dumps(result))
else:
 if (root/'publication-returned').exists():
  if mode=='unavailable': sys.exit(73)
  (root/'snapshot-paused').write_text('yes')
  end=time.monotonic()+7
  while not (root/'snapshot-release').exists() and time.monotonic()<end: time.sleep(.005)
 print(json.dumps(b.owner_boundary_io(root,command)))
"##,
            root = self.root.to_string_lossy(),
            mode = mode
        );
        std::fs::write(&wrapper, body).unwrap();
        std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut cfg = self.config.clone();
        cfg.board.python = Pin {
            sha256: hash(std::fs::read(&wrapper).unwrap()),
            path: wrapper,
        };
        cfg
    }
}
fn wait_file(path: &std::path::Path) {
    let end = std::time::Instant::now() + std::time::Duration::from_secs(8);
    while !path.is_file() && std::time::Instant::now() < end {
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(path.is_file(), "missing {}", path.display());
}
#[test]
fn raced_ordinary_exact_row_is_historical_in_rust() {
    let f = Fixture::new();
    let cfg = f.interleaving_config("race");
    let mut o = SyntheticOwner::open_synthetic(cfg).unwrap();
    let request = f.request(&mut o, 9020, "defer");
    std::fs::write(f.root.join("snapshot-release"), b"yes").unwrap();
    let decision = o.consume(&request).unwrap();
    assert!(decision.historical);
    assert_eq!(decision.signed, request.signed);
    let mut changed = request.clone();
    changed.signed.signature.push('A');
    assert!(matches!(o.consume(&changed), Err(Error::Conflict)));
    assert_eq!(f.snapshot()["rows"].as_array().unwrap().len(), 2);
    f.empty_effects();
}
#[test]
fn fresh_defer_and_ask_expire_during_post_publication_acquisition() {
    for disposition in ["defer", "ask"] {
        let f = Fixture::new();
        let cfg = f.interleaving_config("pause");
        let mut o = SyntheticOwner::open_synthetic(cfg).unwrap();
        let mut request = f.request(&mut o, 9021, disposition);
        let mut answer: Value = serde_json::from_str(&request.signed.answer_json).unwrap();
        answer["expires_at"] = "2026-11-02T01:00:01Z".into();
        request.signed = sign(answer.to_string());
        let original = request.signed.clone();
        let thread = std::thread::spawn(move || o.consume(&request));
        wait_file(&f.root.join("snapshot-paused"));
        std::thread::sleep(std::time::Duration::from_millis(1100));
        std::fs::write(f.root.join("snapshot-release"), b"yes").unwrap();
        let d = thread.join().unwrap().unwrap();
        assert!(!d.historical);
        assert_eq!(d.current_standing, Standing::Expired);
        assert_eq!(d.signed, original);
        assert_eq!(f.snapshot()["rows"].as_array().unwrap().len(), 2);
        f.empty_effects();
    }
}
#[test]
fn fresh_control_revocation_during_post_publication_acquisition() {
    let f = Fixture::new();
    let (mut cfg, _, pp) = f.scoped();
    let prior = f.control(&pp, "active", 1, None);
    cfg.board.python = f.interleaving_config("pause").board.python;
    let mut o = SyntheticOwner::open_synthetic(cfg).unwrap();
    let request = f.request(&mut o, 9022, "defer");
    let t = std::thread::spawn(move || o.consume(&request));
    wait_file(&f.root.join("snapshot-paused"));
    f.control(&pp, "revoked", 2, Some(prior));
    std::fs::write(f.root.join("snapshot-release"), b"yes").unwrap();
    let d = t.join().unwrap().unwrap();
    assert!(!d.historical);
    assert_eq!(d.current_standing, Standing::Revoked);
    f.empty_effects();
}

#[test]
fn final_git_survives_rust_and_python_and_fences_real_registry_writer() {
    use std::{os::unix::fs::PermissionsExt, time::Duration};
    // Adopt only descendants orphaned by this fixture, so each is collected.
    assert_eq!(unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1) }, 0);
    let f = Fixture::new();
    let mut o = f.owner();
    let request = f.request(&mut o, 9023, "defer");
    std::fs::write(
        f.root.join("consume-input.json"),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();
    std::fs::write(f.root.join("raw-task.json"), &f.raw).unwrap();
    let hook = f.root.join(".git/hooks/reference-transaction");
    std::fs::write(&hook, format!(r##"#!/usr/bin/python3
import os,sys,json,time
from pathlib import Path
root=Path({root:?})
if sys.argv[1]=='prepared':
 fd=int(os.environ['WORK_ENGINE_REGISTRY_FENCE_FD'])
 os.fstat(fd)
 git=os.getppid()
 python=int(next(l.split()[1] for l in Path('/proc/'+str(git)+'/status').read_text().splitlines() if l.startswith('PPid:')))
 (root/'git-paused').write_text(json.dumps([git,python,fd]))
 end=time.monotonic()+7
 while not (root/'git-release').exists() and time.monotonic()<end: time.sleep(.005)
 if not (root/'git-release').exists(): sys.exit(1)
if sys.argv[1]=='committed': (root/'git-committed').write_text('yes')
"##,root=f.root.to_string_lossy())).unwrap();
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o700)).unwrap();
    let launch = |phase: &str| {
        Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "owner_boundary_process_child", "--nocapture"])
            .env("OWNER_BOUNDARY_CHILD_ROOT", &f.root)
            .env("OWNER_BOUNDARY_CRASH_PHASE", phase)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap()
    };
    let mut consumer = launch("inflight");
    wait_file(&f.root.join("git-paused"));
    let ids: Vec<i32> =
        serde_json::from_slice(&std::fs::read(f.root.join("git-paused")).unwrap()).unwrap();
    // Retain a passive reader for Git's protocol output. Without a reader,
    // killing Python would make Git die from SIGPIPE before its final effect.
    let _git_output_reader = std::fs::File::open(format!("/proc/{}/fd/1", ids[0])).unwrap();
    consumer.kill().unwrap();
    let consumer_status = consumer.wait().unwrap();
    assert!(!consumer_status.success());
    // The real final Git and its prepared hook survive both ancestors.
    assert_eq!(unsafe { libc::kill(ids[1], libc::SIGKILL) }, 0);
    let mut status = 0;
    assert_eq!(unsafe { libc::waitpid(ids[1], &mut status, 0) }, ids[1]);
    let python_status = status;
    assert_eq!(unsafe { libc::kill(ids[0], 0) }, 0);
    let mut writer = launch("writer");
    wait_file(&f.root.join("writer-started"));
    std::thread::sleep(Duration::from_millis(150));
    assert!(!f.root.join("writer-done").exists());
    let db = rusqlite::Connection::open(&f.config.database).unwrap();
    let state: String = db
        .query_row("SELECT state FROM task_head", [], |r| r.get(0))
        .unwrap();
    assert_eq!(state, "live");
    std::fs::write(f.root.join("git-release"), b"yes").unwrap();
    assert_eq!(unsafe { libc::waitpid(ids[0], &mut status, 0) }, ids[0]);
    assert!(libc::WIFEXITED(status));
    assert_eq!(libc::WEXITSTATUS(status), 0);
    wait_file(&f.root.join("git-committed"));
    let git_status = status;
    let writer_status = writer.wait().unwrap();
    assert!(writer_status.success());
    assert!(f.root.join("writer-done").exists());
    let historical = o.read_exact(&request).unwrap();
    assert!(historical.historical);
    assert_eq!(historical.current_standing, Standing::Revoked);
    let snapshot = f.snapshot();
    assert_eq!(snapshot["rows"].as_array().unwrap().len(), 2);
    f.empty_effects();
    if let Ok(out) = std::env::var("OWNER_BOUNDARY_FIXTURE_DIR") {
        use std::os::unix::process::ExitStatusExt;
        let out = PathBuf::from(out);
        std::fs::write(out.join("f1-process-evidence.json"), serde_json::to_vec_pretty(&json!({
            "synthetic":true,"real_git_update_ref":true,"real_sqlite_transaction_owner_killed":true,
            "consumer_wait_status":consumer_status.into_raw(),"python_wait_status":python_status,
            "git_wait_status":git_status,"writer_wait_status":writer_status.into_raw(),
            "git_pid":ids[0],"python_pid":ids[1],"inherited_fd_in_actual_git_hook":ids[2],
            "registry_state_while_git_paused":state,"writer_finished_while_paused":false,
            "registry_after_publication":historical.current_standing,"snapshot":snapshot,
            "decision":historical,"input":request,"all_orphaned_processes_reaped":true
        })).unwrap()).unwrap();
        copy_fixture(&f.root, &out.join("f1-real-store"));
    }
}

fn copy_fixture(from: &std::path::Path, to: &std::path::Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_fixture(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}
#[test]
fn final_git_failure_releases_fence_without_append() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::new();
    let mut o = f.owner();
    let request = f.request(&mut o, 9024, "defer");
    let hook = f.root.join(".git/hooks/reference-transaction");
    std::fs::write(
        &hook,
        "#!/usr/bin/python3\nimport sys\nsys.exit(1 if sys.argv[1]=='prepared' else 0)\n",
    )
    .unwrap();
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o700)).unwrap();
    // Failure after CAS entry cannot be rewritten as known no-effect by Rust.
    assert!(matches!(o.consume(&request), Err(Error::Unknown)));
    assert_eq!(f.snapshot()["rows"].as_array().unwrap().len(), 1);
    assert_eq!(
        o.register_task_fixture(&f.raw, "withdrawn", Some(1))
            .unwrap(),
        2
    );
    assert_eq!(
        o.register_proposal_fixture(&f.proposal, "withdrawn", Some(1))
            .unwrap(),
        2
    );
    f.empty_effects();
}

#[test]
fn durable_receipt_survives_unavailable_post_publication_snapshot() {
    let f = Fixture::new();
    let mut o = SyntheticOwner::open_synthetic(f.interleaving_config("unavailable")).unwrap();
    let request = f.request(&mut o, 9025, "defer");
    let d = o.consume(&request).unwrap();
    assert!(!d.historical);
    assert_eq!(d.current_standing, Standing::Unavailable);
    assert_eq!(d.signed, request.signed);
    assert_eq!(f.snapshot()["rows"].as_array().unwrap().len(), 2);
    f.empty_effects();
}

#[test]
fn actual_committed_git_death_is_unknown_and_reconciles_without_republication() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::new();
    let mut o = f.owner();
    let request = f.request(&mut o, 9030, "defer");
    let hook = f.root.join(".git/hooks/reference-transaction");
    std::fs::write(
        &hook,
        format!(
            r##"#!/usr/bin/python3
import os,sys,signal
from pathlib import Path
root=Path({root:?})
if sys.argv[1]=='prepared':
 with (root/'cas-count').open('a') as f: f.write('1\n')
if sys.argv[1]=='committed':
 (root/'committed-git-pid').write_text(str(os.getppid()))
 os.kill(os.getppid(),signal.SIGKILL)
"##,
            root = f.root.to_string_lossy()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert!(matches!(o.consume(&request), Err(Error::Unknown)));
    let d = o.read_exact(&request).unwrap();
    assert!(d.historical);
    assert_eq!(d.signed, request.signed);
    assert_eq!(d.selection, request.selection);
    let again = o.consume(&request).unwrap();
    assert!(again.historical);
    assert_eq!(again.decision, d.decision);
    assert_eq!(
        std::fs::read_to_string(f.root.join("cas-count")).unwrap(),
        "1\n"
    );
    assert_eq!(f.snapshot()["rows"].as_array().unwrap().len(), 2);
    f.empty_effects();
    if let Ok(out) = std::env::var("OWNER_BOUNDARY_FIXTURE_DIR") {
        copy_fixture(&f.root, &PathBuf::from(&out).join("r1-real-store"));
        std::fs::write(PathBuf::from(out).join("r1-evidence.json"),serde_json::to_vec_pretty(&json!({"actual_committed_hook_kills_git":true,"rust_result":"unknown","cas_count":1,"decision":d,"input":request})).unwrap()).unwrap();
    }
}

#[test]
fn rust_death_during_real_initial_git_read_closes_within_original_deadline() {
    use std::time::{Duration, Instant};
    assert_eq!(unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1) }, 0);
    let f = Fixture::new();
    let cfg = f.interleaving_config("initial-read");
    let mut o = SyntheticOwner::open_synthetic(cfg).unwrap();
    let mut request = f.request(&mut o, 9031, "defer");
    let mut answer: Value = serde_json::from_str(&request.signed.answer_json).unwrap();
    answer["expires_at"] = "2026-11-02T01:00:03Z".into();
    request.signed = sign(answer.to_string());
    std::fs::write(
        f.root.join("consume-input.json"),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();
    std::fs::write(f.root.join("raw-task.json"), &f.raw).unwrap();
    let launch = |phase: &str| {
        Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "owner_boundary_process_child", "--nocapture"])
            .env("OWNER_BOUNDARY_CHILD_ROOT", &f.root)
            .env("OWNER_BOUNDARY_CRASH_PHASE", phase)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap()
    };
    let started = Instant::now();
    let mut consumer = launch("prepare");
    wait_file(&f.root.join("preparation-python"));
    let python: i32 = std::fs::read_to_string(f.root.join("preparation-python"))
        .unwrap()
        .parse()
        .unwrap();
    let mut git = 0;
    let end = Instant::now() + Duration::from_secs(2);
    while Instant::now() < end {
        let children = std::fs::read_to_string(format!("/proc/{python}/task/{python}/children"))
            .unwrap_or_default();
        for child in children.split_whitespace() {
            let cmd = std::fs::read(format!("/proc/{child}/cmdline")).unwrap_or_default();
            if String::from_utf8_lossy(&cmd).contains("cat-file") {
                git = child.parse::<i32>().unwrap();
            }
        }
        if git != 0 {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_ne!(git, 0);
    assert!(
        std::fs::read_link(format!("/proc/{git}/exe"))
            .unwrap()
            .to_string_lossy()
            .ends_with("/git")
    );
    consumer.kill().unwrap();
    assert!(!consumer.wait().unwrap().success());
    let mut writer = launch("writer");
    wait_file(&f.root.join("writer-started"));
    std::thread::sleep(Duration::from_millis(100));
    assert!(!f.root.join("writer-done").exists());
    let mut status = 0;
    assert_eq!(unsafe { libc::waitpid(python, &mut status, 0) }, python);
    assert!(started.elapsed() < Duration::from_secs(6));
    assert_eq!(unsafe { libc::kill(git, 0) }, -1);
    assert!(writer.wait().unwrap().success());
    let rev = &request.selection.board_revision;
    let blob = f.root.join(".git/objects").join(&rev[..2]).join(&rev[2..]);
    std::fs::remove_file(&blob).unwrap();
    std::fs::rename(format!("{}.saved", blob.display()), &blob).unwrap();
    assert_eq!(f.snapshot()["rows"].as_array().unwrap().len(), 1);
    assert!(matches!(o.read_exact(&request), Err(Error::Unknown)));
    f.empty_effects();
    if let Ok(out) = std::env::var("OWNER_BOUNDARY_FIXTURE_DIR") {
        copy_fixture(&f.root, &PathBuf::from(&out).join("r2-real-store"));
        std::fs::write(PathBuf::from(out).join("r2-evidence.json"),serde_json::to_vec_pretty(&json!({"rust_ancestor_killed":true,"actual_native_git_cat_file_pid":git,"python_pid":python,"python_wait_status":status,"elapsed_seconds":started.elapsed().as_secs_f64(),"original_deadline":std::fs::read_to_string(f.root.join("original-deadline")).unwrap(),"fault":"fixture replaces actual loose Git object with FIFO; real native Git blocks reading it; no fake Git executable","no_append":true,"writer_progress_after_closure":true})).unwrap()).unwrap();
    }
}
