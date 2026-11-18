//! Outcome 5: result check and envelope (with remediation 1: run binding, body model/tools, exact
//! user-text length, Host-reported fields, unknowns as null).

mod common;

use serde_json::{Value, json};
use std::collections::BTreeMap;
use work_engine_review_hosted::check::{
    AnalysisResult, Envelope, Outcome, RetainedRequest, RunArtifacts, check, load_run, load_state,
};
use work_engine_review_hosted::inputs::{self, JobInputs};
use work_engine_review_hosted::revision::{
    Profile, REVISION_R2_ID, REVISION_R2_REQUIREMENTS, REVISION_REQUIREMENTS, frozen_world,
    frozen_world_for, prepare_world, prepare_world_for,
};
use work_engine_review_hosted::sha256_hex;
use work_engine_review_hosted::subject::{REVIEW_PATCH, Snapshot};
use work_engine_review_hosted::task::{
    HOST_SUFFIX_BYTES, HOST_SUFFIX_SHA256, Limits, render, render_for,
};

const ACCEPTABLE: &str = include_str!(
    "../../../../app-server/tests/fixtures/implementation-review/acceptable-as-is.json"
);
const REMEDIATION: &str = include_str!(
    "../../../../app-server/tests/fixtures/implementation-review/remediation-required.json"
);
const JOB: &str = "dev-vertical-hosted-analysis-review-1";
const SRC: &str = "pub fn task() {}\n";

struct World {
    record: JobInputs,
    run: RunArtifacts,
    task: String,
}

/// The served manifest in `analyze.py`'s inventory shape (`:31-37`, `:64`).
fn manifest_bytes(manifest: &BTreeMap<String, String>) -> Vec<u8> {
    let inv: BTreeMap<&String, Value> = manifest
        .iter()
        .map(|(p, s)| (p, json!({"kind": "file", "sha256": s})))
        .collect();
    serde_json::to_vec(&inv).unwrap()
}

fn run_for(record: &JobInputs) -> RunArtifacts {
    let manifest = manifest_bytes(&record.snapshot_manifest);
    RunArtifacts {
        packet: json!({"schema": "work-engine.pi-analysis-packet.v1",
            "max_provider_requests": record.limits.max_requests,
            "max_snapshot_reads": record.limits.max_reads}),
        authority: json!({"job_id": record.job_id, "task_sha256": record.task_sha256}),
        job: json!({"job_id": record.job_id,
            "coding": {"provider": {
                "provider": record.profile.provider(),
                "model": record.profile.model()}},
            "workspace": {"manifest": {"sha256": sha256_hex(&manifest)}}}),
        manifest_bytes: manifest,
    }
}

fn world() -> World {
    let subject = common::frozen_subject();
    let frozen = prepare_world(REVISION_REQUIREMENTS, &subject, JOB).unwrap();
    let limits = Limits {
        max_requests: 8,
        max_reads: 16,
    };
    let task = render(frozen.contract(), &subject, limits).unwrap();
    let snapshot = Snapshot {
        parent: "1".repeat(40),
        members: [("src/task.mjs".to_owned(), SRC.as_bytes().to_vec())].into(),
    };
    let mut record = inputs::record(
        Profile::OpenaiCodex,
        REVISION_REQUIREMENTS,
        &frozen,
        &task,
        limits,
        &snapshot,
        frozen_world(&subject, JOB).unwrap(),
    );
    // The frozen patch's bytes are not available here; its manifest digest is the patchIdentity.
    record
        .snapshot_manifest
        .insert(REVIEW_PATCH.to_owned(), subject.patch_identity.clone());
    let run = run_for(&record);
    World { record, run, task }
}

/// The Host's analysis source, whose format string supplies the suffix bytes.
const HOST_ANALYSIS_SOURCE: &str = include_str!("../../pi-pressure-context-host/src/analysis.rs");

/// The bytes the Host appends to the task: what follows the `{}` in its prompt format string
/// (`analysis.rs:42-47`), read from the source so the suffix text is not copied into the crate.
fn host_suffix() -> String {
    let line = HOST_ANALYSIS_SOURCE
        .lines()
        .find(|l| l.contains("Bounded read-only analysis job"))
        .expect("the Host prompt format string");
    let start = line.find("\"{}").expect("format string start") + 3;
    let end = line.rfind('"').expect("format string end");
    line[start..end].replace("\\n", "\n")
}

fn host_text(task: &str) -> String {
    format!("{task}{}", host_suffix())
}

#[test]
fn host_suffix_constants_match_the_host_source() {
    let suffix = host_suffix();
    assert_eq!(suffix.len(), HOST_SUFFIX_BYTES);
    assert_eq!(sha256_hex(suffix.as_bytes()), HOST_SUFFIX_SHA256);
}

/// Same length as the Host suffix, different bytes.
fn same_length_other_suffix() -> String {
    let mut other = host_suffix().into_bytes();
    let last = other.len() - 1;
    other[last] = if other[last] == b'x' { b'y' } else { b'x' };
    String::from_utf8(other).unwrap()
}

#[test]
fn delivery_refuses_a_same_length_different_suffix() {
    let w = world();
    let mut reqs = requests(&w.task);
    let text = format!("{}{}", w.task, same_length_other_suffix());
    assert_eq!(text.len(), host_text(&w.task).len());
    reqs[0] = retained(body_with(&text, 1, |_| {}), 1);
    let e = run_check(&w, &answer_from(ACCEPTABLE, |_| {}), &reqs);
    failed_only(&e, "delivery");
    assert!(delivery_detail(&e).contains("not the Host suffix bytes"));

    let cw = claude_world();
    let mut reqs = claude_requests(&cw.task);
    let text = format!("{}{}", cw.task, same_length_other_suffix());
    reqs[0] = retained(messages_body_with(&text, 1, |_| {}), 1);
    let e = run_check(&cw, &answer_from(ACCEPTABLE, |_| {}), &reqs);
    failed_only(&e, "delivery");
    assert!(delivery_detail(&e).contains("not the Host suffix bytes"));
}

