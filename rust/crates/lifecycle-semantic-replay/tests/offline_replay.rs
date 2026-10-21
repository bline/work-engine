use bounded_inference::{Bounds, ToolFreeProfile, assemble_request};
use lifecycle_core::{SubjectId, TransitionId};
use lifecycle_semantic::*;
use lifecycle_semantic_replay::*;
use serde_json::{Value, json};
use std::io::Write;
use std::process::{Command, Stdio};

fn event(value: Value) -> Vec<u8> {
    format!(
        "event: {}\ndata: {}\n\n",
        value["type"].as_str().unwrap(),
        value
    )
    .into_bytes()
}

fn stream(status: &str, output: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut push = |value| bytes.extend(event(value));
    push(
        json!({"type":"response.created","sequence_number":1,"response":{"id":"response-fixture","model":"fixture-model"}}),
    );
    push(
        json!({"type":"response.output_item.added","sequence_number":2,"output_index":0,"item":{"id":"item-fixture","type":"message"}}),
    );
    push(
        json!({"type":"response.output_text.delta","sequence_number":3,"output_index":0,"content_index":0,"item_id":"item-fixture","delta":output}),
    );
    push(
        json!({"type":"response.output_text.done","sequence_number":4,"output_index":0,"content_index":0,"item_id":"item-fixture","text":output}),
    );
    let mut response = json!({"id":"response-fixture","model":"fixture-model","status":status});
    if status == "completed" {
        response["output"] = json!([{"id":"item-fixture","type":"message","role":"assistant","content":[{"type":"output_text","text":output}]}]);
    } else if status == "failed" {
        response["error"] = json!({"code":"fixture-failure"});
    } else {
        response["incomplete_details"] = json!({"reason":"fixture-limit"});
    }
    push(json!({"type":format!("response.{status}"),"sequence_number":5,"response":response}));
    bytes
}

