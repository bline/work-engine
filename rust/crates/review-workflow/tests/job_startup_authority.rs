#![cfg(all(unix, feature = "test-support"))]
#[path = "../../slice-campaign/tests/support/mod.rs"]
mod support;
use claim_evidence::sha256_bytes;
use review_workflow::startup::{QUALIFIED_PROFILE, TrustedStartup};
use serde_json::{Value, json};
use slice_campaign::job_admission::{
    JobAdmissionEffect, JobReviewPolicy, JobReviewPolicyDocument, JobReviewRequest,
    JobStartupAuthority,
};
use slice_campaign::{CampaignIdentity, CandidateRef};
use std::{fs, os::unix::fs::PermissionsExt};

fn digest(value: &Value) -> String {
    claim_evidence::codec::digest(&claim_evidence::codec::parse_json(&value.to_string()).unwrap())
        .unwrap()
}
fn claims(change: &str) -> (Value, Value, Value) {
    let production = if change == "production-grant" {
        "other-production"
    } else {
        "production"
    };
    let completion = if change == "completion-grant" {
        "other-completion"
    } else {
        "completion"
    };
    let scope = if change == "decision-scope" {
        "other-scope"
    } else {
        "scope"
    };
    let owner = if change == "source-identity" {
        "other-owner"
    } else {
        "owner"
    };
    let custody = json!({"owner":"review-execution-evidence","verifier":"review-execution-evidence-claude-v1","profile":"claude-qualified-v1","artifact_schemes":["execution-artifact:"],"reference_schemes":["execution-evidence:"]});
    let source = json!({"schema_version":2,"owner":owner,"reference":"fixture-authority","revision":"v1","freshness":"current","profile":"native-review-claims-v1","actors":["campaign"],"decision_scopes":[scope],"grant_ids":[production,completion],"custody_config_sha256":digest(&custody)});
    let authority = json!({"owner":owner,"reference":"fixture-authority","revision":"v1","freshness":"current","integrity_sha256":sha256_bytes(&serde_json::to_vec(&source).unwrap()),"status":"verified"});
    let production_grant = json!({"schema_version":1,"grant_id":production,"actor":"campaign","profile":"production-path-v1","permissions":["record_observation","establish_claim","read_admission"],"decision_scope":scope,"authority_reference":authority});
    let completion_grant = json!({"schema_version":1,"grant_id":completion,"actor":"campaign","profile":"revision-bound-review-finding-v1","permissions":["create_claim","publish_revision","record_reliance"],"decision_scope":scope,"authority_reference":authority});
    let configuration = json!({"schema_version":2,"profile":"native-review-claims-v1","root_id":"claims-root","trusted_custody":custody,"grants":[production_grant,completion_grant]});
    (source, configuration, authority)
}
fn check(change: &str) {
    let temp = tempfile::tempdir().unwrap();
    fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let repo = temp.path().join("repo");
    fs::create_dir(&repo).unwrap();
    let identity = CampaignIdentity {
        run_id: "fixture-run".into(),
        plan_version: "fixture-plan".into(),
        slice_number: 1,
        attempt_id: "attempt".into(),
    };
    let candidate = CandidateRef {
        commit: "1".repeat(40),
        tree: "1".repeat(40),
        patch_identity: "1".repeat(64),
        receipt_sha256: "2".repeat(64),
        physical_profile_sha256: "3".repeat(64),
    };
    let subject = json!({"commit":candidate.commit,"tree":candidate.tree,"patchIdentity":candidate.patch_identity});
    let selection = json!({"schemaVersion":2,"owner":"slice-supervisor","selectionId":"policy-selection","subject":subject,"specialists":[{"obligationId":"review","skill":"implementation-review","selection":"selected","requiredClaims":[support::claim(&identity,"review",&subject,"builder_projection"),support::claim(&identity,"review",&subject,"campaign_terminalization")]}]});
    let acceptance = json!({"schemaVersion":1,"decision":"accepted","reference":"fixture-template","revision":"v1","identity":identity,"obligationId":"review","candidateTree":candidate.tree,"builderClaimId":"builder","campaignClaimId":"campaign","selectionDigest":"1".repeat(64),"physicalProfileDigest":"2".repeat(64),"claudeProfileDigest":"3".repeat(64),"programSha256":"4".repeat(64),"guardSha256":"5".repeat(64),"cbmStoreSha256":"6".repeat(64),"catalogSha256":"7".repeat(64),"requestedModel":"fixture-model","credentialSourceMetadataSha256":"8".repeat(64),"resourcePolicy":"strict_aggregate_session_64mib_v1","sessionId":"template-session","attemptId":"attempt","acceptedBoundarySha256":"9".repeat(64),"promptTemplateSha256":"a".repeat(64),"attemptHorizonMillis":1000});
    let (original_source, original_config, _) = claims("");
    let policy_doc = JobReviewPolicyDocument {
        schema_version: 1,
        policy_id: "fixture-policy".into(),
        revision: "v1".into(),
        active: true,
        expires_at_millis: u64::MAX,
        admitted_uid: rustix::process::geteuid().as_raw(),
        repository: repo.clone(),
        jobs_root: temp.path().to_path_buf(),
        execution_template: acceptance,
        review_selection_template: selection,
        startup_authority: JobStartupAuthority {
            claims_source_sha256: sha256_bytes(&serde_json::to_vec(&original_source).unwrap()),
            claims_configuration_sha256: sha256_bytes(
                &serde_json::to_vec(&original_config).unwrap(),
            ),
            completion_consumer: "completion-reader".into(),
        },
    };
    let policy_path = temp.path().join("policy.json");
    fs::write(&policy_path, serde_json::to_vec(&policy_doc).unwrap()).unwrap();
    fs::set_permissions(&policy_path, fs::Permissions::from_mode(0o600)).unwrap();
    let policy = JobReviewPolicy::open_fixture(&policy_path).unwrap();
    let request = JobReviewRequest {
        identity: identity.clone(),
        candidate,
        physical_profile_digest: "2".repeat(64),
        cbm_store_sha256: "6".repeat(64),
        session_id: "session".into(),
    };
    let dir = temp.path().join(request.directory_name().unwrap());
    fs::create_dir(&dir).unwrap();
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).unwrap();
    let JobAdmissionEffect::Committed(pin) = policy.admit(&request).unwrap() else {
        panic!("fresh fixture admission")
    };
    let (source, config, authority) = claims(change);
    let source_path = dir.join("source.json");
    let config_path = dir.join("configuration.json");
    fs::write(&source_path, serde_json::to_vec(&source).unwrap()).unwrap();
    fs::write(&config_path, serde_json::to_vec(&config).unwrap()).unwrap();
    for path in [&source_path, &config_path] {
        fs::set_permissions(path, fs::Permissions::from_mode(0o400)).unwrap();
    }
    let file_pin = |path: &std::path::Path, sha: String| json!({"path":path,"sha256":sha,"mode":0o400,"ownerUid":null});
    let missing = file_pin(&dir.join("missing"), "0".repeat(64));
    let grants = config["grants"].as_array().unwrap();
    let document = json!({"schemaVersion":2,"profile":QUALIFIED_PROFILE,"base":{
        "schemaVersion":1,"profile":QUALIFIED_PROFILE,"evidenceProfile":"claude-qualified-v1",
        "identity":identity,"repository":repo,"workspace":dir.join("workspace"),
        "acceptedBoundary":{"reference":"fixture","sha256":"9".repeat(64)},
        "baseline":{"acceptedCommit":"1".repeat(40),"acceptedTree":"1".repeat(40),"interSliceCommit":"1".repeat(40)},
        "roots":{"campaign":dir.join("campaign"),"episode":dir.join("episode"),"claims":dir.join("claims"),"evidence":dir.join("evidence")},
        "host":{"executable":missing,"sourceArchive":missing,"toolchain":missing,"lockfile":missing,"target":"fixture","features":[],"runtimeDependencies":[]},
        "gateCapture":missing,"gateManifest":missing,"gateReceipt":missing,"candidate":missing,"physicalProfile":missing,"selection":missing,"episodeAuthority":missing,
        "compiler":{"mode":"manifest-only-unverified","document":missing,"options":missing,"role":"reviewer","requirements":null},
        "requestInput":missing,"peer":{"executable":missing,"source":missing,"arguments":[],"session":"session","evidenceRootId":"evidence-root"},
        "claims":{"rootId":"claims-root","source":file_pin(&source_path,sha256_bytes(&fs::read(&source_path).unwrap())),"configuration":file_pin(&config_path,sha256_bytes(&fs::read(&config_path).unwrap())),"productionGrantId":grants[0]["grant_id"],"productionGrantSha256":digest(&grants[0])},
        "obligationId":"review","operationId":"operation"
    },"qualified":{
        "bwrap":missing,"cbm":missing,"cbmLauncher":missing,"cbmClient":missing,"guard":missing,"catalog":missing,
        "boundaryRoot":dir.join("boundary"),"boundaryManifestSha256":"0".repeat(64),"requestedModel":"fixture-model","requestedEffort":"high","acceptedObservedModels":["fixture-model"],
        "credentialSource":dir.join("never-opened-credential"),"credentialSourceMetadataSha256":"0".repeat(64),"resourcePolicy":"strict_aggregate_session_64mib_v1","attemptHorizonMillis":1000,
        "completionGrantId":grants[1]["grant_id"],"completionGrantSha256":digest(&grants[1]),
        "completionConsumer":if change=="completion-consumer" {"other-reader"} else {"completion-reader"},
        "completionDecisionScope":grants[1]["decision_scope"],"completionAuthoritySource":authority,
        "offlineAcceptance":null,"offlineWitness":null
    }});
    let path = dir.join("startup.json");
    fs::write(&path, serde_json::to_vec(&document).unwrap()).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
    let error = TrustedStartup::load_for_job(&path, pin)
        .err()
        .expect("fixture has no runtime artifacts");
    if change.is_empty() {
        assert!(
            !error.to_string().contains("job startup authority differs"),
            "{error}"
        );
        assert!(
            error.to_string().contains("No such file"),
            "expected later independent file guard: {error}"
        );
    } else {
        assert!(
            error
                .to_string()
                .contains("job startup authority differs from policy"),
            "{change}: {error}"
        );
    }
    for child in ["campaign", "episode", "claims", "evidence"] {
        assert!(!dir.join(child).exists());
    }
}
#[test]
fn policy_selected_claims_pass_authority_guard_before_missing_runtime_artifacts() {
    check("");
}
#[test]
fn internally_consistent_replaced_source_identity_is_refused() {
    check("source-identity");
}
#[test]
fn internally_consistent_replaced_production_grant_is_refused() {
    check("production-grant");
}
#[test]
fn internally_consistent_replaced_completion_grant_is_refused() {
    check("completion-grant");
}
#[test]
fn internally_consistent_replaced_decision_scope_is_refused() {
    check("decision-scope");
}
#[test]
fn replaced_completion_consumer_is_refused() {
    check("completion-consumer");
}
