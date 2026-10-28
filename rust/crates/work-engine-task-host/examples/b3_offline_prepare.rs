//! Offline B3 preparation and fail-closed normal-host preflight.
//!
//! `prepare` canonicalizes a public corpus through the task owner's own
//! `TaskStore::submit`/`observe` and `CanonicalMaterialPort::project`, then
//! previews the Direct compile input. It reads only the public files it is
//! given, makes no provider or native call, and its fresh store is a
//! preparation store: it is not the host's runtime store and carries no
//! lifecycle admission, grant, snapshot custody or capture.
//!
//! `provenance` binds a built host executable to the exact source it was built
//! from: it writes the host-format source manifest that `validate_startup`
//! checks and records lock, toolchain, executable and compiled public-route
//! constants. It reads files and runs nothing.
//!
//! `layout` proposes the host's local runtime locations under one private
//! root: it creates only empty 0700 directories and records the paths. It
//! creates no store, socket, journal, release, grant, custody or credential.
//!
//! `template` writes the normal-host run package with every locally derivable
//! field bound from that provenance (and, when given, a checked layout) and
//! every owner-bound input as an explicit `unresolved` marker. `preflight`
//! refuses while any marker remains, recomputes every bound digest and
//! rechecks the layout; `selfcheck` proves, in memory only, that the
//! template's startup section has exactly the `HostStartup` shape.

#![recursion_limit = "512"]

use bounded_inference::{ActionGrant, PublicHostEndpoint, direct_identity_key};
use lifecycle_core::{SubjectId, TransitionId};
use lifecycle_semantic::{
    DirectOperationBinding, DirectRole, SemanticPolicy, direct_compile_input, project_material,
};
use lifecycle_service::CanonicalMaterialPort;
use retained_task_workflow::{PROFILE, SubmitTask, TaskStore, obligations, sha256};
use serde_json::{Value, json};
use std::{
    fs,
    io::ErrorKind,
    os::unix::fs::{DirBuilderExt, FileTypeExt, MetadataExt, PermissionsExt},
    path::{Component, Path, PathBuf},
    process::ExitCode,
};
use work_engine_task_host::HostStartup;

type Res<T> = Result<T, String>;

/// The finite envelope's exact model. The host refuses any per-send grant
/// naming another model; this only fixes the startup selection.
const MODEL: &str = "gpt-6-astra";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["prepare", public, out] => prepare(Path::new(public), Path::new(out)).map(|_| 0),
        ["provenance", root, paths, exe, rustc, out] => provenance(
            Path::new(root), Path::new(paths), Path::new(exe), Path::new(rustc), Path::new(out),
        )
        .map(|_| 0),
        ["layout", root, out] => layout(Path::new(root), Path::new(out)).map(|_| 0),
        ["template", report, prov, out] => {
            template(Path::new(report), Path::new(prov), Path::new(out), None).map(|_| 0)
        }
        ["template", report, prov, out, layout] => {
            template(Path::new(report), Path::new(prov), Path::new(out), Some(Path::new(layout)))
                .map(|_| 0)
        }
        ["selfcheck", package] => selfcheck(Path::new(package)).map(|_| 0),
        ["preflight", package, out] => preflight(Path::new(package), Path::new(out)),
        _ => Err("usage: prepare <public-dir> <out-dir> | provenance <source-root> \
                  <subject-manifest> <host-executable> <rustc-vV> <out-dir> | layout \
                  <private-root> <out> | template <prepare-report> <provenance> <out> \
                  [<layout>] | selfcheck <package> | preflight <package> <out>"
            .into()),
    };
    match result {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("b3_offline_prepare: {error}");
            ExitCode::from(1)
        }
    }
}

fn read(path: &Path) -> Res<Vec<u8>> {
    fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))
}

fn write_json(path: &Path, value: &Value) -> Res<String> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    fs::write(path, &bytes).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(sha256(&bytes))
}

fn private_dir(path: &Path) -> Res<()> {
    fs::create_dir_all(path).map_err(|e| e.to_string())?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|e| e.to_string())
}

/// Only these public files are opened. The private rubric and construction
/// records are never read here, so they cannot reach a request.
const PUBLIC_INPUTS: [&str; 3] = ["submit-task.json", "semantic-policy.json", "identities.json"];