fn fixture() -> OfflineReplayInput {
    let policy = SemanticPolicy::default();
    let source = SourceMaterial {
        id: "governing".into(),
        owner_namespace: "fixture-owner".into(),
        owner_ref: "rule-1".into(),
        revision: 1,
        dependency_key: "rule-1".into(),
        current: true,
        dependency_required: true,
        kind: SourceKind::Governing,
        trust: SourceTrust::HostGoverning,
        availability: SourceAvailability::Complete,
        bytes: b"Finish the bounded fixture".to_vec(),
        sha256: sha256_hex(b"Finish the bounded fixture"),
    };
    let reference = source.reference();
    let material = project_material(MaterialInput {
        subject: SubjectId::parse("fixture-subject").unwrap(),
        transition: TransitionId::parse("fixture-transition").unwrap(),
        capture_id: "fixture-capture".into(),
        sources: vec![source],
        obligations: vec![RequiredObligation {
            id: "obligation-1".into(),
            kind: ObligationKind::Commitment,
            verifier_scope: "preserve the stated fixture task".into(),
            description: "synthetic expected consequence".into(),
            sources: vec![reference.clone()],
        }],
        policy: policy.clone(),
    })
    .unwrap();
    let statement = AttributedStatement {
        text: "Finish the bounded fixture".into(),
        sources: vec![reference.clone()],
    };
    let plan = ContinuationPlan {
        schema_version: 1,
        objective: statement.clone(),
        work_position: statement.clone(),
        completed: vec![],
        commitments: vec![],
        decisions: vec![],
        uncertainties: vec![],
        interactions: vec![],
        governing: vec![statement.clone()],
        canonical_refs: vec![],
        delivery_custody: vec![],
        next_action: statement,
    };
    let scope = FixtureScope {
        owner: "b1b".into(),
        reference: "replay-1".into(),
        sha256: sha256_hex(b"replay fixture one"),
    };
    let candidate = synthetic_candidate(
        &material,
        serde_json::to_vec(&plan).unwrap(),
        scope.clone(),
        &policy,
    )
    .unwrap();
    let request = direct_verify_candidate(
        &material,
        &candidate,
        DirectInvocationBinding {
            subject: material.subject.clone(),
            transition: material.transition.clone(),
            capture_id: material.capture_id.clone(),
            phase: SemanticPhase::CandidateVerification,
            policy_id: material.policy_id.clone(),
            operation_id: "operation-1".into(),
            attempt_id: "attempt-1".into(),
            exposure_id: "exposure-1".into(),
            scope: scope.clone(),
        },
        &policy,
    )
    .unwrap();
    let profile = ToolFreeProfile {
        revision: "fixture-profile-r1".into(),
        model: "fixture-model".into(),
        bounds: Bounds {
            request_bytes: 100_000,
            stream_bytes: 200_000,
            frame_bytes: 100_000,
            output_bytes: 100_000,
            events: 16,
            local_deadline_ms: 1000,
        },
    };
    let instructions = "Assess the fixture report";
    let assembled = assemble_request(instructions, &request.semantic_prompt, &profile).unwrap();
    let prepared = DirectPreparedRequest {
        request,
        instructions: instructions.into(),
        input: assembled_input(&assembled.bytes),
        wire_bytes: assembled.bytes,
        wire_sha256: assembled.sha256,
        profile_revision: assembled.profile_revision,
        requested_provider: "fixture-provider".into(),
        requested_model: assembled.selected_model,
        binding_sha256: String::new(),
    }
    .bind()
    .unwrap();
    let known_false = || Established::Known {
        value: false,
        evidence_ref: "fixture-observation".into(),
    };
    let exposure = DirectExposure {
        operation_id: "operation-1".into(),
        attempt_id: "attempt-1".into(),
        exposure_id: "exposure-1".into(),
        prepared_wire_sha256: prepared.wire_sha256.clone(),
        prepared_binding_sha256: prepared.binding_sha256.clone(),
        candidate_origin: candidate.origin.clone(),
        source_basis_sha256: material.sha256.clone(),
        source_custody_ref: "fixture-custody".into(),
        capability_profile_revision: profile.revision.clone(),
        scope,
        compiler_reasoning_exposed: known_false(),
        hidden_answer_exposed: known_false(),
        dynamic_tools_enabled: known_false(),
        human_side_effects_enabled: known_false(),
        observed_provider: Established::Known {
            value: "fixture-provider".into(),
            evidence_ref: "fixture-observation".into(),
        },
        observed_model: Established::Known {
            value: "fixture-model".into(),
            evidence_ref: "fixture-observation".into(),
        },
    };
    let report = VerifierReport {
        whole_response_assessed: true,
        findings: vec![AttributedFinding {
            obligation_id: "obligation-1".into(),
            status: FindingStatus::Satisfied,
            sources: vec![reference],
            explanation: "Fixture task retained".into(),
        }],
        critical_errors: vec![],
    };
    let capture = CapturedFixtureStream::new(
        "capture-replay-1".into(),
        &prepared,
        stream("completed", &serde_json::to_string(&report).unwrap()),
    )
    .unwrap();
    OfflineReplayInput {
        material,
        candidate,
        prepared,
        exposure,
        profile,
        capture,
    }
}

fn assembled_input(bytes: &[u8]) -> String {
    let value: Value = serde_json::from_slice(bytes).unwrap();
    value["input"][0]["content"][0]["text"]
        .as_str()
        .unwrap()
        .into()
}

#[test]
fn complete_stream_reaches_common_semantic_consumer_with_fixture_scope() {
    let input = fixture();
    let verdict = replay_fixture(&input, &SemanticPolicy::default()).unwrap();
    match verdict.payload {
        NativeOrDirect::DirectV1(result) => {
            assert_eq!(result.status, VerdictStatus::Accepted);
            assert_eq!(result.scope.reference, "replay-1");
            assert_eq!(result.binding.operation_id, "operation-1");
            assert_eq!(result.response_id.as_deref(), Some("response-fixture"));
        }
        _ => panic!("native verdict from direct replay"),
    }
}

#[test]
fn executable_reads_fixture_stdin_and_emits_tagged_verdict() {
    let input = fixture();
    let mut child = Command::new(env!("CARGO_BIN_EXE_lifecycle-semantic-replay"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&input).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let verdict: AssessedResultEnvelope = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        matches!(verdict.payload, NativeOrDirect::DirectV1(result) if result.status == VerdictStatus::Accepted)
    );
}

