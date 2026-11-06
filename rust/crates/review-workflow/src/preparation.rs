//! Deterministic proposal data. This module never opens installed policy,
//! creates domain stores, reads credential contents, or obtains an entry permit.
use crate::startup::{
    FilePin, QUALIFIED_PROFILE, QualifiedStartupDocument, qualified_prompt_template_for_document,
};
use crate::{Result, refuse};
use claim_evidence::sha256_bytes;
use review_execution_evidence::claude::{
    CandidateBoundary, PinnedProgram, TrustedClaudeProfile, TrustedRp0Profile,
};
use serde::Deserialize;
use serde_json::{Value, json};
use slice_campaign::{
    job_admission::{JobReviewPolicyDocument, JobReviewRequest, JobStartupAuthority},
    subject_binding, verify_existing_subject_dynamic,
};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::Path,
    process::{Command, Output},
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreparationInput {
    pub schema_version: u32,
    pub standing: String,
    pub provenance: FilePin,
    pub startup: QualifiedStartupDocument,
    pub policy: JobReviewPolicyDocument,
    pub checkpoint_request: Value,
    pub checkpoint_validator: FilePin,
    pub profile_analyzer: FilePin,
    pub gate_runner: FilePin,
    pub candidate_paths: Vec<String>,
    pub project: String,
}
fn error(message: impl Into<String>) -> crate::Error {
    refuse("preparation", message)
}
fn run(program: &Path, args: &[String], cwd: &Path) -> Result<Output> {
    let result = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()?;
    if !result.status.success() {
        return Err(error(format!(
            "{} failed: {}",
            program.display(),
            String::from_utf8_lossy(&result.stderr)
        )));
    }
    Ok(result)
}
fn git(repo: &Path, args: &[&str]) -> Result<String> {
    let mut argv = vec!["-C".to_owned(), repo.display().to_string()];
    argv.extend(args.iter().map(|v| (*v).to_owned()));
    Ok(
        String::from_utf8(run(Path::new("/usr/bin/git"), &argv, repo)?.stdout)
            .map_err(|e| error(e.to_string()))?
            .trim()
            .to_owned(),
    )
}
fn write(path: &Path, bytes: &[u8], mode: u32) -> Result<FilePin> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(mode)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    let metadata = file.metadata()?;
    Ok(FilePin {
        path: path.to_owned(),
        sha256: sha256_bytes(bytes),
        mode,
        owner_uid: Some(metadata.uid()),
    })
}
fn data(path: &Path, value: &Value) -> Result<FilePin> {
    write(path, &serde_json::to_vec_pretty(value)?, 0o400)
}
fn copy_pin(root: &Path, name: &str, pin: &FilePin) -> Result<FilePin> {
    let bytes = pin.read(512 * 1024 * 1024)?;
    write(&root.join(name), &bytes, 0o400)
}
fn pinned_program(pin: &FilePin) -> Result<PinnedProgram> {
    pin.read(512 * 1024 * 1024)?;
    PinnedProgram::load(&pin.path, &pin.sha256).map_err(|e| error(e.to_string()))
}
fn snapshot(repo: &Path, paths: &[String]) -> Result<Value> {
    let mut hashes = serde_json::Map::new();
    for path in paths {
        if path.is_empty()
            || Path::new(path).is_absolute()
            || Path::new(path)
                .components()
                .any(|p| !matches!(p, std::path::Component::Normal(_)))
        {
            return Err(error("candidate path must be a confined relative path"));
        }
        hashes.insert(
            path.clone(),
            json!(sha256_bytes(&fs::read(repo.join(path))?)),
        );
    }
    Ok(
        json!({"all_status":git(repo,&["status","--porcelain","--untracked-files=all"])?,
        "head":git(repo,&["rev-parse","HEAD"])?,"head_tree":git(repo,&["rev-parse","HEAD^{tree}"])?,
        "path_sha256":hashes,"tracked_paths":git(repo,&["ls-files"])?.lines().collect::<Vec<_>>()}),
    )
}
fn subject(input: &mut PreparationInput, root: &Path) -> Result<()> {
    let repo = &input.startup.base.repository;
    let before = snapshot(repo, &input.candidate_paths)?;
    if before["all_status"] != "" || input.candidate_paths.is_empty() {
        return Err(error("candidate must be a clean, explicit projection"));
    }
    let tools = [
        (
            &input.checkpoint_validator,
            subject_binding::CHECKPOINT_SOURCE_SHA256,
            "checkpoint.py",
        ),
        (
            &input.profile_analyzer,
            subject_binding::ANALYZER_SOURCE_SHA256,
            "profile.py",
        ),
        (
            &input.gate_runner,
            subject_binding::GATE_RUNNER_SOURCE_SHA256,
            "run_gate.py",
        ),
    ];
    for (pin, expected, name) in tools {
        if pin.sha256 != expected {
            return Err(error(format!(
                "{name} differs from its owner-selected source"
            )));
        }
        copy_pin(root, name, pin)?;
    }
    // Fixed owner commands only. Preparation input cannot supply shell/provider argv.
    let manifest = json!({"checks":[{"requirement":"projected candidate whitespace consistency",
        "identity":"git-diff-check-projection-v1","command":["/usr/bin/git","diff","--check",input.startup.base.baseline.accepted_commit],"timeout_seconds":30}]});
    let manifest_raw = serde_json::to_vec(&manifest)?;
    let argv = vec![
        "-I".into(),
        root.join("run_gate.py").display().to_string(),
        "--manifest-json".into(),
        String::from_utf8(manifest_raw.clone()).unwrap(),
        "--cwd".into(),
        repo.display().to_string(),
    ];
    let python = Path::new("/usr/bin/python3.14");
    let python_hash = sha256_bytes(&fs::read(python)?);
    let git_hash = sha256_bytes(&fs::read("/usr/bin/git")?);
    let result = run(python, &argv, repo)?;
    let after = snapshot(repo, &input.candidate_paths)?;
    if before != after
        || !result.stderr.is_empty()
        || python_hash != sha256_bytes(&fs::read(python)?)
        || git_hash != sha256_bytes(&fs::read("/usr/bin/git")?)
    {
        return Err(error("gate inputs or tools changed during capture"));
    }
    let tree = before["head_tree"]
        .as_str()
        .ok_or_else(|| error("candidate tree absent"))?;
    let mut tree_hashes = serde_json::Map::new();
    for path in &input.candidate_paths {
        let raw = run(
            Path::new("/usr/bin/git"),
            &[
                "-C".into(),
                repo.display().to_string(),
                "show".into(),
                format!("{tree}:{path}"),
            ],
            repo,
        )?
        .stdout;
        tree_hashes.insert(path.clone(), json!(sha256_bytes(&raw)));
    }
    let capture = json!({"producer":"skills/slice-builder/scripts/run_gate.py","producer_sha256_before_after":input.gate_runner.sha256,
        "argv":[python.display().to_string(),argv[0],argv[1],argv[2],argv[3],argv[4],argv[5]],
        "argv_manifest_sha256":sha256_bytes(&manifest_raw),"child_interpreter_sha256":python_hash,
        "child_mode":{"isolated":1,"optimize":0,"explicit_conditional_exit":true},
        "git_runtime_before_after":{"executable_sha256":git_hash},"python_runtime_before_after":{"executable_sha256":python_hash},
        "before":before,"after":after,"frozen_gate_tree_oid":tree,"tested_path_sha256":before["path_sha256"],
        "candidate_tree_path_sha256":tree_hashes,"candidate_manifest_path_sha256":before["path_sha256"],"exit_status":0,"stderr":""});
    input.startup.base.gate_manifest =
        write(&root.join("gate-manifest.json"), &manifest_raw, 0o400)?;
    input.startup.base.gate_receipt =
        write(&root.join("gate-receipt.json"), &result.stdout, 0o400)?;
    input.startup.base.gate_capture = data(&root.join("gate-capture.json"), &capture)?;
    input.checkpoint_request["repository"] = json!(repo);
    input.checkpoint_request["gate_receipt_digest"] = json!(input.startup.base.gate_receipt.sha256);
    let request = serde_json::to_string(&input.checkpoint_request)?;
    let receipt = run(
        python,
        &[
            "-I".into(),
            "-B".into(),
            root.join("checkpoint.py").display().to_string(),
            "--request-json".into(),
            request,
        ],
        repo,
    )?
    .stdout;
    input.startup.base.candidate = write(&root.join("candidate.json"), &receipt, 0o400)?;
    data(
        &root.join("checkpoint-subject.json"),
        &json!({"schema_version":2,
        "construction_method":"slice_checkpoint_candidate_receipt",
        "evidence_cutoff":input.checkpoint_request["created_at"],
        "checkpoint":serde_json::from_slice::<Value>(&receipt)?}),
    )?;
    let profile = run(
        python,
        &[
            "-I".into(),
            "-B".into(),
            root.join("profile.py").display().to_string(),
            "profile".into(),
            "--receipt".into(),
            root.join("checkpoint-subject.json").display().to_string(),
        ],
        repo,
    )?
    .stdout;
    input.startup.base.physical_profile =
        write(&root.join("physical-profile.json"), &profile, 0o400)?;
    Ok(())
}