fn prepare(public: &Path, out: &Path) -> Res<()> {
    if public.file_name().and_then(|n| n.to_str()) != Some("public") || !out.is_absolute() {
        return Err("prepare needs the corpus public/ directory and an absolute out dir".into());
    }
    let mut inputs = serde_json::Map::new();
    for name in PUBLIC_INPUTS {
        let bytes = read(&public.join(name))?;
        inputs.insert(name.into(), json!({"path": public.join(name), "sha256": sha256(&bytes)}));
    }
    let task: SubmitTask = serde_json::from_slice(&read(&public.join("submit-task.json"))?)
        .map_err(|e| format!("submit-task.json is not an exact SubmitTask: {e}"))?;
    let declared: SemanticPolicy =
        serde_json::from_slice(&read(&public.join("semantic-policy.json"))?)
            .map_err(|e| format!("semantic-policy.json: {e}"))?;
    let ids: Value = serde_json::from_slice(&read(&public.join("identities.json"))?)
        .map_err(|e| format!("identities.json: {e}"))?;
    let label = |k: &str| -> Res<String> {
        ids[k].as_str().map(str::to_owned).ok_or(format!("identities.json lacks {k}"))
    };

    private_dir(out)?;
    let store_dir = out.join("taskstore");
    if store_dir.exists() {
        return Err(format!("{} exists; preparation needs a fresh store", store_dir.display()));
    }
    private_dir(&store_dir)?;
    let store_path = store_dir.join("owner.sqlite");
    // Gates answer() only; this store answers no question.
    let store = TaskStore::open(&store_path, "b3-offline-preparation-no-answers".into())
        .map_err(|e| format!("TaskStore::open: {e:?}"))?;
    let (commit, first) = store.submit_with_status(&task).map_err(|e| format!("submit: {e:?}"))?;
    let (again, again_first) =
        store.submit_with_status(&task).map_err(|e| format!("resubmit: {e:?}"))?;
    let mut altered = task.clone();
    altered.objective.push_str(" (altered)");
    let conflict = store.submit(&altered).err().map(|e| format!("{e:?}"));
    let record = store
        .observe(&task.task_id)
        .map_err(|e| format!("observe: {e:?}"))?
        .ok_or("observe: submitted task is absent")?;
    let owned_obligations = obligations(&record);

    let subject = SubjectId::parse(task.subject_id.clone()).map_err(|e| format!("{e:?}"))?;
    let transition = TransitionId::parse(label("transition")?).map_err(|e| format!("{e:?}"))?;
    let capture = label("capture")?;
    let material = CanonicalMaterialPort::project(&store, &subject, &transition, &capture)
        .map_err(|e| format!("project: {e:?}"))?;
    let policy_matches = material.policy == declared
        && declared == SemanticPolicy::retained_repository_analysis()
        && declared.id == PROFILE;
    let basis = project_material(material.clone()).map_err(|e| format!("project_material: {e:?}"))?;
    basis.verify_digest().map_err(|e| format!("verify_digest: {e:?}"))?;
    let dependencies = store
        .observe_dependencies(&basis.dependencies)
        .map_err(|e| format!("observe_dependencies: {e:?}"))?;

    // The compile prompt does not read the binding's operation/attempt/exposure
    // ids, but it embeds the basis digest, which binds subject, transition and
    // capture. With proposal labels this is a preview, not a request.
    let binding = DirectOperationBinding {
        subject: subject.clone(),
        transition: transition.clone(),
        capture_id: capture.clone(),
        role: DirectRole::Compile,
        policy_id: basis.policy_id.clone(),
        operation_id: label("compile_operation")?,
        attempt_id: label("compile_attempt")?,
        exposure_id: "unbound-offline-preview".into(),
    };
    let (input, input_sha) = direct_compile_input(&basis, &binding, &declared)
        .map_err(|e| format!("direct_compile_input: {e:?}"))?;
    let input_path = out.join("compile-input.preview.json");
    fs::write(&input_path, input.as_bytes()).map_err(|e| e.to_string())?;

    // Negative: an authored fact cannot use the reserved runtime effect prefix.
    let neg_dir = out.join("negative-store");
    private_dir(&neg_dir)?;
    let neg_store = TaskStore::open(&neg_dir.join("owner.sqlite"), "b3-offline-negative".into())
        .map_err(|e| format!("negative open: {e:?}"))?;
    let mut reserved = task.clone();
    reserved.facts[2].id = "effect-local-inventory".into();
    let reserved_rejected = neg_store.submit(&reserved).err().map(|e| format!("{e:?}"));

    let sources: Vec<Value> = record
        .sources
        .iter()
        .map(|s| {
            json!({"id": s.id, "revision": s.revision, "current": s.current,
                   "dependency_required": s.dependency_required, "kind": s.kind,
                   "trust": s.trust, "availability": s.availability,
                   "bytes": s.bytes.len(), "sha256": s.sha256, "owner_ref": s.owner_ref,
                   "dependency_key": s.dependency_key})
        })
        .collect();
    let record_sha = write_json(&out.join("task-record.json"), &json!(record))?;
    let material_sha = write_json(&out.join("material-input.json"), &json!(material))?;
    let basis_sha = write_json(&out.join("source-basis.json"), &json!(basis))?;
    let report = json!({
        "schema": "b3_offline_canonicalization_v1",
        "inputs": inputs,
        "private_inputs_opened": [],
        "store": {"path": store_path, "role": "preparation store; not the host runtime store",
                  "lifecycle_admission": "none (submit without admission builder)"},
        "submit": {"commit": commit, "first_publication": first,
                   "resubmit_commit_equal": again == commit, "resubmit_first": again_first,
                   "altered_payload_same_command": conflict},
        "record": {"task_id": record.task_id, "subject_id": record.subject_id,
                   "revision": record.revision, "cursor_revision": record.cursor_revision,
                   "snapshot_ref": record.snapshot_ref, "sources": sources,
                   "completed_effects": record.completed_effects.len(),
                   "questions": record.questions.len(), "result_present": record.result.is_some(),
                   "file": {"path": out.join("task-record.json"), "sha256": record_sha}},
        "obligations": owned_obligations.iter().map(|o| json!({"id": o.id, "kind": o.kind,
                   "sources": o.sources})).collect::<Vec<_>>(),
        "material": {"subject": subject, "transition_label": transition, "capture_label": capture,
                     "policy_matches_declared_and_owner": policy_matches,
                     "obligations_equal_owner": material.obligations == owned_obligations,
                     "file": {"path": out.join("material-input.json"), "sha256": material_sha}},
        "basis": {"sha256": basis.sha256, "eligible": basis.is_eligible(), "issues": basis.issues,
                  "dependencies": basis.dependencies, "observed_dependencies":
                  format!("{dependencies:?}"),
                  "file": {"path": out.join("source-basis.json"), "sha256": basis_sha}},
        "compile_input_preview": {"path": input_path, "sha256": input_sha,
                  "bytes": input.len(), "binding_labels": "proposal labels; exposure unbound",
                  "binding_dependent_fields": ["source_basis_sha256 (subject, transition, capture_id)"]},
        "negatives": {"reserved_effect_fact_id": reserved_rejected},
        "not_established": ["lifecycle admission", "snapshot custody", "native domain_work",
                  "capture/transition identities", "grants", "any provider exchange"]
    });
    write_json(&out.join("prepare-report.json"), &report)?;
    println!("{}", serde_json::to_string(&json!({"basis": basis.sha256,
        "compile_input": input_sha, "sources": record.sources.len(),
        "obligations": owned_obligations.len(), "reserved_rejected": reserved_rejected.is_some()}))
        .unwrap_or_default());
    Ok(())
}

fn relative_plain(path: &str) -> bool {
    let p = Path::new(path);
    !path.is_empty()
        && !p.is_absolute()
        && p.components().all(|c| matches!(c, Component::Normal(_)))
}

/// Recheck a host-format manifest the way `validate_startup` does: sorted,
/// unique, relative, every listed file present under `root` with that digest.
fn verify_manifest(root: &Path, manifest: &Path) -> Res<usize> {
    let entries: Vec<Value> =
        serde_json::from_slice(&read(manifest)?).map_err(|e| format!("manifest: {e}"))?;
    let root = fs::canonicalize(root).map_err(|e| format!("source root: {e}"))?;
    let mut prior: Option<String> = None;
    for entry in &entries {
        let (Some(path), Some(digest)) = (entry["path"].as_str(), entry["sha256"].as_str()) else {
            return Err("manifest entry lacks path or sha256".into());
        };
        if entry.as_object().map(|m| m.len()) != Some(2)
            || !relative_plain(path)
            || prior.as_deref().is_some_and(|p| p >= path)
        {
            return Err(format!("manifest path/order/fields: {path}"));
        }
        let file = fs::canonicalize(root.join(path)).map_err(|e| format!("{path}: {e}"))?;
        if !file.starts_with(&root) || sha256(&read(&file)?) != digest {
            return Err(format!("source digest mismatch: {path}"));
        }
        prior = Some(path.to_owned());
    }
    if entries.is_empty() {
        return Err("empty source manifest".into());
    }
    Ok(entries.len())
}

fn provenance(root: &Path, paths: &Path, exe: &Path, rustc: &Path, out: &Path) -> Res<()> {
    if !root.is_absolute() || !exe.is_absolute() || !out.is_absolute() {
        return Err("provenance needs absolute source root, executable and out dir".into());
    }
    // The path set is the frozen subject's; the bytes are the tree's now, so a
    // dirty overlay is bound exactly and drift from the subject is reported.
    let subject_bytes = read(paths)?;
    let subject: Value = serde_json::from_slice(&subject_bytes).map_err(|e| e.to_string())?;
    let listed = subject["entries"].as_array().ok_or("subject manifest lacks entries")?;
    let mut entries = Vec::new();
    let mut changed = Vec::new();
    for item in listed {
        let path = item["path"].as_str().ok_or("subject entry lacks path")?;
        if !relative_plain(path) {
            return Err(format!("subject path is not plain relative: {path}"));
        }
        let digest = sha256(&read(&root.join(path))?);
        if item["sha256"].as_str() != Some(digest.as_str()) {
            changed.push(json!({"path": path, "subject_sha256": item["sha256"], "sha256": digest}));
        }
        entries.push(json!({"path": path, "sha256": digest}));
    }
    entries.sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));
    entries.dedup_by(|a, b| a["path"] == b["path"]);
    if entries.len() != listed.len() {
        return Err("subject manifest repeats a path".into());
    }
    private_dir(out)?;
    let manifest_path = out.join("host-source-manifest.json");
    let manifest_sha = write_json(&manifest_path, &json!(entries))?;
    let verified = verify_manifest(root, &manifest_path)?;
    let file = |p: &Path| -> Res<Value> {
        let bytes = read(p)?;
        Ok(json!({"path": p, "sha256": sha256(&bytes), "bytes": bytes.len()}))
    };
    let exe_meta = file(exe)?;
    let exe_sha = exe_meta["sha256"].as_str().unwrap_or_default().to_owned();
    let rustc_text = String::from_utf8(read(rustc)?).map_err(|e| e.to_string())?;
    let report = json!({
        "schema": "b3_host_build_provenance_v1",
        "source_root": root,
        "source_manifest": {"path": manifest_path, "sha256": manifest_sha, "entries": verified,
            "format": "host SourceManifestEntry [{path, sha256}], sorted, relative to source_root",
            "path_set": {"path": paths, "sha256": sha256(&subject_bytes),
                         "schema": subject["schema"], "version": subject["version"]},
            "changed_vs_path_set_manifest": changed},
        "lock": file(&root.join("rust/Cargo.lock"))?,
        "toolchain": file(&root.join("rust/rust-toolchain.toml"))?,
        "rustc_vV": {"path": rustc, "sha256": sha256(rustc_text.as_bytes()), "text": rustc_text},
        "executable": exe_meta,
        "features": [],
        "build_id": format!("b3-task-host-{}", &exe_sha[..16]),
        "library_source_sha256": ActionGrant::source_sha256(),
        "public_route": {"provider": PublicHostEndpoint::PROVIDER,
            "profile_revision": PublicHostEndpoint::PROFILE_REVISION,
            "bounds": PublicHostEndpoint::bounds()},
        "meaning": "executable digest is the file the build produced; startup recomputes \
            /proc/self/exe, lock, toolchain and manifest digests and every manifest entry. \
            library_source_sha256 is ActionGrant::source_sha256() compiled from this same tree; \
            each action grant must name it."
    });
    write_json(&out.join("provenance.json"), &report)?;
    println!("{}", json!({"source_manifest": manifest_sha, "entries": verified,
        "changed_vs_path_set": changed.len(), "executable": exe_sha,
        "library_source_sha256": report["library_source_sha256"]}));
    Ok(())
}

