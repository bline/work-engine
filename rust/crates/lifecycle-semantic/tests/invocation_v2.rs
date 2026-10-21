mod common;

use common::*;
use lifecycle_semantic::*;

fn fixture() -> (
    SourceBasis,
    DirectCandidate,
    DirectPreparedRequest,
    DirectVerifierResult,
    DirectExposure,
) {
    let material = basis("C1");
    let policy = SemanticPolicy::default();
    let scope = FixtureScope {
        owner: "b1b".into(),
        reference: "synthetic-c1".into(),
        sha256: sha256_hex(b"synthetic fixture C1"),
    };
    let raw = serde_json::to_vec(&complete_plan(&material)).unwrap();
    let candidate = synthetic_candidate(&material, raw, scope.clone(), &policy).unwrap();
    let binding = DirectInvocationBinding {
        subject: material.subject.clone(),
        transition: material.transition.clone(),
        capture_id: material.capture_id.clone(),
        phase: SemanticPhase::CandidateVerification,
        policy_id: material.policy_id.clone(),
        operation_id: "operation-1".into(),
        attempt_id: "attempt-1".into(),
        exposure_id: "exposure-1".into(),
        scope,
    };
    let request = direct_verify_candidate(&material, &candidate, binding, &policy).unwrap();
    let wire_bytes = b"synthetic wire bytes".to_vec();
    let prepared = DirectPreparedRequest {
        instructions: "synthetic instructions".into(),
        input: request.semantic_prompt.clone(),
        wire_sha256: sha256_hex(&wire_bytes),
        wire_bytes,
        profile_revision: "fixture-profile-r1".into(),
        requested_provider: "fixture-provider".into(),
        requested_model: "fixture-model".into(),
        binding_sha256: String::new(),
        request,
    }
    .bind()
    .unwrap();
    let raw = serde_json::to_vec(&report(&material, FindingStatus::Satisfied)).unwrap();
    let result = DirectVerifierResult {
        operation_id: prepared.request.binding.operation_id.clone(),
        attempt_id: prepared.request.binding.attempt_id.clone(),
        prepared_wire_sha256: prepared.wire_sha256.clone(),
        prepared_binding_sha256: prepared.binding_sha256.clone(),
        capture_ref: "fixture-capture".into(),
        captured_stream_sha256: sha256_hex(b"fixture stream"),
        semantic_prompt_sha256: prepared.request.semantic_prompt_sha256.clone(),
        response_id: Some("response-1".into()),
        observed_model: Some("fixture-model".into()),
        sha256: sha256_hex(&raw),
        raw,
    };
    let known_false = || Established::Known {
        value: false,
        evidence_ref: "fixture-observation".into(),
    };
    let exposure = DirectExposure {
        operation_id: result.operation_id.clone(),
        attempt_id: result.attempt_id.clone(),
        exposure_id: prepared.request.binding.exposure_id.clone(),
        prepared_wire_sha256: prepared.wire_sha256.clone(),
        prepared_binding_sha256: prepared.binding_sha256.clone(),
        candidate_origin: candidate.origin.clone(),
        source_basis_sha256: material.sha256.clone(),
        source_custody_ref: "fixture-source-custody".into(),
        capability_profile_revision: prepared.profile_revision.clone(),
        scope: prepared.request.binding.scope.clone(),
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
    (material, candidate, prepared, result, exposure)
}

fn status(value: AssessedResultEnvelope) -> VerdictStatus {
    match value.payload {
        NativeOrDirect::DirectV1(result) => result.status,
        _ => panic!("wrong tag"),
    }
}

#[test]
fn tags_are_explicit_and_native_v1_payload_is_unchanged() {
    let native = serde_json::from_slice::<Candidate>(include_bytes!(
        "fixtures/native-codec-v1/candidate.json"
    ))
    .unwrap();
    let envelope: CandidateEnvelope = VersionedPayload {
        schema_version: 2,
        payload: NativeOrDirect::NativeV1(native.clone()),
    };
    let encoded = serde_json::to_vec(&envelope).unwrap();
    let decoded: CandidateEnvelope = serde_json::from_slice(&encoded).unwrap();
    decoded.validate_version().unwrap();
    assert_eq!(decoded, envelope);
    assert!(serde_json::from_slice::<Candidate>(&encoded).is_err());
    assert_eq!(
        serde_json::to_vec(&native).unwrap(),
        include_bytes!("fixtures/native-codec-v1/candidate.json")
    );
    let unknown = serde_json::json!({"schema_version":2,"kind":"future","payload":native});
    assert!(serde_json::from_value::<CandidateEnvelope>(unknown).is_err());
    let mixed = serde_json::json!({"schema_version":2,"kind":"direct_v1","payload":native});
    assert!(serde_json::from_value::<CandidateEnvelope>(mixed).is_err());
    let wrong_version: CandidateEnvelope = VersionedPayload {
        schema_version: 3,
        payload: decoded.payload,
    };
    assert!(wrong_version.validate_version().is_err());
    assert!(
        serde_json::from_slice::<CandidateEnvelope>(&serde_json::to_vec(&wrong_version).unwrap())
            .is_err()
    );
}

#[test]
fn direct_fixture_reuses_report_derivation_without_compiler_session() {
    let (material, candidate, prepared, result, exposure) = fixture();
    assert!(matches!(
        candidate.origin,
        CandidateOrigin::SyntheticFixture { .. }
    ));
    assert_eq!(
        status(
            assess_direct_candidate(
                &material,
                &candidate,
                &prepared,
                result,
                exposure,
                &SemanticPolicy::default()
            )
            .unwrap()
        ),
        VerdictStatus::Accepted
    );
}

#[test]
fn swaps_fail_even_with_identical_raw_candidate_or_wire_bytes() {
    let (material, candidate, prepared, result, exposure) = fixture();
    let policy = SemanticPolicy::default();
    let mut changed = candidate.clone();
    changed.origin = CandidateOrigin::SyntheticFixture {
        owner: "b1b".into(),
        reference: "other".into(),
        sha256: sha256_hex(b"other"),
    };
    assert!(
        assess_direct_candidate(
            &material,
            &changed,
            &prepared,
            result.clone(),
            exposure.clone(),
            &policy
        )
        .is_err()
    );
    let mut changed = prepared.clone();
    changed.request.binding.attempt_id = "attempt-2".into();
    assert!(
        assess_direct_candidate(
            &material,
            &candidate,
            &changed,
            result.clone(),
            exposure.clone(),
            &policy
        )
        .is_err()
    );
    let mut changed = prepared.clone();
    changed.request.binding.exposure_id = "exposure-2".into();
    assert!(
        assess_direct_candidate(
            &material,
            &candidate,
            &changed,
            result.clone(),
            exposure.clone(),
            &policy
        )
        .is_err()
    );
    let mut changed = prepared.clone();
    changed.instructions.push('x');
    // Exact wire assembly is additionally checked by the replay adapter.
    assert!(changed.instructions != prepared.instructions);
    let mut changed = prepared.clone();
    changed.input.push('x');
    assert!(
        assess_direct_candidate(
            &material,
            &candidate,
            &changed,
            result.clone(),
            exposure.clone(),
            &policy
        )
        .is_err()
    );
    let mut changed = result.clone();
    changed.attempt_id = "attempt-2".into();
    assert!(
        assess_direct_candidate(
            &material,
            &candidate,
            &prepared,
            changed,
            exposure.clone(),
            &policy
        )
        .is_err()
    );
    let mut changed = exposure.clone();
    changed.scope.reference = "other".into();
    assert!(
        assess_direct_candidate(&material, &candidate, &prepared, result, changed, &policy)
            .is_err()
    );
}

#[test]
fn unknown_or_forbidden_exposure_cannot_establish_fixture_acceptance() {
    let (material, candidate, prepared, result, exposure) = fixture();
    let policy = SemanticPolicy::default();
    let variants: Vec<DirectExposure> = [
        {
            let mut x = exposure.clone();
            x.compiler_reasoning_exposed = Established::Unknown;
            x
        },
        {
            let mut x = exposure.clone();
            x.hidden_answer_exposed = Established::Unknown;
            x
        },
        {
            let mut x = exposure.clone();
            x.dynamic_tools_enabled = Established::Unknown;
            x
        },
        {
            let mut x = exposure.clone();
            x.human_side_effects_enabled = Established::Unknown;
            x
        },
        {
            let mut x = exposure.clone();
            x.observed_provider = Established::Unknown;
            x
        },
        {
            let mut x = exposure.clone();
            x.observed_model = Established::Unknown;
            x
        },
        {
            let mut x = exposure.clone();
            x.compiler_reasoning_exposed = Established::Known {
                value: true,
                evidence_ref: "fixture-observation".into(),
            };
            x
        },
        {
            let mut x = exposure.clone();
            x.observed_provider = Established::Known {
                value: "wrong-provider".into(),
                evidence_ref: "fixture-observation".into(),
            };
            x
        },
    ]
    .into();
    for variant in variants {
        assert_eq!(
            status(
                assess_direct_candidate(
                    &material,
                    &candidate,
                    &prepared,
                    result.clone(),
                    variant,
                    &policy
                )
                .unwrap()
            ),
            VerdictStatus::Unresolved
        );
    }
    let mut absent_model = result;
    absent_model.observed_model = None;
    assert_eq!(
        status(
            assess_direct_candidate(
                &material,
                &candidate,
                &prepared,
                absent_model,
                exposure,
                &policy
            )
            .unwrap()
        ),
        VerdictStatus::Unresolved
    );
}

#[test]
fn report_findings_preserve_rejection_and_unresolved_precedence() {
    let (material, candidate, prepared, mut result, exposure) = fixture();
    let policy = SemanticPolicy::default();
    let mut invalid = report(&material, FindingStatus::Satisfied);
    invalid.findings.pop();
    result.raw = serde_json::to_vec(&invalid).unwrap();
    result.sha256 = sha256_hex(&result.raw);
    assert_eq!(
        status(
            assess_direct_candidate(
                &material,
                &candidate,
                &prepared,
                result.clone(),
                exposure.clone(),
                &policy
            )
            .unwrap()
        ),
        VerdictStatus::Unresolved
    );
    invalid.findings[0].status = FindingStatus::Violated;
    result.raw = serde_json::to_vec(&invalid).unwrap();
    result.sha256 = sha256_hex(&result.raw);
    assert_eq!(
        status(
            assess_direct_candidate(&material, &candidate, &prepared, result, exposure, &policy)
                .unwrap()
        ),
        VerdictStatus::Rejected
    );
}

#[test]
fn malformed_attribution_and_duplicate_findings_cannot_accept() {
    let (material, candidate, prepared, mut result, exposure) = fixture();
    let policy = SemanticPolicy::default();
    let mut report = report(&material, FindingStatus::Satisfied);
    report.findings.push(report.findings[0].clone());
    result.raw = serde_json::to_vec(&report).unwrap();
    result.sha256 = sha256_hex(&result.raw);
    assert_eq!(
        status(
            assess_direct_candidate(
                &material,
                &candidate,
                &prepared,
                result.clone(),
                exposure.clone(),
                &policy
            )
            .unwrap()
        ),
        VerdictStatus::Unresolved
    );
    report.findings.pop();
    report.findings[0].sources[0].owner_ref = "wrong-source".into();
    result.raw = serde_json::to_vec(&report).unwrap();
    result.sha256 = sha256_hex(&result.raw);
    assert_eq!(
        status(
            assess_direct_candidate(&material, &candidate, &prepared, result, exposure, &policy)
                .unwrap()
        ),
        VerdictStatus::Unresolved
    );
}

#[test]
fn unproduced_direct_compiler_coordinates_are_refused() {
    let (material, mut candidate, prepared, result, exposure) = fixture();
    candidate.origin = CandidateOrigin::DirectCompilation {
        operation_id: "compile-operation".into(),
        attempt_id: "compile-attempt".into(),
        prepared_request_sha256: sha256_hex(b"some request"),
    };
    assert!(
        assess_direct_candidate(
            &material,
            &candidate,
            &prepared,
            result,
            exposure,
            &SemanticPolicy::default(),
        )
        .is_err()
    );
}

#[test]
fn old_result_rejects_coherent_non_wire_preparation_swaps() {
    let (material, candidate, prepared, result, exposure) = fixture();
    let policy = SemanticPolicy::default();
    let cases = [
        {
            let mut p = prepared.clone();
            let mut e = exposure.clone();
            p.profile_revision = "fixture-profile-r2".into();
            e.capability_profile_revision = p.profile_revision.clone();
            (p.bind().unwrap(), e)
        },
        {
            let mut p = prepared.clone();
            let mut e = exposure.clone();
            p.request.binding.exposure_id = "exposure-2".into();
            e.exposure_id = "exposure-2".into();
            (p.bind().unwrap(), e)
        },
        {
            let mut p = prepared.clone();
            let mut e = exposure.clone();
            p.requested_provider = "other-provider".into();
            e.observed_provider = Established::Known {
                value: "other-provider".into(),
                evidence_ref: "fixture-observation".into(),
            };
            (p.bind().unwrap(), e)
        },
        {
            let mut p = prepared.clone();
            p.instructions = "different instructions".into();
            (p.bind().unwrap(), exposure.clone())
        },
    ];
    for (changed, mut changed_exposure) in cases {
        assert_eq!(changed.wire_bytes, prepared.wire_bytes);
        assert_ne!(changed.binding_sha256, prepared.binding_sha256);
        changed_exposure.prepared_binding_sha256 = changed.binding_sha256.clone();
        assert!(
            assess_direct_candidate(
                &material,
                &candidate,
                &changed,
                result.clone(),
                changed_exposure,
                &policy,
            )
            .is_err()
        );
    }
    let mut changed_candidate = candidate.clone();
    let scope = FixtureScope {
        owner: "other-owner".into(),
        reference: "other-fixture".into(),
        sha256: sha256_hex(b"other scope"),
    };
    changed_candidate.origin = CandidateOrigin::SyntheticFixture {
        owner: scope.owner.clone(),
        reference: scope.reference.clone(),
        sha256: scope.sha256.clone(),
    };
    let mut changed = prepared.clone();
    changed.request.binding.scope = scope.clone();
    changed.request.candidate_origin = changed_candidate.origin.clone();
    changed = changed.bind().unwrap();
    let mut changed_exposure = exposure;
    changed_exposure.scope = scope;
    changed_exposure.candidate_origin = changed_candidate.origin.clone();
    changed_exposure.prepared_binding_sha256 = changed.binding_sha256.clone();
    assert_eq!(changed.wire_bytes, prepared.wire_bytes);
    assert!(
        assess_direct_candidate(
            &material,
            &changed_candidate,
            &changed,
            result,
            changed_exposure,
            &policy,
        )
        .is_err()
    );
}

#[test]
fn v2_codec_rejects_mixed_nested_provenance_without_changing_legacy_decode() {
    let native: serde_json::Value =
        serde_json::from_slice(include_bytes!("fixtures/native-codec-v1/candidate.json")).unwrap();
    let (material, candidate, _, _, _) = fixture();
    let mut mixed_native = native.clone();
    mixed_native["origin"] = serde_json::to_value(&candidate.origin).unwrap();
    let tagged = serde_json::json!({"schema_version":2,"kind":"native_v1","payload":mixed_native});
    assert!(serde_json::from_value::<CandidateEnvelope>(tagged).is_err());
    assert!(serde_json::from_value::<Candidate>(native).is_ok());
    let mixed_origin = serde_json::json!({
        "kind":"synthetic_fixture","owner":"b1b","reference":"synthetic-c1",
        "sha256":sha256_hex(b"fixture"),"operation_id":"compiler-operation",
        "attempt_id":"compiler-attempt","prepared_request_sha256":sha256_hex(b"compiler request")
    });
    assert!(serde_json::from_value::<CandidateOrigin>(mixed_origin).is_err());

    let binding: serde_json::Value =
        serde_json::from_slice(include_bytes!("fixtures/native-codec-v1/binding.json")).unwrap();
    let mut malformed = binding;
    malformed["session_id"] = serde_json::json!("");
    let envelope: InvocationEnvelope = serde_json::from_value(serde_json::json!({
        "schema_version":2,"kind":"native_v1","payload":malformed
    }))
    .unwrap();
    match envelope.payload {
        NativeOrDirect::NativeV1(native) => assert!(
            native
                .validate(&material, SemanticPhase::CandidateVerification)
                .is_err()
        ),
        _ => panic!("wrong variant"),
    }
}

#[test]
fn v2_codec_rejects_duplicate_keys_before_json_normalization() {
    let native = include_str!("fixtures/native-codec-v1/candidate.json");
    let duplicate_native = native.replacen(
        "\"compile_session_id\":",
        "\"compile_session_id\":\"contradictory-session\",\"compile_session_id\":",
        1,
    );
    assert!(serde_json::from_str::<Candidate>(&duplicate_native).is_err());
    let wrapped =
        format!("{{\"schema_version\":2,\"kind\":\"native_v1\",\"payload\":{duplicate_native}}}");
    assert!(serde_json::from_str::<CandidateEnvelope>(&wrapped).is_err());

    let (_, direct, _, _, _) = fixture();
    let direct = serde_json::to_string(&direct).unwrap();
    let duplicate_origin = direct.replacen(
        "\"kind\":\"synthetic_fixture\"",
        "\"kind\":\"direct_compilation\",\"kind\":\"synthetic_fixture\"",
        1,
    );
    assert!(serde_json::from_str::<DirectCandidate>(&duplicate_origin).is_err());
    let wrapped =
        format!("{{\"schema_version\":2,\"kind\":\"direct_v1\",\"payload\":{duplicate_origin}}}");
    assert!(serde_json::from_str::<CandidateEnvelope>(&wrapped).is_err());

    let (_, direct, _, _, _) = fixture();
    let mut direct = serde_json::to_string(&direct).unwrap();
    direct = direct.replacen("\"text\":", "\"text\":\"forged\",\"text\":", 1);
    let wrapped = format!("{{\"schema_version\":2,\"kind\":\"direct_v1\",\"payload\":{direct}}}");
    assert!(serde_json::from_str::<CandidateEnvelope>(&wrapped).is_err());
}