fn tools() -> Value {
    json!([
        {"type": "function", "name": "snapshot_list", "parameters": {}},
        {"type": "function", "name": "snapshot_read", "parameters": {}}
    ])
}

/// A Responses-shaped body as the Host retains it.
fn body_with(text: &str, request: u64, edit: impl FnOnce(&mut Value)) -> Vec<u8> {
    let mut input = vec![json!({"role":"user","content":[{"type":"input_text","text":text}]})];
    if request > 1 {
        input.push(
            json!({"type":"function_call","call_id":"c1","name":"snapshot_list","arguments":"{}"}),
        );
        input.push(
            json!({"type":"function_call_output","call_id":"c1","output":"{\"members\":[]}"}),
        );
    }
    let mut v = json!({"model":"gpt-6-luna","stream":true,"store":false,"instructions":"system","input":input,"tools":tools()});
    edit(&mut v);
    serde_json::to_vec(&v).unwrap()
}

fn retained(body: Vec<u8>, request: u64) -> RetainedRequest {
    RetainedRequest {
        request,
        attempt_digests: vec![Some(sha256_hex(&body))],
        body: Some(body),
    }
}

fn requests(task: &str) -> Vec<RetainedRequest> {
    (1..=2)
        .map(|n| retained(body_with(&host_text(task), n, |_| {}), n))
        .collect()
}

fn answer_from(sample: &str, edit: impl FnOnce(&mut Value)) -> AnalysisResult {
    let subject = common::frozen_subject();
    let mut v: Value = serde_json::from_str(sample).unwrap();
    v["subject"] =
        json!({"commit":subject.commit,"tree":subject.tree,"patchIdentity":subject.patch_identity});
    edit(&mut v);
    result(serde_json::to_string(&v).unwrap())
}

fn result(answer: String) -> AnalysisResult {
    let sha = sha256_hex(answer.as_bytes());
    AnalysisResult::new(answer, sha)
}

fn run_check(w: &World, answer: &AnalysisResult, reqs: &[RetainedRequest]) -> Envelope {
    check(&w.record, &w.run, answer, reqs)
}

fn outcome(e: &Envelope, id: &str) -> Outcome {
    e.checks
        .iter()
        .find(|c| c.id == id)
        .unwrap_or_else(|| panic!("{id}"))
        .outcome
        .clone()
}

fn assert_fixed_standing(e: &Envelope) {
    let v = serde_json::to_value(e).unwrap();
    assert_eq!(v["standing"], "owner_pending");
    assert_eq!(v["org_admission"], "none_requested");
    assert_eq!(v["consumable"], false);
    assert_eq!(v["proposition"], "route_qualification_only");
    assert_eq!(v["world"]["label"], "hosted-route-qualification-r1");
    assert_eq!(v["world"]["run_id"], "hosted-route-qualification-20261117");
    assert_eq!(v["requested_model"], "gpt-6-luna");
    assert_eq!(v["requested_provider"], "openai-codex");
    assert_eq!(v["frozen_world"]["outcome"], "refused");
    assert_eq!(v["frozen_world"]["known_incompatibilities"], 2);
    assert!(
        v["answer"].is_string(),
        "the answer is only a string member"
    );
    assert!(v["undischarged_blockers"].as_u64().unwrap() > 0);
    assert!(!e.not_claimed.is_empty());
}

fn failed_only(e: &Envelope, id: &str) {
    assert_eq!(e.outcome, "checks_failed");
    let failed: Vec<&str> = e
        .checks
        .iter()
        .filter(|c| c.outcome == Outcome::Failed)
        .map(|c| c.id.as_str())
        .collect();
    assert_eq!(failed, [id], "{:#?}", e.checks);
}

#[test]
fn acceptable_fixture_is_accepted_and_ungrounded_evidence_is_flagged() {
    let w = world();
    let e = run_check(&w, &answer_from(ACCEPTABLE, |_| {}), &requests(&w.task));
    assert_eq!(e.outcome, "checks_passed", "{:#?}", e.checks);
    assert_fixed_standing(&e);
    let g = e.grounding.as_ref().unwrap();
    assert_eq!((g.grounded, g.total), (0, 1));
    assert_eq!(g.ungrounded_decisive, [0]);
    assert_eq!(g.flags, ["evidence_ungrounded"]);
}

#[test]
fn grounded_evidence_is_counted_against_the_served_manifest() {
    let w = world();
    let sha = sha256_hex(SRC.as_bytes());
    let answer = answer_from(ACCEPTABLE, |v| {
        v["decisiveEvidence"][0]["sha256"] = json!(sha)
    });
    let e = run_check(&w, &answer, &requests(&w.task));
    assert_eq!(e.outcome, "checks_passed");
    let g = e.grounding.unwrap();
    assert_eq!((g.grounded, g.total), (1, 1));
    assert!(g.flags.is_empty());
}

#[test]
fn remediation_fixture_is_accepted() {
    let w = world();
    let e = run_check(&w, &answer_from(REMEDIATION, |_| {}), &requests(&w.task));
    assert_eq!(e.outcome, "checks_passed", "{:#?}", e.checks);
    assert_fixed_standing(&e);
    assert!(e.grounding.unwrap().ungrounded_decisive.is_empty());
}

#[test]
fn fenced_answer_is_refused_without_repair() {
    let w = world();
    let inner = answer_from(ACCEPTABLE, |_| {}).answer;
    let e = run_check(
        &w,
        &result(format!("```json\n{inner}\n```")),
        &requests(&w.task),
    );
    failed_only(&e, "strict_json");
    assert_eq!(outcome(&e, "result_contract"), Outcome::NotRun);
    assert_fixed_standing(&e);
}