fn u(owner: &str, gap: &str, ty: &str, note: &str) -> Value {
    json!({"unresolved": {"owner": owner, "gap": gap, "type": ty, "note": note}})
}

fn template(report: &Path, prov: &Path, out: &Path, layout: Option<&Path>) -> Res<()> {
    let bytes = read(report)?;
    let parsed: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let prov_bytes = read(prov)?;
    let pv: Value = serde_json::from_slice(&prov_bytes).map_err(|e| e.to_string())?;
    let subject = parsed["record"]["subject_id"].clone();
    // Public labels only, from the exact file the preparation opened.
    let ids_input = &parsed["inputs"]["identities.json"];
    let ids_bytes = read(Path::new(ids_input["path"].as_str().ok_or("report lacks identities")?))?;
    if ids_input["sha256"].as_str() != Some(sha256(&ids_bytes).as_str()) {
        return Err("identities.json changed since preparation".into());
    }
    let ids: Value = serde_json::from_slice(&ids_bytes).map_err(|e| e.to_string())?;
    let host = "task-host runtime operator (R01)";
    let native = "lifecycle/native domain and snapshot custody owner (R02)";
    let root = "B3 root release owner";
    let custodian = "credential/account custodian (R04)";
    let p = |n: &str| u(host, "R01", "path", n);
    let bounds = PublicHostEndpoint::bounds();
    let package = json!({
        "schema": "b3_normal_host_run_package_v2",
        "status": "NOT_READY: every unresolved marker is a genuine missing owner binding; \
                   every other value is derived from the provenance and source named here",
        "host_startup": {
            "profile": PROFILE,
            "task_store_path": p("host runtime TaskStore; parent 0700; not the preparation store"),
            "socket_path": p("host admission socket; parent 0700"),
            "approved_snapshot_ref": u(native, "R02", "string", "approved snapshot; corpus label is a proposal only"),
            "answer_authority_ref": u(root, "R05", "string", "human answer authority for cedar-publish-17"),
            "enqueue_grant_ref": u(root, "AR01", "string", "must appear in lifecycle.grants"),
            "replacement_grant_ref": u(root, "AR01", "string", "must appear in lifecycle.grants"),
            "executable_sha256": pv["executable"]["sha256"],
            "source_root": pv["source_root"],
            "source_manifest_path": pv["source_manifest"]["path"],
            "source_sha256": pv["source_manifest"]["sha256"],
            "lock_path": pv["lock"]["path"], "lock_sha256": pv["lock"]["sha256"],
            "toolchain_path": pv["toolchain"]["path"], "toolchain_sha256": pv["toolchain"]["sha256"],
            "features": pv["features"],
            "provider_identity": u(native, "R02", "string", "must equal native.provider_route identity"),
            "control": {
                "kind": "direct",
                "credential_dir": u(custodian, "R04", "path", "existing Ready private store; not inspected here"),
                "credential_host_id": u(custodian, "R04", "string", "issuer host id"),
                "journal_dir": p("private 0700 journal dir (or 0700 parent), disjoint from credential_dir and release_dir; same dir on every restart"),
                "release_dir": u(root, "R03", "path", "existing owner-only (mode &077 == 0, host uid) release directory, disjoint from credential_dir and journal_dir"),
                "model": MODEL,
                "bounds": bounds
            },
            "lifecycle": {
                "profile": PROFILE,
                "store_root": p("lifecycle store root"),
                "socket_path": p("lifecycle socket; distinct from host socket"),
                "peer_socket_path": p("lifecycle peer socket"),
                "verifier_executable": u(native, "R02", "optional_path", "isolated verifier executable or null by owner"),
                "verifier_ledger_path": u(native, "R02", "optional_path", "verifier ledger or null by owner"),
                "proof_barrier_dir": null,
                "trusted_issuer": u(root, "AR01", "string", "grant issuer"),
                "principal": u(root, "AR01", "principal", "uid must equal host uid"),
                "subject_id": subject,
                "context_generation": u(root, "AR01", "string", "grant context generation"),
                "build_id": pv["build_id"],
                "proof_run_id": u(root, "AR03", "string", "run identity for recovery/status"),
                "grants": u(root, "AR01", "grants", "full grant tuple incl. enqueue/replacement/turn/snapshot/control"),
                "max_executions": u(root, "AR03", "usize", "bound"),
                "max_wait_ms": u(root, "AR03", "u64", "bound"),
                "native": {
                    "executable": u(native, "R02", "path", "native runtime executable"),
                    "root": u(native, "R02", "path", "native root"),
                    "provider_route": u(native, "R02", "enum:native-sim-proof-v1", "owner choice among routes this default build accepts (native-sim-proof-v1, openai-responses-live-v1, openai-chatgpt-access-lease-v1); no product native inference authorized here"),
                    "simulator_base_url": u(native, "R02", "string", "route base URL"),
                    "session_id": u(native, "R02", "string", "domain session"),
                    "invocation_id": u(native, "R02", "string", "domain_work invocation"),
                    "attempt_id": u(native, "R02", "string", "domain_work attempt"),
                    "prompt": u(native, "R02", "string", "domain turn; transition requires completed_settled domain_work"),
                    "turn_grant_ref": u(root, "AR01", "string", "turn grant"),
                    "turn_grant_revision": u(root, "AR01", "u64", "turn grant revision"),
                    "snapshot_grant_ref": u(root, "AR01", "string", "snapshot grant"),
                    "snapshot_grant_revision": u(root, "AR01", "u64", "snapshot grant revision"),
                    "same_native_session_required": u(native, "R02", "bool", "native session continuity choice"),
                    "followup_turns": u(native, "R02", "string_array", "native follow-up turns, if any"),
                    "snapshot": u(native, "R02", "snapshot", "TrustedSnapshot {id, source_revision, members}"),
                    "semantic": {
                        "capture_id": u(native, "R02", "string", "capture; corpus label is proposal"),
                        "compile_session_id": u(native, "R02", "string", ""),
                        "verify_session_id": u(native, "R02", "string", ""),
                        "successor_verify_session_id": u(native, "R02", "string", "unused: stop at checkpointed"),
                        "compile_invocation_id": u(root, "R03", "string", "compile operation"),
                        "compile_attempt_id": u(root, "R03", "string", "compile attempt"),
                        "candidate_verify_invocation_id": u(root, "R03", "string", "verify operation"),
                        "candidate_verify_attempt_id": u(root, "R03", "string", "verify attempt"),
                        "successor_verify_invocation_id": u(native, "R02", "string", "unused: stop at checkpointed"),
                        "successor_verify_attempt_id": u(native, "R02", "string", "unused: stop at checkpointed"),
                        "challenge_id": u(native, "R02", "string", ""),
                        "challenge_text": u(native, "R02", "string", ""),
                        "control_grant_ref": u(root, "AR01", "string", "semantic control grant"),
                        "control_grant_revision": u(root, "AR01", "u64", "")
                    }
                }
            }
        },
        "run_bindings": {
            "executable_path": pv["executable"]["path"],
            "build_provenance": {"path": prov, "sha256": sha256(&prov_bytes)},
            "library_source_sha256": pv["library_source_sha256"],
            "canonical_preparation": {"path": report, "sha256": sha256(&bytes),
                "meaning": "offline canonical sources/obligations/revisions; runtime store must reproduce them through host submit"},
            "runtime_transition_and_capture": u(native, "R02", "string", "assigned at admission; basis digest and exact compile bytes depend on them"),
            "direct_release_protocol": release_protocol(&pv, &ids),
            "grant_tuple_stage": u(root, "AR01", "string", "exact tuple and stage at each send"),
            "material_fence": u(native, "AR02", "string", "currentness fence at commit (with_current_fence)"),
            "uncertainty_recovery_status": u(root, "AR03", "string", "no-resend recovery for possible sends"),
            "normal_host_reachability": u(native, "AR04", "string", "Direct reached only through normal host serve"),
            "stopped_checkpoint_consumer": u(root, "R02", "string", "consumer stopping at checkpointed; no native-only successor"),
            "root_case_review_and_release": u(root, "R05", "string", "case review then exact release"),
            // Null until a checked layout is bound; the R01 path markers say what is missing.
            "runtime_layout": null
        },
        "proposed_envelope": {
            "status": "root-reviewable proposal; no release exists",
            "direct_posts_max": 2,
            "sequence": "compile once; only a validated complete candidate permits one separate verify",
            "retry": false, "feedback": false, "fallback": false,
            "model": MODEL, "tools": "none (tool-free wire)", "store": false, "stream": true,
            "omitted_wire_fields": ["reasoning (effort)", "max_output_tokens"],
            "local_bounds": bounds,
            "local_bounds_meaning": "local request/stream/frame/output/event/time limits; they do not cap remote tokens or spend",
            "remote_token_cap": null, "remote_spend_cap": null
        }
    });
    let mut package = package;
    if let Some(layout) = layout {
        bind_layout(&mut package, layout)?;
    }
    write_json(out, &package)?;
    Ok(())
}

