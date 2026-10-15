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
use serde_json::json;

use common::{ANSWER_AUTHORITY, BUILD_ID, TASK_ID, create_fixture, initial_submit};

const QUESTION_ID: &str = "question:s8:pending";
const SCOPE: &str = "scope:s8:analysis";
const ANSWER: &str = "The approved snapshot permits attributed analysis only; repository mutation still awaits approval.";

#[test]
fn installed_pending_question_requires_separate_authorized_answer() {
    let to2_authority =
        to2_authority_from_environment("s8-installed-pending-question-authorized-answer").unwrap();
    let tmp = std::env::var_os("TMPDIR").expect("TMPDIR must be pinned for S8 proof");
    let build_provenance = common::preflight_installed_host();
    let provider_root = tempfile::Builder::new()
        .prefix("s8-provider-question-")
        .tempdir_in(tmp)
        .unwrap();
    let final_release = provider_root.path().join("release-final");
    let provider = ScriptedProvider::start(
        vec![
            Step {
                required: vec!["input:command:s8:d0".into()],
                forbidden: Vec::new(),
                reply: Reply::ToolCall {
                    call_id: "call:s8:question".into(),
                    namespace: "task".into(),
                    name: "question_open".into(),
                    arguments: json!({
                        "question_id":QUESTION_ID,
                        "recipient_ref":ANSWER_AUTHORITY,
                        "scope_ref":SCOPE,
                        "text":"May this task mutate the repository?"
                    }),
                },
                release_path: None,
            },
            Step {
                required: vec![QUESTION_ID.into()],
                forbidden: Vec::new(),
                reply: Reply::Text(
                    "The question is pending with the named recipient and scope. Continue analysis without repository mutation."
                        .into(),
                ),
                release_path: Some(final_release.clone()),
            },
        ],
        provider_root.path().join("provider-ledger.jsonl"),
    )
    .unwrap();
    let fixture = create_fixture(&provider.base_url(), build_provenance);
    let mut host = HostProcess::launch(&fixture.host_executable, &fixture.startup_path).unwrap();
    let client = HostClient::new(&fixture.host_socket, Duration::from_secs(5)).unwrap();
    client.await_ready(Duration::from_secs(10)).unwrap();
    host.assert_same_execution().unwrap();

    let admitted = client.submit(initial_submit()).unwrap();
    assert_eq!(admitted["status"], "admitted", "{admitted}");
    assert_eq!(admitted["lifecycle"]["outcome_kind"], "enqueued");
    let delivery_id = admitted["lifecycle"]["outcome_ref"]
        .as_str()
        .unwrap()
        .to_owned();
    let reported = HostReportedIdentity::from_reply(&admitted).unwrap();
    reported.bind_process(host.identity(), BUILD_ID).unwrap();
    reported.bind_startup(&fixture.startup).unwrap();
    let mut observations = vec![admitted];

    let until = Instant::now() + Duration::from_secs(30);
    let mut last_readback = String::new();
    let pending = loop {
        if provider.entry_count() >= 1 {
            match client.observe(TASK_ID, Some(&delivery_id)) {
                Ok(readback) => {
                    if readback["components"]["owner"]["value"]["questions"][0]["state"]
                        == "pending"
                    {
                        break readback;
                    }
                    last_readback = readback.to_string();
                }
                Err(error) => last_readback = error.to_string(),
            }
        }
        if Instant::now() >= until {
            let root = fixture.root.path().to_path_buf();
            let provider_path = provider_root.path().to_path_buf();
            let entries = provider.entry_count();
            let provider_outcome = format!("{:?}", provider.finish());
            std::mem::forget(fixture);
            std::mem::forget(provider_root);
            panic!(
                "question did not reach owner pending state; root={}, provider_root={}, entries={}, provider_outcome={}, last_readback={last_readback}",
                root.display(),
                provider_path.display(),
                entries,
                provider_outcome
            );
        }
        thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(
        pending["components"]["owner"]["value"]["questions"][0]["id"],
        QUESTION_ID
    );
    assert_eq!(
        pending["components"]["owner"]["value"]["questions"][0]["recipient_ref"],
        ANSWER_AUTHORITY
    );
    assert_eq!(
        pending["components"]["owner"]["value"]["questions"][0]["scope_ref"],
        SCOPE
    );
    assert!(pending["components"]["owner"]["value"]["result"].is_null());
    observations.push(pending.clone());
    common::ui_observe(
        &fixture,
        json!({"task_id":TASK_ID,"delivery_id":delivery_id}),
        "question",
        pending["host_identity"].clone(),
    );
    let revision = pending["components"]["owner"]["value"]["revision"]
        .as_u64()
        .unwrap();

    let rejected = client
        .answer(json!({
            "command_id":"command:s8:answer-wrong-authority",
            "task_id":TASK_ID,"question_id":QUESTION_ID,
            "recipient_ref":ANSWER_AUTHORITY,"scope_ref":SCOPE,
            "answer":ANSWER,"expected_revision":revision,
            "authority_ref":"operator:s8:untrusted"
        }))
        .unwrap();
    assert_eq!(rejected["status"], "error", "{rejected}");
    let still_pending = client.observe(TASK_ID, Some(&delivery_id)).unwrap();
    assert_eq!(
        still_pending["components"]["owner"]["value"]["revision"],
        revision
    );
    assert_eq!(
        still_pending["components"]["owner"]["value"]["questions"][0]["state"],
        "pending"
    );
    observations.push(still_pending);

    let answered = client
        .answer(json!({
            "command_id":"command:s8:answer-authorized",
            "task_id":TASK_ID,"question_id":QUESTION_ID,
            "recipient_ref":ANSWER_AUTHORITY,"scope_ref":SCOPE,
            "answer":ANSWER,"expected_revision":revision,
            "authority_ref":ANSWER_AUTHORITY
        }))
        .unwrap();
    assert_eq!(answered["status"], "answered", "{answered}");
    assert_eq!(answered["owner"]["owner_revision"], revision + 1);
    observations.push(answered);
    fs::write(&final_release, b"release\n").unwrap();

    let until = Instant::now() + Duration::from_secs(30);
    let complete = loop {
        if let Ok(readback) = client.observe(TASK_ID, Some(&delivery_id))
            && readback["components"]["lifecycle"]["value"]["delivery"]["value"]["outcome"]
                == "completed"
        {
            break readback;
        }
        assert!(Instant::now() < until, "question delivery did not complete");
        thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(
        complete["components"]["owner"]["value"]["questions"][0]["state"]["answered"]["answer"],
        ANSWER
    );
    assert_eq!(
        complete["components"]["owner"]["value"]["questions"][0]["state"]["answered"]["authority_ref"],
        ANSWER_AUTHORITY
    );
    assert_eq!(
        complete["components"]["owner"]["value"]["revision"],
        revision + 1
    );
    assert_eq!(
        complete["components"]["native_delivery_status"]["value"]["segments"][0]["segment_state"],
        "completed"
    );
    assert!(
        complete["components"]["owner"]["value"]["sources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|source| {
                source["id"] == format!("question-{QUESTION_ID}")
                    && serde_json::from_value::<Vec<u8>>(source["bytes"].clone())
                        .ok()
                        .and_then(|bytes| String::from_utf8(bytes).ok())
                        .is_some_and(|text| text.contains(ANSWER))
            })
    );
    observations.push(complete.clone());
    assert_eq!(
        HostReportedIdentity::from_reply(&complete).unwrap(),
        reported
    );
    host.assert_same_execution().unwrap();
    let host_identity = host.identity().clone();
    let host_identity_after = observe_process(host_identity.pid).unwrap();
    let ledger_path = provider.ledger().to_path_buf();
    let close = host.terminate_and_reap(Duration::from_secs(15)).unwrap();
    let entries = provider.finish().unwrap();
    assert_eq!(entries.len(), 2);
    assert!(close.reaped);

    if let Some(destination) = std::env::var_os("S8_EVIDENCE_DIR") {
        let destination = std::path::PathBuf::from(destination);
        assert!(destination.is_absolute());
        fs::create_dir_all(&destination).unwrap();
        let repo = common::repository_root();
        let contract = ArtifactBinding::read(std::path::Path::new(
            "/home/bline/.local/state/work-engine/rust-context-lifecycle-s8-20261008/admission-contract.v5.md",
        )).unwrap();
        assert_eq!(
            contract.sha256,
            "33e6d653bba08d89dbd3de968140d4d7e4f7c17a8807e627b2a73fe27243b230"
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
            case_id: "s8-installed-pending-question-authorized-answer".into(),
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
            subject_kind: "repository_analysis".into(),
            host_identity_before: host_identity,
            host_identity_after,
            host_incarnation: reported.incarnation,
            provider_ledger: ArtifactBinding::read(&ledger_path).unwrap(),
            provider_entries: entries,
            provider_reply_artifacts: Vec::new(),
            fault_events: Vec::new(),
            lifecycle_observations: observations
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
            host_observations: observations,
            close,
        };
        let path = destination.join(format!(
            "s8-question-answer-{}.json",
            evidence.host_identity_before.pid
        ));
        let digest = evidence.write_once(&path).unwrap();
        eprintln!("S8 evidence receipt {} sha256 {digest}", path.display());
        let _ = fixture.root.keep();
        let _ = provider_root.keep();
    }
}