#[test]
fn subject_mismatch_is_refused() {
    let w = world();
    let answer = answer_from(ACCEPTABLE, |v| v["subject"]["tree"] = json!("f".repeat(40)));
    failed_only(&run_check(&w, &answer, &requests(&w.task)), "subject_equal");
}

#[test]
fn answer_sha_mismatch_is_refused() {
    let w = world();
    let mut answer = answer_from(ACCEPTABLE, |_| {});
    answer.answer_sha256 = "0".repeat(64);
    let e = run_check(&w, &answer, &requests(&w.task));
    failed_only(&e, "answer_sha256");
    assert_eq!(outcome(&e, "strict_json"), Outcome::NotRun);
}

#[test]
fn schema_violation_is_refused() {
    let w = world();
    let answer = answer_from(ACCEPTABLE, |v| {
        v.as_object_mut().unwrap().remove("limitations");
    });
    failed_only(
        &run_check(&w, &answer, &requests(&w.task)),
        "result_contract",
    );
}

#[test]
fn contract_digest_mismatch_is_refused() {
    let mut w = world();
    w.record.contract_sha256 = "0".repeat(64);
    let e = run_check(&w, &answer_from(ACCEPTABLE, |_| {}), &requests(&w.task));
    failed_only(&e, "contract_recomputed");
}

#[test]
fn delivery_prefix_mismatch_is_refused() {
    let w = world();
    let mut reqs = requests(&w.task);
    reqs[0] = retained(
        body_with(
            &host_text(&w.task.replacen("Subject.", "Subjekt.", 1)),
            1,
            |_| {},
        ),
        1,
    );
    failed_only(
        &run_check(&w, &answer_from(ACCEPTABLE, |_| {}), &reqs),
        "delivery",
    );
}

#[test]
fn delivery_refuses_extra_text_after_the_task_and_suffix() {
    let w = world();
    let mut reqs = requests(&w.task);
    reqs[1] = retained(body_with(&format!("{}x", host_text(&w.task)), 2, |_| {}), 2);
    failed_only(
        &run_check(&w, &answer_from(ACCEPTABLE, |_| {}), &reqs),
        "delivery",
    );
}

#[test]
fn delivery_refuses_another_model_or_tool_set() {
    let w = world();
    let answer = answer_from(ACCEPTABLE, |_| {});
    let text = host_text(&w.task);
    for edit in [
        (|v: &mut Value| v["model"] = json!("gpt-6-sol")) as fn(&mut Value),
        |v| v["tools"].as_array_mut().unwrap().truncate(1),
        |v| v["tools"][1]["name"] = json!("workspace_exec"),
        |v| {
            v["input"]
                .as_array_mut()
                .unwrap()
                .push(json!({"type":"additional_tools","role":"developer","tools":[{"type":"function","name":"bash"}]}))
        },
    ] {
        let mut reqs = requests(&w.task);
        reqs[0] = retained(body_with(&text, 1, edit), 1);
        failed_only(&run_check(&w, &answer, &reqs), "delivery");
    }
}

#[test]
fn delivery_refuses_attempt_digest_mismatch_extra_user_text_and_missing_body() {
    let w = world();
    let answer = answer_from(ACCEPTABLE, |_| {});
    let failed = |reqs: Vec<RetainedRequest>| {
        outcome(&run_check(&w, &answer, &reqs), "delivery") == Outcome::Failed
    };
    let mut digest = requests(&w.task);
    digest[1].attempt_digests.push(Some("0".repeat(64)));
    assert!(failed(digest));
    let mut extra = requests(&w.task);
    extra[0] = retained(
        body_with(&host_text(&w.task), 1, |v| {
            v["input"]
                .as_array_mut()
                .unwrap()
                .push(json!({"role":"user","content":"more"}))
        }),
        1,
    );
    assert!(failed(extra));
    let mut missing = requests(&w.task);
    missing[1].body = None;
    assert!(failed(missing));
    assert!(failed(vec![]));
}

#[test]
fn served_snapshot_must_be_the_vetted_one_and_pinned_by_the_job() {
    let w = world();
    let answer = answer_from(ACCEPTABLE, |_| {});
    // Another member was served.
    let mut other = w.record.snapshot_manifest.clone();
    other.insert("notes.txt".into(), "0".repeat(64));
    let mut run = run_for(&w.record);
    run.manifest_bytes = manifest_bytes(&other);
    run.job["workspace"]["manifest"]["sha256"] = json!(sha256_hex(&run.manifest_bytes));
    let e = check(&w.record, &run, &answer, &requests(&w.task));
    failed_only(&e, "snapshot_served");
    // The job pins a different manifest file.
    let mut run = run_for(&w.record);
    run.job["workspace"]["manifest"]["sha256"] = json!("0".repeat(64));
    failed_only(
        &check(&w.record, &run, &answer, &requests(&w.task)),
        "snapshot_served",
    );
}

#[test]
fn subject_must_be_bound_to_scope_and_served_patch() {
    let mut w = world();
    let answer = answer_from(ACCEPTABLE, |_| {});
    w.record
        .snapshot_manifest
        .insert(REVIEW_PATCH.to_owned(), "0".repeat(64));
    w.run = run_for(&w.record);
    failed_only(
        &run_check(&w, &answer, &requests(&w.task)),
        "subject_bound_to_scope",
    );
}

#[test]
fn job_id_and_limits_must_be_the_runs() {
    let w = world();
    let answer = answer_from(ACCEPTABLE, |_| {});
    let mut run = run_for(&w.record);
    run.job["job_id"] = json!("dev-vertical-hosted-analysis-other");
    failed_only(
        &check(&w.record, &run, &answer, &requests(&w.task)),
        "job_id_bound",
    );
    let mut run = run_for(&w.record);
    run.packet["max_snapshot_reads"] = json!(17);
    failed_only(
        &check(&w.record, &run, &answer, &requests(&w.task)),
        "limits_bound",
    );
    let mut run = run_for(&w.record);
    run.authority["task_sha256"] = json!("0".repeat(64));
    failed_only(
        &check(&w.record, &run, &answer, &requests(&w.task)),
        "limits_bound",
    );
}

