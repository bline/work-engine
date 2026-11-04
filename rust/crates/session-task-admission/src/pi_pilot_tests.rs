use super::*;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as B64};
use p256::{
    ecdsa::{Signature, SigningKey, signature::Signer},
    pkcs8::EncodePublicKey,
};
use sha2::{Digest, Sha256};
use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};
fn put(p: &Path, b: impl AsRef<[u8]>) {
    std::fs::write(p, b).unwrap();
    std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o600)).unwrap();
}
fn pin(p: PathBuf) -> Pin {
    Pin {
        sha256: bytes_sha(&std::fs::read(&p).unwrap()),
        path: p,
    }
}
struct Fixture {
    dir: PathBuf,
    pin: Pin,
    rows: Vec<String>,
    key: SigningKey,
    keep: bool,
}
impl Fixture {
    fn new(export: Option<PathBuf>, job: &str, exe: &str) -> Self {
        Self::new_kind(export, job, exe, false)
    }
    fn new_kind(export: Option<PathBuf>, job: &str, exe: &str, subscription: bool) -> Self {
        static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let keep = export.is_some();
        let dir = export.unwrap_or_else(|| {
            std::env::temp_dir().join(format!(
                "pi-owner-{}-{}",
                std::process::id(),
                N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ))
        });
        std::fs::create_dir(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        let now = clock();
        let template: Value =
            serde_json::from_str(include_str!("../tests/projection-fixture.json")).unwrap();
        let mut task: Value =
            serde_json::from_str(template["tasks"][0]["request_json"].as_str().unwrap()).unwrap();
        task["sequence"] = 1.into();
        let i = &mut task["data"]["intent"];
        i["schema"] = "dot.task-intent/pi-coding-v1".into();
        i["provider"] = "pi".into();
        i["execution_profile"] = "disposable-pi-coding-fixture-v1".into();
        i["workspace"]["profile_id"] = PROFILE.into();
        i["effects"]["write_paths"] = json!(["src/lib.rs"]);
        i["effects"]["network"] = false.into();
        i["model"] = json!({"id":"fake-pi-model","effort":"off","tier":"fixture"});
        i["clock"]["admit_until"] = (now + time::Duration::minutes(15))
            .format(&Rfc3339)
            .unwrap()
            .into();
        i["clock"]["close_by"] = (now + time::Duration::minutes(20))
            .format(&Rfc3339)
            .unwrap()
            .into();
        if subscription {
            i["schema"] = "dot.task-intent/pi-coding-subscription-v1".into();
            i["execution_profile"] = "disposable-pi-coding-subscription-v1".into();
            i["workspace"]["profile_id"] = SUBSCRIPTION_PROFILE.into();
            i["effects"]["network"] = true.into();
            i["model"] = json!({"id":"gpt-6-luna","effort":"off","tier":"subscription"});
        }
        let raw = task.to_string();
        let task_ref = crate::validate_review(&raw).unwrap();
        let i = &task["data"]["intent"];
        let key = SigningKey::from_slice(&[7; 32]).unwrap();
        let der = key.verifying_key().to_public_key_der().unwrap();
        put(&dir.join("keys.json"),json!({"keys":{"fixture-key":{"spki_b64url":B64.encode(der.as_bytes()),"fingerprint":bytes_sha(der.as_bytes()),"cose_alg":-7}}}).to_string());
        let py = pin(std::fs::canonicalize("/usr/bin/python3").unwrap());
        let program = |name: &str, body: String| {
            let p = dir.join(name);
            put(&p, body);
            source::Program {
                python: py.clone(),
                script: pin(p),
                dependencies: vec![],
            }
        };
        let board = program(
            "board.py",
            format!(
                "from pathlib import Path\nprint(Path({:?}).read_text())",
                dir.join("board.json").to_str().unwrap()
            ),
        );
        // Explicit synthetic budget source in this fixture; canonical owner semantics
        // still validate the exact units, limits, freshness and coverage fields.
        let cost = json!({"schema":"objective-budget-control-v1","topic":"control:budget:fixture-objective","closed_at":null,"snapshot":{"snapshot_at":"REPLACED","status":"complete","totals":{"codex":{"quota_usd_low":0,"quota_usd_high":0,"unpriced_calls":0},"claude_all_models":{"quota_usd_low":0,"quota_usd_high":0,"unpriced_calls":0},"claude_fable":{"quota_usd_low":0,"quota_usd_high":0,"unpriced_calls":0}},"cost_controls":[{"pool":"codex","node_id":null,"max_usd":5,"basis":"speed_adjusted_api_usd","remaining_usd":5,"observed":{"quota_usd_low":0,"quota_usd_high":0,"unpriced_calls":0,"cost_basis":"speed_adjusted_api_usd"}}]}});
        let ledger = program(
            "ledger.py",
            format!(
                "import json,datetime\nx=json.loads({:?})\nx['snapshot']['snapshot_at']=datetime.datetime.now(datetime.timezone.utc).isoformat()\nprint(json.dumps(x))",
                cost.to_string()
            ),
        );
        put(
            &dir.join("record"),
            b"explicit synthetic fixture budget source",
        );
        let dbp = dir.join("registry.sqlite");
        let mut db = Connection::open(&dbp).unwrap();
        std::fs::set_permissions(&dbp, std::fs::Permissions::from_mode(0o600)).unwrap();
        store::initialize(&db).unwrap();
        let row: Row = serde_json::from_str(&raw).unwrap();
        store::register_proposal(&mut db, &row.data.intent.proposal, "live", None).unwrap();
        store::register_task(&mut db, &raw, "live", None).unwrap();
        drop(db);
        put(&dir.join("admission-config.json"),json!({"schema":"dot.admission-config/v2-owned-source","database":dbp,"sources":{"board":board,"board_repository":dir,"key_policy":dir.join("keys.json"),"owner":"owner","rp_id":"auth.example","origin":"https://auth.example","actor":"fixture-actor","ledger":ledger,"budget_record":pin(dir.join("record"))}}).to_string());
        let transport = Transport {
            helper_sha256: exe.into(),
            principal: "fixture-principal".into(),
            subject: "fixture-subject".into(),
            handle: "one-signed-pi-fixture".into(),
            revision: "r1".into(),
            idempotency_key: "one-exact-job".into(),
        };
        let lifetime = if keep {
            std::env::var("PI_FIXTURE_LIFETIME_SECONDS")
                .ok()
                .map(|v| time::Duration::seconds(v.parse().unwrap()))
        } else {
            None
        }
        .unwrap_or(time::Duration::minutes(15));
        let profile = json!({"schema":if subscription {SUBSCRIPTION_PROFILE} else {PROFILE},"classification":if subscription {SUBSCRIPTION_CLASSIFICATION} else {"fixture-signing-and-identity; fake-provider; network-none; actual-pi-sdk-and-docker"},"task":task_ref,"proposal":i["proposal"],"mode":i["authority"]["mode"],"configuration":i["workspace"]["configuration"],"parent":i["prerequisites"][0],"actor":"fixture-actor","objective":"fixture-objective","job_sha256":job,"native_executable_sha256":exe,"transport_sha256":transport.sha256(),"expires_at":(now+lifetime).format(&Rfc3339).unwrap()});
        put(&dir.join("profile.json"), profile.to_string());
        let mut f = Self {
            pin: Pin {
                path: dir.join("descriptor.json"),
                sha256: String::new(),
            },
            dir,
            rows: vec![raw],
            key,
            keep,
        };
        f.sign();
        let reply: Value = serde_json::from_str(&f.rows[1]).unwrap();
        let request=json!({"schema":"dot.admission-request/v1-proposal","request_id":"signed-pi-request","task":task_ref,"decision":{"id":reply["message_id"],"revision":"2","sha256":crate::hash(&f.rows[1])},"expected_task_revision":"r1"}).to_string();
        let d = Descriptor {
            schema: "dot.pi-coding-descriptor/v1".into(),
            config: pin(f.dir.join("admission-config.json")),
            profile: pin(f.dir.join("profile.json")),
            admission_request: request,
            transport,
        };
        put(&f.pin.path, serde_json::to_vec(&d).unwrap());
        f.pin = pin(f.pin.path.clone());
        f.control("active");
        f
    }
    fn sign(&mut self) {
        let raw = self.rows.last().unwrap();
        let row: Value = serde_json::from_str(raw).unwrap();
        let seq = row["sequence"].as_u64().unwrap();
        let id = format!("00000000-0000-4000-8000-{seq:012}");
        let now = clock();
        let answer=json!({"v":1,"type":"work-engine.owner-answer","answer_id":id,"board_seq":seq,"request_digest":crate::hash(raw),"decision":"approve","comment":"","owner":"owner","created_at":(now-time::Duration::minutes(1)).format(&Rfc3339).unwrap(),"expires_at":(now+time::Duration::days(1)).format(&Rfc3339).unwrap(),"rp_id":"auth.example"}).to_string();
        let client=json!({"type":"webauthn.get","challenge":B64.encode(Sha256::digest(answer.as_bytes())),"origin":"https://auth.example","crossOrigin":false}).to_string();
        let mut auth = Sha256::digest(b"auth.example").to_vec();
        auth.extend_from_slice(&[5, 0, 0, 0, 0]);
        let mut msg = auth.clone();
        msg.extend_from_slice(&Sha256::digest(client.as_bytes()));
        let sig: Signature = self.key.sign(&msg);
        self.rows.push(json!({"kind":"reply","sequence":seq+1,"in_reply_to":seq,"topic":row["topic"],"to":[format!("session:{}",row["session_id"].as_str().unwrap())],"message_id":id,"data":{"disposition":"answered","decision":"approve","comment":"","answer_id":id,"signed_answer":{"format":"work-engine.owner-answer/v1","answer_json":answer,"credential_id":"fixture-key","authenticator_data":B64.encode(auth),"client_data_json":B64.encode(client),"signature":B64.encode(sig.to_der().as_bytes())}}}).to_string());
    }
    fn control(&mut self, state: &str) {
        let d = descriptor(&self.pin).unwrap();
        let seq = self.rows.len() + 1;
        let predecessor = if seq > 3 {
            Some(crate::hash(&self.rows[self.rows.len() - 2]))
        } else {
            None
        };
        self.rows.push(json!({"kind":"request","sequence":seq,"topic":"pi:fixture-control","session_id":"fixture-session","data":{"scoped_control":{"schema":"dot.scoped-control/v1","profile_sha256":d.profile.sha256,"generation":(seq-1)/2,"predecessor":predecessor,"state":state}}}).to_string());
        self.sign();
        self.save();
    }
    fn save(&self) {
        put(
            &self.dir.join("board.json"),
            json!({"revision":"a".repeat(40),"next_sequence":self.rows.len()+1,"rows":self.rows})
                .to_string(),
        );
    }
    fn owner(&self) -> PiOwner {
        PiOwner::open(self.pin.clone()).unwrap()
    }
    fn db(&self) -> Connection {
        Connection::open(self.dir.join("registry.sqlite")).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !self.keep {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }
}
fn exe() -> String {
    bytes_sha(&std::fs::read(std::env::current_exe().unwrap()).unwrap())
}
fn admitted() -> (Fixture, PiOwner) {
    let f = Fixture::new(None, &"a".repeat(64), &exe());
    let o = f.owner();
    o.admit(&"a".repeat(64), &exe()).unwrap();
    (f, o)
}
#[test]
fn signed_pi_proof_current_and_one_time_native_fences() {
    let (f, o) = admitted();
    assert!(o.enter(Effect::Initialization).is_err());
    let allocation = o.enter(Effect::Allocation).unwrap();
    allocation.check_current().unwrap();
    drop(allocation);
    assert!(o.enter(Effect::Allocation).is_err());
    let id = "00000000-0000-4000-8000-000000000001";
    o.seal(&"a".repeat(64), id).unwrap();
    assert!(o.seal(&"a".repeat(64), id).is_err());
    assert!(o.check_native("replacement").is_err());
    let init = o.enter(Effect::Initialization).unwrap();
    init.check_current().unwrap();
    drop(init);
    assert!(o.enter(Effect::Initialization).is_err());
    for effect in [Effect::Provider(1), Effect::Tool("one".into())] {
        let lease = o.enter(effect.clone()).unwrap();
        drop(lease);
        assert!(o.enter(effect).is_err());
    }
    let before = std::fs::read(f.dir.join("registry.sqlite")).unwrap();
    assert_eq!(read(&f.pin).unwrap(), read(&f.pin).unwrap());
    assert_eq!(
        before,
        std::fs::read(f.dir.join("registry.sqlite")).unwrap()
    );
}
#[test]
fn revoked_key_control_stale_heads_and_transport_fail_closed() {
    for case in [
        "key",
        "control",
        "task",
        "proposal",
        "cross_lane",
        "signature",
        "closing_reply",
        "profile",
        "budget",
    ] {
        let (mut f, o) = admitted();
        match case {
            "key" => put(&f.dir.join("keys.json"), b"{\"keys\":{}}"),
            "control" => {
                f.control("revoked");
                ingest_control(&f.pin).unwrap();
            }
            "task" => {
                f.db()
                    .execute("UPDATE task_head SET state='withdrawn'", [])
                    .unwrap();
            }
            "proposal" => {
                f.db()
                    .execute("UPDATE proposal_head SET state='withdrawn'", [])
                    .unwrap();
            }
            "cross_lane" => {
                f.db()
                    .execute("UPDATE task_head SET lane_id='foreign'", [])
                    .unwrap();
            }
            "signature" => {
                let mut row: Value = serde_json::from_str(&f.rows[1]).unwrap();
                row["data"]["signed_answer"]["signature"] = "bad".into();
                f.rows[1] = row.to_string();
                f.save();
            }
            "closing_reply" => {
                let mut row: Value = serde_json::from_str(&f.rows[1]).unwrap();
                row["data"]["decision"] = "reject".into();
                f.rows[1] = row.to_string();
                f.save();
            }
            "profile" => put(&f.dir.join("profile.json"), b"{}"),
            "budget" => put(&f.dir.join("record"), b"changed"),
            _ => unreachable!(),
        }
        assert!(o.enter(Effect::Allocation).is_err(), "{case}");
    }
    let (f, _) = admitted();
    let d = descriptor(&f.pin).unwrap();
    assert!(
        check_transport(
            &f.pin,
            "foreign",
            &d.transport.handle,
            &d.transport.revision,
            &d.transport.idempotency_key,
            &exe()
        )
        .is_err()
    );
}
#[test]
fn exact_admission_and_result_reuse_never_reenters() {
    let (f, o) = admitted();
    let one = read(&f.pin).unwrap();
    assert_eq!(one, o.admit(&"a".repeat(64), &exe()).unwrap());
    assert!(o.admit(&"b".repeat(64), &exe()).is_err());
    let mut d = descriptor(&f.pin).unwrap();
    d.transport.idempotency_key = "other".into();
    put(&f.pin.path, serde_json::to_vec(&d).unwrap());
    let changed = pin(f.pin.path.clone());
    assert!(read(&changed).is_err());
}
#[test]
fn lease_rechecks_revocation_and_preserves_unknown() {
    let (mut f, o) = admitted();
    let lease = o.enter(Effect::Allocation).unwrap();
    f.control("revoked");
    assert!(lease.check_current().is_err());
    drop(lease);
    let state: String = f
        .db()
        .query_row("SELECT state FROM pi_effect", [], |r| r.get(0))
        .unwrap();
    assert_eq!(state, "unknown");
    assert!(o.enter(Effect::Allocation).is_err());
}
#[test]
#[ignore = "explicit local fixture bundle export; no provider or Docker"]
fn export_signed_pi_fixture() {
    let out = std::env::var_os("PI_FIXTURE_EXPORT").expect("owned output required");
    let job = std::env::var("PI_FIXTURE_JOB_SHA").unwrap();
    let executor = std::env::var("PI_FIXTURE_EXECUTOR_SHA").unwrap();
    let f = Fixture::new(Some(PathBuf::from(out)), &job, &executor);
    let p: Profile =
        serde_json::from_slice(&std::fs::read(f.dir.join("profile.json")).unwrap()).unwrap();
    put(
        &f.dir.join("export-clock.json"),
        json!({"profile_expires_unix":utc(&p.expires_at).unwrap().unix_timestamp()}).to_string(),
    );
    put(
        &f.dir.join("descriptor-pin.json"),
        serde_json::to_vec(&f.pin).unwrap(),
    );
    put(&f.dir.join("ui-golden-vectors.json"),json!({"standing":"local fixture signing only; fake identity/key/provider; no human approval","schema":"dot.task-intent/pi-coding-v1","persistent_write_paths":["src/lib.rs"],"scratch":"ephemeral","network":"none","native_rule":"first actual SDK allocation for exact job before initialization; one-time seal; no replacement on unknown","descriptor":descriptor(&f.pin).unwrap(),"profile_json":std::fs::read_to_string(f.dir.join("profile.json")).unwrap(),"signed_rows":f.rows}).to_string());
}

#[test]
fn each_pi_effect_boundary_rechecks_revocation_and_keeps_unknown() {
    for effect in [
        Effect::Initialization,
        Effect::Provider(1),
        Effect::Tool("one".into()),
    ] {
        let (mut f, o) = admitted();
        drop(o.enter(Effect::Allocation).unwrap());
        o.seal(&"a".repeat(64), "00000000-0000-4000-8000-000000000001")
            .unwrap();
        let lease = o.enter(effect.clone()).unwrap();
        lease.check_current().unwrap();
        f.control("revoked");
        assert!(lease.check_current().is_err());
        drop(lease);
        assert!(o.enter(effect.clone()).is_err());
        let state: String = f
            .db()
            .query_row(
                "SELECT state FROM pi_effect WHERE effect_key=?1",
                [effect.key().unwrap()],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(state, "unknown");
    }
}
#[test]
fn ordered_revocation_waits_for_inflight_entry_and_blocks_next() {
    let (mut f, o) = admitted();
    let lease = o.enter(Effect::Allocation).unwrap();
    lease.check_current().unwrap();
    f.control("revoked");
    let pin = f.pin.clone();
    let (send, recv) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        send.send(ingest_control(&pin)).unwrap();
    });
    std::thread::sleep(std::time::Duration::from_millis(100));
    assert!(recv.try_recv().is_err());
    lease.observed(&"b".repeat(64)).unwrap();
    assert_eq!(
        recv.recv_timeout(std::time::Duration::from_secs(5))
            .unwrap(),
        Ok(())
    );
    worker.join().unwrap();
    assert!(o.enter(Effect::Initialization).is_err());
}
#[test]
fn signed_expiry_and_lease_expiry_fail_closed() {
    let (mut f, o) = admitted();
    let mut lease = o.enter(Effect::Allocation).unwrap();
    lease.expires = clock() - time::Duration::milliseconds(1);
    assert_eq!(lease.check_current(), Err("pi_lease_expired"));
    drop(lease);
    // Independently sign an already-expired profile; possession of the correct
    // signing key is insufficient for current execution authority.
    let mut d = descriptor(&f.pin).unwrap();
    let mut profile: Value =
        serde_json::from_slice(&std::fs::read(&d.profile.path).unwrap()).unwrap();
    profile["expires_at"] = (clock() - time::Duration::seconds(1))
        .format(&Rfc3339)
        .unwrap()
        .into();
    put(&d.profile.path, profile.to_string());
    d.profile = pin(d.profile.path);
    put(&f.pin.path, serde_json::to_vec(&d).unwrap());
    f.pin = pin(f.pin.path.clone());
    f.rows.truncate(2);
    f.control("active");
    assert!(PiOwner::open(f.pin.clone()).is_err());
    assert_eq!(
        f.db()
            .query_row(
                "SELECT state FROM pi_effect WHERE effect_key='allocation'",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
        "unknown"
    );
}

#[test]
fn observed_revocation_cannot_be_rolled_back_after_denied_dispatch() {
    let (mut f, o) = admitted();
    let lease = o.enter(Effect::Allocation).unwrap();
    let active = f.rows.clone();
    f.control("revoked");
    assert!(lease.check_current().is_err());
    assert!(lease.observed(&"c".repeat(64)).is_err());
    // Simulate a stale source export after the denied attempt. The current
    // registry remembers the signed newer head and refuses source rollback.
    f.rows = active;
    f.save();
    assert!(PiOwner::open(f.pin.clone()).is_err());
    assert!(o.enter(Effect::Allocation).is_err());
    assert_eq!(
        f.db()
            .query_row(
                "SELECT state FROM pi_effect WHERE effect_key='allocation'",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
        "unknown"
    );
}

#[test]
fn explicit_pi_review_variant_rejects_generic_codex_and_widened_effects() {
    let f = Fixture::new(None, &"a".repeat(64), &exe());
    for (field, value) in [
        ("provider", json!("codex")),
        ("provider", json!("generic")),
        ("execution_profile", json!("disposable-codex-write-v1")),
        ("schema", json!("dot.task-intent/v1-proposal")),
    ] {
        let mut task: Value = serde_json::from_str(&f.rows[0]).unwrap();
        task["data"]["intent"][field] = value;
        assert!(crate::validate_review(&task.to_string()).is_err());
    }
    for (section, field, value) in [
        ("effects", "network", json!(true)),
        ("effects", "write_paths", json!(["src/lib.rs", "other"])),
        ("model", "id", json!("live-model")),
        ("model", "tier", json!("default")),
        ("effects", "approval", json!(true)),
    ] {
        let mut task: Value = serde_json::from_str(&f.rows[0]).unwrap();
        task["data"]["intent"][section][field] = value;
        assert!(crate::validate_review(&task.to_string()).is_err());
    }
}

// The pinned board program changes its export at an exact acquisition count,
// after initial ingestion and before the transaction's signed-head fetch.
#[test]
fn transactional_denials_retain_new_signed_head_without_action_writes() {
    for (boundary, switch_at) in [
        ("prepare", 3),
        ("post_fence", 5),
        ("seal", 3),
        ("claim", 5),
        ("action", 2),
    ] {
        let mut f = Fixture::new(None, &"a".repeat(64), &exe());
        let active = std::fs::read(f.dir.join("board.json")).unwrap();
        f.control(if boundary == "action" {
            "active"
        } else {
            "revoked"
        });
        put(
            &f.dir.join("revoked.json"),
            std::fs::read(f.dir.join("board.json")).unwrap(),
        );
        put(&f.dir.join("board.json"), &active);
        let script = format!(
            "from pathlib import Path\np=Path({dir:?})\nc=p/'count'\nn=int(c.read_text())+1 if c.exists() else 1\nc.write_text(str(n))\ns=int((p/'switch').read_text()) if (p/'switch').exists() else 999999\nprint((p/('revoked.json' if n>=s else 'board.json')).read_text())",
            dir = f.dir.to_str().unwrap()
        );
        put(&f.dir.join("board.py"), script);
        let mut config: Value =
            serde_json::from_slice(&std::fs::read(f.dir.join("admission-config.json")).unwrap())
                .unwrap();
        config["sources"]["board"]["script"] =
            serde_json::to_value(pin(f.dir.join("board.py"))).unwrap();
        put(&f.dir.join("admission-config.json"), config.to_string());
        let mut d = descriptor(&f.pin).unwrap();
        d.config = pin(f.dir.join("admission-config.json"));
        put(&f.pin.path, serde_json::to_vec(&d).unwrap());
        f.pin = pin(f.pin.path.clone());
        let o = f.owner();
        if boundary != "claim" {
            o.admit(&"a".repeat(64), &exe()).unwrap();
        }
        if boundary == "seal" {
            drop(o.enter(Effect::Allocation).unwrap());
        }
        put(&f.dir.join("count"), "0");
        put(&f.dir.join("switch"), switch_at.to_string());
        let action_db = f.db();
        let denied = match boundary {
            "action" => o.action(&action_db, || {
                action_db
                    .execute("UPDATE outbox SET state='unknown'", [])
                    .unwrap();
                Err("injected_action_denial")
            }),
            "claim" => o.admit(&"a".repeat(64), &exe()).map(|_| ()),
            "seal" => o.seal(&"a".repeat(64), "00000000-0000-4000-8000-000000000001"),
            _ => o.enter(Effect::Allocation).map(drop),
        };
        assert_eq!(
            denied,
            Err(if boundary == "action" {
                "injected_action_denial"
            } else {
                "pi_current_authority"
            }),
            "{boundary}"
        );
        let db = f.db();
        let generation: i64 = db
            .query_row("SELECT max(generation) FROM scoped_control", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(generation, 2, "{boundary}");
        let effects: i64 = db
            .query_row("SELECT count(*) FROM pi_effect", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            effects,
            i64::from(boundary == "post_fence" || boundary == "seal"),
            "{boundary}"
        );
        let sealed: i64 = db
            .query_row(
                "SELECT count(*) FROM pi_claim WHERE native_id IS NOT NULL",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(sealed, 0, "{boundary}");
        if boundary == "action" {
            let state: String = db
                .query_row("SELECT state FROM outbox", [], |r| r.get(0))
                .unwrap();
            assert_eq!(
                state, "pending",
                "failed action must roll back allocation mutation"
            );
        }
        if boundary == "claim" {
            let claims: i64 = db
                .query_row("SELECT count(*) FROM pi_claim", [], |r| r.get(0))
                .unwrap();
            assert_eq!(claims, 0);
        }
        // Restore the old export: no path may resurrect the revoked authority.
        put(&f.dir.join("switch"), "999999");
        assert!(o.enter(Effect::Allocation).is_err(), "{boundary}");
        assert!(o.admit(&"a".repeat(64), &exe()).is_err(), "{boundary}");
        assert!(
            o.seal(&"a".repeat(64), "00000000-0000-4000-8000-000000000001")
                .is_err(),
            "{boundary}"
        );
    }
}

#[test]
fn subscription_exact_schema_and_field_negatives() {
    let f = Fixture::new_kind(None, &"a".repeat(64), &exe(), true);
    assert!(f.owner().is_subscription());
    assert!(crate::validate_review(&f.rows[0]).is_ok());
    for (section, field, value) in [
        ("effects", "network", json!(false)),
        ("effects", "write_paths", json!(["src/lib.rs", "other"])),
        ("model", "id", json!("fake-pi-model")),
        ("model", "tier", json!("fixture")),
        ("model", "effort", json!("high")),
        ("workspace", "profile_id", json!(PROFILE)),
    ] {
        let mut row: Value = serde_json::from_str(&f.rows[0]).unwrap();
        row["data"]["intent"][section][field] = value;
        assert!(
            crate::validate_review(&row.to_string()).is_err(),
            "{section}.{field}"
        );
    }
    for (field, value) in [
        ("schema", "dot.task-intent/pi-coding-v1"),
        ("provider", "codex"),
        ("execution_profile", "disposable-pi-coding-fixture-v1"),
    ] {
        let mut row: Value = serde_json::from_str(&f.rows[0]).unwrap();
        row["data"]["intent"][field] = value.into();
        assert!(crate::validate_review(&row.to_string()).is_err());
    }
}

#[test]
fn subscription_unknowns_native_seal_idempotency_and_result() {
    let f = Fixture::new_kind(None, &"a".repeat(64), &exe(), true);
    let o = f.owner();
    let before = o.admit(&"a".repeat(64), &exe()).unwrap();
    assert_eq!(before, o.admit(&"a".repeat(64), &exe()).unwrap());
    assert!(o.admit(&"b".repeat(64), &exe()).is_err());
    assert!(o.enter(Effect::Initialization).is_err());
    drop(o.enter(Effect::Allocation).unwrap());
    let id = "00000000-0000-4000-8000-000000000001";
    assert!(o.seal(&"b".repeat(64), id).is_err());
    o.seal(&"a".repeat(64), id).unwrap();
    assert!(
        o.check_native("00000000-0000-4000-8000-000000000002")
            .is_err()
    );
    assert!(o.seal(&"a".repeat(64), id).is_err());
    for effect in [
        Effect::Initialization,
        Effect::Provider(1),
        Effect::Tool("one".into()),
    ] {
        drop(o.enter(effect.clone()).unwrap());
        assert!(o.enter(effect).is_err());
    }
    assert!(o.enter(Effect::Allocation).is_err());
    drop(o);
    assert!(f.owner().enter(Effect::Allocation).is_err());
    let r = f.owner().record_result(&json!({"receipt":{"input_digest":"a".repeat(64),"session_id":id,"owner_pending":true},"classification":"synthetic signing/access/provider"})).unwrap();
    assert_eq!(r["schema"], "dot.pi-coding-subscription-result/v1");
    assert_eq!(r["owner_acceptance"], "not_claimed");
    assert_eq!(r["retry_permitted"], false);
    assert!(
        f.owner()
            .record_result(&json!({"input_digest":"a".repeat(64)}))
            .is_err()
    );
}

#[test]
fn subscription_current_boundaries_revocation_and_monotone_observation() {
    for effect in [
        Effect::Initialization,
        Effect::Provider(1),
        Effect::Tool("one".into()),
    ] {
        let mut f = Fixture::new_kind(None, &"a".repeat(64), &exe(), true);
        let o = f.owner();
        o.admit(&"a".repeat(64), &exe()).unwrap();
        drop(o.enter(Effect::Allocation).unwrap());
        o.seal(&"a".repeat(64), "00000000-0000-4000-8000-000000000001")
            .unwrap();
        let lease = o.enter(effect.clone()).unwrap();
        lease.check_current().unwrap();
        let active = f.rows.clone();
        f.control("revoked");
        assert!(lease.check_current().is_err());
        drop(lease);
        f.rows = active;
        f.save();
        assert!(o.enter(effect).is_err());
        assert!(PiOwner::open(f.pin.clone()).is_err());
    }
    for case in [
        "key",
        "task",
        "proposal",
        "budget",
        "profile",
        "transport",
        "expiry",
    ] {
        let mut f = Fixture::new_kind(None, &"a".repeat(64), &exe(), true);
        let o = f.owner();
        o.admit(&"a".repeat(64), &exe()).unwrap();
        match case {
            "key" => put(&f.dir.join("keys.json"), b"{\"keys\":{}}"),
            "task" => {
                f.db()
                    .execute("UPDATE task_head SET state='withdrawn'", [])
                    .unwrap();
            }
            "proposal" => {
                f.db()
                    .execute("UPDATE proposal_head SET state='withdrawn'", [])
                    .unwrap();
            }
            "budget" => put(&f.dir.join("record"), b"changed"),
            "profile" => put(&f.dir.join("profile.json"), b"{}"),
            "transport" => put(&f.pin.path, b"{}"),
            "expiry" => {
                let mut l = o.enter(Effect::Allocation).unwrap();
                l.expires = clock() - time::Duration::seconds(1);
                assert!(l.check_current().is_err());
                drop(l);
            }
            _ => unreachable!(),
        }
        assert!(o.enter(Effect::Allocation).is_err(), "{case}");
        f.rows.clear();
    }
}

#[test]
#[ignore = "explicit synthetic subscription authority export; no provider or Docker"]
fn export_signed_pi_subscription() {
    let out = std::env::var_os("PI_SUBSCRIPTION_EXPORT").expect("owned output required");
    let job = std::env::var("PI_SUBSCRIPTION_JOB_SHA").unwrap();
    let executor = std::env::var("PI_SUBSCRIPTION_EXECUTOR_SHA").unwrap();
    let f = Fixture::new_kind(Some(PathBuf::from(out)), &job, &executor, true);
    put(
        &f.dir.join("descriptor-pin.json"),
        serde_json::to_vec(&f.pin).unwrap(),
    );
    put(&f.dir.join("ui-golden-vectors.json"), json!({"standing":"synthetic owner signing/identity only; subscription backend candidate; no live qualification","schema":"dot.task-intent/pi-coding-subscription-v1","persistent_write_paths":["src/lib.rs"],"network":"inference fixed subscription adapter; tool/validator none","descriptor":descriptor(&f.pin).unwrap(),"profile_json":std::fs::read_to_string(f.dir.join("profile.json")).unwrap(),"signed_rows":f.rows}).to_string());
}