/// Where a runtime location must be when the host starts.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Slot {
    /// Absent, or an owner-only regular file from an earlier run.
    File,
    /// Absent, or our socket from an earlier run; at most `SOCKET_PATH_MAX` bytes.
    Socket,
    /// Absent under a private parent, or an existing 0700 directory of ours.
    Dir,
    /// An existing owner-only directory, as the host requires at startup.
    Release,
}

/// The locations a local layout binds: the six R01 runtime paths and the
/// release directory the root later writes per-send releases into. The last
/// field is the default place under the layout root.
const LAYOUT: [(&str, &str, Slot, &str); 7] = [
    ("task_store_path", "/host_startup/task_store_path", Slot::File, "host/task-store.sqlite3"),
    ("socket_path", "/host_startup/socket_path", Slot::Socket, "host/host.sock"),
    ("journal_dir", "/host_startup/control/journal_dir", Slot::Dir, "direct/journal"),
    ("release_dir", "/host_startup/control/release_dir", Slot::Release, "release"),
    ("store_root", "/host_startup/lifecycle/store_root", Slot::Dir, "lifecycle/store"),
    ("lifecycle_socket_path", "/host_startup/lifecycle/socket_path", Slot::Socket,
        "lifecycle/lifecycle.sock"),
    ("peer_socket_path", "/host_startup/lifecycle/peer_socket_path", Slot::Socket,
        "lifecycle/peer.sock"),
];

/// The only directories `layout` creates, each empty and 0700.
const LAYOUT_DIRS: [&str; 4] = ["host", "direct", "lifecycle", "release"];

/// `sun_path` holds 108 bytes including the terminating NUL.
const SOCKET_PATH_MAX: usize = 107;

const LAYOUT_SCHEMA: &str = "b3_runtime_layout_v1";