#[test]
fn job_provider_and_model_must_be_the_profiles() {
    for w in [world(), claude_world()] {
        let answer = answer_from(ACCEPTABLE, |_| {});
        let reqs = if w.record.profile == Profile::ClaudeSdk {
            claude_requests(&w.task)
        } else {
            requests(&w.task)
        };
        let e = check(&w.record, &w.run, &answer, &reqs);
        assert_eq!(outcome(&e, "job_profile_bound"), Outcome::Passed);
        for field in ["provider", "model"] {
            let mut run = w.run.clone();
            run.job["coding"]["provider"][field] = json!("other");
            failed_only(&check(&w.record, &run, &answer, &reqs), "job_profile_bound");
        }
    }
}

#[test]
fn frozen_world_detail_prints_numbers_not_rust_options() {
    let w = world();
    let e = run_check(&w, &answer_from(ACCEPTABLE, |_| {}), &requests(&w.task));
    let detail = &e
        .checks
        .iter()
        .find(|c| c.id == "frozen_world_unmet")
        .unwrap()
        .detail;
    assert!(!detail.contains("Some("), "{detail}");
    assert!(
        detail.contains("2 known incompatibilities, not established unestablished confinement"),
        "{detail}"
    );
}

#[test]
fn host_reported_fields_are_carried_and_unknowns_are_null() {
    let w = world();
    let mut answer = answer_from(ACCEPTABLE, |_| {});
    answer.settlement = Some(json!("CompletedSettled"));
    answer.hosted_attempts = Some(json!([{"route": "secondary", "payer": "openai-api"}]));
    let e = run_check(&w, &answer, &requests(&w.task));
    assert_eq!(e.host_reported.settlement, Some(json!("CompletedSettled")));
    assert_eq!(
        e.host_reported.hosted_attempts.as_ref().unwrap()[0]["route"],
        "secondary"
    );
    // A subject the preparation refuses: recomputation fails, unknowns stay null.
    let mut bad = w.record.clone();
    bad.subject.commit = String::new();
    let e = check(&bad, &w.run, &answer, &requests(&w.task));
    let v = serde_json::to_value(&e).unwrap();
    assert_eq!(e.outcome, "checks_failed");
    assert!(v["undischarged_blockers"].is_null());
    assert!(v["world"]["run_id"].is_null());
    assert!(v["frozen_world"]["known_incompatibilities"].is_null());
}

fn write_run(w: &World, dir: &std::path::Path, answer: &AnalysisResult) {
    let state = dir.join("state");
    let inputs_dir = dir.join("inputs");
    std::fs::create_dir_all(&state).unwrap();
    std::fs::create_dir_all(&inputs_dir).unwrap();
    let mut ledger = BTreeMap::new();
    for r in requests(&w.task) {
        std::fs::write(
            state.join(format!("coding-hosted-{}.body.json", r.request)),
            r.body.as_ref().unwrap(),
        )
        .unwrap();
        ledger.insert(
            r.request.to_string(),
            json!({"settled_microusd": 1, "attempts": [{"attempt": 1, "request_body_sha256": r.attempt_digests[0]}]}),
        );
    }
    std::fs::write(
        state.join("pi-state.json"),
        json!({"coding": {"provider_requests": ledger}}).to_string(),
    )
    .unwrap();
    std::fs::write(
        state.join("analysis-result.json"),
        json!({"schema":"work-engine.pi-analysis-result.v1","answer":answer.answer,"answer_sha256":answer.answer_sha256,"settlement":"CompletedSettled","provider_requests":2,"snapshot_deliveries":3,"hosted_attempts":[]}).to_string(),
    )
    .unwrap();
    std::fs::write(
        inputs_dir.join("snapshot-manifest.json"),
        &w.run.manifest_bytes,
    )
    .unwrap();
    std::fs::write(inputs_dir.join("packet.json"), w.run.packet.to_string()).unwrap();
    std::fs::write(
        inputs_dir.join("authority.json"),
        w.run.authority.to_string(),
    )
    .unwrap();
    std::fs::write(dir.join("job.json"), w.run.job.to_string()).unwrap();
    std::fs::write(
        dir.join("job-inputs.json"),
        inputs::pretty(&w.record).unwrap(),
    )
    .unwrap();
}

#[test]
fn run_directory_loads_result_ledger_bodies_and_job_artifacts() {
    let w = world();
    let dir = tempfile::tempdir().unwrap();
    let answer = answer_from(ACCEPTABLE, |_| {});
    write_run(&w, dir.path(), &answer);
    let (loaded, reqs) = load_state(&dir.path().join("state")).unwrap();
    assert_eq!(loaded.answer, answer.answer);
    assert_eq!(loaded.provider_requests, Some(json!(2)));
    assert_eq!(reqs, requests(&w.task));
    let run = load_run(&dir.path().join("inputs"), &dir.path().join("job.json")).unwrap();
    assert_eq!(run, w.run);
    assert_eq!(
        check(&w.record, &run, &loaded, &reqs).outcome,
        "checks_passed"
    );
}

#[test]
fn result_check_binary_writes_the_envelope_and_exits_by_outcome() {
    let w = world();
    let dir = tempfile::tempdir().unwrap();
    write_run(&w, dir.path(), &answer_from(REMEDIATION, |_| {}));
    let p = |name: &str| dir.path().join(name).to_str().unwrap().to_owned();
    let run = || {
        std::process::Command::new(env!("CARGO_BIN_EXE_review-result-check"))
            .args([
                "--inputs",
                &p("job-inputs.json"),
                "--run-inputs",
                &p("inputs"),
                "--job",
                &p("job.json"),
                "--state-dir",
                &p("state"),
                "--out",
                &p("envelope.json"),
            ])
            .output()
            .unwrap()
    };
    let first = run();
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stdout)
    );
    let e: Envelope =
        serde_json::from_slice(&std::fs::read(dir.path().join("envelope.json")).unwrap()).unwrap();
    assert_fixed_standing(&e);
    // A second run refuses to overwrite the envelope.
    assert_eq!(run().status.code(), Some(2));
}