/// Create a fresh proposal package. Success never means installed, issued,
/// admitted, live-qualified, reviewed or accepted.
pub fn prepare(input_path: &Path, output: &Path) -> Result<Value> {
    if !input_path.is_absolute()
        || !output.is_absolute()
        || fs::canonicalize(input_path)? != input_path
        || output.exists()
        || fs::canonicalize(
            output
                .parent()
                .ok_or_else(|| error("output parent absent"))?,
        )? != output.parent().unwrap()
    {
        return Err(error(
            "input must be canonical and output a new canonical child",
        ));
    }
    let file = fs::File::open(input_path)?;
    if !file.metadata()?.is_file() {
        return Err(error("input must be a regular file"));
    }
    let mut bytes = Vec::new();
    file.take(1024 * 1024 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > 1024 * 1024 {
        return Err(error("input exceeds1MiB"));
    }
    let mut input: PreparationInput = serde_json::from_slice(&bytes)?;
    if input.schema_version != 1
        || input.standing != "uninstalled-proposal"
        || input.startup.profile != QUALIFIED_PROFILE
        || input.startup.base.profile != QUALIFIED_PROFILE
        || input.startup.base.evidence_profile != "claude-qualified-v1"
    {
        return Err(error(
            "only explicit qualified uninstalled proposal data is supported; fixture/control standing refused",
        ));
    }
    let provenance_raw = input.provenance.read(1024 * 1024)?;
    let provenance: Value = serde_json::from_slice(&provenance_raw)?;
    let origin = provenance["repository_path"]
        .as_str()
        .ok_or_else(|| error("genuine-origin repository_path absent"))?;
    if Path::new(origin).is_absolute()
        || Path::new(origin)
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
        || !input.candidate_paths.iter().any(|path| path == origin)
    {
        return Err(error(
            "genuine-origin path must be an explicit confined candidate path",
        ));
    }
    // Establish complete authority data before checkpoint/index effects. This is
    // syntax/pin validation only; issuance remains with the selected owner.
    for pin in [
        &input.startup.base.claims.source,
        &input.startup.base.claims.configuration,
        &input.startup.base.episode_authority,
    ] {
        let _: Value = serde_json::from_slice(&pin.read(1024 * 1024)?)?;
    }
    if sha256_bytes(&fs::read(input.startup.base.repository.join(origin))?)
        != provenance["candidate_file_sha256"]
        || provenance["actual_source_receipt"]
            .as_str()
            .is_none_or(str::is_empty)
    {
        return Err(error(
            "genuine-origin candidate bytes/receipt provenance differ",
        ));
    }
    if input.policy.repository != input.startup.base.repository
        || input.policy.admitted_uid != fs::metadata(output.parent().unwrap())?.uid()
    {
        return Err(error(
            "policy repository/admitted UID differ from preparation selections",
        ));
    }
    fs::create_dir(output)?;
    fs::set_permissions(output, fs::Permissions::from_mode(0o700))?;
    write(&output.join("input.json"), &bytes, 0o400)?;
    write(&output.join("provenance.json"), &provenance_raw, 0o400)?;
    subject(&mut input, output)?;
    let d = &mut input.startup.base;
    let q = &mut input.startup.qualified;
    let verified = verify_existing_subject_dynamic(
        &d.repository,
        &d.identity,
        &d.baseline,
        &d.candidate.read(1024 * 1024)?,
        &d.physical_profile.read(4 * 1024 * 1024)?,
        &d.gate_capture.read(1024 * 1024)?,
        &d.gate_manifest.read(1024 * 1024)?,
        &d.gate_receipt.read(1024 * 1024)?,
    )?;
    let profile = TrustedClaudeProfile::load_installed(
        pinned_program(&d.peer.executable)?,
        pinned_program(&q.guard)?,
        &q.requested_model,
        &q.requested_effort,
        q.accepted_observed_models.clone(),
        &q.catalog.read(64 * 1024)?,
    )
    .map_err(|e| error(e.to_string()))?;
    let rp0 = TrustedRp0Profile::load(
        pinned_program(&q.bwrap)?,
        pinned_program(&q.cbm)?,
        pinned_program(&q.cbm_launcher)?,
        pinned_program(&q.cbm_client)?,
    )
    .map_err(|e| error(e.to_string()))?;
    let boundary = CandidateBoundary::prepare_selected(
        rp0,
        &d.repository,
        output.join("native-boundary"),
        &input.project,
        &verified.reference.tree,
    )
    .map_err(|e| error(e.to_string()))?;
    let (boundary, boundary_hash) = boundary
        .publish_manifest()
        .map_err(|e| error(e.to_string()))?;
    q.boundary_root = output.join("native-boundary");
    q.boundary_manifest_sha256 = boundary_hash;
    let request = JobReviewRequest {
        identity: d.identity.clone(),
        candidate: verified.reference,
        physical_profile_digest: verified.profile_digest,
        cbm_store_sha256: boundary.index().store_sha256().into(),
        session_id: d.peer.session.clone(),
    };
    let job = input.policy.jobs_root.join(request.directory_name()?);
    if !input.policy.jobs_root.is_absolute()
        || fs::canonicalize(&input.policy.jobs_root)? != input.policy.jobs_root
        || fs::metadata(&input.policy.jobs_root)?.mode() & 0o077 != 0
        || fs::metadata(&input.policy.jobs_root)?.uid() != input.policy.admitted_uid
        || job.exists()
    {
        return Err(error(
            "jobsRoot must be existing uid-private canonical directory and exact job must be absent",
        ));
    }
    fs::create_dir(&job)?;
    fs::set_permissions(&job, fs::Permissions::from_mode(0o700))?;
    d.workspace = job.join("workspace");
    run(
        Path::new("/usr/bin/git"),
        &[
            "clone".into(),
            "-q".into(),
            "--no-hardlinks".into(),
            d.repository.display().to_string(),
            d.workspace.display().to_string(),
        ],
        output,
    )?;
    d.roots = crate::startup::Roots {
        campaign: job.join("campaign"),
        episode: job.join("episode"),
        claims: job.join("claims"),
        evidence: job.join("evidence"),
    };
    d.claims.source = copy_pin(output, "claims-source.json", &d.claims.source)?;
    d.claims.configuration =
        copy_pin(output, "claims-configuration.json", &d.claims.configuration)?;
    // This is the proposed installed form, never an issued authority record.
    // Preserve explicit source/writer selections and derive only exact subject identity.
    let mut episode: Value = serde_json::from_slice(&d.episode_authority.read(1024 * 1024)?)?;
    let episode_id = slice_campaign::contract::episode_digest(
        &json!({"identity":d.identity,"obligationId":d.obligation_id}),
    )?;
    episode["identity"] = json!({"runId":d.identity.run_id,"sliceNumber":d.identity.slice_number,
        "attemptId":d.identity.attempt_id,"planVersion":d.identity.plan_version,
        "reviewObligationId":d.obligation_id,"reviewEpisodeId":&episode_id[..32]});
    let subject_value = json!({"commit":request.candidate.commit,"tree":request.candidate.tree,"patchIdentity":request.candidate.patch_identity});
    episode["initialSubject"]["revision"] = json!(request.candidate.commit);
    episode["initialSubject"]["sha256"] = json!(review_episode_core::codec::digest(
        &review_episode_core::codec::parse_json(&subject_value.to_string())?
    ));
    review_episode_core::identity::Authority::parse(review_episode_core::codec::parse_json(
        &episode.to_string(),
    )?)?;
    d.episode_authority = data(&output.join("episode-authority.json"), &episode)?;
    input.policy.startup_authority = JobStartupAuthority {
        claims_source_sha256: d.claims.source.sha256.clone(),
        claims_configuration_sha256: d.claims.configuration.sha256.clone(),
        completion_consumer: q.completion_consumer.clone(),
    };
    let a = &mut input.policy.execution_template;
    a["reference"] = json!(output.join("acceptance.proposed.json"));
    a["revision"] = json!("unissued-preparation-proposal-v1");
    for (key, value) in [
        ("identity", json!(d.identity)),
        ("attemptId", json!(d.identity.attempt_id)),
        ("sessionId", json!(d.peer.session)),
        ("candidateTree", json!(request.candidate.tree)),
        (
            "physicalProfileDigest",
            json!(request.physical_profile_digest),
        ),
        ("cbmStoreSha256", json!(request.cbm_store_sha256)),
        ("claudeProfileDigest", json!(profile.profile_digest())),
        ("programSha256", json!(d.peer.executable.sha256)),
        ("guardSha256", json!(q.guard.sha256)),
        ("catalogSha256", json!(q.catalog.sha256)),
        ("requestedModel", json!(q.requested_model)),
        ("resourcePolicy", json!(q.resource_policy)),
        (
            "credentialSourceMetadataSha256",
            json!(q.credential_source_metadata_sha256),
        ),
        ("acceptedBoundarySha256", json!(d.accepted_boundary.sha256)),
        ("attemptHorizonMillis", json!(q.attempt_horizon_millis)),
    ] {
        a[key] = value;
    }
    let selection = input.policy.proposed_review_selection(&request)?;
    d.selection = data(&output.join("selection.json"), &selection)?;
    input.policy.execution_template["selectionDigest"] = json!(
        slice_campaign::contract::validate_selection(
            &selection,
            &request.identity,
            &request.candidate
        )?
        .campaign_digest
    );
    for claim in selection["specialists"][0]["requiredClaims"]
        .as_array()
        .ok_or_else(|| error("claim contracts absent"))?
    {
        let key = if claim["consumptionBoundary"] == "builder_projection" {
            "builderClaimId"
        } else {
            "campaignClaimId"
        };
        input.policy.execution_template[key] = claim["claimId"].clone();
    }
    input.policy.execution_template["promptTemplateSha256"] =
        json!(sha256_bytes(&qualified_prompt_template_for_document(d)?));
    let startup_path = job.join("startup.json");
    let startup_pin = data(&startup_path, &serde_json::to_value(&input.startup)?)?;
    let request_pin = write(
        &output.join("job-request.json"),
        &serde_json::to_vec_pretty(&request)?,
        0o600,
    )?;
    let policy_pin = data(
        &output.join("job-policy.proposed.json"),
        &serde_json::to_value(&input.policy)?,
    )?;
    let manifest = json!({"schemaVersion":1,"standing":"UNINSTALLED PROPOSAL; no issued grants, admission, provider qualification or operator approval",
        "fixedRuntimePolicyTarget":"/etc/work-engine/review-host/job-policy.json","proposedStartup":&input.startup,"proposedPolicy":&input.policy,"policy":policy_pin,"startup":startup_pin,"request":request_pin,
        "provenance":provenance,"candidate":request.candidate,"coverageComplete":boundary.index().coverage_complete(),
        "authorityInputs":{"claimsSource":input.policy.startup_authority.claims_source_sha256,"claimsConfiguration":input.policy.startup_authority.claims_configuration_sha256,"completionConsumer":input.policy.startup_authority.completion_consumer,"status":"proposed records require explicit operator/authority-owner review"},
        "credentialEligibility":"unchecked; metadata selection only; no credential contents read",
        "operatorGates":["independent source/package review","exact program/profile/catalog and credential eligibility qualification","approve exact policy/claims/episode selections","root installs exact policy with protected ancestry and readback"],
        "readback":{"initialize":["initialize","--startup",startup_path,"--job-request",request_pin.path],"serve":["serve","--startup",startup_path,"--job-request",request_pin.path]},
        "limitations":["No original full-repository index or new coding job claimed","Same UID preparation is not production operator/agent isolation","No campaign/episode/claims/evidence stores or InstalledAcceptancePin created"]});
    data(&output.join("manifest.json"), &manifest)?;
    Ok(manifest)
}
