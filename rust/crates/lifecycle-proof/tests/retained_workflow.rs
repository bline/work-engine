#![recursion_limit = "256"]

mod common;

use std::{
    fs, thread,
    time::{Duration, Instant},
};

use lifecycle_proof::{
    host::{HostClient, HostReportedIdentity},
    identity::{HostProcess, observe_process},
    provider::{Reply, ScriptedProvider, Step},
    receipt::{
        ArtifactBinding, EvidenceBundle, TO2_RECEIPT_SCHEMA, to2_authority_from_environment,
        to2_lifecycle_projection,
    },
};
use serde_json::{Value, json};

use common::{
    ANSWER_AUTHORITY, BUILD_ID, CHECKPOINT, Q1_QUESTION, Q2_QUESTION, SNAPSHOT_ID, TASK_ID,
    create_fixture, initial_submit, later_submit,
};

fn wait_observe(
    client: &HostClient,
    delivery_id: &str,
    budget: Duration,
    ready: impl Fn(&Value) -> bool,
) -> Value {
    let until = Instant::now() + budget;
    loop {
        let observed = client.observe(TASK_ID, Some(delivery_id)).unwrap();
        if ready(&observed) {
            return observed;
        }
        assert!(
            Instant::now() < until,
            "qualified host observation timed out: {observed}"
        );
        thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn installed_retained_workflow_vertical() {
    let to2_authority = to2_authority_from_environment("s8-installed-d0-snapshot-child").unwrap();
    let tmp = std::env::var_os("TMPDIR").expect("TMPDIR must be pinned for S8 proof");
    let build_provenance = common::preflight_installed_host();
    let provider_root = tempfile::Builder::new()
        .prefix("s8-provider-")
        .tempdir_in(tmp)
        .unwrap();
    let two_cycles = std::env::var_os("S8_TWO_CYCLES").is_some();
    let transition_a = two_cycles || std::env::var_os("S8_TRANSITION_A").is_some();
    let design_inquiry = std::env::var_os("S8_DESIGN_INQUIRY").is_some();
    let hold_q1 = std::env::var_os("S8_HOLD_Q1").is_some();
    assert!(!design_inquiry || !transition_a);
    assert!(!hold_q1 || two_cycles);
    let final_release = provider_root.path().join("release-d0-final");
    let d0_g1_release = provider_root.path().join("release-d0-g1-final");
    let q1_release = provider_root.path().join("release-q1-final");
    let mut script = vec![
        Step {
            required: vec![SNAPSHOT_ID.into()],
            forbidden: Vec::new(),
            reply: Reply::ToolCall {
                call_id: "call:s8:read0".into(),
                namespace: "workspace".into(),
                name: "snapshot_read".into(),
                arguments: json!({"snapshot_id":SNAPSHOT_ID,"member_id":"m1","offset":0,"length":7}),
            },
            release_path: None,
        },
        Step {
            required: vec![CHECKPOINT.into()],
            forbidden: Vec::new(),
            reply: Reply::Text(format!(
                "Approved snapshot member m1 contains {CHECKPOINT}. Repository mutation approval remains pending."
            )),
            release_path: transition_a.then(|| final_release.clone()),
        },
    ];
    if transition_a {
        script.push(Step {
            required: vec!["compile_attributed_continuation".into()],
            forbidden: Vec::new(),
            reply: Reply::SemanticCompile,
            release_path: None,
        });
        script.push(Step {
            required: vec!["verify_candidate_semantics".into()],
            forbidden: Vec::new(),
            reply: Reply::SemanticVerify,
            release_path: None,
        });
        script.push(Step {
            required: vec!["Choose the next authorized analysis action".into()],
            forbidden: Vec::new(),
            reply: Reply::SuccessorUse,
            release_path: None,
        });
        script.push(Step {
            required: vec!["verify_successor_use_of_continuation".into()],
            forbidden: Vec::new(),
            reply: Reply::SemanticSuccessorVerify,
            release_path: None,
        });
        script.push(Step {
            required: vec!["ORION-7".into()],
            forbidden: Vec::new(),
            reply: Reply::Text(format!(
                "The approved snapshot member m1 contains {CHECKPOINT}. Its earlier read receipt is retained; repository mutation approval remains pending. Continue attributed analysis without repeating the completed snapshot read."
            )),
            release_path: two_cycles.then(|| d0_g1_release.clone()),
        });
    }
    if two_cycles {
        script.extend([
            Step {
                required: vec!["compile_attributed_continuation".into()],
                forbidden: Vec::new(),
                reply: Reply::SemanticCompile,
                release_path: None,
            },
            Step {
                required: vec!["verify_candidate_semantics".into()],
                forbidden: Vec::new(),
                reply: Reply::SemanticVerify,
                release_path: None,
            },
            Step {
                required: vec!["Choose the next authorized analysis action for queued Q1".into()],
                forbidden: Vec::new(),
                reply: Reply::SuccessorUse,
                release_path: None,
            },
            Step {
                required: vec!["verify_successor_use_of_continuation".into()],
                forbidden: Vec::new(),
                reply: Reply::SemanticSuccessorVerify,
                release_path: None,
            },
            Step {
                required: vec!["input:command:s8:d0".into()],
                forbidden: Vec::new(),
                reply: Reply::Text(format!(
                    "D0 continues in the fresh second successor context: the approved member m1 contains {CHECKPOINT}; the completed snapshot read is retained once, and repository mutation approval remains pending."
                )),
                release_path: None,
            },
            Step {
                required: vec![Q1_QUESTION.into()],
                forbidden: Vec::new(),
                reply: Reply::Text(format!(
                    "Q1 retains the governing rule: use only approved snapshot member m1, which contained {CHECKPOINT}. Repository mutation approval remains pending."
                )),
                release_path: hold_q1.then(|| q1_release.clone()),
            },
            Step {
                required: vec![Q2_QUESTION.into()],
                forbidden: Vec::new(),
                reply: Reply::Text(format!(
                    "Q2 attributed result: approved member m1 contains {CHECKPOINT}; the earlier snapshot read remains completed and repository mutation approval remains pending."
                )),
                release_path: None,
            },
        ]);
    }
    let provider =
        ScriptedProvider::start(script, provider_root.path().join("provider-ledger.jsonl"))
            .unwrap();
    let fixture = create_fixture(&provider.base_url(), build_provenance);
    let mut host = HostProcess::launch(&fixture.host_executable, &fixture.startup_path).unwrap();
    let client = HostClient::new(&fixture.host_socket, Duration::from_secs(5)).unwrap();
    client.await_ready(Duration::from_secs(10)).unwrap();
    host.assert_same_execution().unwrap();
    let mut submit = initial_submit();
    if design_inquiry {
        submit["kind"] = json!("design_inquiry");
    }
    let admitted = client.submit(submit).unwrap();
    assert_eq!(admitted["status"], "admitted", "{admitted}");
    assert_eq!(admitted["lifecycle"]["accepted"], true, "{admitted}");
    assert_eq!(
        admitted["lifecycle"]["outcome_kind"], "enqueued",
        "{admitted}"
    );
    let delivery_id = admitted["lifecycle"]["outcome_ref"]
        .as_str()
        .expect("admission omitted logical delivery ID");
    let reported = HostReportedIdentity::from_reply(&admitted).unwrap();
    reported.bind_process(host.identity(), BUILD_ID).unwrap();
    reported.bind_startup(&fixture.startup).unwrap();
    let mut host_observations = vec![admitted.clone()];
    let mut observed = if transition_a {
        let until = Instant::now() + Duration::from_secs(30);
        while provider.entry_count() < 2 {
            assert!(
                Instant::now() < until,
                "D0 final response did not enter provider"
            );
            thread::sleep(Duration::from_millis(20));
        }
        let before = client.observe(TASK_ID, Some(delivery_id)).unwrap();
        host_observations.push(before.clone());
        eprintln!(
            "before pressure: delivery={}, transition={}",
            before["components"]["lifecycle"]["value"]["delivery"],
            before["components"]["lifecycle"]["value"]["transition"]
        );
        let revision = before["components"]["owner"]["value"]["revision"]
            .as_u64()
            .unwrap();
        let wrong_authority = client
            .request_replacement(json!({
                "task_id":TASK_ID,"command_id":"command:s8:replace-wrong-authority",
                "expected_task_revision":revision,"authority_ref":"operator:s8:untrusted",
                "reason":"Unauthorized replacement probe"
            }))
            .unwrap();
        assert_eq!(wrong_authority["status"], "error", "{wrong_authority}");
        assert_eq!(wrong_authority["code"], "unauthorized");
        let after_refusal = client.observe(TASK_ID, Some(delivery_id)).unwrap();
        assert_eq!(
            after_refusal["components"]["lifecycle"]["value"]["transition"]["state"],
            "unknown"
        );
        host_observations.push(after_refusal);
        let replacement_command = json!({
            "task_id":TASK_ID,"command_id":"command:s8:replace-a",
            "expected_task_revision":revision,"authority_ref":ANSWER_AUTHORITY,
            "reason":"Context replacement A after the approved snapshot read"
        });
        let replacement = client
            .request_replacement(replacement_command.clone())
            .unwrap();
        eprintln!("replacement reply: {replacement}");
        assert_eq!(
            replacement["status"], "replacement_requested",
            "{replacement}"
        );
        host_observations.push(replacement.clone());
        let duplicate = client.request_replacement(replacement_command).unwrap();
        assert_eq!(duplicate["status"], "replacement_requested", "{duplicate}");
        assert_eq!(
            duplicate["lifecycle"]["outcome_ref"],
            replacement["lifecycle"]["outcome_ref"]
        );
        assert_eq!(
            duplicate["lifecycle"]["resulting_revision"],
            replacement["lifecycle"]["resulting_revision"]
        );
        host_observations.push(duplicate);
        fs::write(&final_release, b"release\n").unwrap();
        let until = Instant::now() + Duration::from_secs(30);
        while provider.entry_count() < 7 {
            if Instant::now() >= until {
                let readback = client.observe(TASK_ID, Some(delivery_id)).unwrap();
                let lifecycle = &readback["components"]["lifecycle"]["value"];
                let root = fixture.root.path().to_path_buf();
                let provider_path = provider_root.path().to_path_buf();
                let provider_outcome = format!("{:?}", provider.finish());
                std::mem::forget(fixture);
                std::mem::forget(provider_root);
                panic!(
                    "transition control absent; root={}, provider_root={}, provider_outcome={}, stage={}, delivery={}, blocking={}, native_status={}",
                    root.display(),
                    provider_path.display(),
                    provider_outcome,
                    lifecycle["transition"],
                    lifecycle["delivery"],
                    lifecycle["blocking_reason"],
                    readback["components"]["native_delivery_status"]["value"]
                );
            }
            thread::sleep(Duration::from_millis(20));
        }
        let until = Instant::now() + Duration::from_secs(45);
        loop {
            let current = client.observe(TASK_ID, Some(delivery_id)).unwrap();
            let segments =
                current["components"]["native_delivery_status"]["value"]["segments"].as_array();
            if current["components"]["lifecycle"]["value"]["transition"]["value"]["completed"]
                == true
                && current["components"]["lifecycle"]["value"]["delivery"]["value"]["outcome"]
                    == if two_cycles { "pending" } else { "completed" }
                && segments.is_some_and(|items| {
                    items.len() == 2
                        && if two_cycles {
                            items[1]["send_intent"] == true && items[1]["final_sha256"].is_null()
                        } else {
                            items[1]["native_state"] == "completed_settled"
                        }
                })
            {
                let segments = segments.unwrap();
                assert_eq!(segments[0]["segment_no"], 0);
                assert_eq!(segments[0]["segment_state"], "yielded");
                assert_eq!(segments[0]["history_status"], "observed_tool_omitted");
                assert_eq!(segments[1]["segment_no"], 1);
                assert_eq!(segments[1]["context_generation"], "context:s8:g1");
                assert_eq!(segments[1]["delivery_id"], delivery_id);
                assert_eq!(
                    segments[1]["predecessor_invocation_id"],
                    segments[0]["invocation_id"]
                );
                assert_ne!(
                    segments[1]["native_attempt_id"],
                    segments[0]["native_attempt_id"]
                );
                assert_eq!(
                    current["components"]["owner"]["value"]["completed_effects"]
                        .as_array()
                        .unwrap()
                        .len(),
                    1
                );
                break current;
            }
            if Instant::now() >= until {
                let root = fixture.root.path().to_path_buf();
                let provider_path = provider_root.path().to_path_buf();
                let provider_outcome = format!("{:?}", provider.finish());
                let stage = current["components"]["lifecycle"]["value"]["transition"].clone();
                let segments = current["components"]["native_delivery_status"]["value"].clone();
                std::mem::forget(fixture);
                std::mem::forget(provider_root);
                panic!(
                    "transition A failed to settle; root={}, provider_root={}, provider_outcome={}, stage={}, segments={}",
                    root.display(),
                    provider_path.display(),
                    provider_outcome,
                    stage,
                    segments
                );
            }
            thread::sleep(Duration::from_millis(20));
        }
    } else {
        wait_observe(&client, delivery_id, Duration::from_secs(30), |value| {
            value["components"]["owner"]["value"]["completed_effects"]
                .as_array()
                .is_some_and(|effects| {
                    effects
                        .iter()
                        .any(|effect| effect["delivery_id"] == delivery_id)
                })
        })
    };
    assert_eq!(observed["status"], "observation", "{observed}");
    host_observations.push(observed.clone());
    if two_cycles {
        let q1 = client
            .submit(later_submit("command:s8:q1", Q1_QUESTION))
            .unwrap();
        assert_eq!(q1["status"], "admitted", "{q1}");
        let q1_delivery = q1["lifecycle"]["outcome_ref"].as_str().unwrap().to_owned();
        assert_ne!(q1_delivery, delivery_id);
        host_observations.push(q1);
        let q2 = client
            .submit(later_submit("command:s8:q2", Q2_QUESTION))
            .unwrap();
        assert_eq!(q2["status"], "admitted", "{q2}");
        let q2_delivery = q2["lifecycle"]["outcome_ref"].as_str().unwrap().to_owned();
        assert_ne!(q2_delivery, q1_delivery);
        host_observations.push(q2);
        let q2_queued = client.observe(TASK_ID, Some(&q2_delivery)).unwrap();
        assert!(
            q2_queued["components"]["native_delivery_status"]["value"]["segments"]
                .as_array()
                .unwrap()
                .iter()
                .all(|segment| {
                    segment["delivery_id"] != q1_delivery && segment["delivery_id"] != q2_delivery
                })
        );
        assert_eq!(
            provider.entry_count(),
            7,
            "queued inputs entered provider early"
        );
        host_observations.push(q2_queued.clone());
        let revision = q2_queued["components"]["owner"]["value"]["revision"]
            .as_u64()
            .unwrap();
        let replacement = client
            .request_replacement(json!({
                "task_id":TASK_ID,"command_id":"command:s8:replace-b",
                "expected_task_revision":revision,"authority_ref":ANSWER_AUTHORITY,
                "reason":"Context replacement B while D0 remains entered and Q1/Q2 remain queued"
            }))
            .unwrap();
        assert_eq!(
            replacement["status"], "replacement_requested",
            "{replacement}"
        );
        let b_transition = replacement["lifecycle"]["outcome_ref"]
            .as_str()
            .unwrap()
            .to_owned();
        host_observations.push(replacement);
        fs::write(&d0_g1_release, b"release\n").unwrap();
        if hold_q1 {
            let until = Instant::now() + Duration::from_secs(60);
            while provider.entry_count() < 13 {
                assert!(Instant::now() < until, "held Q1 did not enter provider");
                thread::sleep(Duration::from_millis(20));
            }
            let held = client.observe(TASK_ID, Some(&q2_delivery)).unwrap();
            let segments = held["components"]["native_delivery_status"]["value"]["segments"]
                .as_array()
                .unwrap();
            assert_eq!(
                segments
                    .iter()
                    .filter(|segment| segment["delivery_id"] == delivery_id)
                    .count(),
                3
            );
            let q1_entered = segments
                .iter()
                .filter(|segment| segment["delivery_id"] == q1_delivery)
                .collect::<Vec<_>>();
            assert_eq!(q1_entered.len(), 1);
            assert_eq!(q1_entered[0]["segment_state"], "entered");
            assert_eq!(q1_entered[0]["send_intent"], true);
            assert!(
                segments
                    .iter()
                    .all(|segment| segment["delivery_id"] != q2_delivery)
            );
            assert_eq!(provider.entry_count(), 13, "Q2 overtook held Q1");
            host_observations.push(held);
            fs::write(&q1_release, b"release\n").unwrap();
        }
        let until = Instant::now() + Duration::from_secs(75);
        let mut readback_timeouts = 0_u32;
        loop {
            let current = match client.observe(TASK_ID, Some(&q2_delivery)) {
                Ok(current) => current,
                Err(error) => {
                    readback_timeouts += 1;
                    if Instant::now() >= until {
                        let root = fixture.root.path().to_path_buf();
                        let provider_path = provider_root.path().to_path_buf();
                        let provider_outcome = format!("{:?}", provider.finish());
                        std::mem::forget(fixture);
                        std::mem::forget(provider_root);
                        panic!(
                            "two-cycle readback unavailable; root={}, provider_root={}, provider_outcome={}, timeouts={}, final_error={error}",
                            root.display(),
                            provider_path.display(),
                            provider_outcome,
                            readback_timeouts
                        );
                    }
                    thread::sleep(Duration::from_millis(100));
                    continue;
                }
            };
            let segments = current["components"]["native_delivery_status"]["value"]["segments"]
                .as_array()
                .unwrap();
            let q1_segments = segments
                .iter()
                .filter(|segment| segment["delivery_id"] == q1_delivery)
                .collect::<Vec<_>>();
            let q2_segments = segments
                .iter()
                .filter(|segment| segment["delivery_id"] == q2_delivery)
                .collect::<Vec<_>>();
            let d0_segments = segments
                .iter()
                .filter(|segment| segment["delivery_id"] == delivery_id)
                .collect::<Vec<_>>();
            if current["components"]["lifecycle"]["value"]["transition"]["value"]["transition_id"]
                == b_transition
                && current["components"]["lifecycle"]["value"]["transition"]["value"]["completed"]
                    == true
                && current["components"]["lifecycle"]["value"]["delivery"]["value"]["outcome"]
                    == "completed"
                && d0_segments.len() == 3
                && q1_segments.len() == 1
                && q2_segments.len() == 1
                && d0_segments[2]["native_state"] == "completed_settled"
                && q1_segments[0]["native_state"] == "completed_settled"
                && q2_segments[0]["native_state"] == "completed_settled"
            {
                assert_eq!(d0_segments[0]["segment_state"], "yielded");
                assert_eq!(d0_segments[1]["segment_state"], "yielded");
                assert_eq!(d0_segments[2]["segment_state"], "completed");
                assert_eq!(d0_segments[2]["context_generation"], "context:s8:g2");
                assert_eq!(
                    d0_segments[2]["predecessor_invocation_id"],
                    d0_segments[1]["invocation_id"]
                );
                assert_eq!(q1_segments[0]["context_generation"], "context:s8:g2");
                assert_eq!(q2_segments[0]["context_generation"], "context:s8:g2");
                assert!(
                    d0_segments[0]["sequence"].as_i64().unwrap()
                        < q1_segments[0]["sequence"].as_i64().unwrap()
                        && q1_segments[0]["sequence"].as_i64().unwrap()
                            < q2_segments[0]["sequence"].as_i64().unwrap()
                );
                assert_eq!(
                    current["components"]["owner"]["value"]["completed_effects"]
                        .as_array()
                        .unwrap()
                        .len(),
                    1
                );
                observed = current;
                host_observations.push(observed.clone());
                break;
            }
            if Instant::now() >= until {
                let root = fixture.root.path().to_path_buf();
                let provider_path = provider_root.path().to_path_buf();
                let provider_outcome = format!("{:?}", provider.finish());
                let stage = current["components"]["lifecycle"]["value"]["transition"].clone();
                let native = current["components"]["native_delivery_status"]["value"].clone();
                std::mem::forget(fixture);
                std::mem::forget(provider_root);
                panic!(
                    "two-cycle FIFO failed; root={}, provider_root={}, provider_outcome={}, stage={}, native={}",
                    root.display(),
                    provider_path.display(),
                    provider_outcome,
                    stage,
                    native
                );
            }
            thread::sleep(Duration::from_millis(20));
        }
    }
    assert!(
        observed["components"]["lifecycle"]["value"].is_object(),
        "{observed}"
    );
    assert_eq!(
        HostReportedIdentity::from_reply(&observed).unwrap(),
        reported
    );
    host.assert_same_execution().unwrap();
    let host_identity = host.identity().clone();
    let host_identity_after = observe_process(host_identity.pid).unwrap();
    assert_eq!(host_identity, host_identity_after);
    let ledger_path = provider.ledger().to_path_buf();
    let close = host.terminate_and_reap(Duration::from_secs(15)).unwrap();
    assert!(close.reaped);
    let entries = provider.finish().unwrap();
    assert_eq!(
        entries.len(),
        if two_cycles {
            14
        } else if transition_a {
            7
        } else {
            2
        }
    );
    assert!(entries[1].request.to_string().contains(CHECKPOINT));
    assert!(fixture.root.path().is_dir());
    assert!(!fixture.sources.is_empty());
    fixture.source_manifest.verify().unwrap();
    if let Some(destination) = std::env::var_os("S8_EVIDENCE_DIR") {
        let destination = std::path::PathBuf::from(destination);
        assert!(destination.is_absolute());
        fs::create_dir_all(&destination).unwrap();
        let repo = common::repository_root();
        let contract_path = if two_cycles {
            "/home/bline/.local/state/work-engine/rust-context-lifecycle-s8-20261008/admission-contract.v5.md"
        } else {
            "/home/bline/.local/state/work-engine/rust-context-lifecycle-s8-20261008/admission-contract.v4.md"
        };
        let contract = ArtifactBinding::read(std::path::Path::new(contract_path)).unwrap();
        assert_eq!(
            contract.sha256,
            if two_cycles {
                "33e6d653bba08d89dbd3de968140d4d7e4f7c17a8807e627b2a73fe27243b230"
            } else {
                "2afeb81499867f59e5d3b21ce45ce5d7dc61cf9cf5b79702cee09cd497e491b1"
            }
        );
        let plan = if let Some(authority) = &to2_authority {
            authority.plan().unwrap()
        } else {
            let plan = ArtifactBinding::read(
                &repo.join("docs/rust-context-lifecycle-s8-implementation-plan.md"),
            )
            .unwrap();
            assert_eq!(
                plan.sha256,
                "c8b81575e8f29627664e5a07e76df5333dffc6d50cd802547dcf88f374be07b7"
            );
            plan
        };
        let evidence = EvidenceBundle {
            schema: if to2_authority.is_some() {
                TO2_RECEIPT_SCHEMA
            } else {
                "s8-external-proof-evidence-v1"
            }
            .into(),
            to2_authority: to2_authority.clone(),
            case_id: if hold_q1 {
                "s8-installed-two-cycle-q1-held-q2-blocked"
            } else if design_inquiry {
                "s8-installed-design-inquiry-snapshot-child"
            } else if two_cycles {
                "s8-installed-two-cycle-fifo"
            } else if transition_a {
                "s8-installed-transition-a"
            } else {
                "s8-installed-d0-snapshot-child"
            }
            .into(),
            contract,
            plan,
            host_executable: ArtifactBinding::read(&fixture.host_executable).unwrap(),
            source_manifest: fixture.source_manifest.clone(),
            sources: fixture.sources.clone(),
            cargo_lock: ArtifactBinding::read(&repo.join("rust/Cargo.lock")).unwrap(),
            toolchain: ArtifactBinding::read(&repo.join("rust/rust-toolchain.toml")).unwrap(),
            startup: ArtifactBinding::read(&fixture.startup_path).unwrap(),
            build_provenance: fixture.build_provenance.clone(),
            build_id: BUILD_ID.into(),
            proof_run_id: fixture.startup["lifecycle"]["proof_run_id"]
                .as_str()
                .unwrap()
                .into(),
            principal_ref: fixture.startup["lifecycle"]["principal"]["principal_ref"]
                .as_str()
                .unwrap()
                .into(),
            features: reported.features.clone(),
            profile: reported.profile.clone(),
            provider_identity: reported.provider_identity.clone(),
            disposable_root: fixture.root.path().to_path_buf(),
            subject_kind: if design_inquiry {
                "design_inquiry"
            } else {
                "repository_analysis"
            }
            .into(),
            host_identity_before: host_identity.clone(),
            host_identity_after,
            host_incarnation: reported.incarnation,
            provider_ledger: ArtifactBinding::read(&ledger_path).unwrap(),
            provider_entries: entries,
            provider_reply_artifacts: Vec::new(),
            fault_events: Vec::new(),
            lifecycle_observations: host_observations
                .iter()
                .map(|item| {
                    if to2_authority.is_some() {
                        to2_lifecycle_projection(item).unwrap()
                    } else if item["status"] == "observation" {
                        item["components"]["lifecycle"].clone()
                    } else {
                        item["lifecycle"].clone()
                    }
                })
                .collect(),
            host_observations,
            close,
        };
        let case = if hold_q1 {
            "two-cycle-q1-held"
        } else if design_inquiry {
            "design-inquiry"
        } else if two_cycles {
            "two-cycle-fifo"
        } else if transition_a {
            "transition-a"
        } else {
            "d0"
        };
        let path = destination.join(format!(
            "s8-{case}-{}.json",
            evidence.host_identity_before.pid
        ));
        let digest = evidence.write_once(&path).unwrap();
        eprintln!("S8 evidence receipt {} sha256 {digest}", path.display());
        let _ = fixture.root.keep();
        let _ = provider_root.keep();
    }
}