// ---------------------------------------------------------------------------------------------
// Remediation 4: the Anthropic Messages wire (`claude-sdk` profile, requirements revision r2).
// Bodies are synthetic, shaped after the Host's own fixtures (`pi_coding_hosted.rs:2919-2926`)
// and tool surface (`pi_coding_analysis.rs:114-143`) at `5af34cbf`.
// ---------------------------------------------------------------------------------------------

fn claude_world() -> World {
    let subject = common::frozen_subject();
    let profile = Profile::ClaudeSdk;
    let frozen = prepare_world_for(profile, REVISION_R2_REQUIREMENTS, &subject, JOB).unwrap();
    let limits = Limits {
        max_requests: 8,
        max_reads: 16,
    };
    let task = render_for(profile, frozen.contract(), &subject, limits).unwrap();
    let snapshot = Snapshot {
        parent: "1".repeat(40),
        members: [("src/task.mjs".to_owned(), SRC.as_bytes().to_vec())].into(),
    };
    let mut record = inputs::record(
        profile,
        REVISION_R2_REQUIREMENTS,
        &frozen,
        &task,
        limits,
        &snapshot,
        frozen_world_for(profile, &subject).unwrap(),
    );
    record
        .snapshot_manifest
        .insert(REVIEW_PATCH.to_owned(), subject.patch_identity.clone());
    let run = run_for(&record);
    World { record, run, task }
}

fn claude_tools() -> Value {
    json!([
        {"name": "DeferredToolPlaceholder", "description": "d", "input_schema": {"type": "object"}, "defer_loading": true},
        {"name": "mcp__pi__snapshot_list", "description": "l", "input_schema": {"type": "object"}},
        {"name": "mcp__pi__snapshot_read", "description": "r", "input_schema": {"type": "object"}}
    ])
}

fn claude_system() -> Value {
    json!([
        {"type": "text", "text": "x-anthropic-billing-header: cc_entrypoint=sdk-ts;"},
        {"type": "text", "text": "You are a Claude agent, built on Anthropic's Claude Agent SDK."},
        {"type": "text", "text": "You are Claude, working as a coding agent in pi."}
    ])
}

/// The environment message Claude Code inserts after the user turn.
fn environment() -> Value {
    json!({"role": "system", "content": [{"type": "text", "text": "environment block"}]})
}

/// A Messages body as the Host retains it: request 1 is the user turn and the environment
/// message; later requests extend it with an assistant turn (thinking + tool call) and a user
/// turn holding the tool result.
fn messages_body_with(text: &str, request: u64, edit: impl FnOnce(&mut Value)) -> Vec<u8> {
    let mut messages = vec![
        json!({"role": "user", "content": [{"type": "text", "text": text}]}),
        environment(),
    ];
    if request > 1 {
        messages.push(json!({"role": "assistant", "content": [
            {"type": "thinking", "thinking": "", "thinkingSignature": "sig"},
            {"type": "tool_use", "id": "toolu_1", "name": "mcp__pi__snapshot_list", "input": {}}
        ]}));
        messages.push(json!({"role": "user", "content": [
            {"type": "tool_result", "tool_use_id": "toolu_1", "content": "{\"members\":[]}"}
        ]}));
    }
    let mut v = json!({
        "model": "claude-haiku-5-5", "stream": true, "max_tokens": 4096,
        "system": claude_system(), "messages": messages, "tools": claude_tools()
    });
    edit(&mut v);
    serde_json::to_vec(&v).unwrap()
}

fn claude_requests(task: &str) -> Vec<RetainedRequest> {
    (1..=2)
        .map(|n| retained(messages_body_with(&host_text(task), n, |_| {}), n))
        .collect()
}

fn delivery_detail(e: &Envelope) -> String {
    e.checks
        .iter()
        .find(|c| c.id == "delivery")
        .unwrap()
        .detail
        .clone()
}

fn sha_of(value: &Value) -> String {
    sha256_hex(&serde_json::to_vec(value).unwrap())
}

#[test]
fn claude_sdk_messages_body_passes_and_records_insertions_by_digest() {
    let w = claude_world();
    let e = run_check(
        &w,
        &answer_from(ACCEPTABLE, |_| {}),
        &claude_requests(&w.task),
    );
    assert_eq!(e.outcome, "checks_passed", "{:#?}", e.checks);
    assert_eq!(outcome(&e, "delivery"), Outcome::Passed);
    let v = serde_json::to_value(&e).unwrap();
    assert_eq!(v["profile"], "claude-sdk");
    assert_eq!(v["requested_provider"], "claude-sdk");
    assert_eq!(v["requested_model"], "claude-haiku-5-5");
    assert_eq!(v["world"]["label"], REVISION_R2_ID);
    assert_eq!(v["consumable"], false);
    assert_eq!(v["standing"], "owner_pending");
    // The frozen candidate stays unmet on this profile: prepared, confinement unestablished.
    assert_eq!(v["frozen_world"]["outcome"], "prepared_conditions_unmet");
    assert_eq!(v["frozen_world"]["known_incompatibilities"], 0);
    assert!(
        v["frozen_world"]["unestablished_confinement"]
            .as_u64()
            .unwrap()
            > 0
    );
    // Insertions: digests only, per retained request, never the text.
    let insertions = e.host_reported.request_insertions.as_ref().unwrap();
    assert_eq!(insertions.len(), 2);
    for (i, ins) in insertions.iter().enumerate() {
        assert_eq!(ins.request, i as u64 + 1);
        let blocks: Vec<String> = claude_system()
            .as_array()
            .unwrap()
            .iter()
            .map(sha_of)
            .collect();
        assert_eq!(ins.system_blocks_sha256, blocks);
        assert_eq!(ins.environment_message_sha256, Some(sha_of(&environment())));
    }
    assert!(
        !serde_json::to_string(&v["host_reported"])
            .unwrap()
            .contains("environment block")
    );
}

