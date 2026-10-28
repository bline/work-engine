use std::{path::PathBuf, sync::Arc, sync::atomic::Ordering};

use lifecycle_core::{
    CheckpointId, ContextGeneration, Revision, RuntimeIncarnation, SnapshotId, SubjectId,
    TransitionId,
};
use lifecycle_semantic::{
    ObservedVerifierExposure, SemanticInvocationBinding, SemanticPhase, VerdictStatus,
    VerifierResult, assess_candidate, assess_successor, candidate_from_output, challenge_successor,
    compile_request, project_material, revalidate_dependencies, successor_response,
    verify_candidate, verify_successor,
};
use lifecycle_store::{SemanticControlRole, SqliteLifecycleStore, StoreError};

use crate::{
    CanonicalMaterialPort, NativeConfig, NativeProviderRoute, SemanticNativeConfig,
    actor::{NativeGate, clock_sample},
    semantic_control::{self, ControlTurnSpec},
};

#[derive(Clone)]
pub struct SemanticRuntime {
    pub material_port: Arc<dyn CanonicalMaterialPort>,
    pub native: NativeConfig,
    pub semantic: SemanticNativeConfig,
    pub incarnation: RuntimeIncarnation,
    pub snapshot: SnapshotId,
    pub snapshot_manifest_sha256: String,
    pub gate: Arc<NativeGate>,
    pub uid: u32,
    pub principal: String,
    pub proof_dir: Option<PathBuf>,
}

fn semantic_store_error(_: lifecycle_semantic::SemanticError) -> StoreError {
    StoreError::Rejected
}

#[allow(clippy::too_many_arguments)]
fn verifier_exposure(
    route: NativeProviderRoute,
    verifier_session_id: &str,
    compiler_session_id: &str,
    successor_session_id: Option<&str>,
    input_sha256: &str,
    observed_provider: &str,
    observed_model: &str,
    exposure_id: &str,
    host_evidence_ref: &str,
) -> ObservedVerifierExposure {
    ObservedVerifierExposure {
        verifier_session_id: verifier_session_id.into(),
        compiler_session_id: compiler_session_id.into(),
        successor_session_id: successor_session_id.map(str::to_owned),
        input_sha256: input_sha256.into(),
        saw_compiler_reasoning: false,
        saw_hidden_oracle: false,
        dynamic_tools_enabled: false,
        human_side_effects_enabled: false,
        requested_provider: route.provider_name().into(),
        requested_model: NativeProviderRoute::MODEL.into(),
        observed_provider: observed_provider.into(),
        observed_model: observed_model.into(),
        exposure_id: exposure_id.into(),
        host_evidence_ref: host_evidence_ref.into(),
    }
}

#[allow(clippy::too_many_arguments)]
fn binding(
    runtime: &SemanticRuntime,
    subject: &SubjectId,
    transition: &TransitionId,
    phase: SemanticPhase,
    session: &str,
    invocation: &str,
    attempt: &str,
    policy_id: &str,
) -> SemanticInvocationBinding {
    let (grant_ref, grant_revision) = runtime.semantic.grant_for_phase(phase);
    SemanticInvocationBinding {
        subject: subject.clone(),
        transition: transition.clone(),
        capture_id: runtime.semantic.capture_id.clone(),
        phase,
        purpose: if phase == SemanticPhase::Compile {
            lifecycle_core::NativeInvocationPurpose::SemanticCompile
        } else {
            lifecycle_core::NativeInvocationPurpose::SemanticVerify
        },
        invocation_id: invocation.into(),
        attempt_id: attempt.into(),
        session_id: session.into(),
        thread_id: "thread:pending".into(),
        policy_id: policy_id.into(),
        exposure_id: format!("exposure:{invocation}"),
        grant_ref: grant_ref.into(),
        grant_revision,
    }
}

#[allow(clippy::too_many_arguments)]
fn spec<'a>(
    runtime: &'a SemanticRuntime,
    subject: SubjectId,
    context: ContextGeneration,
    transition: TransitionId,
    role: SemanticControlRole,
    phase: SemanticPhase,
    session_id: &'a str,
    invocation_id: &'a str,
    attempt_id: &'a str,
    prompt: &'a str,
    policy_id: &'a str,
) -> ControlTurnSpec<'a> {
    ControlTurnSpec {
        subject,
        context,
        transition,
        incarnation: runtime.incarnation.clone(),
        snapshot: runtime.snapshot.clone(),
        snapshot_manifest_sha256: &runtime.snapshot_manifest_sha256,
        role,
        phase,
        session_id,
        invocation_id,
        attempt_id,
        prompt,
        policy_id,
        exposure_id: format!("exposure:{invocation_id}"),
    }
}

