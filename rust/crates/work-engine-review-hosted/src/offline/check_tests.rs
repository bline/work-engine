//! Run the two real wire check paths over synthetic job artifacts.

use super::*;
use crate::check::{
    self, AnalysisResult, Envelope, Outcome, RetainedRequest, RunArtifacts, load_run, load_state,
};
use crate::inputs::JobInputs;
use crate::revision;
use crate::subject::REVIEW_PATCH;
use crate::task::{HOST_SUFFIX_BYTES, HOST_SUFFIX_SHA256};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Component, Path};

const ACCEPTABLE: &str = include_str!(
    "../../../../../app-server/tests/fixtures/implementation-review/acceptable-as-is.json"
);
const REMEDIATION: &str = include_str!(
    "../../../../../app-server/tests/fixtures/implementation-review/remediation-required.json"
);

struct World {
    expected: ExpectedWorld,
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

/// Inventory the actual test-owned snapshot, not a list of expected paths. Reject
/// links and other non-regular entries; this fixture has no concurrent writers.
fn snapshot_inventory(root: &Path) -> io::Result<BTreeMap<String, String>> {
    fn collect(root: &Path, dir: &Path, manifest: &mut BTreeMap<String, String>) -> io::Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let kind = entry.file_type()?;
            if kind.is_dir() {
                collect(root, &path, manifest)?;
            } else if kind.is_file() {
                let name = path
                    .strip_prefix(root)
                    .unwrap()
                    .components()
                    .map(|component| match component {
                        Component::Normal(name) => {
                            name.to_str().ok_or_else(|| invalid_entry(&path))
                        }
                        _ => Err(invalid_entry(&path)),
                    })
                    .collect::<io::Result<Vec<_>>>()?
                    .join("/");
                manifest.insert(name, sha256_hex(&fs::read(&path)?));
            } else {
                return Err(invalid_entry(&path));
            }
        }
        Ok(())
    }

    if !fs::symlink_metadata(root)?.is_dir() {
        return Err(invalid_entry(root));
    }
    let mut manifest = BTreeMap::new();
    collect(root, root, &mut manifest)?;
    Ok(manifest)
}

fn invalid_entry(path: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("unexpected snapshot entry: {}", path.display()),
    )
}

fn pin_served_snapshot(run: &mut RunArtifacts, snapshot: &Path) {
    run.manifest_bytes = manifest_bytes(&snapshot_inventory(snapshot).unwrap());
    run.job["workspace"]["manifest"]["sha256"] = json!(sha256_hex(&run.manifest_bytes));
}

// For artifact-only counterfactuals; the positive writer path inventories disk below.
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
    world_for(Profile::OpenaiCodex)
}

fn world_for(profile: Profile) -> World {
    let repo = git_fixture();
    write_world(profile, repo.path(), &repo.path().join("output"))
}

fn write_world(profile: Profile, repo: &Path, out: &Path) -> World {
    let expected = ExpectedWorld::pinned(profile);
    let subject = pinned_subject();
    let written = inputs::write(&expected, repo, &subject, JOB, LIMITS, out).unwrap();
    let read = |path: &str| fs::read(out.join(path)).unwrap();
    let record: JobInputs = serde_json::from_slice(&read("job-inputs.json")).unwrap();
    assert_eq!(record, written);
    let task = String::from_utf8(read("task.txt")).unwrap();
    let mut run = run_for(&record);
    pin_served_snapshot(&mut run, &out.join("snapshot"));
    assert_eq!(read("snapshot/.review/patch.diff"), PATCH);
    assert_eq!(run.served_manifest().unwrap()[REVIEW_PATCH], PATCH_IDENTITY);
    assert_eq!(read("requirements.json"), expected.requirements());
    assert_eq!(sha256_hex(&read("contract.json")), record.contract_sha256);
    assert!(task.ends_with(std::str::from_utf8(&read("contract.json")).unwrap()));
    assert_eq!(record.parent, PARENT);
    assert_eq!(record.subject, subject);
    World {
        expected,
        record,
        run,
        task,
    }
}