#[test]
fn responses_profile_records_no_request_insertions() {
    let w = world();
    let e = run_check(&w, &answer_from(ACCEPTABLE, |_| {}), &requests(&w.task));
    assert_eq!(e.outcome, "checks_passed");
    assert!(e.host_reported.request_insertions.is_none());
}

/// Request 1 of a claude-sdk run edited by `edit` must fail only the delivery check, with a
/// reason containing `expected` (so a case cannot pass for the wrong reason).
fn assert_claude_delivery_refused(cases: Vec<(&str, fn(&mut Value), &str)>) {
    let w = claude_world();
    let answer = answer_from(ACCEPTABLE, |_| {});
    for (name, edit, expected) in cases {
        let mut reqs = claude_requests(&w.task);
        reqs[0] = retained(messages_body_with(&host_text(&w.task), 1, edit), 1);
        let e = run_check(&w, &answer, &reqs);
        assert_eq!(e.outcome, "checks_failed", "{name} was not refused");
        failed_only(&e, "delivery");
        assert!(
            delivery_detail(&e).contains(expected),
            "{name}: {}",
            delivery_detail(&e)
        );
    }
}

#[test]
fn claude_sdk_refuses_a_request_for_another_model() {
    assert_claude_delivery_refused(vec![
        (
            "wrong model",
            |v| v["model"] = json!("claude-opus-5-5"),
            "model is not",
        ),
        (
            "no model",
            |v| {
                v.as_object_mut().unwrap().remove("model");
            },
            "model is not",
        ),
    ]);
}

#[test]
fn claude_sdk_refuses_a_wrong_tool_set() {
    let cases: Vec<(&str, fn(&mut Value), &str)> = vec![
        (
            "extra tool",
            |v| {
                v["tools"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"name": "mcp__pi__bash", "input_schema": {"type": "object"}}))
            },
            "declared tools are not exactly",
        ),
        (
            "missing tool",
            |v| v["tools"].as_array_mut().unwrap().truncate(2),
            "declared tools are not exactly",
        ),
        (
            "missing placeholder",
            |v| {
                v["tools"].as_array_mut().unwrap().remove(0);
            },
            "declared tools are not exactly",
        ),
        (
            "unprefixed tool name",
            |v| v["tools"][1]["name"] = json!("snapshot_list"),
            "declared tools are not exactly",
        ),
        (
            "duplicate tool",
            |v| {
                let t = v["tools"][2].clone();
                v["tools"].as_array_mut().unwrap().push(t);
            },
            "declared tools are not exactly",
        ),
        (
            "no tools array",
            |v| {
                v.as_object_mut().unwrap().remove("tools");
            },
            "no tools array",
        ),
    ];
    assert_claude_delivery_refused(cases);
}

#[test]
fn claude_sdk_refuses_a_task_that_is_not_the_first_text_block() {
    let w = claude_world();
    let answer = answer_from(ACCEPTABLE, |_| {});
    let text = host_text(&w.task);
    let task_block = json!({"type": "text", "text": text});
    let cases: Vec<(&str, Value, &str)> = vec![
        (
            "another text block first",
            json!([{"type": "text", "text": "Preamble."}, task_block.clone()]),
            "2 content blocks",
        ),
        (
            "non-text block first, task second",
            json!([{"type": "image", "source": {}}, task_block.clone()]),
            "not a text block",
        ),
        (
            "non-text block carrying the task bytes",
            json!([{"type": "document", "text": text}]),
            "not a text block",
        ),
        ("empty content", json!([]), "not a text block"),
        ("no blocks, null content", Value::Null, "has no content"),
    ];
    for (name, content, expected) in cases {
        let mut reqs = claude_requests(&w.task);
        reqs[0] = retained(
            messages_body_with(&text, 1, |v| v["messages"][0]["content"] = content),
            1,
        );
        let e = run_check(&w, &answer, &reqs);
        assert_eq!(e.outcome, "checks_failed", "{name} was not refused");
        failed_only(&e, "delivery");
        assert!(
            delivery_detail(&e).contains(expected),
            "{name}: {}",
            delivery_detail(&e)
        );
    }
    // The task text altered, or followed by anything beyond the Host suffix.
    for (name, text, expected) in [
        (
            "task bytes altered",
            host_text(&w.task.replacen("Subject.", "Subjekt.", 1)),
            "does not start with the task bytes",
        ),
        (
            "text after the suffix",
            format!("{}x", host_text(&w.task)),
            "Host suffix",
        ),
        ("suffix missing", w.task.clone(), "Host suffix"),
    ] {
        let mut reqs = claude_requests(&w.task);
        reqs[1] = retained(messages_body_with(&text, 2, |_| {}), 2);
        let e = run_check(&w, &answer, &reqs);
        assert_eq!(e.outcome, "checks_failed", "{name} was not refused");
        failed_only(&e, "delivery");
        assert!(
            delivery_detail(&e).contains(expected),
            "{name}: {}",
            delivery_detail(&e)
        );
    }
    // A string `messages[0]` content (Claude Code's later rewrite) is the same task + suffix.
    let mut reqs = claude_requests(&w.task);
    reqs[1] = retained(
        messages_body_with(&text, 2, |v| {
            v["messages"][0]["content"] = json!(text.clone())
        }),
        2,
    );
    assert_eq!(run_check(&w, &answer, &reqs).outcome, "checks_passed");
}