#[test]
fn noncompleted_or_changed_wire_never_enters_report_assessment() {
    let valid = fixture();
    let mut truncated = valid.clone();
    truncated
        .capture
        .stream_bytes
        .truncate(truncated.capture.stream_bytes.len() / 2);
    assert!(replay_fixture(&truncated, &SemanticPolicy::default()).is_err());
    for status in ["failed", "incomplete"] {
        let mut changed = valid.clone();
        changed.capture = CapturedFixtureStream::new(
            format!("capture-{status}"),
            &changed.prepared,
            stream(status, "{\"whole_response_assessed\":true}"),
        )
        .unwrap();
        assert!(matches!(
            replay_fixture(&changed, &SemanticPolicy::default()),
            Err(ReplayError::TransportOutcome)
        ));
    }
    let mut changed = valid.clone();
    changed.prepared.instructions.push('x');
    assert!(matches!(
        replay_fixture(&changed, &SemanticPolicy::default()),
        Err(ReplayError::Semantic(SemanticError::WrongBinding))
    ));
    let mut changed = valid.clone();
    changed.profile.revision = "different-profile".into();
    assert!(matches!(
        replay_fixture(&changed, &SemanticPolicy::default()),
        Err(ReplayError::PreparedRequestMismatch)
    ));
    let mut changed = valid.clone();
    changed.profile.model = "different-model".into();
    assert!(matches!(
        replay_fixture(&changed, &SemanticPolicy::default()),
        Err(ReplayError::PreparedRequestMismatch)
    ));
    let mut changed = valid.clone();
    changed.prepared.wire_bytes.push(b'x');
    assert!(replay_fixture(&changed, &SemanticPolicy::default()).is_err());
    let mut changed = valid;
    changed.capture.stream_bytes.extend(stream("failed", ""));
    assert!(replay_fixture(&changed, &SemanticPolicy::default()).is_err());
}

#[test]
fn unchanged_capture_cannot_be_relabelled_as_retry_or_different_candidate() {
    let base = fixture();
    let mut retry = base.clone();
    retry.prepared.request.binding.operation_id = "operation-2".into();
    retry.prepared.request.binding.attempt_id = "attempt-2".into();
    retry.prepared = retry.prepared.bind().unwrap();
    retry.exposure.operation_id = "operation-2".into();
    retry.exposure.attempt_id = "attempt-2".into();
    retry.exposure.prepared_binding_sha256 = retry.prepared.binding_sha256.clone();
    assert_eq!(retry.capture.stream_bytes, base.capture.stream_bytes);
    assert!(matches!(
        replay_fixture(&retry, &SemanticPolicy::default()),
        Err(ReplayError::CaptureMismatch)
    ));

    let mut swapped = base.clone();
    let mut plan = swapped.candidate.plan.clone();
    plan.next_action.text = "Abandon the bounded fixture forever".into();
    swapped.candidate = synthetic_candidate(
        &swapped.material,
        serde_json::to_vec(&plan).unwrap(),
        swapped.prepared.request.binding.scope.clone(),
        &SemanticPolicy::default(),
    )
    .unwrap();
    swapped.prepared.request = direct_verify_candidate(
        &swapped.material,
        &swapped.candidate,
        swapped.prepared.request.binding.clone(),
        &SemanticPolicy::default(),
    )
    .unwrap();
    swapped.prepared.input = swapped.prepared.request.semantic_prompt.clone();
    let assembled = assemble_request(
        &swapped.prepared.instructions,
        &swapped.prepared.input,
        &swapped.profile,
    )
    .unwrap();
    swapped.prepared.wire_bytes = assembled.bytes;
    swapped.prepared.wire_sha256 = assembled.sha256;
    swapped.prepared = swapped.prepared.bind().unwrap();
    swapped.exposure.candidate_origin = swapped.candidate.origin.clone();
    swapped.exposure.prepared_wire_sha256 = swapped.prepared.wire_sha256.clone();
    swapped.exposure.prepared_binding_sha256 = swapped.prepared.binding_sha256.clone();
    assert_eq!(swapped.capture.stream_bytes, base.capture.stream_bytes);
    assert!(matches!(
        replay_fixture(&swapped, &SemanticPolicy::default()),
        Err(ReplayError::CaptureMismatch)
    ));
}