type Bound = Vec<(&'static str, PathBuf)>;

fn layout(root: &Path, out: &Path) -> Res<()> {
    if !plain(root) {
        return Err(format!("layout root {} is not a plain absolute path", root.display()));
    }
    let paths: Bound = LAYOUT.iter().map(|(key, _, _, rel)| (*key, root.join(rel))).collect();
    // A root too long for its sockets is refused before anything is created.
    if let Some((key, _)) = paths.iter().find(|(key, p)| {
        LAYOUT.iter().any(|l| l.0 == *key && l.2 == Slot::Socket)
            && p.as_os_str().len() > SOCKET_PATH_MAX
    }) {
        return Err(format!("layout root is too long: {key} would exceed {SOCKET_PATH_MAX} bytes"));
    }
    let uid = current_uid()?;
    let mut created = Vec::new();
    // The root is checked before anything is created inside it, and an
    // existing directory is never re-moded: a wrong mode is refused, not fixed.
    let mut make = |dir: PathBuf| match fs::DirBuilder::new().mode(0o700).create(&dir) {
        Ok(()) => {
            created.push(dir);
            Ok(())
        }
        Err(e) if e.kind() == ErrorKind::AlreadyExists => Ok(()),
        Err(e) => Err(format!("create {}: {e}", dir.display())),
    };
    make(root.to_path_buf())?;
    if !private_dir_at(root, uid, true) || !canonical(root) {
        return Err(format!("layout root {} is not a canonical 0700 directory of uid {uid}",
            root.display()));
    }
    for dir in LAYOUT_DIRS {
        make(root.join(dir))?;
    }
    let refusals = check_layout(root, &paths, &[])?;
    if !refusals.is_empty() {
        return Err(format!("runtime layout refused: {}", refusals.join("; ")));
    }
    let record = json!({
        "schema": LAYOUT_SCHEMA,
        "root": root,
        "uid": uid,
        "paths": paths.iter().map(|(k, p)| (k.to_string(), json!(p))).collect::<serde_json::Map<_, _>>(),
        "created": created,
        "meaning": "locally chosen proposal, not an owner record: empty 0700 directories only; no \
                    task store, socket, journal, lifecycle store, release, grant, custody or \
                    credential exists or is implied",
        "rules": format!("root and every parent a canonical 0700 directory of uid {uid}; release_dir \
                  existing with mode & 077 == 0; journal_dir and store_root absent or 0700; an \
                  existing task store owner-only; sockets at most {SOCKET_PATH_MAX} bytes; all \
                  locations pairwise disjoint and under the root")
    });
    write_json(out, &record)?;
    println!("{}", json!({"layout": out, "root": root, "created": record["created"]}));
    Ok(())
}

fn current_uid() -> Res<u32> {
    fs::metadata("/proc/self").map(|m| m.uid()).map_err(|e| e.to_string())
}

/// Absolute, already normalized (no `.`, `..`, repeated or trailing `/`).
fn plain(path: &Path) -> bool {
    path.is_absolute()
        && path.as_os_str().len() <= 4096
        && !path.components().any(|c| matches!(c, Component::ParentDir | Component::CurDir))
        && path.components().collect::<PathBuf>().as_os_str() == path.as_os_str()
}

/// No symlink anywhere on the path.
fn canonical(path: &Path) -> bool {
    fs::canonicalize(path).is_ok_and(|c| c == path)
}

/// A directory of `uid` that is not itself a symlink, at exactly 0700 or
/// with no group/other bits.
fn private_dir_at(path: &Path, uid: u32, exact: bool) -> bool {
    fs::symlink_metadata(path).is_ok_and(|m| {
        m.is_dir()
            && m.uid() == uid
            && if exact { m.mode() & 0o777 == 0o700 } else { m.mode() & 0o077 == 0 }
    })
}

/// Refusals for one proposed layout; empty means acceptable. Reads metadata
/// only and creates nothing. No location may equal, contain or lie inside
/// another, or any `avoid` location (source root, preparation store,
/// credential store when bound).
fn check_layout(root: &Path, paths: &[(&str, PathBuf)], avoid: &[(&str, PathBuf)]) -> Res<Vec<String>> {
    let uid = current_uid()?;
    let mut refusals = Vec::new();
    if !plain(root) || !private_dir_at(root, uid, true) || !canonical(root) {
        refusals.push(format!("root {} is not a canonical 0700 directory of uid {uid}",
            root.display()));
    }
    for (key, path) in paths {
        let slot = LAYOUT.iter().find(|l| l.0 == *key).map(|l| l.2);
        let Some(slot) = slot else {
            refusals.push(format!("{key} is not a layout location"));
            continue;
        };
        if !plain(path) {
            refusals.push(format!("{key} {} is not a plain absolute path", path.display()));
            continue;
        }
        if !path.starts_with(root) || path == root {
            refusals.push(format!("{key} is not inside the layout root"));
        }
        let parent = path.parent().unwrap_or(path);
        if !private_dir_at(parent, uid, true) || !canonical(parent) {
            refusals.push(format!("{key} parent {} is not a canonical 0700 directory of uid {uid}",
                parent.display()));
        }
        if slot == Slot::Socket && path.as_os_str().len() > SOCKET_PATH_MAX {
            refusals.push(format!("{key} is {} bytes; a socket path is at most {SOCKET_PATH_MAX}",
                path.as_os_str().len()));
        }
        let existing = match fs::symlink_metadata(path) {
            Ok(meta) => Some(meta),
            Err(e) if e.kind() == ErrorKind::NotFound => None,
            Err(e) => {
                refusals.push(format!("{key}: {e}"));
                continue;
            }
        };
        let ok = match (slot, existing) {
            (Slot::Release, None) => false,
            (_, None) => true,
            (Slot::File, Some(m)) => m.is_file() && m.uid() == uid && m.mode() & 0o077 == 0,
            (Slot::Socket, Some(m)) => m.file_type().is_socket() && m.uid() == uid,
            (Slot::Dir, Some(_)) => private_dir_at(path, uid, true),
            (Slot::Release, Some(_)) => private_dir_at(path, uid, false),
        };
        if !ok {
            refusals.push(format!("{key} {} exists with the wrong type, owner or mode, or a \
                required release directory is absent", path.display()));
        }
    }
    let overlap = |a: &Path, b: &Path| a == b || a.starts_with(b) || b.starts_with(a);
    for (i, (ka, a)) in paths.iter().enumerate() {
        for (kb, b) in &paths[i + 1..] {
            if overlap(a, b) {
                refusals.push(format!("{ka} overlaps {kb}"));
            }
        }
        for (kb, b) in avoid {
            if overlap(a, b) {
                refusals.push(format!("{ka} overlaps {kb}"));
            }
        }
    }
    Ok(refusals)
}

/// The exact layout record: root and the seven locations in `LAYOUT` order.
fn parse_layout(bytes: &[u8]) -> Res<(PathBuf, Bound)> {
    let value: Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let paths = value["paths"].as_object().ok_or("layout lacks paths")?;
    if value["schema"] != LAYOUT_SCHEMA || paths.len() != LAYOUT.len() {
        return Err(format!("layout is not an exact {LAYOUT_SCHEMA} record"));
    }
    let root = value["root"].as_str().ok_or("layout lacks root")?;
    let bound = LAYOUT
        .iter()
        .map(|(key, ..)| {
            paths.get(*key).and_then(Value::as_str).map(|p| (*key, PathBuf::from(p)))
                .ok_or(format!("layout lacks {key}"))
        })
        .collect::<Res<Bound>>()?;
    Ok((PathBuf::from(root), bound))
}

/// Locations the runtime must stay clear of, as far as the package binds them.
fn layout_avoid(package: &Value) -> Bound {
    let startup = &package["host_startup"];
    let prep = package["run_bindings"]["canonical_preparation"]["path"].as_str()
        .and_then(|p| Path::new(p).parent().map(Path::to_path_buf));
    [
        ("source_root", startup["source_root"].as_str().map(PathBuf::from)),
        ("preparation_dir", prep),
        ("credential_dir", startup["control"]["credential_dir"].as_str().map(PathBuf::from)),
    ]
    .into_iter()
    .filter_map(|(k, p)| p.map(|p| (k, p)))
    .collect()
}

/// Replace the seven location markers with a checked layout's paths and bind
/// the exact layout bytes. Any refusal leaves no package written.
fn bind_layout(package: &mut Value, path: &Path) -> Res<()> {
    let bytes = read(path)?;
    let (root, paths) = parse_layout(&bytes)?;
    let refusals = check_layout(&root, &paths, &layout_avoid(package))?;
    if !refusals.is_empty() {
        return Err(format!("runtime layout refused: {}", refusals.join("; ")));
    }
    for ((_, pointer, ..), (_, value)) in LAYOUT.iter().zip(&paths) {
        *package.pointer_mut(pointer).ok_or(format!("template lacks {pointer}"))? = json!(value);
    }
    package["run_bindings"]["runtime_layout"] = json!({
        "path": path, "sha256": sha256(&bytes), "root": root,
        "meaning": "locally chosen runtime locations, checked here and again at preflight; the \
                    release directory's contents, credential_dir and every runtime record stay \
                    with their external owners"
    });
    Ok(())
}

/// Preflight's layout checks: (`runtime_layout`, `runtime_layout_record`,
/// refusals). Both are null while any location is still an owner marker.
fn layout_checks(package: &Value) -> Res<(Value, Value, Vec<String>)> {
    let bound: Option<Bound> = LAYOUT
        .iter()
        .map(|(key, pointer, ..)| {
            package.pointer(pointer).and_then(Value::as_str).map(|p| (*key, PathBuf::from(p)))
        })
        .collect();
    let Some(paths) = bound else {
        return Ok((Value::Null, Value::Null, Vec::new()));
    };
    let record = &package["run_bindings"]["runtime_layout"];
    let Some(root) = record["root"].as_str().map(PathBuf::from) else {
        let missing = "locations bound without a run_bindings.runtime_layout record".to_owned();
        return Ok((json!(false), json!(false), vec![missing]));
    };
    let refusals = check_layout(&root, &paths, &layout_avoid(package))?;
    let same = record["path"].as_str().and_then(|p| fs::read(p).ok()).is_some_and(|bytes| {
        Some(sha256(&bytes).as_str()) == record["sha256"].as_str()
            && parse_layout(&bytes).is_ok_and(|(r, p)| r == root && p == paths)
    });
    Ok((json!(refusals.is_empty()), json!(same), refusals))
}

/// How one exact prepared call becomes an owner release. Every name and
/// constant here is compiled into the host; only the release owner's files
/// and the runtime operation ids are external.
fn release_protocol(pv: &Value, ids: &Value) -> Value {
    let key = |label: &str| direct_identity_key(label);
    json!({
        "status": "mechanics bound; no release present",
        "per_operation_key": "direct_identity_key(operation_id) = \"direct-\" + sha256(operation_id)",
        "host_writes": {
            "request": "<release_dir>/direct-request-<key>.json (schema b3-direct-release-request-v1: exact call, provider, profile_revision, model, bounds, action/grant templates)",
            "check": "<release_dir>/direct-release-check-<key>.json (schema b3-direct-release-check-v1: ready|pending + reason)",
            "when": "before any lifecycle send intent; a missing or inconsistent release stays Pending and consumes nothing"
        },
        "owner_writes": {
            "action": "<release_dir>/direct-action-<key>.json: ActionGrant {action: direct, credential_dir, host_id, work_dir (journal dir of the call), source_sha256, executable_sha256, cleanup_disposition: retain-until-owner-disconnect, smoke_grant_sha256: sha256 of the grant file bytes, issued/valid window <= 45 s, >= 33 s remaining at check}",
            "grant": "<release_dir>/direct-grant-<key>.json: DirectSendGrant {smoke: SmokeGrant (registration, model observation, funding assertion, custody acceptance, operation/attempt = lifecycle binding), role, prepared_binding_sha256, prepared_wire_sha256, source_basis_sha256, candidate_sha256, compile_binding_sha256, exposure_id, profile_revision, host_source_sha256}",
            "file_rules": "owner-uid regular files, not group/other writable, <= 64 KiB, opened O_NOFOLLOW"
        },
        "bound_values": {
            "provider": PublicHostEndpoint::PROVIDER,
            "profile_revision": PublicHostEndpoint::PROFILE_REVISION,
            "model": MODEL,
            "bounds": PublicHostEndpoint::bounds(),
            "action.source_sha256": pv["library_source_sha256"],
            "action.executable_sha256": pv["executable"]["sha256"],
            "grant.host_source_sha256": pv["source_manifest"]["sha256"]
        },
        "compile_then_verify": "The compile release binds the compile call only (candidate and compile binding null). The verify call does not exist until a validated compile candidate does; the host then prepares it with its own operation/attempt/exposure, candidate_sha256 and compile_binding_sha256, projects it under its own key, and needs its own action and grant. No verifier release can be issued in advance.",
        "no_resend": "A claimed operation/attempt identity or an existing journal yields OutcomeUnknown, never Pending and never a resend, including after restart or from another work directory sharing the credential store.",
        "preview_keys": {
            "note": "for the public proposal labels only; runtime operation ids decide the real keys",
            "compile": ids["compile_operation"].as_str().map(key),
            "verify": ids["verify_operation"].as_str().map(key)
        }
    })
}

fn unresolved(value: &Value, pointer: String, found: &mut Vec<Value>) {
    match value {
        Value::Object(map) if map.contains_key("unresolved") && map.len() == 1 => {
            let mut item = map["unresolved"].clone();
            item["pointer"] = json!(pointer);
            found.push(item);
        }
        Value::Object(map) => {
            for (k, v) in map {
                unresolved(v, format!("{pointer}/{k}"), found);
            }
        }
        Value::Array(items) => {
            for (i, v) in items.iter().enumerate() {
                unresolved(v, format!("{pointer}/{i}"), found);
            }
        }
        _ => {}
    }
}

/// In-memory typed stand-ins, used only to compare the template's key set and
/// value types with `HostStartup`. They are never written or used to start.
fn stand_in(value: &mut Value) -> Res<()> {
    if let Some(ty) = value.get("unresolved").and_then(|u| u["type"].as_str()).map(str::to_owned) {
        *value = match ty.as_str() {
            "string" => json!("x"),
            "path" => json!("/selfcheck/x"),
            "optional_path" => Value::Null,
            "digest" => json!("0".repeat(64)),
            "string_array" | "grants" => json!([]),
            "u64" | "usize" => json!(1),
            "bool" => json!(false),
            "principal" => json!({"uid": 0, "principal_ref": "x", "producer_ref": "x"}),
            "snapshot" => json!({"id": "x", "source_revision": "x", "members": []}),
            "bounds" => json!({"request_bytes": 1, "stream_bytes": 1, "frame_bytes": 1,
                               "output_bytes": 1, "events": 1, "local_deadline_ms": 1}),
            enumerated if enumerated.starts_with("enum:") => json!(&enumerated[5..]),
            other => return Err(format!("unknown stand-in type {other}")),
        };
        return Ok(());
    }
    match value {
        Value::Object(map) => map.values_mut().try_for_each(stand_in),
        Value::Array(items) => items.iter_mut().try_for_each(stand_in),
        _ => Ok(()),
    }
}

fn selfcheck(package: &Path) -> Res<()> {
    let parsed: Value = serde_json::from_slice(&read(package)?).map_err(|e| e.to_string())?;
    let raw = serde_json::from_value::<HostStartup>(parsed["host_startup"].clone());
    let mut filled = parsed["host_startup"].clone();
    stand_in(&mut filled)?;
    let shaped = serde_json::from_value::<HostStartup>(filled.clone()).map_err(|e| e.to_string());
    let round_trip = shaped.as_ref().ok().map(|s| {
        let mut back = serde_json::to_value(s).unwrap_or(Value::Null);
        // Optional verifier paths and the proof barrier serialize as null.
        back["lifecycle"]["native"]["semantic_contexts"] = Value::Null;
        key_paths(&back) == key_paths(&filled)
    });
    println!("{}", json!({"marked_template_parses_as_HostStartup": raw.is_ok(),
        "marked_template_error": raw.err().map(|e| e.to_string()),
        "stand_in_shape": shaped.as_ref().map(|_| "HostStartup").map_err(Clone::clone),
        "stand_in_key_paths_round_trip_equal": round_trip}));
    if shaped.is_err() || round_trip != Some(true) {
        return Err("template key set does not match HostStartup".into());
    }
    Ok(())
}

fn key_paths(value: &Value) -> Vec<String> {
    fn walk(v: &Value, p: String, out: &mut Vec<String>) {
        if let Value::Object(map) = v {
            for (k, child) in map {
                if child.is_null() {
                    continue;
                }
                let path = format!("{p}/{k}");
                out.push(path.clone());
                walk(child, path, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(value, String::new(), &mut out);
    out.sort();
    out
}

fn preflight(package: &Path, out: &Path) -> Res<u8> {
    // One read: the digest reported is of the exact bytes every check parsed.
    let package_bytes = read(package)?;
    let parsed: Value = serde_json::from_slice(&package_bytes).map_err(|e| e.to_string())?;
    let mut found = Vec::new();
    unresolved(&parsed, String::new(), &mut found);
    let mut checks = serde_json::Map::new();
    let prep = &parsed["run_bindings"]["canonical_preparation"];
    let prep_ok = prep["path"].as_str().map(PathBuf::from).and_then(|p| fs::read(p).ok())
        .map(|b| Some(sha256(&b).as_str()) == prep["sha256"].as_str());
    checks.insert("canonical_preparation_digest".into(), json!(prep_ok));
    let startup = &parsed["host_startup"];
    checks.insert("control_kind_direct".into(), json!(startup["control"]["kind"] == "direct"));
    checks.insert("semantic_capture_section_present".into(),
        json!(startup["lifecycle"]["native"]["semantic"].is_object()));
    checks.insert("profile".into(), json!(startup["profile"] == PROFILE
        && startup["lifecycle"]["profile"] == PROFILE));
    // Every bound digest is recomputed here; startup recomputes them again.
    // A field that is still an owner marker is reported as null.
    let digest = |path: &Value, sha: &Value| match (path.as_str(), sha.as_str()) {
        (Some(p), Some(s)) => json!(fs::read(p).ok().map(|b| sha256(&b)).as_deref() == Some(s)),
        _ => Value::Null,
    };
    let runs = &parsed["run_bindings"];
    checks.insert("executable_digest".into(),
        digest(&runs["executable_path"], &startup["executable_sha256"]));
    checks.insert("lock_digest".into(), digest(&startup["lock_path"], &startup["lock_sha256"]));
    checks.insert("toolchain_digest".into(),
        digest(&startup["toolchain_path"], &startup["toolchain_sha256"]));
    checks.insert("source_manifest_digest".into(),
        digest(&startup["source_manifest_path"], &startup["source_sha256"]));
    checks.insert("build_provenance_digest".into(),
        digest(&runs["build_provenance"]["path"], &runs["build_provenance"]["sha256"]));
    let manifest = match (startup["source_root"].as_str(), startup["source_manifest_path"].as_str()) {
        (Some(root), Some(m)) => json!(verify_manifest(Path::new(root), Path::new(m)).is_ok()),
        _ => Value::Null,
    };
    checks.insert("source_manifest_entries".into(), manifest);
    checks.insert("features_default_build".into(), json!(startup["features"] == json!([])));
    checks.insert("control_model".into(), json!(startup["control"]["model"] == MODEL));
    checks.insert("control_bounds_public_route".into(),
        json!(startup["control"]["bounds"] == json!(PublicHostEndpoint::bounds())));
    let bound = &runs["direct_release_protocol"]["bound_values"];
    checks.insert("release_protocol_constants".into(), json!(
        bound["provider"] == PublicHostEndpoint::PROVIDER
            && bound["profile_revision"] == PublicHostEndpoint::PROFILE_REVISION
            && bound["model"] == MODEL
            && bound["bounds"] == json!(PublicHostEndpoint::bounds())
            && bound["action.source_sha256"] == ActionGrant::source_sha256()
            && runs["library_source_sha256"] == ActionGrant::source_sha256()
            && bound["action.executable_sha256"] == startup["executable_sha256"]
            && bound["grant.host_source_sha256"] == startup["source_sha256"]));
    let (layout_ok, layout_record, layout_refusals) = layout_checks(&parsed)?;
    checks.insert("runtime_layout".into(), layout_ok);
    checks.insert("runtime_layout_record".into(), layout_record);
    let shape = if found.iter().any(|f| f["pointer"].as_str().is_some_and(|p| p.starts_with("/host_startup"))) {
        "not_attempted: unresolved startup fields".to_owned()
    } else {
        match serde_json::from_value::<HostStartup>(startup.clone()) {
            Ok(_) => "HostStartup".to_owned(),
            Err(e) => format!("invalid: {e}"),
        }
    };
    let any_false = prep_ok != Some(true) || checks.values().any(|v| v == &json!(false));
    let all_true = checks.values().all(|v| v == &json!(true));
    checks.insert("host_startup_shape".into(), json!(shape));
    // A bound value that fails its check is a defect even while owner inputs
    // are still missing, so it takes precedence over the unresolved status.
    let (status, code) = if any_false {
        ("NOT_READY_INVALID_BINDINGS", 3)
    } else if !found.is_empty() {
        ("NOT_READY_UNRESOLVED_OWNER_INPUTS", 2)
    } else if !all_true || shape != "HostStartup" {
        ("NOT_READY_INVALID_BINDINGS", 3)
    } else {
        ("OWNER_INPUTS_BOUND_SHAPE_VALID", 0)
    };
    let mut by_gap = std::collections::BTreeMap::<String, usize>::new();
    for f in &found {
        *by_gap.entry(f["gap"].as_str().unwrap_or("?").to_owned()).or_default() += 1;
    }
    let report = json!({
        "schema": "b3_offline_preflight_v1",
        "package": {"path": package, "sha256": sha256(&package_bytes)},
        "status": status,
        "live_ready": "not_established: host validate_startup, custody, grants and release run only at serve and owner release",
        "unresolved_count": found.len(), "unresolved_by_gap": by_gap,
        "checks": checks, "unresolved": found,
        "runtime_layout_refusals": layout_refusals,
        "runtime_layout_external": "credential_dir is compared only when bound; the host \
            rechecks release/journal/credential disjointness and directory modes at serve"
    });
    write_json(out, &report)?;
    println!("{}", json!({"status": status, "unresolved": report["unresolved_count"],
        "by_gap": report["unresolved_by_gap"], "checks": report["checks"]}));
    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::{fs::symlink, net::UnixListener};

    fn fresh() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(dir.path()).unwrap().join("r");
        (dir, root)
    }

    fn proposed(root: &Path) -> Bound {
        LAYOUT.iter().map(|(key, _, _, rel)| (*key, root.join(rel))).collect()
    }

    fn refusals(root: &Path, paths: &Bound) -> Vec<String> {
        check_layout(root, paths, &[]).unwrap()
    }

    fn chmod(path: &Path, mode: u32) {
        fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
    }

    fn layout_at(root: &Path) -> Value {
        let out = root.with_extension("json");
        layout(root, &out).unwrap();
        serde_json::from_slice(&fs::read(out).unwrap()).unwrap()
    }

    #[test]
    fn fresh_layout_creates_only_private_empty_directories() {
        let (_t, root) = fresh();
        let record = layout_at(&root);
        assert_eq!(record["created"].as_array().unwrap().len(), 1 + LAYOUT_DIRS.len());
        for dir in std::iter::once(root.clone()).chain(LAYOUT_DIRS.iter().map(|d| root.join(d))) {
            let meta = fs::symlink_metadata(&dir).unwrap();
            assert_eq!(meta.mode() & 0o777, 0o700, "{}", dir.display());
        }
        // Only the four directories exist: no store, socket, journal or lifecycle store.
        let mut entries: Vec<_> = LAYOUT_DIRS.iter().flat_map(|d| fs::read_dir(root.join(d)).unwrap())
            .collect();
        assert!(entries.is_empty(), "{entries:?}");
        entries.extend(fs::read_dir(&root).unwrap());
        assert_eq!(entries.len(), LAYOUT_DIRS.len());
        let (r, paths) = parse_layout(&fs::read(root.with_extension("json")).unwrap()).unwrap();
        assert_eq!((r.clone(), paths.clone()), (root.clone(), proposed(&root)));
        assert!(refusals(&root, &paths).is_empty());
        assert!(paths.iter().all(|(_, p)| p.as_os_str().len() <= SOCKET_PATH_MAX));
    }

    #[test]
    fn existing_private_layout_is_accepted_on_rerun_and_restart() {
        let (_t, root) = fresh();
        layout_at(&root);
        let again = layout_at(&root);
        assert_eq!(again["created"], json!([]));
        // State an earlier run leaves behind, all owner-only.
        let store = root.join("host/task-store.sqlite3");
        fs::write(&store, b"").unwrap();
        chmod(&store, 0o600);
        fs::DirBuilder::new().mode(0o700).create(root.join("direct/journal")).unwrap();
        fs::DirBuilder::new().mode(0o700).create(root.join("lifecycle/store")).unwrap();
        let _listener = UnixListener::bind(root.join("host/host.sock")).unwrap();
        assert_eq!(refusals(&root, &proposed(&root)), Vec::<String>::new());
    }

    #[test]
    fn non_private_existing_state_is_refused() {
        for (rel, setup) in [
            ("host/task-store.sqlite3", 0o644),
            ("direct/journal", 0o755),
            ("lifecycle/store", 0o750),
        ] {
            let (_t, root) = fresh();
            layout_at(&root);
            let path = root.join(rel);
            if rel.ends_with("sqlite3") {
                fs::write(&path, b"").unwrap();
            } else {
                fs::create_dir(&path).unwrap();
            }
            chmod(&path, setup);
            let found = refusals(&root, &proposed(&root));
            assert_eq!(found.len(), 1, "{rel}: {found:?}");
            assert!(found[0].contains("wrong type, owner or mode"), "{found:?}");
        }
        // A plain file where a socket belongs is refused too.
        let (_t, root) = fresh();
        layout_at(&root);
        fs::write(root.join("lifecycle/peer.sock"), b"").unwrap();
        let found = refusals(&root, &proposed(&root));
        assert!(found.len() == 1 && found[0].starts_with("peer_socket_path"), "{found:?}");
    }

    #[test]
    fn group_or_other_access_to_root_or_parents_is_refused() {
        let (_t, root) = fresh();
        layout_at(&root);
        chmod(&root.join("host"), 0o750);
        let found = refusals(&root, &proposed(&root));
        assert_eq!(found.len(), 2, "{found:?}");
        assert!(found.iter().all(|r| r.contains("parent") && r.contains("/host ")), "{found:?}");
        chmod(&root.join("host"), 0o700);
        chmod(&root, 0o755);
        assert!(refusals(&root, &proposed(&root))[0].starts_with("root "));
        // `layout` refuses an existing non-private root and creates nothing in it.
        fs::remove_dir_all(root.join("host")).unwrap();
        assert!(layout(&root, &root.with_extension("again.json")).is_err());
        assert!(!root.join("host").exists());
        chmod(&root, 0o700);
    }

    #[test]
    fn release_directory_must_exist_owner_only() {
        let (_t, root) = fresh();
        layout_at(&root);
        chmod(&root.join("release"), 0o750);
        let found = refusals(&root, &proposed(&root));
        assert!(found.len() == 1 && found[0].starts_with("release_dir"), "{found:?}");
        fs::remove_dir(root.join("release")).unwrap();
        let found = refusals(&root, &proposed(&root));
        assert!(found.len() == 1 && found[0].starts_with("release_dir"), "{found:?}");
    }

    #[test]
    fn overlapping_or_outside_locations_are_refused() {
        let (_t, root) = fresh();
        layout_at(&root);
        let mut paths = proposed(&root);
        paths[2].1 = root.join("release/journal"); // journal_dir inside release_dir
        paths[6].1 = paths[5].1.clone(); // peer socket == lifecycle socket
        let found = refusals(&root, &paths);
        assert!(found.contains(&"journal_dir overlaps release_dir".to_owned()), "{found:?}");
        assert!(found.contains(&"lifecycle_socket_path overlaps peer_socket_path".to_owned()));
        // Locations the package binds elsewhere are avoided as well.
        let avoid = vec![("credential_dir", root.join("direct")), ("source_root", root.clone())];
        let found = check_layout(&root, &proposed(&root), &avoid).unwrap();
        assert!(found.contains(&"journal_dir overlaps credential_dir".to_owned()), "{found:?}");
        assert_eq!(found.iter().filter(|r| r.ends_with("overlaps source_root")).count(), 7);
        // Outside the root, even under another private directory.
        let mut paths = proposed(&root);
        paths[0].1 = root.parent().unwrap().join("task-store.sqlite3");
        let found = refusals(&root, &paths);
        assert!(found.iter().any(|r| r == "task_store_path is not inside the layout root"));
    }

    #[test]
    fn unplain_symlinked_or_long_socket_paths_are_refused() {
        let (_t, root) = fresh();
        layout_at(&root);
        for bad in ["host/../host/host.sock", "host//host.sock", "host/./host.sock"] {
            let mut paths = proposed(&root);
            paths[1].1 = PathBuf::from(format!("{}/{bad}", root.display()));
            let found = refusals(&root, &paths);
            assert!(found.iter().any(|r| r.contains("not a plain absolute path")), "{bad}: {found:?}");
        }
        let mut paths = proposed(&root);
        paths[1].1 = PathBuf::from("host/host.sock");
        assert!(refusals(&root, &paths)[0].contains("not a plain absolute path"));
        // A symlinked parent is refused even when its target is private.
        symlink(root.join("lifecycle"), root.join("link")).unwrap();
        let mut paths = proposed(&root);
        paths[5].1 = root.join("link/lifecycle.sock");
        let found = refusals(&root, &paths);
        assert!(found.iter().any(|r| r.starts_with("lifecycle_socket_path parent")), "{found:?}");
        // Socket paths beyond sun_path are refused; other locations may be longer.
        let deep = root.join("host").join("d".repeat(SOCKET_PATH_MAX));
        fs::DirBuilder::new().mode(0o700).create(&deep).unwrap();
        let mut paths = proposed(&root);
        paths[1].1 = deep.join("host.sock");
        paths[0].1 = deep.join("task-store.sqlite3");
        let found = refusals(&root, &paths);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].starts_with("socket_path is ") && found[0].contains("at most 107"));
        // `layout` refuses a root that long before creating anything.
        let long = deep.join("r");
        let refused = layout(&long, &root.with_extension("long.json")).unwrap_err();
        assert!(refused.contains("too long"), "{refused}");
        assert!(!long.exists());
    }

    #[test]
    fn template_binding_replaces_only_the_seven_markers_and_preflight_rechecks() {
        let (t, root) = fresh();
        let record = layout_at(&root);
        let layout_file = root.with_extension("json");
        let marker = u("o", "R01", "path", "n");
        let mut package = json!({
            "host_startup": {"task_store_path": marker, "socket_path": marker,
                "source_root": t.path().join("src"),
                "control": {"journal_dir": marker, "release_dir": marker,
                    "credential_dir": u("c", "R04", "path", "n")},
                "lifecycle": {"store_root": marker, "socket_path": marker,
                    "peer_socket_path": marker}},
            "run_bindings": {"runtime_layout": null}
        });
        assert_eq!(layout_checks(&package).unwrap(), (Value::Null, Value::Null, vec![]));
        bind_layout(&mut package, &layout_file).unwrap();
        let mut found = Vec::new();
        unresolved(&package, String::new(), &mut found);
        assert_eq!(found.len(), 1, "only credential_dir stays an owner marker: {found:?}");
        for (key, pointer, ..) in LAYOUT {
            assert_eq!(package.pointer(pointer).unwrap(), &record["paths"][key]);
        }
        assert_eq!(layout_checks(&package).unwrap(), (json!(true), json!(true), vec![]));
        // A later change to the layout file or to the filesystem is caught.
        let mut tampered = package.clone();
        tampered["host_startup"]["socket_path"] = json!(root.join("host/other.sock"));
        assert_eq!(layout_checks(&tampered).unwrap().1, json!(false));
        chmod(&root.join("release"), 0o755);
        let (ok, same, refused) = layout_checks(&package).unwrap();
        assert_eq!((ok, same), (json!(false), json!(true)));
        assert!(refused[0].starts_with("release_dir"));
        chmod(&root.join("release"), 0o700);
        // Bound paths with no layout record are refused, not trusted.
        let mut bare = package.clone();
        bare["run_bindings"]["runtime_layout"] = Value::Null;
        assert_eq!(layout_checks(&bare).unwrap().0, json!(false));
        // A bound credential store inside the layout is refused at binding.
        let mut clash = package.clone();
        clash["host_startup"]["control"]["credential_dir"] = json!(root.join("direct"));
        assert!(bind_layout(&mut clash, &layout_file).unwrap_err().contains("overlaps credential_dir"));
    }

    #[test]
    fn preflight_reports_the_digest_of_the_bytes_it_checked() {
        let (t, root) = fresh();
        layout_at(&root);
        let package = t.path().join("package.json");
        let bytes = br#"{"host_startup": {}, "run_bindings": {}}"#;
        fs::write(&package, bytes).unwrap();
        let out = t.path().join("report.json");
        assert_eq!(preflight(&package, &out).unwrap(), 3);
        let report: Value = serde_json::from_slice(&fs::read(&out).unwrap()).unwrap();
        assert_eq!(report["package"]["sha256"], sha256(bytes));
        assert_eq!(report["checks"]["runtime_layout"], Value::Null);
    }
}