fn publish_bytes(
    store: &mut SqliteLifecycleStore,
    subject: &SubjectId,
    bytes: &[u8],
) -> Result<String, StoreError> {
    let staged = store.stage_artifact(subject.clone(), bytes)?;
    Ok(store.publish_artifact(staged, clock_sample())?.digest_hex)
}

pub async fn establish_checkpoint(
    store: &mut SqliteLifecycleStore,
    runtime: &SemanticRuntime,
    subject: &SubjectId,
    context: &ContextGeneration,
    transition: &TransitionId,
    semantic_revision: u64,
) -> Result<Option<CheckpointId>, StoreError> {
    if let Some(package) = store.semantic_checkpoint(transition)? {
        return Ok(Some(
            CheckpointId::parse(format!("checkpoint:{}", package.candidate.sha256))
                .map_err(|_| StoreError::Unavailable)?,
        ));
    }
    if runtime.gate.load(Ordering::SeqCst) {
        return Err(StoreError::ClaimConflict);
    }
    // A native compiler turn may have completed while its model-authored
    // content failed deterministic validation. Never send that attempt again.
    if store
        .semantic_compile_refusal(
            transition,
            &runtime.semantic.capture_id,
            &runtime.semantic.compile_invocation_id,
        )?
        .is_some()
    {
        return Err(StoreError::Rejected);
    }
    let input = runtime
        .material_port
        .project(subject, transition, &runtime.semantic.capture_id)
        .map_err(|_| StoreError::Rejected)?;
    if input.subject != *subject
        || input.transition != *transition
        || input.capture_id != runtime.semantic.capture_id
    {
        return Err(StoreError::Rejected);
    }
    let policy = input.policy.clone();
    let basis = match project_material(input.clone()) {
        Ok(basis) => basis,
        Err(error) => {
            store.record_semantic_material_rejection(
                &input,
                &error,
                Revision::new(semantic_revision),
                clock_sample(),
            )?;
            return Ok(None);
        }
    };
    let compile_binding = binding(
        runtime,
        subject,
        transition,
        SemanticPhase::Compile,
        &runtime.semantic.compile_session_id,
        &runtime.semantic.compile_invocation_id,
        &runtime.semantic.compile_attempt_id,
        &policy.id,
    );
    let compile_pre = if basis.is_eligible() {
        match compile_request(&basis, compile_binding, &policy) {
            Ok(request) => Some(request),
            Err(lifecycle_semantic::SemanticError::PromptTooLarge) => {
                store.record_semantic_material_rejection(
                    &input,
                    &lifecycle_semantic::SemanticError::PromptTooLarge,
                    Revision::new(semantic_revision),
                    clock_sample(),
                )?;
                return Ok(None);
            }
            Err(error) => return Err(semantic_store_error(error)),
        }
    } else {
        None
    };
    if runtime.gate.load(Ordering::SeqCst) {
        return Err(StoreError::ClaimConflict);
    }
    store.record_semantic_capture_with_barrier(
        &basis,
        &policy,
        Revision::new(semantic_revision),
        clock_sample(),
        runtime.proof_dir.as_deref(),
    )?;
    let Some(compile_pre) = compile_pre else {
        return Ok(None);
    };
    let compile = semantic_control::run_control_turn(
        store,
        &runtime.native,
        &runtime.semantic,
        spec(
            runtime,
            subject.clone(),
            context.clone(),
            transition.clone(),
            SemanticControlRole::Compile,
            SemanticPhase::Compile,
            &runtime.semantic.compile_session_id,
            &runtime.semantic.compile_invocation_id,
            &runtime.semantic.compile_attempt_id,
            &compile_pre.prompt,
            &policy.id,
        ),
        &runtime.principal,
        runtime.uid,
        runtime.proof_dir.as_deref(),
        &runtime.gate,
    )
    .await?;
    let compile_request =
        compile_request(&basis, compile.binding.clone(), &policy).map_err(semantic_store_error)?;
    if compile_request.prompt_sha256 != compile_pre.prompt_sha256 {
        return Err(StoreError::ClaimConflict);
    }
    let candidate = candidate_from_output(
        &basis,
        &compile_request,
        compile.final_material.final_text.clone(),
        &policy,
    );
    let candidate = match candidate {
        Ok(candidate) => candidate,
        Err(
            error @ (lifecycle_semantic::SemanticError::InvalidOutput
            | lifecycle_semantic::SemanticError::InvalidReference
            | lifecycle_semantic::SemanticError::OutputTooLarge),
        ) => {
            if lifecycle_semantic::sha256_hex(&compile.final_material.final_text)
                != compile.final_material.final_sha256
            {
                return Err(StoreError::Rejected);
            }
            store.record_semantic_compile_refusal(
                transition,
                &basis.capture_id,
                &compile.binding.invocation_id,
                &compile_request.prompt_sha256,
                &compile.final_material.final_sha256,
                &error,
                clock_sample(),
            )?;
            return Err(StoreError::Rejected);
        }
        Err(error) => return Err(semantic_store_error(error)),
    };
    let candidate_artifact = publish_bytes(store, subject, &candidate.raw)?;
    store.link_semantic_attempt_artifact(
        &runtime.semantic.compile_invocation_id,
        &candidate_artifact,
    )?;
    let verify_pre = verify_candidate(
        &basis,
        &candidate,
        binding(
            runtime,
            subject,
            transition,
            SemanticPhase::CandidateVerification,
            &runtime.semantic.verify_session_id,
            &runtime.semantic.candidate_verify_invocation_id,
            &runtime.semantic.candidate_verify_attempt_id,
            &policy.id,
        ),
        &policy,
    )
    .map_err(semantic_store_error)?;
    let verified = semantic_control::run_control_turn(
        store,
        &runtime.native,
        &runtime.semantic,
        spec(
            runtime,
            subject.clone(),
            context.clone(),
            transition.clone(),
            SemanticControlRole::Verify,
            SemanticPhase::CandidateVerification,
            &runtime.semantic.verify_session_id,
            &runtime.semantic.candidate_verify_invocation_id,
            &runtime.semantic.candidate_verify_attempt_id,
            &verify_pre.prompt,
            &policy.id,
        ),
        &runtime.principal,
        runtime.uid,
        runtime.proof_dir.as_deref(),
        &runtime.gate,
    )
    .await?;
    let verify = verify_candidate(&basis, &candidate, verified.binding.clone(), &policy)
        .map_err(semantic_store_error)?;
    if verify.prompt_sha256 != verify_pre.prompt_sha256 {
        return Err(StoreError::ClaimConflict);
    }
    let verifier_artifact = publish_bytes(store, subject, &verified.final_material.final_text)?;
    store.link_semantic_attempt_artifact(
        &runtime.semantic.candidate_verify_invocation_id,
        &verifier_artifact,
    )?;
    let observed = verifier_exposure(
        runtime.native.provider_route,
        &verified.binding.session_id,
        &compile.binding.session_id,
        None,
        &verify.prompt_sha256,
        &verified.observed_provider,
        &verified.observed_model,
        &verified.binding.exposure_id,
        &verified.exposure_ref,
    );
    let result = VerifierResult {
        invocation_id: verified.binding.invocation_id.clone(),
        attempt_id: verified.binding.attempt_id.clone(),
        session_id: verified.binding.session_id.clone(),
        input_sha256: verify.prompt_sha256.clone(),
        raw: verified.final_material.final_text.clone(),
        sha256: verified.final_material.final_sha256.clone(),
    };
    let checked = assess_candidate(&basis, &candidate, &verify, result, observed, &policy)
        .map_err(semantic_store_error)?;
    let observations = runtime
        .material_port
        .observe_dependencies(&basis.dependencies)
        .map_err(|_| StoreError::Rejected)?;
    if !revalidate_dependencies(&basis.dependencies, &observations)
        .map_err(semantic_store_error)?
        .is_empty()
    {
        store.mark_semantic_capture_stale(&basis.capture_id, &basis.sha256, clock_sample())?;
        return Ok(None);
    }
    let result_id = format!("semantic:{}", verified.binding.invocation_id);
    store.record_semantic_result(
        &result_id,
        &basis.capture_id,
        &runtime.semantic.compile_invocation_id,
        &candidate_artifact,
        None,
        None,
        &checked,
        clock_sample(),
    )?;
    crate::operations::optional_native_barrier(
        runtime.proof_dir.as_deref(),
        "semantic_after_candidate_result",
    )?;
    if checked.status != VerdictStatus::Accepted {
        return Ok(None);
    }
    let challenge = challenge_successor(
        &basis,
        &candidate,
        runtime.semantic.challenge_id.clone(),
        runtime.semantic.challenge_text.clone(),
        &policy,
    )
    .map_err(semantic_store_error)?;
    let successor_prompt=serde_json::to_string(&serde_json::json!({
        "task":"use_attributed_continuation","source_basis_sha256":basis.sha256,
        "candidate":candidate.plan,"candidate_sha256":candidate.sha256,
        "challenge":challenge.next_domain_prompt,"challenge_sha256":challenge.sha256,
        "constraints":["Use the continuation to choose the next authorized action; do not echo a digest.",
            "Pending approvals remain pending and completed effects must not replay."]
    })).map_err(|_|StoreError::Rejected)?;
    if successor_prompt.len() > 65_536 {
        return Err(StoreError::Rejected);
    }
    if runtime.gate.load(Ordering::SeqCst) {
        return Err(StoreError::ClaimConflict);
    }
    store.publish_semantic_checkpoint(
        &result_id,
        &candidate,
        &challenge,
        &successor_prompt,
        clock_sample(),
    )?;
    Ok(Some(
        CheckpointId::parse(format!("checkpoint:{}", candidate.sha256))
            .map_err(|_| StoreError::Unavailable)?,
    ))
}