#[test]
fn claude_sdk_refuses_extra_user_text_and_stray_roles() {
    let w = claude_world();
    let answer = answer_from(ACCEPTABLE, |_| {});
    let text = host_text(&w.task);
    let cases: Vec<(&str, fn(&mut Value), &str)> = vec![
        (
            "extra text block in messages[0]",
            |v| {
                v["messages"][0]["content"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"type": "text", "text": "more"}))
            },
            "2 content blocks",
        ),
        (
            "extra user message",
            |v| {
                v["messages"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"role": "user", "content": "more"}))
            },
            "other than tool results",
        ),
        (
            "extra user text block beside a tool result",
            |v| {
                v["messages"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"role": "user", "content": [
                        {"type": "tool_result", "tool_use_id": "t", "content": "x"},
                        {"type": "text", "text": "more"}
                    ]}))
            },
            "other than tool results",
        ),
        (
            "system message after messages[1]",
            |v| {
                v["messages"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"role": "system", "content": "second"}))
            },
            "tolerated only at messages[1] or directly after",
        ),
        (
            "unknown role",
            |v| {
                v["messages"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"role": "tool", "content": "x"}))
            },
            "unknown role",
        ),
        (
            "messages[0] not a user message",
            |v| v["messages"][0]["role"] = json!("assistant"),
            "not a user message",
        ),
        ("no messages", |v| v["messages"] = json!([]), "no messages"),
    ];
    for (name, edit, expected) in cases {
        let mut reqs = claude_requests(&w.task);
        reqs[0] = retained(messages_body_with(&text, 1, edit), 1);
        let e = run_check(&w, &answer, &reqs);
        assert_eq!(e.outcome, "checks_failed", "{name} was not refused");
        failed_only(&e, "delivery");
        assert!(
            delivery_detail(&e).contains(expected),
            "{name}: {}",
            delivery_detail(&e)
        );
    }
}

// --- Claude Code's `<total_tokens>` system messages (synthetic text; observed shape only) ------

fn text_system(text: &str) -> Value {
    json!({"role": "system", "content": [{"type": "text", "text": text}]})
}

fn tool_use_turn(id: &str) -> Value {
    json!({"role": "assistant", "content": [
        {"type": "tool_use", "id": id, "name": "mcp__pi__snapshot_list", "input": {}}
    ]})
}

fn tool_result_turn(id: &str) -> Value {
    json!({"role": "user", "content": [
        {"type": "tool_result", "tool_use_id": id, "content": "{\"members\":[]}"}
    ]})
}

/// The observed messages of request `n` (1..=3): later requests rewrite an earlier system message
/// from a block list to a string and append a `<total_tokens>` system message after the tool result.
fn observed_messages(text: &str, n: u64) -> Vec<Value> {
    let user = json!({"role": "user", "content": [{"type": "text", "text": text}]});
    match n {
        1 => vec![user, text_system("env one")],
        2 => vec![
            user,
            json!({"role": "system", "content": "env one"}),
            tool_use_turn("t1"),
            tool_result_turn("t1"),
            text_system("<total_tokens>1000 tokens left</total_tokens>"),
        ],
        _ => vec![
            user,
            json!({"role": "system", "content": "env one"}),
            tool_use_turn("t1"),
            tool_result_turn("t1"),
            json!({"role": "system", "content": "<total_tokens>1000 tokens left</total_tokens>"}),
            tool_use_turn("t2"),
            tool_result_turn("t2"),
            text_system("<total_tokens>900 tokens left</total_tokens>"),
        ],
    }
}

fn body_of_messages(text: &str, request: u64, messages: Vec<Value>) -> RetainedRequest {
    retained(
        messages_body_with(text, request, |v| v["messages"] = Value::Array(messages)),
        request,
    )
}

#[test]
fn claude_sdk_tolerates_total_tokens_system_messages_and_records_digests() {
    let w = claude_world();
    let text = host_text(&w.task);
    let reqs: Vec<RetainedRequest> = (1..=3)
        .map(|n| body_of_messages(&text, n, observed_messages(&text, n)))
        .collect();
    let e = run_check(&w, &answer_from(ACCEPTABLE, |_| {}), &reqs);
    assert_eq!(e.outcome, "checks_passed", "{:#?}", e.checks);
    assert_eq!(outcome(&e, "delivery"), Outcome::Passed);
    let ins = e.host_reported.request_insertions.as_ref().unwrap();
    assert_eq!(ins.len(), 3);
    assert!(ins[0].inserted_system_messages.is_empty());
    assert_eq!(
        ins[0].environment_message_sha256,
        Some(sha_of(&text_system("env one")))
    );
    let tokens_1 = text_system("<total_tokens>1000 tokens left</total_tokens>");
    assert_eq!(
        ins[1]
            .inserted_system_messages
            .iter()
            .map(|m| (m.message_index, m.sha256.clone()))
            .collect::<Vec<_>>(),
        [(4, sha_of(&tokens_1))]
    );
    assert_eq!(
        ins[1].environment_message_sha256,
        Some(sha_of(&json!({"role": "system", "content": "env one"})))
    );
    assert_eq!(
        ins[2]
            .inserted_system_messages
            .iter()
            .map(|m| (m.message_index, m.sha256.clone()))
            .collect::<Vec<_>>(),
        [
            (
                4,
                sha_of(
                    &json!({"role": "system", "content": "<total_tokens>1000 tokens left</total_tokens>"})
                )
            ),
            (
                7,
                sha_of(&text_system("<total_tokens>900 tokens left</total_tokens>"))
            )
        ]
    );
    // Digests only: the inserted text is never carried into the envelope.
    assert!(
        !serde_json::to_string(&serde_json::to_value(&e).unwrap()["host_reported"])
            .unwrap()
            .contains("tokens left")
    );
}