/// The Host's analysis source, whose format string supplies the suffix bytes.
const HOST_ANALYSIS_SOURCE: &str =
    include_str!("../../../pi-pressure-context-host/src/analysis.rs");

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

#[test]
fn unselected_messages_materializes_and_compares_exact_bytes() {
    use crate::messages_artifacts::{MessagesMaterialization, REVISION};

    let repo = git_fixture();
    let subject = pinned_subject();
    let make = |baseline, scope| {
        MessagesMaterialization::prepare(
            repo.path(),
            baseline,
            &subject,
            scope,
            "test-only contract",
            revision::CLAUDE_MODEL,
            LIMITS,
        )
    };
    assert!(matches!(
        make(TWIN, EXACT_SCOPE),
        Err(Refusal::BaselineMismatch { .. })
    ));
    assert!(matches!(
        make(PARENT, PATCH_IDENTITY),
        Err(Refusal::ScopeDigestMismatch { .. })
    ));
    let expected = make(PARENT, EXACT_SCOPE).unwrap();
    let manifest = expected.manifest_bytes();
    assert_eq!(manifest, manifest_bytes(&expected.snapshot().manifest()));
    let good = answer_from(ACCEPTABLE, |_| {});
    let reqs = claude_requests(expected.task());
    let report = expected.compare(manifest, &reqs, &good).unwrap();
    assert_eq!(report.revision, REVISION);
    assert_eq!(report.subject_digest, EXACT_SCOPE);
    assert_eq!(report.answer_sha256, good.answer_sha256);
    assert_eq!(report.request_insertions.len(), reqs.len());

    let mut different_manifest = manifest.to_owned();
    different_manifest.push(b' '); // Equal parsed inventory does not suffice.
    assert!(expected.compare(&different_manifest, &reqs, &good).is_err());
    let twin = answer_from(ACCEPTABLE, |v| v["subject"]["commit"] = json!(TWIN));
    assert!(expected.compare(manifest, &reqs, &twin).is_err());
    let duplicated = result(good.answer.replacen('{', "{\"subject\":{},", 1));
    assert!(expected.compare(manifest, &reqs, &duplicated).is_err());
    let wrapped = result(format!("```json\n{}\n```", good.answer));
    assert!(expected.compare(manifest, &reqs, &wrapped).is_err());

    let wrong_task = claude_requests(&format!("different {}", expected.task()));
    assert!(expected.compare(manifest, &wrong_task, &good).is_err());
    let mut replay = reqs.clone();
    let original_digest = replay[0].attempt_digests[0].clone();
    replay[0].attempt_digests.push(original_digest);
    assert!(expected.compare(manifest, &replay, &good).is_err());
    let mut ambiguous = reqs.clone();
    let body = reqs[0].body.as_deref().unwrap();
    let body = std::str::from_utf8(body)
        .unwrap()
        .replacen('{', "{\"model\":\"other\",", 1);
    ambiguous[0] = retained(body.into_bytes(), 1);
    assert!(expected.compare(manifest, &ambiguous, &good).is_err());
}