pub async fn assess_successor_use(
    store: &mut SqliteLifecycleStore,
    runtime: &SemanticRuntime,
    subject: &SubjectId,
    context: &ContextGeneration,
    transition: &TransitionId,
) -> Result<bool, StoreError> {
    let Some(package) = store.semantic_checkpoint(transition)? else {
        return Ok(false);
    };
    let input = runtime
        .material_port
        .project(subject, transition, &runtime.semantic.capture_id)
        .map_err(|_| StoreError::Rejected)?;
    let policy = input.policy;
    package
        .basis
        .validate(&policy)
        .map_err(semantic_store_error)?;
    let observations = runtime
        .material_port
        .observe_dependencies(&package.basis.dependencies)
        .map_err(|_| StoreError::Rejected)?;
    if !revalidate_dependencies(&package.basis.dependencies, &observations)
        .map_err(semantic_store_error)?
        .is_empty()
    {
        store.mark_semantic_capture_stale(
            &package.basis.capture_id,
            &package.basis.sha256,
            clock_sample(),
        )?;
        return Ok(false);
    }
    if let Some((_, verdict)) = store.semantic_successor_verdict(transition)? {
        return Ok(verdict == "accepted");
    }
    let Some(receipt) = store.native_rehydration_receipt(transition)? else {
        return Ok(false);
    };
    let invocation = receipt
        .strip_prefix("native:")
        .ok_or(StoreError::Unavailable)?;
    let final_material = store
        .native_final_material(invocation)?
        .ok_or(StoreError::Unavailable)?;
    let successor = successor_response(
        &package.challenge,
        invocation.into(),
        final_material.final_text.clone(),
        &policy,
    )
    .map_err(semantic_store_error)?;
    let response_artifact = publish_bytes(store, subject, &successor.raw)?;
    let verify_pre = verify_successor(
        &package.basis,
        &package.candidate,
        &package.challenge,
        &successor,
        binding(
            runtime,
            subject,
            transition,
            SemanticPhase::SuccessorAdequacy,
            &runtime.semantic.successor_verify_session_id,
            &runtime.semantic.successor_verify_invocation_id,
            &runtime.semantic.successor_verify_attempt_id,
            &policy.id,
        ),
        &policy,
    )
    .map_err(semantic_store_error)?;
    let verified = semantic_control::run_control_turn(
        store,
        &runtime.native,
        &runtime.semantic,
        spec(
            runtime,
            subject.clone(),
            context.clone(),
            transition.clone(),
            SemanticControlRole::SuccessorVerify,
            SemanticPhase::SuccessorAdequacy,
            &runtime.semantic.successor_verify_session_id,
            &runtime.semantic.successor_verify_invocation_id,
            &runtime.semantic.successor_verify_attempt_id,
            &verify_pre.prompt,
            &policy.id,
        ),
        &runtime.principal,
        runtime.uid,
        runtime.proof_dir.as_deref(),
        &runtime.gate,
    )
    .await?;
    let verify = verify_successor(
        &package.basis,
        &package.candidate,
        &package.challenge,
        &successor,
        verified.binding.clone(),
        &policy,
    )
    .map_err(semantic_store_error)?;
    if verify.prompt_sha256 != verify_pre.prompt_sha256 {
        return Err(StoreError::ClaimConflict);
    }
    let verifier_artifact = publish_bytes(store, subject, &verified.final_material.final_text)?;
    store.link_semantic_attempt_artifact(
        &runtime.semantic.successor_verify_invocation_id,
        &verifier_artifact,
    )?;
    let exposure = verifier_exposure(
        runtime.native.provider_route,
        &verified.binding.session_id,
        &package.candidate.compile_session_id,
        Some(&runtime.native.session_id),
        &verify.prompt_sha256,
        &verified.observed_provider,
        &verified.observed_model,
        &verified.binding.exposure_id,
        &verified.exposure_ref,
    );
    let result = VerifierResult {
        invocation_id: verified.binding.invocation_id.clone(),
        attempt_id: verified.binding.attempt_id.clone(),
        session_id: verified.binding.session_id.clone(),
        input_sha256: verify.prompt_sha256.clone(),
        raw: verified.final_material.final_text.clone(),
        sha256: verified.final_material.final_sha256.clone(),
    };
    let checked = assess_successor(
        &package.basis,
        &package.candidate,
        &package.challenge,
        &successor,
        &verify,
        result,
        exposure,
        &policy,
    )
    .map_err(semantic_store_error)?;
    let challenge_bytes =
        serde_json::to_vec(&package.challenge).map_err(|_| StoreError::Unavailable)?;
    let challenge_artifact = lifecycle_semantic::sha256_hex(&challenge_bytes);
    let result_id = format!("semantic:{}", verified.binding.invocation_id);
    store.record_semantic_result(
        &result_id,
        &package.basis.capture_id,
        &package.candidate.compile_invocation_id,
        &package.candidate.sha256,
        Some(&challenge_artifact),
        Some(&response_artifact),
        &checked,
        clock_sample(),
    )?;
    crate::operations::optional_native_barrier(
        runtime.proof_dir.as_deref(),
        "semantic_after_successor_result",
    )?;
    Ok(checked.status == VerdictStatus::Accepted)
}