#[test]
fn claude_sdk_refuses_system_messages_outside_the_observed_positions() {
    let w = claude_world();
    let answer = answer_from(ACCEPTABLE, |_| {});
    let text = host_text(&w.task);
    let user = |t: &str| json!({"role": "user", "content": [{"type": "text", "text": t}]});
    let cases: Vec<(&str, Vec<Value>, &str)> = vec![
        (
            "system message after an assistant message",
            vec![
                user(&text),
                text_system("env"),
                tool_use_turn("t1"),
                text_system("<total_tokens>1 tokens left</total_tokens>"),
            ],
            "tolerated only at messages[1] or directly after",
        ),
        (
            "two system messages in a row",
            vec![
                user(&text),
                text_system("env"),
                tool_use_turn("t1"),
                tool_result_turn("t1"),
                text_system("<total_tokens>1 tokens left</total_tokens>"),
                text_system("<total_tokens>2 tokens left</total_tokens>"),
            ],
            "tolerated only at messages[1] or directly after",
        ),
        (
            "system message with a non-text block",
            vec![
                user(&text),
                text_system("env"),
                tool_use_turn("t1"),
                tool_result_turn("t1"),
                json!({"role": "system", "content": [
                    {"type": "text", "text": "ok"},
                    {"type": "tool_use", "id": "x", "name": "n", "input": {}}
                ]}),
            ],
            "other than a string or text blocks",
        ),
        (
            "messages[1] system message with a non-text block",
            vec![
                user(&text),
                json!({"role": "system", "content": [{"type": "image", "source": {}}]}),
            ],
            "other than a string or text blocks",
        ),
        (
            "system message without string or list content",
            vec![
                user(&text),
                text_system("env"),
                tool_use_turn("t1"),
                tool_result_turn("t1"),
                json!({"role": "system"}),
            ],
            "other than a string or text blocks",
        ),
        // The user text message is itself refused before the system message after it is reached.
        (
            "system message right after a user text message",
            vec![
                user(&text),
                text_system("env"),
                tool_use_turn("t1"),
                user("more"),
                text_system("<total_tokens>1 tokens left</total_tokens>"),
            ],
            "other than tool results",
        ),
    ];
    for (name, messages, expected) in cases {
        let mut reqs = claude_requests(&w.task);
        reqs[0] = body_of_messages(&text, 1, messages);
        let e = run_check(&w, &answer, &reqs);
        assert_eq!(e.outcome, "checks_failed", "{name} was not refused");
        failed_only(&e, "delivery");
        assert!(
            delivery_detail(&e).contains(expected),
            "{name}: {}",
            delivery_detail(&e)
        );
    }
}

#[test]
fn claude_sdk_refuses_digest_mismatch_and_missing_body() {
    let w = claude_world();
    let answer = answer_from(ACCEPTABLE, |_| {});
    let failed = |reqs: Vec<RetainedRequest>| {
        let e = run_check(&w, &answer, &reqs);
        failed_only(&e, "delivery");
        delivery_detail(&e)
    };
    let mut digest = claude_requests(&w.task);
    digest[1].attempt_digests.push(Some("0".repeat(64)));
    assert!(failed(digest).contains("request_body_sha256 is not the retained body's"));
    let mut none = claude_requests(&w.task);
    none[0].attempt_digests = vec![None];
    assert!(failed(none).contains("request_body_sha256"));
    let mut missing = claude_requests(&w.task);
    missing[1].body = None;
    assert!(failed(missing).contains("body not retained"));
    assert!(failed(vec![]).contains("no retained request"));
    // A body edited after its digest was recorded.
    let mut edited = claude_requests(&w.task);
    edited[0].body = Some(messages_body_with(&host_text(&w.task), 1, |v| {
        v["max_tokens"] = json!(1)
    }));
    assert!(failed(edited).contains("request_body_sha256 is not the retained body's"));
}

#[test]
fn claude_sdk_thinking_is_neither_answer_nor_evidence() {
    let w = claude_world();
    let answer = answer_from(ACCEPTABLE, |_| {});
    let other = answer_from(REMEDIATION, |_| {}).answer;
    // A thinking block carrying a complete, valid review answer in an assistant turn: not read.
    let with_thinking = |thinking: &str| {
        let mut reqs = claude_requests(&w.task);
        reqs[1] = retained(
            messages_body_with(&host_text(&w.task), 2, |v| {
                v["messages"][2]["content"][0]["thinking"] = json!(thinking)
            }),
            2,
        );
        run_check(&w, &answer, &reqs)
    };
    let a = with_thinking(&other);
    let b = with_thinking("different thinking");
    for e in [&a, &b] {
        assert_eq!(e.outcome, "checks_passed", "{:#?}", e.checks);
        assert_eq!(
            e.answer, answer.answer,
            "the answer is the Host's answer only"
        );
        assert_ne!(e.answer, other);
    }
    // The recorded insertions do not depend on assistant content.
    assert_eq!(
        a.host_reported.request_insertions,
        b.host_reported.request_insertions
    );
    // A thinking-only final message leaves the Host no answer: the empty answer fails strictly.
    let none = run_check(&w, &result(String::new()), &claude_requests(&w.task));
    failed_only(&none, "strict_json");
    // A thinking block offered as the answer is not an implementation-review result.
    let block = json!({"type": "thinking", "thinking": other, "thinkingSignature": "sig"});
    let as_answer = run_check(&w, &result(block.to_string()), &claude_requests(&w.task));
    failed_only(&as_answer, "result_contract");
}

#[test]
fn claude_sdk_envelope_never_claims_confinement_or_route() {
    let w = claude_world();
    let mut answer = answer_from(ACCEPTABLE, |_| {});
    answer.hosted_attempts = Some(json!([{"route": "claude-code", "payer": "subscription"}]));
    let e = run_check(&w, &answer, &claude_requests(&w.task));
    assert_eq!(
        e.host_reported.hosted_attempts.as_ref().unwrap()[0]["route"],
        "claude-code",
        "carried as reported"
    );
    let not_claimed = e.not_claimed.join("\n");
    assert!(not_claimed.contains("execution confinement is unestablished"));
    assert!(
        not_claimed
            .contains("serving route, payer and any subscription billing guard are Host-reported")
    );
    assert!(
        not_claimed.contains("system blocks and environment message are recorded by digest only")
    );
}
