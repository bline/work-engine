mod common;

use common::*;
use lifecycle_semantic::*;

#[test]
fn native_v1_payloads_remain_byte_stable() {
    let material = basis("C1");
    let policy = SemanticPolicy::default();
    let (compile, candidate) = candidate(&material);
    let verify = verify_candidate(
        &material,
        &candidate,
        binding(&material, SemanticPhase::CandidateVerification),
        &policy,
    )
    .unwrap();
    let result = result(&verify, &report(&material, FindingStatus::Satisfied));
    let exposure = exposure(&verify);
    let checked = assess_candidate(
        &material,
        &candidate,
        &verify,
        result.clone(),
        exposure.clone(),
        &policy,
    )
    .unwrap();
    macro_rules! assert_frozen {
        ($value:expr, $bytes:expr, $ty:ty) => {{
            let frozen = $bytes;
            assert_eq!(serde_json::to_vec(&$value).unwrap(), frozen);
            let decoded: $ty = serde_json::from_slice(frozen).unwrap();
            assert_eq!(serde_json::to_vec(&decoded).unwrap(), frozen);
        }};
    }
    assert_frozen!(
        verify.binding,
        include_bytes!("fixtures/native-codec-v1/binding.json"),
        SemanticInvocationBinding
    );
    assert_frozen!(
        compile,
        include_bytes!("fixtures/native-codec-v1/compile-request.json"),
        CompileRequest
    );
    assert_frozen!(
        candidate,
        include_bytes!("fixtures/native-codec-v1/candidate.json"),
        Candidate
    );
    assert_frozen!(
        verify,
        include_bytes!("fixtures/native-codec-v1/verify-request.json"),
        VerifyRequest
    );
    assert_frozen!(
        result,
        include_bytes!("fixtures/native-codec-v1/verifier-result.json"),
        VerifierResult
    );
    assert_frozen!(
        exposure,
        include_bytes!("fixtures/native-codec-v1/exposure.json"),
        ObservedVerifierExposure
    );
    assert_frozen!(
        checked,
        include_bytes!("fixtures/native-codec-v1/checked-result.json"),
        CheckedSemanticResult
    );
    let frozen_verify: VerifyRequest = serde_json::from_slice(include_bytes!(
        "fixtures/native-codec-v1/verify-request.json"
    ))
    .unwrap();
    assert_eq!(verify.prompt, frozen_verify.prompt);
    assert_eq!(verify.prompt_sha256, frozen_verify.prompt_sha256);
    assert_eq!(checked.status, VerdictStatus::Accepted);
}