#[cfg(test)]
mod provider_exposure_tests {
    use super::*;

    #[test]
    fn candidate_and_successor_exposures_bind_selected_route_independently_of_observation() {
        for (route, expected_provider) in [
            (NativeProviderRoute::NativeSimProofV1, "local_probe"),
            (
                NativeProviderRoute::OpenaiResponsesLiveV1,
                "openai_responses_live",
            ),
            (
                NativeProviderRoute::OpenaiResponsesFakeV1,
                "openai_responses_fake",
            ),
        ] {
            for successor in [None, Some("successor-session")] {
                let exposure = verifier_exposure(
                    route,
                    "verifier-session",
                    "compiler-session",
                    successor,
                    "input-digest",
                    "local_probe",
                    NativeProviderRoute::MODEL,
                    "exposure-id",
                    "host-evidence",
                );
                assert_eq!(exposure.requested_provider, expected_provider);
                assert_eq!(exposure.observed_provider, "local_probe");
                assert_eq!(exposure.requested_model, NativeProviderRoute::MODEL);
                assert_eq!(exposure.observed_model, NativeProviderRoute::MODEL);
                assert_eq!(exposure.verifier_session_id, "verifier-session");
                assert_eq!(exposure.compiler_session_id, "compiler-session");
                assert_eq!(exposure.successor_session_id.as_deref(), successor);
                assert_eq!(exposure.input_sha256, "input-digest");
                assert_eq!(exposure.exposure_id, "exposure-id");
                assert_eq!(exposure.host_evidence_ref, "host-evidence");
                assert!(!exposure.saw_compiler_reasoning);
                assert!(!exposure.saw_hidden_oracle);
                assert!(!exposure.dynamic_tools_enabled);
                assert!(!exposure.human_side_effects_enabled);
            }
        }
    }
}
