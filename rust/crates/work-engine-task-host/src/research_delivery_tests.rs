//! Full synthetic native producer -> production TaskStore port -> service read -> host.
use crate::research_admission::{ControlledFixture, ResearchNomination};
use crate::*;
use retained_task_workflow::research_admission::*;
use retained_task_workflow::{FactInput, FactKind, SubjectKind, sha256};
use std::os::unix::fs::PermissionsExt;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
fn dir(p: &Path) {
    fs::create_dir_all(p).unwrap();
    fs::set_permissions(p, fs::Permissions::from_mode(0o700)).unwrap();
}
fn http(s: &mut TcpStream) -> (Vec<u8>, Vec<u8>) {
    s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let mut h = Vec::new();
    let mut b = [0];
    while !h.ends_with(b"\r\n\r\n") {
        s.read_exact(&mut b).unwrap();
        h.push(b[0]);
        assert!(h.len() < 131072);
    }
    let h = String::from_utf8(h).unwrap();
    assert!(!h.to_lowercase().contains("authorization:"));
    let n = h
        .lines()
        .find_map(|l| {
            l.split_once(':')
                .filter(|(k, _)| k.eq_ignore_ascii_case("content-length"))
                .map(|(_, v)| v.trim().parse::<usize>().unwrap())
        })
        .unwrap();
    assert!(n < 8 * 1024 * 1024);
    let mut raw = vec![0; n];
    s.read_exact(&mut raw).unwrap();
    let decoded = if h.to_lowercase().contains("content-encoding: zstd") {
        zstd::stream::decode_all(raw.as_slice()).unwrap()
    } else {
        raw.clone()
    };
    (raw, decoded)
}
async fn stop_service(
    job: tokio::task::JoinHandle<Result<(), lifecycle_service::ServiceError>>,
) -> Result<(), lifecycle_service::ServiceError> {
    // The exact test runs alone. The real service owns the installed SIGTERM
    // listener and its runner shutdown; collect it rather than aborting its task.
    assert!(
        std::process::Command::new("/bin/kill")
            .args(["-TERM", &std::process::id().to_string()])
            .status()
            .unwrap()
            .success()
    );
    tokio::time::timeout(Duration::from_secs(30), job)
        .await
        .unwrap()
        .unwrap()
}
// Read exact owner rows for restart/no-new-execution assertions. These reads
// are observations only; all mutations in this fixture use production ports.
fn owner_rows(path: &Path, tables: &[&str]) -> Vec<Vec<Vec<rusqlite::types::Value>>> {
    let db =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    tables
        .iter()
        .map(|table| {
            let mut q = db
                .prepare(&format!("SELECT * FROM {table} ORDER BY rowid"))
                .unwrap();
            let columns = q.column_count();
            q.query_map([], |r| {
                (0..columns)
                    .map(|i| r.get(i))
                    .collect::<Result<Vec<rusqlite::types::Value>, _>>()
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
        })
        .collect()
}
async fn socket_ready(path: &Path) {
    let end = Instant::now() + Duration::from_secs(60);
    while !path.exists() {
        assert!(Instant::now() < end);
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}
fn native_config(root: &Path, url: &str, material: &[u8]) -> ServiceConfig {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    let grants=[("grant:turn","native_turn"),("grant:snapshot","snapshot_read"),("grant:input","enqueue_input"),("grant:result","task_result_publish")].map(|(id,scope)|json!({"grant_ref":id,"context_generation":"context:input","scope":scope,"expires_wall_ms":now+600000,"revision":1}));
    let native = json!({"executable":"/native-app-server","root":root.join("native"),"provider_route":"native-sim-proof-v1","chatgpt_access_source":null,"simulator_base_url":url,"session_id":"session:input","invocation_id":"invocation:unused","attempt_id":"attempt:unused","prompt":"bounded canonical task input","turn_grant_ref":"grant:turn","turn_grant_revision":1,"snapshot_grant_ref":"grant:snapshot","snapshot_grant_revision":1,"same_native_session_required":false,"followup_turns":[],"snapshot":{"id":"snapshot:input","source_revision":"synthetic:a4","members":[{"id":"research","sha256":sha256(material),"bytes":material}]},"semantic":{"capture_id":"capture:input","compile_session_id":"session:compile","verify_session_id":"session:verify","successor_verify_session_id":"session:successor","compile_invocation_id":"invocation:compile","compile_attempt_id":"attempt:compile","candidate_verify_invocation_id":"invocation:verify","candidate_verify_attempt_id":"attempt:verify","successor_verify_invocation_id":"invocation:successor","successor_verify_attempt_id":"attempt:successor","challenge_id":"challenge:input","challenge_text":"synthetic","control_grant_ref":"grant:control","control_grant_revision":1,"successor_control_grant_ref":"grant:next","successor_control_grant_revision":1},"semantic_contexts":[]});
    serde_json::from_value(json!({"profile":"retained-repository-analysis-v1","store_root":root.join("store"),"socket_path":root.join("service.sock"),"peer_socket_path":root.join("unused.sock"),"verifier_executable":null,"verifier_ledger_path":null,"proof_barrier_dir":null,"trusted_issuer":"issuer:input","principal":{"uid":1000,"principal_ref":"principal:input","producer_ref":"producer:input"},"subject_id":"advisory","context_generation":"context:input","build_id":"build:input","proof_run_id":"proof:input","grants":grants,"max_executions":1,"max_wait_ms":1000,
    "native":native})).unwrap()
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires exact provider-disabled network-none composition and derived A4 fixture"]
async fn full_native_a4_host_preparation_restart() {
    let root = PathBuf::from(std::env::var("WE_JOIN_OUT").unwrap());
    dir(&root);
    dir(&root.join("store"));
    dir(&root.join("native"));
    let a4 = PathBuf::from(std::env::var("WE_A4_FIXTURE").unwrap());
    let request = fs::read(a4.join("request.json")).unwrap();
    let card = fs::read(a4.join("card.json")).unwrap();
    let reconstruction = fs::read(a4.join("reconstruction.json")).unwrap();
    let target = fs::read(a4.join("target.txt")).unwrap();
    let mut material = request.clone();
    material.push(b'\n');
    material.extend(&card);
    material.push(b'\n');
    material.extend(&reconstruction);
    assert!(material.len() > 8192);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    let received = Arc::new(Mutex::new(Vec::<(Vec<u8>, Vec<u8>)>::new()));
    let stopped = Arc::new(AtomicBool::new(false));
    let dependencies =
        std::collections::BTreeMap::from([("decision.target".to_string(), sha256(&target))]);
    let mut host = crate::tests::admission_host(&root, &root.join("service.sock"), false);
    let config = native_config(&root, &url, &material);
    host.startup.lifecycle = config.clone();
    host.startup.approved_snapshot_ref = "snapshot:input".into();
    host.startup.enqueue_grant_ref = "grant:input".into();
    let input = SubmitTask {
        command_id: "input".into(),
        task_id: "advisory".into(),
        subject_id: "advisory".into(),
        kind: SubjectKind::RepositoryAnalysis,
        objective: "Read the approved research snapshot and provide a qualified recommendation"
            .into(),
        engineering_question: "Synthetic private audit only".into(),
        snapshot_ref: "snapshot:input".into(),
        facts: vec![FactInput {
            id: "target".into(),
            kind: FactKind::Decision,
            text: String::from_utf8(target.clone()).unwrap(),
            current: true,
        }],
    };
    // Publication arguments refer to exact canonical owner source references.
    // Source identity is stable through snapshot-read effects; publication uses
    // the production TaskStore port and its actual actor grant fence.
    host.store.submit(&input).unwrap();
    let record = host.store.observe("advisory").unwrap().unwrap();
    let source = record
        .sources
        .iter()
        .find(|s| s.id == "target")
        .unwrap()
        .reference();
    let args = json!({"result_id":"result:joined","body":serde_json::to_string(&json!({"dependencies":dependencies,"text":"Unknown transfer; alternatives include defer. Synthetic evidence supports only a private audit; execution and approval are refused."})).unwrap(),"source_refs":[source],"attribution":"native:synthetic-loopback"});
    let chunks = material.len().div_ceil(32768);
    let total = material.len();
    let peer_received = received.clone();
    let peer_stop = stopped.clone();
    let peer = std::thread::spawn(move || {
        let end = Instant::now() + Duration::from_secs(200);
        while !peer_stop.load(Ordering::SeqCst) && Instant::now() < end {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let bytes = http(&mut stream);
                    let n = {
                        let mut v = peer_received.lock().unwrap();
                        v.push(bytes);
                        v.len()
                    };
                    let output = if n <= chunks {
                        let offset = (n - 1) * 32768;
                        json!({"type":"function_call","call_id":format!("read-{n}"),"namespace":"workspace","name":"snapshot_read","arguments":json!({"snapshot_id":"snapshot:input","member_id":"research","offset":offset,"length":(total-offset).min(32768)}).to_string()})
                    } else if n == chunks + 1 {
                        json!({"type":"function_call","call_id":"joined-publish","namespace":"task","name":"result_publish","arguments":args.to_string()})
                    } else {
                        json!({"type":"message","id":"final","role":"assistant","content":[{"type":"output_text","text":"synthetic joined done"}]})
                    };
                    let id = format!("joined-response-{n}");
                    let body=[json!({"type":"response.created","response":{"id":id}}),json!({"type":"response.output_item.done","output_index":0,"item":output.clone()}),json!({"type":"response.completed","response":{"id":id,"status":"completed","output":[output],"usage":{"input_tokens":0,"output_tokens":0,"total_tokens":0}}})].iter().map(|v|format!("data: {v}\n\n")).collect::<String>();
                    write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
                    stream.flush().unwrap();
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(2))
                }
                Err(e) => panic!("{e}"),
            }
        }
    });
    // Use production owner implementations for BOTH service ports.
    let service_config = config.clone();
    let material_port = host.store.clone();
    let operation_port = host.store.clone();
    let service = tokio::spawn(async move {
        lifecycle_service::run_with_task_ports(service_config, material_port, operation_port)
            .await
            .inspect_err(|error| eprintln!("joined service failed: {error:?}"))
    });
    socket_ready(&config.socket_path).await;
    // Canonical input was already created by the owner. Bind its existing text
    // using the exact normal host admission constructor and service command.
    let basis = admission_basis(&host, "advisory").unwrap();
    let text = host.store.input_text("input").unwrap().unwrap();
    let command = build_admission(&host, &input, &basis, &text).unwrap();
    let reply=exchange(&config.socket_path,&json!({"op":"command","command":serde_json::from_str::<Value>(&command.command_json).unwrap()})).unwrap();
    fs::write(root.join("enqueue.json"), reply.to_string()).unwrap();
    let end = Instant::now() + Duration::from_secs(120);
    loop {
        if host
            .store
            .observe("advisory")
            .unwrap()
            .unwrap()
            .result
            .is_some()
            && received.lock().unwrap().len() > chunks + 1
        {
            break;
        }
        assert!(Instant::now() < end, "native publication timeout");
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    // Capture settles after the final response; wait for the original scoped
    // actor read to become available without ever replaying publication.
    tokio::time::sleep(Duration::from_secs(2)).await;
    stop_service(service).await.unwrap();
    fs::write(
        root.join("peer.json"),
        serde_json::to_vec(&*received.lock().unwrap()).unwrap(),
    )
    .unwrap();
    let native_tables = [
        "native_sessions",
        "native_turns",
        "native_ingress",
        "native_operations",
        "native_task_operations",
        "native_process_exits",
        "native_input_observations",
        "native_causal_evidence",
    ];
    let native_rows = owner_rows(&root.join("store/lifecycle.sqlite"), &native_tables);
    let sends_before_recovery = received.lock().unwrap().len();
    let original = host.store.observe("advisory").unwrap().unwrap();
    fs::write(
        root.join("original-owner.json"),
        serde_json::to_vec_pretty(&original).unwrap(),
    )
    .unwrap();
    let mut f: ControlledFixture = crate::research_admission::tests::fixture();
    let b = &mut f.bases[0];
    b.submit.snapshot_ref = "snapshot:input".into();
    b.snapshot_ref = "snapshot:input".into();
    b.dependencies = dependencies;
    b.target_digest = sha256(&target);
    b.delivery.advisory_task = "advisory".into();
    b.delivery.advisory_subject = "advisory".into();
    b.delivery.request_digest = sha256(&request);
    b.delivery.card_digest = sha256(&card);
    b.key.request_digest = sha256(&request);
    b.recommendation.request_digest = sha256(&request);
    b.recommendation.target_digest = sha256(&target);
    b.reconstruction_digest = sha256(&reconstruction);
    f.key = b.key.clone();
    let rv: Value = serde_json::from_slice(&reconstruction).unwrap();
    let manifest = rv["objects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["oid"] == rv["request"]["source"]["bundle"]["blob"])
        .unwrap();
    let source_bytes: Vec<u8> = serde_json::from_value(manifest["bytes"].clone()).unwrap();
    b.source_bundle_digest = sha256(&source_bytes);
    let result_bytes=serde_json::to_vec(&json!({"dependencies":b.dependencies,"text":"candidate only; actual original owner result replaces this during checked join"})).unwrap();
    for (id, bytes) in [
        ("target", target),
        ("card", card.clone()),
        ("retained", reconstruction.clone()),
        ("source", source_bytes),
        ("result", result_bytes),
        ("input", card.clone()),
    ] {
        let a = b.artifacts.iter_mut().find(|a| a.id == id).unwrap();
        a.bytes = bytes;
        a.digest = sha256(&a.bytes);
    }
    let mapping = std::collections::BTreeMap::from([(
        "decision.target".to_string(),
        ("advisory".to_string(), source),
    )]);
    for (id, bytes) in [
        ("research-request", request),
        ("research-card", card),
        ("research-reconstruction", reconstruction),
        (
            "research-selector-map",
            serde_json::to_vec(&mapping).unwrap(),
        ),
    ] {
        b.artifacts.push(RetainedArtifact {
            id: id.into(),
            digest: sha256(&bytes),
            bytes,
        });
    }
    let fixture_path = root.join("fixture.json");
    host.startup.research = Some(ResearchNomination {
        task_id: f.key.task_id.clone(),
        subject_id: f.key.subject_id.clone(),
        controlled_fixture: Some(fixture_path.clone()),
        installation: None,
        controlled_installation: None,
    });
    // Switch current execution identity to destination D. Historical read must
    // remain anchored to original advisory A, not this current actor subject.
    // Reopen the original retained profile in its existing recovery-only path.
    // Initially persist a candidate; actual execution identities come solely
    // from the owner's scoped retained read below, never fixture assertions.
    fs::write(&fixture_path, serde_json::to_vec(&f).unwrap()).unwrap();
    fs::set_permissions(&fixture_path, fs::Permissions::from_mode(0o600)).unwrap();
    host.store = Arc::new(
        crate::research_admission::scope_causal_read(
            &host.startup,
            Arc::try_unwrap(host.store).ok().unwrap(),
        )
        .unwrap(),
    );
    let config = host.startup.lifecycle.clone();
    let service_config = config.clone();
    let material_port = host.store.clone();
    let operation_port = host.store.clone();
    let service = tokio::spawn(async move {
        lifecycle_service::run_with_task_ports(service_config, material_port, operation_port)
            .await
            .inspect_err(|error| eprintln!("joined service failed: {error:?}"))
    });
    socket_ready(&config.socket_path).await;
    let retained =
        crate::research_delivery::acquire(&config.socket_path, "advisory", "advisory").unwrap();
    // Retain exact source response before matching; no fabricated native bytes.
    fs::write(root.join("retained.json"), &retained.bytes).unwrap();
    let original_retained = retained.bytes.clone();
    let b = &mut f.bases[0];
    b.delivery.context = retained.value["input"]["context_id"]
        .as_str()
        .unwrap()
        .into();
    b.delivery.attempt = retained.value["input"]["attempt_id"]
        .as_str()
        .unwrap()
        .into();
    b.recommendation.context = b.delivery.context.clone();
    b.recommendation.delivery_digest = research_digest("delivery", &b.delivery).unwrap();
    fs::write(&fixture_path, serde_json::to_vec(&f).unwrap()).unwrap();
    // The distinct destination exception requires the checked joined fixture and
    // the original process-local owner scope, before nomination is published.
    let mut uncontrolled = host.startup.clone();
    uncontrolled.research.as_mut().unwrap().controlled_fixture = None;
    assert!(crate::research_admission::validate_nomination(&uncontrolled).is_err());
    for control in ["wrong-subject", "wrong-task", "mixed-unjoined"] {
        let mut bad = f.clone();
        match control {
            "wrong-subject" => bad.bases[0].delivery.advisory_subject = "foreign".into(),
            "wrong-task" => bad.bases[0].delivery.advisory_task = "foreign".into(),
            _ => {
                let mut unjoined = bad.bases[0].clone();
                unjoined.artifacts.retain(|a| {
                    !matches!(
                        a.id.as_str(),
                        "research-request"
                            | "research-card"
                            | "research-reconstruction"
                            | "research-selector-map"
                    )
                });
                bad.bases.push(unjoined);
            }
        }
        let bad_path = root.join(format!("{control}.json"));
        fs::write(&bad_path, serde_json::to_vec(&bad).unwrap()).unwrap();
        fs::set_permissions(&bad_path, fs::Permissions::from_mode(0o600)).unwrap();
        let mut startup = host.startup.clone();
        startup.research.as_mut().unwrap().controlled_fixture = Some(bad_path);
        assert!(
            crate::research_admission::install(&startup, &host.store, None).is_err(),
            "{control}"
        );
        assert!(
            host.store
                .read_research_preparation(&f.key)
                .unwrap()
                .is_none()
        );
    }
    host.research = OnceLock::from(
        crate::research_admission::install(&host.startup, &host.store, None).unwrap(),
    );
    let q = json!({"op":"research.prepare.v1","payload":{"key":f.key,"expected_revision":null,"basis_digest":research_digest("basis",&f.bases[0]).unwrap()}});
    let response = dispatch(&host, q);
    fs::write(root.join("prepare.json"), response.to_string()).unwrap();
    assert_eq!(response["status"], "prepared", "{response}");
    // Validate the actual delivered payload at the host boundary twice, then
    // hold one checked value before mint and another after owner mint.
    let p = host
        .store
        .controlled_research_producer(&f.key, ProducerRole::Preparer)
        .unwrap();
    let make_checked = |case: Option<&str>| {
        let mut b = f.bases[0].clone();
        let find = |id: &str| {
            b.artifacts
                .iter()
                .find(|a| a.id == id)
                .unwrap()
                .bytes
                .clone()
        };
        let mut request = find("research-request");
        let mut card = find("research-card");
        let mut reconstruction = find("research-reconstruction");
        let mapping = serde_json::from_slice(&find("research-selector-map")).unwrap();
        let original = host
            .store
            .research_original_evidence("advisory", &mapping)
            .unwrap();
        let mut retained = crate::research_delivery::RetainedDelivery {
            bytes: original_retained.clone(),
            value: serde_json::from_slice(&original_retained).unwrap(),
        };
        match case {
            Some("wrong-operation") => {
                retained.value["causal"]["operation_id"] = json!("foreign-operation")
            }
            Some("consistent-foreign-target") => {
                let other = b
                    .artifacts
                    .iter()
                    .find(|a| a.id == "source")
                    .unwrap()
                    .digest
                    .clone();
                assert_ne!(other, b.target_digest);
                b.target_digest = other.clone();
                b.recommendation.target_digest = other;
            }
            Some("basis-request") => b.key.request_digest = "f".repeat(64),
            Some("wrong-request") => request.push(b' '),
            Some("changed-card") => card.push(b' '),
            Some("changed-reconstruction") => reconstruction[0] = b'[',
            Some("ineligible") => retained.value["input"]["eligible"] = json!(false),
            Some("unknown-outcome") => {
                retained.value["input"]["eligible"] = json!(false);
                retained.value["input"]["outcome"] = json!("unknown");
            }
            Some("missing-wire") => {
                retained.value["input"]["payloads"]
                    .as_array_mut()
                    .unwrap()
                    .retain(|p| p["representation"] != "wire");
            }
            Some(
                "unsupported-codec"
                | "wire-digest"
                | "decoded-content"
                | "duplicate-wire"
                | "duplicate-json-keys"
                | "suppressed-wire"
                | "malformed-zstd"
                | "expanded-zstd"
                | "combined-excess",
            ) => {
                let causal_bundle: Value = serde_json::from_slice(
                    &serde_json::from_value::<Vec<u8>>(retained.value["causal"]["bundle"].clone())
                        .unwrap(),
                )
                .unwrap();
                let call = causal_bundle["calls"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|c| c["call_id"] == retained.value["causal"]["call_id"])
                    .unwrap();
                let inference = call["inference_call_id"].clone();
                let send = call["physical_send"].clone();
                let p = retained.value["input"]["payloads"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|p| {
                        p["inference_call_id"] == inference
                            && p["physical_send"] == send
                            && p["representation"]
                                == if case == Some("decoded-content") {
                                    "decoded"
                                } else {
                                    "wire"
                                }
                    })
                    .unwrap();
                match case.unwrap() {
                    "unsupported-codec" => p["original_codec"] = json!("unsupported"),
                    "wire-digest" => p["original_sha256"] = json!("0".repeat(64)),
                    "duplicate-wire" => {}
                    selected => {
                        let mut bytes: Vec<u8> =
                            serde_json::from_value(p["bytes"].clone()).unwrap();
                        match selected {
                            "decoded-content" => bytes.push(b' '),
                            "duplicate-json-keys" => {
                                bytes = br#"{"input":[],"input":[]}"#.to_vec();
                                p["original_codec"] = json!("identity");
                            }
                            "suppressed-wire" => {
                                bytes = br#"{"input":[]}"#.to_vec();
                                p["original_codec"] = json!("identity");
                            }
                            "malformed-zstd" => {
                                bytes = b"invalid-zstd".to_vec();
                                p["original_codec"] = json!("zstd");
                            }
                            "expanded-zstd" => {
                                bytes = zstd::stream::encode_all(
                                    vec![b'x'; 8 * 1024 * 1024 + 1].as_slice(),
                                    0,
                                )
                                .unwrap();
                                p["original_codec"] = json!("zstd");
                            }
                            "combined-excess" => bytes = vec![b'x'; 8 * 1024 * 1024],
                            _ => unreachable!(),
                        }
                        p["original_sha256"] = json!(sha256(&bytes));
                        p["bytes"] = json!(bytes);
                    }
                }
                if case == Some("duplicate-wire") {
                    let extra = p.clone();
                    retained.value["input"]["payloads"]
                        .as_array_mut()
                        .unwrap()
                        .push(extra);
                }
                if matches!(case, Some("duplicate-json-keys" | "suppressed-wire")) {
                    // Reach wire decoding rather than failing on the unchanged
                    // decoded companion first. This is a negative retained-byte mutation.
                    retained.value["input"]["payloads"]
                        .as_array_mut()
                        .unwrap()
                        .retain(|p| {
                            !(p["inference_call_id"] == inference
                                && p["physical_send"] == send
                                && p["representation"] == "decoded")
                        });
                }
            }
            _ => {}
        }
        crate::research_delivery::checked_unminted_basis(
            b,
            original,
            retained,
            &request,
            &card,
            &reconstruction,
        )
    };
    let mut negative_results = Vec::new();
    for case in [
        "wrong-operation",
        "wrong-request",
        "basis-request",
        "consistent-foreign-target",
        "changed-card",
        "changed-reconstruction",
        "ineligible",
        "unsupported-codec",
        "wire-digest",
        "decoded-content",
        "unknown-outcome",
        "missing-wire",
        "duplicate-wire",
        "duplicate-json-keys",
        "suppressed-wire",
        "malformed-zstd",
        "expanded-zstd",
        "combined-excess",
    ] {
        let result = make_checked(Some(case));
        assert!(result.is_err(), "{case}");
        let Err(HostError::ResearchDelivery(diagnostic)) = result else {
            panic!("scoped delivery diagnostic required: {case}");
        };
        assert!(diagnostic["sources"].is_object());
        assert!(diagnostic["reason"].as_str().is_some_and(|s| !s.is_empty()));
        let expected = match case {
            "unknown-outcome" | "changed-reconstruction" => "unknown",
            "missing-wire" => "unavailable",
            "unsupported-codec"
            | "ineligible"
            | "wire-digest"
            | "duplicate-json-keys"
            | "expanded-zstd"
            | "combined-excess" => "refused",
            "malformed-zstd" => "unknown",
            _ => "mismatch",
        };
        assert_eq!(diagnostic["status"], expected, "{case}: {diagnostic}");
        negative_results.push(json!({"case":case,"diagnostic":diagnostic}));
    }
    fs::write(
        root.join("host-negative-controls.json"),
        serde_json::to_vec(&negative_results).unwrap(),
    )
    .unwrap();
    let before_mint = make_checked(None).unwrap();
    let after_mint = make_checked(None).unwrap().mint(&host.store, &p).unwrap();
    let research_tables = [
        "research_keys",
        "research_history",
        "research_basis",
        "research_notices",
    ];
    let successful_state = owner_rows(&root.join("tasks.sqlite"), &research_tables);
    let old = host.store.observe("advisory").unwrap().unwrap();
    let change = b"new independently completed owner evidence";
    host.store
        .record_completed_effect(
            "advisory",
            retained_task_workflow::CompletedEffect {
                effect_id: "effect:post-validation".into(),
                delivery_id: "delivery:post-validation".into(),
                receipt_digest: sha256(change),
                attribution: Some(retained_task_workflow::CompletedSnapshotReadAttribution {
                    schema: "completed-snapshot-read-attribution-v2".into(),
                    parent_invocation_id: "independent-owner-parent".into(),
                    call_id: "independent-owner-call".into(),
                    contract_id: "workspace.snapshot.read.v1".into(),
                    snapshot_id: "snapshot:input".into(),
                    member_id: "independent-evidence".into(),
                    offset: 0,
                    requested_length: change.len() as u64,
                    returned_length: change.len() as u64,
                    result_sha256: sha256(change),
                    result: retained_task_workflow::CompletedSnapshotResult::InlineUtf8 {
                        text: String::from_utf8(change.to_vec()).unwrap(),
                    },
                }),
            },
            old.cursor_revision,
        )
        .unwrap();
    assert_eq!(
        host.store.observe("advisory").unwrap().unwrap().result,
        old.result
    );
    assert!(matches!(
        before_mint.mint(&host.store, &p),
        Err(HostError::Task(TaskError::Stale))
    ));
    assert!(matches!(
        host.store.prepare_research(&f.key, Some(1), after_mint, &p),
        Err(TaskError::Stale)
    ));
    assert_eq!(
        owner_rows(&root.join("tasks.sqlite"), &research_tables),
        successful_state
    );
    fs::write(root.join("owner-race-controls.json"),json!({"before_mint":"stale","after_mint":"stale","original_result_unchanged":true,"successful_state_unchanged":true,"notice_written":false}).to_string()).unwrap();
    // Recovery-only startup never launches a new native process. Its existing
    // owner marks close unsafe rather than asserting a resumed native close.
    // Preserve that exact diagnostic; it does not invalidate historical reads.
    let recovery_close = stop_service(service).await;
    fs::write(
        root.join("recovery-close.txt"),
        format!("{recovery_close:?}\n"),
    )
    .unwrap();
    assert_eq!(format!("{recovery_close:?}"), "Err(Store(Unavailable))");
    stopped.store(true, Ordering::SeqCst);
    peer.join().unwrap();
    assert_eq!(response["status"], "prepared", "{response}");
    let saved = host
        .store
        .read_research_preparation(&f.key)
        .unwrap()
        .unwrap();
    assert_eq!(received.lock().unwrap().len(), sends_before_recovery);
    assert_eq!(
        owner_rows(&root.join("store/lifecycle.sqlite"), &native_tables),
        native_rows
    );
    let native_db = rusqlite::Connection::open_with_flags(
        root.join("store/lifecycle.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let closes: Vec<(i64, String)> = native_db
        .prepare("SELECT unsafe_at_close,native_close_json FROM service_closes ORDER BY rowid")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(closes.len(), 2);
    assert_eq!(closes[0].0, 0);
    assert_eq!(closes[1], (1, "[]".into()));
    drop(native_db);
    let db = rusqlite::Connection::open_with_flags(
        root.join("tasks.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let bytes: Vec<u8> = db
        .query_row(
            "SELECT basis_json FROM research_basis WHERE revision=1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    drop(db);
    let committed: BasisBundle = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        Some(research_digest("basis", &committed).unwrap()),
        saved.basis_digest
    );
    let retained_artifact = committed
        .artifacts
        .iter()
        .find(|a| a.id == "native-causal-delivery")
        .unwrap();
    assert_eq!(retained_artifact.bytes, original_retained);
    for a in &committed.artifacts {
        assert_eq!(sha256(&a.bytes), a.digest);
    }
    assert!(
        committed
            .artifacts
            .iter()
            .any(|a| a.id == "research-decoding-map")
    );
    assert!(
        committed
            .artifacts
            .iter()
            .any(|a| a.id == "research-original-operation")
    );
    let owner_tables = [
        "commands",
        "operations",
        "admission_commands",
        "research_keys",
        "research_history",
        "research_basis",
        "research_adoptions",
    ];
    let saved_rows = owner_rows(&root.join("tasks.sqlite"), &owner_tables);
    fs::write(root.join("committed-basis.json"), &bytes).unwrap();
    let count = received.lock().unwrap().len();
    drop(host);
    let reopened = TaskStore::open(&root.join("tasks.sqlite"), "human-operator".into()).unwrap();
    assert_eq!(
        reopened.read_research_preparation(&f.key).unwrap(),
        Some(saved)
    );
    assert_eq!(
        reopened.observe("advisory").unwrap().unwrap().result,
        original.result
    );
    assert_eq!(received.lock().unwrap().len(), count);
    assert_eq!(
        owner_rows(&root.join("tasks.sqlite"), &owner_tables),
        saved_rows
    );
}