#[test]
fn unselected_messages_rechecks_raw_request_and_answer_byte_bounds() {
    use crate::messages_artifacts::{
        MAX_ANSWER_BYTES, MAX_REQUEST_BODY_BYTES, MessagesMaterialization,
    };

    let repo = git_fixture();
    let expected = MessagesMaterialization::prepare(
        repo.path(),
        PARENT,
        &pinned_subject(),
        EXACT_SCOPE,
        "test-only contract",
        revision::CLAUDE_MODEL,
        LIMITS,
    )
    .unwrap();
    let manifest = expected.manifest_bytes();
    let good = answer_from(ACCEPTABLE, |_| {});
    let mut reqs = claude_requests(expected.task());
    // Padding changes only the retained bytes, not JSON or artifact meaning.
    // Keep reported digests exact; bytes at the ceiling are allowed.
    let mut body = reqs[0].body.clone().unwrap();
    body.resize(MAX_REQUEST_BODY_BYTES, b' ');
    reqs[0] = retained(body.clone(), 1);
    assert!(expected.compare(manifest, &reqs, &good).is_ok());
    body.push(b' ');
    reqs[0] = retained(body, 1);
    assert_eq!(
        expected.compare(manifest, &reqs, &good).unwrap_err(),
        Refusal::Artifact("request 1: body exceeds analysis byte bound".into()),
    );

    let reqs = claude_requests(expected.task());
    let mut answer = good.answer;
    answer.push_str(&" ".repeat(MAX_ANSWER_BYTES - answer.len()));
    assert!(
        expected
            .compare(manifest, &reqs, &result(answer.clone()))
            .is_ok()
    );
    answer.push(' ');
    assert_eq!(
        expected
            .compare(manifest, &reqs, &result(answer))
            .unwrap_err(),
        Refusal::Artifact("answer exceeds analysis byte bound".into()),
    );
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
    let subject = pinned_subject();
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
    check::check(&w.expected, &w.record, &w.run, answer, reqs)
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
    assert_eq!(v["proposition"], "offline_synthetic_exact_scope_only");
    assert_eq!(v["world"]["label"], "hosted-offline-exact-v1-responses");
    assert_eq!(
        v["world"]["run_id"],
        "hosted-offline-exact-v1-responses-fixture"
    );
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
    assert_eq!(e.standing, "owner_pending");
    assert_eq!(e.org_admission, "none_requested");
    assert!(!e.consumable);
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
fn served_snapshot_refuses_extra_nested_file_in_reinventoried_snapshot() {
    let repo = git_fixture();
    let out = repo.path().join("output");
    let mut w = write_world(Profile::OpenaiCodex, repo.path(), &out);
    let answer = answer_from(ACCEPTABLE, |_| {});
    let reqs = requests(&w.task);
    assert_eq!(run_check(&w, &answer, &reqs).outcome, "checks_passed");

    let snapshot = out.join("snapshot");
    let extra = "notes/nested/a.txt";
    let contents = b"ordinary extra fixture file\n";
    let path = snapshot.join(extra);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
    // Keep the vetted writer record (and task/body/answer) fixed. Re-pin the actual
    // new manifest so it is the map comparison, not the raw pin, that must fail.
    pin_served_snapshot(&mut w.run, &snapshot);
    let e = run_check(&w, &answer, &reqs);
    failed_only(&e, "snapshot_served");
    assert_eq!(outcome(&e, "subject_exact_scope"), Outcome::Passed);
    assert_eq!(outcome(&e, "served_patch_identity"), Outcome::Passed);
    assert!(!w.record.snapshot_manifest.contains_key(extra));
    let served = w.run.served_manifest().unwrap();
    assert_eq!(served.get(extra), Some(&sha256_hex(contents)));
    assert_eq!(served.len(), w.record.snapshot_manifest.len() + 1);
}

#[test]
fn served_snapshot_refuses_stale_raw_job_pin_with_unchanged_manifest_entries() {
    let mut w = world();
    let answer = answer_from(ACCEPTABLE, |_| {});
    let reqs = requests(&w.task);
    // Change just the raw bytes under the original job pin, not the inventory.
    w.run.manifest_bytes.push(b'\n');
    assert_eq!(
        w.run.served_manifest().as_ref(),
        Some(&w.record.snapshot_manifest)
    );
    let e = run_check(&w, &answer, &reqs);
    failed_only(&e, "snapshot_served");
    assert_eq!(outcome(&e, "subject_exact_scope"), Outcome::Passed);
    assert_eq!(outcome(&e, "served_patch_identity"), Outcome::Passed);
}

#[cfg(unix)]
#[test]
fn snapshot_inventory_refuses_symlink_roots_and_entries() {
    let dir = tempfile::tempdir().unwrap();
    let snapshot = dir.path().join("snapshot");
    fs::create_dir(&snapshot).unwrap();
    let outside = dir.path().join("outside");
    fs::write(&outside, b"outside the snapshot\n").unwrap();
    let root_link = dir.path().join("root-link");
    std::os::unix::fs::symlink(&snapshot, &root_link).unwrap();
    assert_eq!(
        snapshot_inventory(&root_link).unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
    let entry = snapshot.join("link");
    let outside_dir = dir.path().join("outside-dir");
    fs::create_dir(&outside_dir).unwrap();
    for target in [&outside, &outside_dir] {
        std::os::unix::fs::symlink(target, &entry).unwrap();
        assert_eq!(
            snapshot_inventory(&snapshot).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        fs::remove_file(&entry).unwrap();
    }
}

#[test]
fn served_patch_identity_is_independent_of_manifest_pin_and_exact_scope() {
    for replacement in [None, Some("0".repeat(64)), Some(EXACT_SCOPE.to_owned())] {
        let mut w = world();
        let answer = answer_from(ACCEPTABLE, |_| {});
        match replacement {
            Some(sha) => {
                w.record
                    .snapshot_manifest
                    .insert(REVIEW_PATCH.to_owned(), sha);
            }
            None => {
                w.record.snapshot_manifest.remove(REVIEW_PATCH);
            }
        }
        w.run = run_for(&w.record);
        let e = run_check(&w, &answer, &requests(&w.task));
        failed_only(&e, "served_patch_identity");
        assert_eq!(outcome(&e, "subject_exact_scope"), Outcome::Passed);
        assert_eq!(outcome(&e, "snapshot_served"), Outcome::Passed);
    }
}

#[test]
fn job_id_and_limits_must_be_the_runs() {
    let w = world();
    let answer = answer_from(ACCEPTABLE, |_| {});
    let mut run = run_for(&w.record);
    run.job["job_id"] = json!("dev-vertical-hosted-analysis-other");
    failed_only(
        &check::check(&w.expected, &w.record, &run, &answer, &requests(&w.task)),
        "job_id_bound",
    );
    let mut run = run_for(&w.record);
    run.authority["job_id"] = json!("other-authority-job");
    failed_only(
        &check::check(&w.expected, &w.record, &run, &answer, &requests(&w.task)),
        "job_id_bound",
    );
    let mut run = run_for(&w.record);
    run.packet["max_snapshot_reads"] = json!(17);
    failed_only(
        &check::check(&w.expected, &w.record, &run, &answer, &requests(&w.task)),
        "limits_bound",
    );
    let mut run = run_for(&w.record);
    run.authority["task_sha256"] = json!("0".repeat(64));
    failed_only(
        &check::check(&w.expected, &w.record, &run, &answer, &requests(&w.task)),
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
        let e = check::check(&w.expected, &w.record, &w.run, &answer, &reqs);
        assert_eq!(outcome(&e, "job_profile_bound"), Outcome::Passed);
        for field in ["provider", "model"] {
            let mut run = w.run.clone();
            run.job["coding"]["provider"][field] = json!("other");
            failed_only(
                &check::check(&w.expected, &w.record, &run, &answer, &reqs),
                "job_profile_bound",
            );
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
fn synthetic_run_fields_are_carried_and_unknowns_are_null() {
    let w = world();
    let mut answer = answer_from(ACCEPTABLE, |_| {});
    answer.settlement = Some(json!("CompletedSettled"));
    answer.hosted_attempts = Some(json!([{"route": "secondary", "payer": "openai-api"}]));
    let e = run_check(&w, &answer, &requests(&w.task));
    assert_eq!(e.synthetic_run.settlement, Some(json!("CompletedSettled")));
    assert_eq!(
        e.synthetic_run.hosted_attempts.as_ref().unwrap()[0]["route"],
        "secondary"
    );
    // A subject the preparation refuses: recomputation fails, unknowns stay null.
    let mut bad = w.record.clone();
    bad.subject.commit = String::new();
    let e = check::check(&w.expected, &bad, &w.run, &answer, &requests(&w.task));
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
        check::check(&w.expected, &w.record, &run, &loaded, &reqs).outcome,
        "checks_passed"
    );
}

// Anthropic Messages shape retained as a regression from the archived r2 proof.
fn claude_world() -> World {
    world_for(Profile::ClaudeSdk)
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
    assert_eq!(v["world"]["label"], "hosted-offline-exact-v1-messages");
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
    let insertions = e.synthetic_run.request_insertions.as_ref().unwrap();
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
        !serde_json::to_string(&v["synthetic_run"])
            .unwrap()
            .contains("environment block")
    );
}

#[test]
fn responses_profile_records_no_request_insertions() {
    let w = world();
    let e = run_check(&w, &answer_from(ACCEPTABLE, |_| {}), &requests(&w.task));
    assert_eq!(e.outcome, "checks_passed");
    assert!(e.synthetic_run.request_insertions.is_none());
}

type ClaudeDeliveryRefusalCase<'a> = (&'a str, fn(&mut Value), &'a str);

/// Request 1 of a claude-sdk run edited by `edit` must fail only the delivery check, with a
/// reason containing `expected` (so a case cannot pass for the wrong reason).
fn assert_claude_delivery_refused(cases: Vec<ClaudeDeliveryRefusalCase<'_>>) {
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
    let cases: Vec<ClaudeDeliveryRefusalCase<'_>> = vec![
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
    let cases: Vec<ClaudeDeliveryRefusalCase<'_>> = vec![
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
    let ins = e.synthetic_run.request_insertions.as_ref().unwrap();
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
        !serde_json::to_string(&serde_json::to_value(&e).unwrap()["synthetic_run"])
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
            "the answer is only the separately supplied result string"
        );
        assert_ne!(e.answer, other);
    }
    // The recorded insertions do not depend on assistant content.
    assert_eq!(
        a.synthetic_run.request_insertions,
        b.synthetic_run.request_insertions
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
        e.synthetic_run.hosted_attempts.as_ref().unwrap()[0]["route"],
        "claude-code",
        "carried as reported"
    );
    let not_claimed = e.not_claimed.join("\n");
    assert!(not_claimed.contains("execution confinement are unestablished"));
    assert!(
        not_claimed
            .contains("serving route, payer and any subscription billing guard are unobserved")
    );
    assert!(
        not_claimed.contains("system blocks and environment message are recorded by digest only")
    );
}

/// Same obligation for both worlds. These pins are independent of the current preparation output.
const BLOCKER_IDS: &[&str] = &[
    "scope-digest-to-subject:rp3-initial-implementation-review",
    "problem-scope-to-episode-identity",
    "separation:cond-builder-projection",
    "separation:cond-campaign-terminalization",
    "consumption-evidence-mapping:cond-builder-projection",
    "observer-identity:cond-builder-projection",
    "realization-admission:cond-builder-projection",
    "consumption-evidence-mapping:cond-campaign-terminalization",
    "observer-identity:cond-campaign-terminalization",
    "realization-admission:cond-campaign-terminalization",
    "component-binding:component:campaign",
    "component-binding:component:episode_host",
    "authority-requirement:auth-accept-builder-projection",
    "authority-requirement:auth-accept-campaign-terminalization",
    "realization:cond-builder-projection:mutation",
    "realization:cond-builder-projection:capabilities",
    "realization:cond-campaign-terminalization:mutation",
    "realization:cond-campaign-terminalization:capabilities",
];

fn requests_for(profile: Profile, task: &str) -> Vec<RetainedRequest> {
    match profile {
        Profile::OpenaiCodex => requests(task),
        Profile::ClaudeSdk => claude_requests(task),
    }
}

#[test]
fn exact_scope_crosses_git_writer_and_independent_checker_on_both_wires() {
    assert_eq!(scope(&pinned_subject()), EXACT_SCOPE);
    assert_ne!(EXACT_SCOPE, PATCH_IDENTITY);
    for profile in Profile::ALL {
        let w = world_for(profile);
        let prep =
            revision::preparation(profile, w.expected.requirements(), &pinned_subject()).unwrap();
        assert_eq!(prep.model_context.obligation.scope_digest, EXACT_SCOPE);
        assert_eq!(w.record.revision, w.expected.id);
        assert_eq!(w.record.run_id, w.expected.run_id);
        assert_eq!(w.record.requirements_sha256, w.expected.requirements_sha256);
        assert_eq!(
            w.record.schema,
            "work-engine.review-hosted-offline-exact-inputs.v1"
        );
        assert_eq!(w.record.standing, "synthetic_inputs_only_not_an_admission");
        assert_eq!(
            w.record
                .blockers
                .iter()
                .map(|b| b.id.as_str())
                .collect::<Vec<_>>(),
            BLOCKER_IDS
        );
        assert!(w.record.blockers.iter().all(|b| b.class == "unestablished"));
        let answer = answer_from(ACCEPTABLE, |_| {});
        let reqs = requests_for(profile, &w.task);
        let e = run_check(&w, &answer, &reqs);
        assert_eq!(e.outcome, "checks_passed", "{:#?}", e.checks);
        for id in [
            "subject_exact_scope",
            "served_patch_identity",
            "snapshot_served",
            "blockers_carried",
            "frozen_world_record",
            "delivery",
            "subject_equal",
        ] {
            assert_eq!(outcome(&e, id), Outcome::Passed);
        }
        let serialized = serde_json::to_value(&e).unwrap();
        assert_eq!(
            serialized["schema"],
            "work-engine.review-hosted-offline-exact-envelope.v1"
        );
        assert_eq!(
            serialized["proposition"],
            "offline_synthetic_exact_scope_only"
        );
        assert_eq!(serialized["standing"], "owner_pending");
        assert_eq!(serialized["org_admission"], "none_requested");
        assert_eq!(serialized["consumable"], false);
        assert_eq!(serialized["answer"], answer.answer);
        assert!(serialized.get("host_reported").is_none());
        assert_eq!(e.world.label, w.expected.id);
        assert_eq!(e.world.run_id.as_deref(), Some(w.expected.run_id));
        assert_eq!(e.world.requirements_sha256, w.expected.requirements_sha256);
        assert_eq!(e.blockers.as_ref(), Some(&w.record.blockers));
        assert_eq!(e.undischarged_blockers, Some(BLOCKER_IDS.len()));
        let frozen = &w.record.frozen_world;
        assert_eq!(
            frozen.requirements_sha256,
            "7085318950aceb6f7071d29dec83e13da610b20d110f435e3399c9de57d3b292"
        );
        match profile {
            Profile::OpenaiCodex => {
                assert_eq!(
                    frozen.known_incompatibilities,
                    [
                        "realization:cond-builder-projection:realization:provider_family",
                        "realization:cond-campaign-terminalization:realization:provider_family",
                    ]
                );
                assert!(frozen.unestablished_confinement.is_empty());
                assert_eq!(e.frozen_world.outcome, "refused");
                assert_eq!(e.frozen_world.known_incompatibilities, Some(2));
                assert_eq!(e.frozen_world.unestablished_confinement, None);
            }
            Profile::ClaudeSdk => {
                assert!(frozen.known_incompatibilities.is_empty());
                assert_eq!(
                    frozen.unestablished_confinement,
                    [
                        "realization:cond-builder-projection:realization:execution_confinement",
                        "realization:cond-campaign-terminalization:realization:execution_confinement",
                    ]
                );
                assert_eq!(e.frozen_world.outcome, "prepared_conditions_unmet");
                assert_eq!(e.frozen_world.known_incompatibilities, Some(0));
                assert_eq!(e.frozen_world.unestablished_confinement, Some(2));
            }
        }
    }
}

/// Regenerate the consistency checks for a negative without calling the writer's rejecting
/// materializer. This is not a positive snapshot: the manifest comes from the known baseline
/// and is changed only where needed to isolate the checker row under test.
fn coherent_negative(expected: ExpectedWorld, subject: &Subject, base: &World) -> World {
    let profile = expected.profile;
    let frozen =
        revision::prepare_world_for(profile, expected.requirements(), subject, JOB).unwrap();
    let task = crate::task::render_for(profile, frozen.contract(), subject, LIMITS).unwrap();
    let mut record = base.record.clone();
    record.subject = subject.clone();
    record.revision = expected.id.to_owned();
    record.requirements_sha256 = sha256_hex(expected.requirements());
    record.contract_sha256 = frozen.contract_sha256().to_owned();
    record.contract_bytes = frozen.contract().len();
    record.task_sha256 = sha256_hex(task.as_bytes());
    record.task_bytes = task.len();
    record
        .snapshot_manifest
        .insert(REVIEW_PATCH.to_owned(), subject.patch_identity.clone());
    let run = run_for(&record);
    World {
        expected,
        record,
        run,
        task,
    }
}

#[test]
fn coherent_patch_as_scope_and_unrelated_scope_fail_only_the_exact_scope_row() {
    let base = world();
    let answer = answer_from(ACCEPTABLE, |_| {});
    for kind in [WrongScope::Patch, WrongScope::Other] {
        let w = coherent_negative(
            ExpectedWorld::wrong_scope(Profile::OpenaiCodex, kind),
            &pinned_subject(),
            &base,
        );
        let e = run_check(&w, &answer, &requests(&w.task));
        failed_only(&e, "subject_exact_scope");
        assert_eq!(outcome(&e, "served_patch_identity"), Outcome::Passed);
        assert_eq!(outcome(&e, "snapshot_served"), Outcome::Passed);
    }
}

#[test]
fn changing_each_tuple_member_fails_exact_scope_despite_regenerated_task_and_answer() {
    let base = world();
    for (field, value) in [
        ("commit", TWIN.to_owned()),
        ("tree", "a".repeat(40)),
        ("patchIdentity", "b".repeat(64)),
    ] {
        let mut s = serde_json::to_value(pinned_subject()).unwrap();
        s[field] = json!(value);
        let subject = serde_json::from_value(s.clone()).unwrap();
        assert_ne!(scope(&subject), EXACT_SCOPE, "{field}");
        let w = coherent_negative(ExpectedWorld::pinned(Profile::OpenaiCodex), &subject, &base);
        let e = run_check(
            &w,
            &answer_from(ACCEPTABLE, |a| a["subject"] = s),
            &requests(&w.task),
        );
        failed_only(&e, "subject_exact_scope");
        assert_eq!(outcome(&e, "served_patch_identity"), Outcome::Passed);
        assert_eq!(outcome(&e, "delivery"), Outcome::Passed);
        assert_eq!(outcome(&e, "subject_equal"), Outcome::Passed);
    }
    // Conversely, keep the whole job on the correct tuple and give a schema-valid answer for
    // the metadata twin with its own correct answer SHA-256.
    let answer = answer_from(ACCEPTABLE, |a| a["subject"]["commit"] = json!(TWIN));
    let e = run_check(&base, &answer, &requests(&base.task));
    failed_only(&e, "subject_equal");
    assert_eq!(outcome(&e, "result_contract"), Outcome::Passed);
    assert_eq!(outcome(&e, "answer_sha256"), Outcome::Passed);
}

#[test]
fn checker_uses_closed_expected_profile_revision_and_run_not_record_labels() {
    let w = world();
    let answer = answer_from(ACCEPTABLE, |_| {});
    let reqs = requests(&w.task);
    let trial = |record: &JobInputs| check::check(&w.expected, record, &w.run, &answer, &reqs);
    let mut altered = w.record.clone();
    altered.run_id = "unrelated-run".to_owned();
    failed_only(&trial(&altered), "requirements_run");
    for id in [
        Profile::OpenaiCodex.revision().id,
        "hosted-offline-exact-v1-messages",
    ] {
        let mut altered = w.record.clone();
        altered.revision = id.to_owned();
        let e = trial(&altered);
        failed_only(&e, "requirements_revision");
        assert_eq!(e.world.label, w.expected.id);
    }
    let mut altered = w.record.clone();
    altered.requirements_sha256 = sha256_hex(Profile::OpenaiCodex.revision().requirements);
    failed_only(&trial(&altered), "requirements_revision");
    let mut altered = w.record.clone();
    altered.profile = Profile::ClaudeSdk;
    failed_only(&trial(&altered), "record_profile_bound");
    let mut altered = w.record.clone();
    altered.standing = "admitted".to_owned();
    failed_only(&trial(&altered), "offline_record_kind");
    let mut altered = w.record.clone();
    altered.schema = "work-engine.review-hosted-job-inputs.v2".to_owned();
    failed_only(&trial(&altered), "offline_record_kind");
    let swapped = claude_world();
    let e = trial(&swapped.record);
    assert_eq!(e.outcome, "checks_failed");
    assert_eq!(e.profile, Profile::OpenaiCodex);
    for id in [
        "requirements_revision",
        "requirements_run",
        "record_profile_bound",
    ] {
        assert_eq!(outcome(&e, id), Outcome::Failed);
    }
}

#[test]
fn checker_refuses_parse_equivalent_requirements_bytes_under_stale_pin() {
    for profile in Profile::ALL {
        let w = world_for(profile);
        let answer = answer_from(ACCEPTABLE, |_| {});
        let reqs = requests_for(profile, &w.task);
        assert_eq!(run_check(&w, &answer, &reqs).outcome, "checks_passed");

        // Substitute checker input bytes only after authoring the valid baseline.
        // Keep the same profile, revision, run and pinned hash, not a new revision.
        let original = w.expected.requirements();
        let mut bytes = original.to_vec();
        bytes.push(b'\n');
        assert_ne!(sha256_hex(&bytes), w.expected.requirements_sha256);
        assert_eq!(
            serde_json::from_slice::<Value>(&bytes).unwrap(),
            serde_json::from_slice::<Value>(original).unwrap(),
        );
        let expected = ExpectedWorld {
            requirements: Cow::Owned(bytes),
            ..w.expected
        };
        let e = check::check(&expected, &w.record, &w.run, &answer, &reqs);
        assert_eq!(outcome(&e, "requirements_revision"), Outcome::Failed);
        assert_eq!(outcome(&e, "contract_recomputed"), Outcome::Failed);
        for id in [
            "requirements_run",
            "blockers_carried",
            "task_recomputed",
            "subject_exact_scope",
            "delivery",
        ] {
            assert_eq!(outcome(&e, id), Outcome::NotRun, "{id}");
        }
        for id in [
            "snapshot_served",
            "served_patch_identity",
            "answer_sha256",
            "strict_json",
            "subject_equal",
        ] {
            assert_eq!(outcome(&e, id), Outcome::Passed, "{id}");
        }
        assert_eq!(e.outcome, "checks_failed");
        assert_eq!(e.standing, "owner_pending");
        assert_eq!(e.org_admission, "none_requested");
        assert!(!e.consumable);
    }
}

#[test]
fn checker_keeps_transport_task_and_blocker_counterfactual_recomputations_distinct() {
    let w = claude_world();
    let answer = answer_from(ACCEPTABLE, |_| {});
    let reqs = claude_requests(&w.task);
    let trial = |record: &JobInputs| check::check(&w.expected, record, &w.run, &answer, &reqs);
    let mut altered = w.record.clone();
    altered.transport.generation += 1;
    failed_only(&trial(&altered), "contract_recomputed");
    let mut altered = w.record.clone();
    altered.task_bytes += 1;
    let e = trial(&altered);
    failed_only(&e, "task_recomputed");
    assert_eq!(outcome(&e, "delivery"), Outcome::NotRun);
    let mut altered = w.record.clone();
    altered.blockers.remove(0);
    let e = trial(&altered);
    failed_only(&e, "blockers_carried");
    assert_eq!(e.blockers.as_ref(), Some(&w.record.blockers));
    let mut altered = w.record.clone();
    altered
        .frozen_world
        .unestablished_confinement
        .pop()
        .unwrap();
    let e = trial(&altered);
    failed_only(&e, "frozen_world_record");
    assert_eq!(e.frozen_world.unestablished_confinement, Some(2));
}
