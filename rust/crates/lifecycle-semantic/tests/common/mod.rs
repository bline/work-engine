#![allow(dead_code)]

use lifecycle_core::{NativeInvocationPurpose, SubjectId, TransitionId};
use lifecycle_semantic::*;
use serde_json::Value;

pub fn corpus() -> Value {
    serde_json::from_str(include_str!(
        "../../../../tests/fixtures/s67-preparation/semantic-corpus.v1.json"
    ))
    .unwrap()
}

pub fn case(id: &str) -> Value {
    corpus()["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["id"] == id)
        .unwrap()
        .clone()
}

fn source_kind(id: &str) -> (SourceKind, SourceTrust) {
    if id == "governing" {
        (SourceKind::Governing, SourceTrust::HostGoverning)
    } else if id.starts_with("question-") {
        (SourceKind::HumanInteraction, SourceTrust::HostCanonical)
    } else if id.starts_with("decision-") {
        (SourceKind::Decision, SourceTrust::HostCanonical)
    } else if id.starts_with("grant-") {
        (
            SourceKind::GrantReference,
            SourceTrust::HistoricalAuthorityReference,
        )
    } else if id.starts_with("reference-") {
        (SourceKind::CanonicalRecord, SourceTrust::HostCanonical)
    } else if id.starts_with("delivery-") || id.starts_with("queue-") {
        (SourceKind::DeliveryConsequence, SourceTrust::HostCanonical)
    } else if id == "untrusted-note" {
        (SourceKind::ExternalData, SourceTrust::UntrustedData)
    } else if id.starts_with("native-") {
        (SourceKind::NativeHistory, SourceTrust::HostCanonical)
    } else {
        (SourceKind::CanonicalRecord, SourceTrust::HostCanonical)
    }
}

fn dependency_key(id: &str) -> String {
    if id.starts_with("question-q1-") {
        "question-q1".into()
    } else if id.starts_with("grant-g1-") {
        "grant-g1".into()
    } else if id.starts_with("reference-r1") {
        "reference-r1".into()
    } else if id.starts_with("decision-d") {
        "decision-route".into()
    } else {
        id.into()
    }
}

pub fn input(id: &str) -> MaterialInput {
    let corpus = corpus();
    let case = case(id);
    let source_ids = case["source_ids"].as_array().unwrap();
    let sources: Vec<_> = source_ids
        .iter()
        .map(|value| {
            let id = value.as_str().unwrap();
            let revision = corpus["source_revisions"]
                .as_array()
                .unwrap()
                .iter()
                .find(|item| item["id"] == id)
                .unwrap();
            let (kind, trust) = source_kind(id);
            let availability = if id == "native-history-partial" {
                SourceAvailability::Truncated
            } else {
                SourceAvailability::Complete
            };
            let current = !matches!(
                id,
                "decision-d1" | "question-q1-r7" | "grant-g1-r3" | "reference-r1"
            ) || !source_ids.iter().any(|other| {
                let other = other.as_str().unwrap();
                other != id && dependency_key(other) == dependency_key(id)
            });
            SourceMaterial {
                id: id.into(),
                owner_namespace: revision["owner"].as_str().unwrap().into(),
                owner_ref: id.into(),
                revision: revision["revision"].as_u64().unwrap(),
                dependency_key: dependency_key(id),
                current,
                dependency_required: id != "unrelated-r9",
                kind,
                trust,
                availability,
                bytes: if availability == SourceAvailability::Complete {
                    revision["bytes_utf8"].as_str().unwrap().as_bytes().to_vec()
                } else {
                    Vec::new()
                },
                sha256: revision["sha256"].as_str().unwrap().into(),
            }
        })
        .collect();
    let obligations = case["obligations"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(index, obligation)| RequiredObligation {
            id: format!("{id}-O{}", index + 1),
            kind: if id == "C7" {
                ObligationKind::DeliveryCustody
            } else if id == "C2" || id == "C4" {
                ObligationKind::HumanInteraction
            } else if id == "C3" {
                ObligationKind::Decision
            } else {
                ObligationKind::Commitment
            },
            verifier_scope: match (id, index) {
                ("C1", 0) => {
                    "Current decision and supersession among cited decision revisions".into()
                }
                ("C1", 1) => "Human interaction and governing publication scope".into(),
                ("C1", 2) => "Task commitment, uncertainty, and canonical reference custody".into(),
                ("C1", 3) => "Queued delivery effect and its current disposition".into(),
                ("C5", 0) => "Current versus historical canonical record revisions".into(),
                ("C5", 1) => "External source text and governing trust boundary".into(),
                _ => format!(
                    "Case {id} consequence {} in the cited source scope",
                    index + 1
                ),
            },
            description: obligation["consequence"].as_str().unwrap().into(),
            sources: obligation["attributed_to"]
                .as_array()
                .unwrap()
                .iter()
                .map(|citation| SourceReference {
                    id: citation["source_id"].as_str().unwrap().into(),
                    owner_namespace: citation["owner"].as_str().unwrap().into(),
                    owner_ref: citation["source_id"].as_str().unwrap().into(),
                    revision: citation["revision"].as_u64().unwrap(),
                    sha256: citation["sha256"].as_str().unwrap().into(),
                })
                .collect(),
        })
        .collect();
    MaterialInput {
        subject: SubjectId::parse("subject-T-1").unwrap(),
        transition: TransitionId::parse(format!("transition-{id}")).unwrap(),
        capture_id: format!("capture-{id}"),
        sources,
        obligations,
        policy: SemanticPolicy::default(),
    }
}

pub fn basis(id: &str) -> SourceBasis {
    project_material(input(id)).unwrap()
}

pub fn binding(material: &SourceBasis, phase: SemanticPhase) -> SemanticInvocationBinding {
    let (purpose, prefix, session) = match phase {
        SemanticPhase::Compile => (
            NativeInvocationPurpose::SemanticCompile,
            "compile",
            "session-compiler",
        ),
        SemanticPhase::CandidateVerification => (
            NativeInvocationPurpose::SemanticVerify,
            "verify-candidate",
            "session-verifier",
        ),
        SemanticPhase::SuccessorAdequacy => (
            NativeInvocationPurpose::SemanticVerify,
            "verify-successor",
            "session-verifier",
        ),
    };
    SemanticInvocationBinding {
        subject: material.subject.clone(),
        transition: material.transition.clone(),
        capture_id: material.capture_id.clone(),
        phase,
        purpose,
        invocation_id: format!("{prefix}-{}", material.capture_id),
        attempt_id: format!("attempt-{prefix}"),
        session_id: session.into(),
        thread_id: format!("thread-{prefix}"),
        policy_id: material.policy_id.clone(),
        exposure_id: "isolated-same-qualified-model".into(),
        grant_ref: "synthetic-control-grant".into(),
        grant_revision: 1,
    }
}

pub fn complete_plan(material: &SourceBasis) -> ContinuationPlan {
    let preferred = material
        .sources
        .iter()
        .find(|source| source.id == "governing")
        .or_else(|| {
            material.sources.iter().find(|source| {
                source.current
                    && matches!(
                        source.trust,
                        SourceTrust::HostCanonical | SourceTrust::HostGoverning
                    )
            })
        })
        .unwrap();
    let statement = |source: &SourceMaterial| AttributedStatement {
        text: String::from_utf8(source.bytes.clone()).unwrap(),
        sources: vec![source.reference()],
    };
    let decisions: Vec<_> = material
        .sources
        .iter()
        .filter(|source| source.kind == SourceKind::Decision)
        .map(|source| RetainedDecision {
            statement: statement(source),
            current: source.current,
            supersedes: if source.current {
                material
                    .sources
                    .iter()
                    .filter(|other| other.kind == SourceKind::Decision && !other.current)
                    .map(SourceMaterial::reference)
                    .collect()
            } else {
                vec![]
            },
            invalidation_condition: source
                .current
                .then(|| "Prior route is no longer actionable".into()),
        })
        .collect();
    let interactions = material
        .sources
        .iter()
        .filter(|source| source.kind == SourceKind::HumanInteraction && source.current)
        .map(|source| RetainedInteraction {
            question_id: if source.id.starts_with("question-q1") {
                "Q-1"
            } else {
                "Q-2"
            }
            .into(),
            recipient: "O-1".into(),
            scope: if source.id.starts_with("question-q1") {
                "publish T-1 report to P-1"
            } else {
                "choose title for T-1"
            }
            .into(),
            status: if source.id == "question-q1-r8" {
                InteractionStatus::ScopedAnswer
            } else {
                InteractionStatus::Pending
            },
            answer_id: (source.id == "question-q1-r8").then(|| "A-1".into()),
            statement: statement(source),
        })
        .collect();
    ContinuationPlan {
        schema_version: material.schema_version,
        objective: statement(preferred),
        work_position: statement(
            material
                .sources
                .iter()
                .find(|source| source.id == "commitment-k1" || source.id == "delivery-i1")
                .unwrap_or(preferred),
        ),
        completed: material
            .sources
            .iter()
            .filter(|source| source.id == "delivery-i1")
            .map(statement)
            .collect(),
        commitments: material
            .sources
            .iter()
            .filter(|source| source.id == "commitment-k1")
            .map(statement)
            .collect(),
        decisions,
        uncertainties: material
            .sources
            .iter()
            .filter(|source| source.id.starts_with("uncertainty-"))
            .map(statement)
            .collect(),
        interactions,
        governing: material
            .sources
            .iter()
            .filter(|source| source.kind == SourceKind::Governing)
            .map(statement)
            .collect(),
        canonical_refs: material
            .sources
            .iter()
            .filter(|source| source.kind == SourceKind::CanonicalRecord && source.current)
            .map(statement)
            .collect(),
        delivery_custody: material
            .sources
            .iter()
            .filter(|source| source.kind == SourceKind::DeliveryConsequence)
            .map(statement)
            .collect(),
        next_action: AttributedStatement {
            text: "Propose the next admissible step, subject to fresh authority".into(),
            sources: vec![preferred.reference()],
        },
    }
}

pub fn candidate(material: &SourceBasis) -> (CompileRequest, Candidate) {
    let policy = SemanticPolicy::default();
    let request =
        compile_request(material, binding(material, SemanticPhase::Compile), &policy).unwrap();
    let raw = serde_json::to_vec(&complete_plan(material)).unwrap();
    let candidate = candidate_from_output(material, &request, raw, &policy).unwrap();
    (request, candidate)
}

pub fn report(material: &SourceBasis, status: FindingStatus) -> VerifierReport {
    VerifierReport {
        whole_response_assessed: true,
        findings: material
            .obligations
            .iter()
            .map(|obligation| AttributedFinding {
                obligation_id: obligation.id.clone(),
                status,
                sources: obligation.sources.clone(),
                explanation: "Controlled synthetic adjudication for this exact obligation".into(),
            })
            .collect(),
        critical_errors: vec![],
    }
}

pub fn result(request: &VerifyRequest, report: &VerifierReport) -> VerifierResult {
    let raw = serde_json::to_vec(report).unwrap();
    VerifierResult {
        invocation_id: request.binding.invocation_id.clone(),
        attempt_id: request.binding.attempt_id.clone(),
        session_id: request.binding.session_id.clone(),
        input_sha256: request.prompt_sha256.clone(),
        sha256: sha256_hex(&raw),
        raw,
    }
}

pub fn exposure(request: &VerifyRequest) -> ObservedVerifierExposure {
    ObservedVerifierExposure {
        verifier_session_id: request.binding.session_id.clone(),
        compiler_session_id: "session-compiler".into(),
        successor_session_id: if request.binding.phase == SemanticPhase::SuccessorAdequacy {
            Some("session-successor".into())
        } else {
            None
        },
        input_sha256: request.prompt_sha256.clone(),
        saw_compiler_reasoning: false,
        saw_hidden_oracle: false,
        dynamic_tools_enabled: false,
        human_side_effects_enabled: false,
        requested_provider: "synthetic-provider".into(),
        requested_model: "qualified-model".into(),
        observed_provider: "synthetic-provider".into(),
        observed_model: "qualified-model".into(),
        exposure_id: request.binding.exposure_id.clone(),
        host_evidence_ref: "observed-exposure-1".into(),
    }
}
