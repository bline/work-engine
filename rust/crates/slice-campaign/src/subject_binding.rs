//! Read-only verification of the frozen SC0 historical input profile. The
//! legacy gate receipt is not self-binding: this owner also checks the pinned
//! producer observation, manifest and exact tested tree/path bytes.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::Read;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::{Map, Value, json};

use crate::codec::{historical_digest, raw_sha256};
use crate::contract::{Baseline, CampaignIdentity, CandidateRef};
use crate::{CampaignError, Result, require_sha};

pub const CHECKPOINT_SOURCE_SHA256: &str =
    "1b814da3ef9e1f20804b934b91f6621ed65159dd9dfab5bfbfdd2df95f82d20b";
pub const ANALYZER_SOURCE_SHA256: &str =
    "47e486ce1539bbfe1ab466d77650d9bb3e0338222478ae445fe6a07ba95e2003";
pub const GATE_RUNNER_SOURCE_SHA256: &str =
    "c8fee4d50fc5689c253fb1ef76a4b86159a3c8a916b76db9a764fbe5c0ed0dba";
pub const SC0_GATE_CAPTURE_SHA256: &str =
    "f52792e22d2a1b73e007bc7078607d6657989248a35de1371a1180d23101d206";
pub const SC0_GATE_MANIFEST_SHA256: &str =
    "d155cc2ac1cf1ab26b0e186d6a6ac86f547b1a0b9c3ce74f0ba69eecd37a2f34";
pub const SC0_GATE_RECEIPT_SHA256: &str =
    "c927b51b887e37f6fdc02e1fabfadddd211d0ac30712cbc6c5028993ad3889c2";
pub const PYTHON_EXE_SHA256: &str =
    "52e0a13e60a981d8c4b6478be2ba5176f69da07948a056bf49cf6f077e30cb41";
pub const GIT_EXE_SHA256: &str = "5516c9f362c29376ab9a499a33082f9f611941d8c75930c880e30ad109e39c9a";

#[derive(Clone, Debug)]
pub struct VerifiedGate {
    pub capture_digest: String,
    pub receipt_digest: String,
    pub tested_tree: String,
    pub paths: BTreeMap<String, String>,
}

#[cfg(test)]
mod selected_snapshot_tests {
    use super::verify_candidate_snapshot;
    use std::fs;
    use std::os::unix::fs::{PermissionsExt, symlink};
    use std::path::Path;
    use std::process::Command;

    fn git(root: &Path, args: &[&str]) -> String {
        let output = Command::new("/usr/bin/git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().into()
    }

    #[test]
    fn selected_tree_refuses_wrong_checkout_dirty_extra_link_and_mode() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        git(root, &["init", "-q"]);
        fs::create_dir(root.join("nested")).unwrap();
        fs::write(root.join("nested/source.rs"), "first\n").unwrap();
        git(root, &["add", "nested/source.rs"]);
        git(
            root,
            &[
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.invalid",
                "commit",
                "-qm",
                "first",
            ],
        );
        let first_commit = git(root, &["rev-parse", "HEAD"]);
        let first_tree = git(root, &["rev-parse", "HEAD^{tree}"]);
        verify_candidate_snapshot(root, &first_tree).unwrap();

        fs::write(root.join("nested/source.rs"), "second\n").unwrap();
        assert!(
            verify_candidate_snapshot(root, &first_tree).is_err(),
            "dirty tracked bytes"
        );
        git(root, &["add", "nested/source.rs"]);
        git(
            root,
            &[
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.invalid",
                "commit",
                "-qm",
                "second",
            ],
        );
        assert!(
            verify_candidate_snapshot(root, &first_tree).is_err(),
            "wrong checkout"
        );
        git(root, &["checkout", "-q", &first_commit]);
        verify_candidate_snapshot(root, &first_tree).unwrap();

        fs::write(root.join("untracked.bin"), b"not indexed").unwrap();
        assert!(
            verify_candidate_snapshot(root, &first_tree).is_err(),
            "extra exposed file"
        );
        fs::remove_file(root.join("untracked.bin")).unwrap();
        fs::create_dir(root.join("empty-extra")).unwrap();
        assert!(
            verify_candidate_snapshot(root, &first_tree).is_err(),
            "extra exposed directory"
        );
        fs::remove_dir(root.join("empty-extra")).unwrap();
        symlink("nested/source.rs", root.join("source-link")).unwrap();
        assert!(
            verify_candidate_snapshot(root, &first_tree).is_err(),
            "symlink"
        );
        fs::remove_file(root.join("source-link")).unwrap();
        fs::set_permissions(
            root.join("nested/source.rs"),
            fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        assert!(
            verify_candidate_snapshot(root, &first_tree).is_err(),
            "executable mode drift"
        );
        fs::set_permissions(
            root.join("nested/source.rs"),
            fs::Permissions::from_mode(0o644),
        )
        .unwrap();
        let admin = tempfile::tempdir().unwrap();
        fs::rename(root.join(".git"), admin.path().join("git-data")).unwrap();
        symlink(admin.path().join("git-data"), root.join(".git")).unwrap();
        assert!(
            verify_candidate_snapshot(root, &first_tree).is_err(),
            "Git admin link"
        );
    }
}

#[derive(Clone, Debug)]
pub struct VerifiedCandidate {
    pub reference: CandidateRef,
    pub candidate_receipt: Value,
    pub physical_profile: Value,
    pub profile_digest: String,
    pub gate: VerifiedGate,
}

fn obj<'a>(value: &'a Value, label: &str) -> Result<&'a Map<String, Value>> {
    value
        .as_object()
        .ok_or_else(|| CampaignError::Subject(format!("{label} must be an object")))
}
fn array<'a>(value: &'a Value, label: &str) -> Result<&'a [Value]> {
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| CampaignError::Subject(format!("{label} must be an array")))
}
fn str_at<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| CampaignError::Subject(format!("missing text {key}")))
}
fn exact(value: &Value, fields: &[&str], label: &str) -> Result<()> {
    let names: BTreeSet<&str> = obj(value, label)?.keys().map(String::as_str).collect();
    if names != fields.iter().copied().collect() {
        return Err(CampaignError::Subject(format!("{label} fields differ")));
    }
    Ok(())
}
fn parse(raw: &[u8], label: &str) -> Result<Value> {
    if raw.len() > 1_048_576 {
        return Err(CampaignError::Subject(format!(
            "{label} exceeds input bound"
        )));
    }
    serde_json::from_slice(raw).map_err(|e| CampaignError::Subject(format!("{label}: {e}")))
}
fn git(repository: &Path, args: &[&str], max: usize) -> Result<Vec<u8>> {
    let mut child = Command::new("/usr/bin/timeout")
        .args(["--kill-after=1s", "10s", "/usr/bin/git"])
        .arg("-C")
        .arg(repository)
        .args(args)
        .env_clear()
        .env("GIT_NO_REPLACE_OBJECTS", "1")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| CampaignError::Subject(format!("git invocation: {e}")))?;
    let mut bytes = Vec::new();
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| CampaignError::Subject("git stdout unavailable".into()))?;
    stdout
        .take((max + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| CampaignError::Subject(e.to_string()))?;
    if bytes.len() > max {
        let _ = child.kill();
        let _ = child.wait();
        return Err(CampaignError::Subject("Git output exceeds bound".into()));
    }
    let status = child
        .wait()
        .map_err(|e| CampaignError::Subject(e.to_string()))?;
    if !status.success() {
        return Err(CampaignError::Subject(format!(
            "bounded git {:?} failed",
            args
        )));
    }
    Ok(bytes)
}
fn git_text(repository: &Path, args: &[&str], max: usize) -> Result<String> {
    let bytes = git(repository, args, max)?;
    String::from_utf8(bytes)
        .map(|text| text.trim_end_matches('\n').to_owned())
        .map_err(|e| CampaignError::Subject(e.to_string()))
}
fn full_oid(value: &str) -> Result<()> {
    if ![40, 64].contains(&value.len())
        || !value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err(CampaignError::Subject("invalid full Git OID".into()));
    }
    Ok(())
}
fn anchored_repository(repository: &Path) -> Result<()> {
    if !repository.is_absolute() || fs::canonicalize(repository).ok().as_deref() != Some(repository)
    {
        return Err(CampaignError::Subject("repository is not anchored".into()));
    }
    let meta =
        fs::symlink_metadata(repository).map_err(|e| CampaignError::Subject(e.to_string()))?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err(CampaignError::Subject(
            "repository is not a real directory".into(),
        ));
    }
    Ok(())
}

/// Bind every file exposed by the candidate mount to the selected immutable
/// Git tree. Git administrative data alone is outside the exposed source.
/// This read-only check is repeated on cold open and immediately before E1
/// records launch intent; a clean `git status` is not used as a substitute for
/// checking ignored and untracked files.
pub fn verify_candidate_snapshot(repository: &Path, tree: &str) -> Result<()> {
    anchored_repository(repository)?;
    full_oid(tree)?;
    const MAX_INVENTORY_BYTES: usize = 16 * 1024 * 1024;
    const MAX_FILES: usize = 65_536;
    const MAX_FILE_BYTES: u64 = 512 * 1024 * 1024;
    const MAX_TOTAL_BYTES: u64 = 2 * 1024 * 1024 * 1024;
    let raw = git(
        repository,
        &["ls-tree", "-r", "-z", "--full-tree", tree],
        MAX_INVENTORY_BYTES,
    )?;
    let mut expected = BTreeMap::new();
    let mut expected_dirs = BTreeSet::new();
    if !raw.is_empty() && raw.last() != Some(&0) {
        return Err(CampaignError::Subject(
            "Git tree inventory is truncated".into(),
        ));
    }
    for item in raw.split(|byte| *byte == 0).filter(|item| !item.is_empty()) {
        let mut fields = item.splitn(2, |byte| *byte == b'\t');
        let header = fields.next().unwrap_or_default();
        let path = fields
            .next()
            .ok_or_else(|| CampaignError::Subject("Git tree entry malformed".into()))?;
        let header = std::str::from_utf8(header)
            .map_err(|_| CampaignError::Subject("Git tree entry header invalid".into()))?;
        let mut parts = header.split(' ');
        let mode = parts.next().unwrap_or("");
        let kind = parts.next().unwrap_or("");
        let oid = parts.next().unwrap_or("");
        if parts.next().is_some() || kind != "blob" || !matches!(mode, "100644" | "100755") {
            return Err(CampaignError::Subject(
                "candidate Git tree contains unsupported file kind".into(),
            ));
        }
        full_oid(oid)?;
        let path = std::str::from_utf8(path)
            .map_err(|_| CampaignError::Subject("candidate path is not UTF-8".into()))?;
        let relative = Path::new(path);
        if path.is_empty()
            || relative.is_absolute()
            || relative
                .components()
                .any(|part| !matches!(part, std::path::Component::Normal(_)))
            || path
                .split('/')
                .any(|part| part.is_empty() || part == ".git")
            || expected
                .insert(path.to_owned(), (oid.to_owned(), mode == "100755"))
                .is_some()
            || expected.len() > MAX_FILES
        {
            return Err(CampaignError::Subject(
                "candidate Git path inventory invalid".into(),
            ));
        }
        let mut parent = relative.parent();
        while let Some(directory) = parent {
            if directory.as_os_str().is_empty() {
                break;
            }
            expected_dirs.insert(directory.to_string_lossy().into_owned());
            parent = directory.parent();
        }
    }
    let mut observed = BTreeMap::new();
    let mut observed_dirs = BTreeSet::new();
    let mut stack = vec![repository.to_owned()];
    let mut total_bytes = 0_u64;
    while let Some(directory) = stack.pop() {
        for entry in fs::read_dir(&directory).map_err(|e| CampaignError::Subject(e.to_string()))? {
            let entry = entry.map_err(|e| CampaignError::Subject(e.to_string()))?;
            if directory == repository && entry.file_name() == ".git" {
                let admin = fs::symlink_metadata(entry.path())
                    .map_err(|e| CampaignError::Subject(e.to_string()))?;
                if admin.file_type().is_symlink()
                    || (!admin.file_type().is_dir() && !admin.file_type().is_file())
                {
                    return Err(CampaignError::Subject(
                        "Git administrative path is not a regular file or directory".into(),
                    ));
                }
                continue;
            }
            let path = entry.path();
            let relative = path
                .strip_prefix(repository)
                .map_err(|_| CampaignError::Subject("candidate path escaped root".into()))?;
            let name = relative
                .to_str()
                .ok_or_else(|| CampaignError::Subject("candidate path is not UTF-8".into()))?
                .to_owned();
            let meta =
                fs::symlink_metadata(&path).map_err(|e| CampaignError::Subject(e.to_string()))?;
            if meta.file_type().is_dir() {
                observed_dirs.insert(name);
                stack.push(path);
            } else if meta.file_type().is_file() {
                total_bytes = total_bytes.saturating_add(meta.len());
                if meta.len() > MAX_FILE_BYTES || total_bytes > MAX_TOTAL_BYTES {
                    return Err(CampaignError::Subject(
                        "candidate snapshot exceeds bound".into(),
                    ));
                }
                if observed.insert(name, path).is_some() || observed.len() > MAX_FILES {
                    return Err(CampaignError::Subject(
                        "candidate snapshot inventory invalid".into(),
                    ));
                }
            } else {
                return Err(CampaignError::Subject(
                    "candidate snapshot has link or special file".into(),
                ));
            }
        }
    }
    if observed_dirs != expected_dirs || observed.keys().ne(expected.keys()) {
        return Err(CampaignError::Subject(
            "candidate snapshot inventory differs from selected Git tree".into(),
        ));
    }
    for (name, (oid, executable)) in expected {
        let path = &observed[&name];
        let before =
            fs::symlink_metadata(path).map_err(|e| CampaignError::Subject(e.to_string()))?;
        if !before.is_file()
            || before.file_type().is_symlink()
            || (before.permissions().mode() & 0o111 != 0) != executable
        {
            return Err(CampaignError::Subject(
                "candidate file mode differs from Git".into(),
            ));
        }
        let mut file = File::open(path).map_err(|e| CampaignError::Subject(e.to_string()))?;
        let opened = file
            .metadata()
            .map_err(|e| CampaignError::Subject(e.to_string()))?;
        if (opened.dev(), opened.ino()) != (before.dev(), before.ino()) {
            return Err(CampaignError::Subject(
                "candidate file changed while opening".into(),
            ));
        }
        let mut child = Command::new("/usr/bin/timeout")
            .args(["--kill-after=1s", "10s", "/usr/bin/git"])
            .arg("-C")
            .arg(repository)
            .args(["hash-object", "--no-filters", "--stdin"])
            .env_clear()
            .env("GIT_NO_REPLACE_OBJECTS", "1")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| CampaignError::Subject(format!("Git file hash failed: {e}")))?;
        let copied = std::io::copy(&mut file, child.stdin.as_mut().unwrap())
            .map_err(|e| CampaignError::Subject(e.to_string()))?;
        child.stdin.take();
        let output = child
            .wait_with_output()
            .map_err(|e| CampaignError::Subject(e.to_string()))?;
        let after =
            fs::symlink_metadata(path).map_err(|e| CampaignError::Subject(e.to_string()))?;
        if !output.status.success()
            || output.stdout.len() > 128
            || output.stdout != format!("{oid}\n").as_bytes()
            || copied != before.len()
            || (after.dev(), after.ino(), after.len(), after.mode())
                != (before.dev(), before.ino(), before.len(), before.mode())
        {
            return Err(CampaignError::Subject(
                "candidate file bytes differ from Git".into(),
            ));
        }
    }
    Ok(())
}

pub(crate) fn verify_gate(
    capture_raw: &[u8],
    manifest_raw: &[u8],
    receipt_raw: &[u8],
) -> Result<VerifiedGate> {
    if raw_sha256(capture_raw) != SC0_GATE_CAPTURE_SHA256
        || raw_sha256(manifest_raw) != SC0_GATE_MANIFEST_SHA256
        || raw_sha256(receipt_raw) != SC0_GATE_RECEIPT_SHA256
    {
        return Err(CampaignError::Subject(
            "gate input differs from frozen trusted SC0 capture".into(),
        ));
    }
    verify_gate_dynamic(capture_raw, manifest_raw, receipt_raw)
}

/// Schema 4 accepts a new immutable gate capture while retaining the pinned
/// producer, isolated interpreter and exact tree/path checks. The accepted
/// capture bytes are bound into the trusted configuration digest.
pub(crate) fn verify_gate_dynamic(
    capture_raw: &[u8],
    manifest_raw: &[u8],
    receipt_raw: &[u8],
) -> Result<VerifiedGate> {
    let capture_digest = raw_sha256(capture_raw);
    let manifest_digest = raw_sha256(manifest_raw);
    let receipt_digest = raw_sha256(receipt_raw);
    let capture = parse(capture_raw, "gate capture")?;
    let manifest = parse(manifest_raw, "gate manifest")?;
    let receipt = parse(receipt_raw, "gate receipt")?;
    let checks = receipt["checks"]
        .as_array()
        .ok_or_else(|| CampaignError::Subject("gate checks absent".into()))?;
    if checks.is_empty()
        || checks.iter().any(|check| check["state"] != "passed")
        || receipt["totals"]
            != json!({"configured":checks.len(),"executed":checks.len(),
            "failed":0,"passed":checks.len()})
    {
        return Err(CampaignError::Subject(
            "gate checks or totals differ".into(),
        ));
    }
    if str_at(&capture, "producer")? != "skills/slice-builder/scripts/run_gate.py"
        || str_at(&capture, "producer_sha256_before_after")? != GATE_RUNNER_SOURCE_SHA256
        || capture["argv_manifest_sha256"] != manifest_digest
        || capture["child_interpreter_sha256"] != PYTHON_EXE_SHA256
        || capture["git_runtime_before_after"]["executable_sha256"] != GIT_EXE_SHA256
        || capture["python_runtime_before_after"]["executable_sha256"] != PYTHON_EXE_SHA256
        || capture["exit_status"] != 0
        || capture["stderr"] != ""
        || capture["before"] != capture["after"]
        || capture["before"]["all_status"] != ""
        || capture["before"]["head_tree"] != capture["frozen_gate_tree_oid"]
        || capture["tested_path_sha256"] != capture["before"]["path_sha256"]
        || capture["tested_path_sha256"] != capture["candidate_tree_path_sha256"]
        || capture["tested_path_sha256"] != capture["candidate_manifest_path_sha256"]
        || receipt["status"] != "passed"
        || receipt["failed_check"] != Value::Null
        || manifest
            != serde_json::from_str::<Value>(
                capture["argv"][4]
                    .as_str()
                    .ok_or_else(|| CampaignError::Subject("gate argv absent".into()))?,
            )
            .map_err(|e| CampaignError::Subject(e.to_string()))?
        || capture["argv"][1] != "-I"
        || capture["child_mode"]
            != json!({"isolated":1,"optimize":0,"explicit_conditional_exit":true})
    {
        return Err(CampaignError::Subject(
            "gate producer capture or receipt is inconsistent".into(),
        ));
    }
    let paths = obj(&capture["tested_path_sha256"], "tested paths")?
        .iter()
        .map(|(path, hash)| {
            Ok((
                path.clone(),
                hash.as_str()
                    .ok_or_else(|| CampaignError::Subject("gate path hash invalid".into()))?
                    .to_owned(),
            ))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    for hash in paths.values() {
        require_sha(hash, "gate path SHA").map_err(|e| CampaignError::Subject(e.to_string()))?;
    }
    Ok(VerifiedGate {
        capture_digest,
        receipt_digest,
        tested_tree: str_at(&capture, "frozen_gate_tree_oid")?.into(),
        paths,
    })
}

#[allow(clippy::too_many_arguments)] // Exact historical artifact and gate evidence boundaries are separate raw inputs.
pub fn verify_existing_subject(
    repository: &Path,
    identity: &CampaignIdentity,
    baseline: &Baseline,
    candidate_raw: &[u8],
    profile_raw: &[u8],
    gate_capture_raw: &[u8],
    gate_manifest_raw: &[u8],
    gate_receipt_raw: &[u8],
) -> Result<VerifiedCandidate> {
    anchored_repository(repository)?;
    let gate = verify_gate(gate_capture_raw, gate_manifest_raw, gate_receipt_raw)?;
    verify_existing_subject_with_gate(
        repository,
        identity,
        baseline,
        candidate_raw,
        profile_raw,
        gate,
    )
}

#[allow(clippy::too_many_arguments)] // Exact successor receipt, profile and gate evidence are separate raw inputs.
pub fn verify_existing_subject_dynamic(
    repository: &Path,
    identity: &CampaignIdentity,
    baseline: &Baseline,
    candidate_raw: &[u8],
    profile_raw: &[u8],
    gate_capture_raw: &[u8],
    gate_manifest_raw: &[u8],
    gate_receipt_raw: &[u8],
) -> Result<VerifiedCandidate> {
    anchored_repository(repository)?;
    let gate = verify_gate_dynamic(gate_capture_raw, gate_manifest_raw, gate_receipt_raw)?;
    verify_existing_subject_with_gate(
        repository,
        identity,
        baseline,
        candidate_raw,
        profile_raw,
        gate,
    )
}

fn verify_existing_subject_with_gate(
    repository: &Path,
    identity: &CampaignIdentity,
    baseline: &Baseline,
    candidate_raw: &[u8],
    profile_raw: &[u8],
    gate: VerifiedGate,
) -> Result<VerifiedCandidate> {
    let candidate = parse(candidate_raw, "candidate receipt")?;
    let profile = parse(profile_raw, "physical profile")?;
    verify_candidate(repository, identity, baseline, &candidate, &gate)?;
    verify_profile(&candidate, &profile)?;
    let candidate_ref = CandidateRef {
        commit: str_at(&candidate, "checkpoint_commit_oid")?.into(),
        tree: str_at(&candidate, "checkpoint_tree_oid")?.into(),
        patch_identity: str_at(&candidate, "task_patch_digest")?.into(),
        receipt_sha256: raw_sha256(candidate_raw),
        physical_profile_sha256: raw_sha256(profile_raw),
    };
    let profile_digest = str_at(&profile, "profile_digest")?.to_owned();
    Ok(VerifiedCandidate {
        reference: candidate_ref,
        candidate_receipt: candidate,
        physical_profile: profile,
        profile_digest,
        gate,
    })
}

fn verify_candidate(
    repository: &Path,
    identity: &CampaignIdentity,
    baseline: &Baseline,
    candidate: &Value,
    gate: &VerifiedGate,
) -> Result<()> {
    exact(
        candidate,
        &[
            "schema_version",
            "checkpoint_id",
            "checkpoint_kind",
            "parent_checkpoint_commit_oid",
            "baseline_commit_oid",
            "baseline_tree_oid",
            "checkpoint_commit_oid",
            "checkpoint_tree_oid",
            "task_patch_digest",
            "manifest_digest",
            "gate_receipt_digest",
            "run_id",
            "slice_number",
            "candidate_attempt",
            "plan_version",
            "scope_revision",
            "paths",
            "ref",
            "repository",
            "created_at",
            "limitations",
        ],
        "candidate",
    )?;
    if candidate["schema_version"] != 1
        || candidate["checkpoint_kind"] != "candidate"
        || candidate["run_id"] != identity.run_id
        || candidate["slice_number"] != identity.slice_number
        || candidate["plan_version"] != identity.plan_version
        || candidate["repository"] != repository.to_str().unwrap_or("")
        || candidate["baseline_commit_oid"] != baseline.accepted_commit
        || candidate["baseline_tree_oid"] != baseline.accepted_tree
        || candidate["parent_checkpoint_commit_oid"] != baseline.inter_slice_commit
        || candidate["gate_receipt_digest"] != gate.receipt_digest
        || candidate["checkpoint_tree_oid"] != gate.tested_tree
    {
        return Err(CampaignError::Subject(
            "candidate differs from admitted campaign or tested tree".into(),
        ));
    }
    if !string_array(&candidate["limitations"])
        || candidate["created_at"].as_str().is_none_or(str::is_empty)
    {
        return Err(CampaignError::Subject(
            "candidate limitations or evidence cutoff invalid".into(),
        ));
    }
    let attempt = candidate["candidate_attempt"]
        .as_u64()
        .filter(|n| *n > 0)
        .ok_or_else(|| CampaignError::Subject("candidate attempt invalid".into()))?;
    let expected_ref = format!(
        "refs/work-engine/checkpoints/{}/slice-{}/candidate-{attempt}",
        identity.run_id, identity.slice_number
    );
    if candidate["ref"] != expected_ref {
        return Err(CampaignError::Subject(
            "candidate private ref differs".into(),
        ));
    }
    let commit = str_at(candidate, "checkpoint_commit_oid")?;
    let tree = str_at(candidate, "checkpoint_tree_oid")?;
    let baseline_commit = str_at(candidate, "baseline_commit_oid")?;
    let parent = str_at(candidate, "parent_checkpoint_commit_oid")?;
    for oid in [commit, tree, baseline_commit, parent] {
        full_oid(oid)?;
    }
    for oid in [commit, baseline_commit, parent] {
        let result = git_text(
            repository,
            &[
                "rev-parse",
                "--verify",
                "--quiet",
                &format!("{oid}^{{commit}}"),
            ],
            128,
        )?;
        if result != oid {
            return Err(CampaignError::Subject("Git commit OID differs".into()));
        }
    }
    if git_text(
        repository,
        &["rev-parse", "--verify", "--quiet", &expected_ref],
        128,
    )? != commit
    {
        return Err(CampaignError::Subject("candidate ref differs".into()));
    }
    let baseline_tree = git_text(
        repository,
        &["rev-parse", &format!("{baseline_commit}^{{tree}}")],
        128,
    )?;
    if baseline_tree != baseline.accepted_tree {
        return Err(CampaignError::Subject("baseline tree differs".into()));
    }
    let commit_bytes = git(repository, &["cat-file", "-p", commit], 65_536)?;
    let commit_text =
        String::from_utf8(commit_bytes).map_err(|e| CampaignError::Subject(e.to_string()))?;
    let (header, message) = commit_text
        .split_once("\n\n")
        .ok_or_else(|| CampaignError::Subject("commit has no message".into()))?;
    let trees: Vec<_> = header
        .lines()
        .filter_map(|line| line.strip_prefix("tree "))
        .collect();
    let parents: Vec<_> = header
        .lines()
        .filter_map(|line| line.strip_prefix("parent "))
        .collect();
    if trees != [tree] || parents != [parent] {
        return Err(CampaignError::Subject(
            "candidate commit tree or parent differs".into(),
        ));
    }
    let metadata: Value = serde_json::from_str(message.trim_end_matches('\n'))
        .map_err(|e| CampaignError::Subject(e.to_string()))?;
    let expected = json!({"work_engine_checkpoint":1,"kind":"candidate","request_id":candidate["checkpoint_id"],"run_id":candidate["run_id"],"slice_number":candidate["slice_number"],"candidate_attempt":candidate["candidate_attempt"],"tree":tree,"task_patch_digest":candidate["task_patch_digest"],"manifest_digest":candidate["manifest_digest"],"gate_receipt_digest":candidate["gate_receipt_digest"],"plan_version":candidate["plan_version"],"scope_revision":candidate["scope_revision"]});
    if metadata != expected {
        return Err(CampaignError::Subject(
            "immutable candidate metadata differs".into(),
        ));
    }
    let paths = array(&candidate["paths"], "candidate paths")?;
    if paths.is_empty() || historical_digest(&candidate["paths"])? != candidate["manifest_digest"] {
        return Err(CampaignError::Subject(
            "candidate path manifest digest differs".into(),
        ));
    }
    let mut path_map = BTreeMap::new();
    for entry in paths {
        exact(
            entry,
            &["path", "action", "attribution", "content_digest"],
            "candidate path",
        )?;
        let path = str_at(entry, "path")?;
        if path.starts_with('/')
            || path
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
            || entry["action"] != "include"
            || entry["attribution"] != "task_owned"
        {
            return Err(CampaignError::Subject(
                "unsupported candidate path attribution".into(),
            ));
        }
        let digest = str_at(entry, "content_digest")?;
        require_sha(digest, "path digest").map_err(|e| CampaignError::Subject(e.to_string()))?;
        if path_map
            .insert(path.to_owned(), digest.to_owned())
            .is_some()
        {
            return Err(CampaignError::Subject("duplicate candidate path".into()));
        }
        let bytes = git(repository, &["show", &format!("{tree}:{path}")], 1_048_576)?;
        if raw_sha256(&bytes) != digest {
            return Err(CampaignError::Subject(
                "candidate tree path bytes differ".into(),
            ));
        }
    }
    if path_map != gate.paths {
        return Err(CampaignError::Subject(
            "candidate paths differ from tested gate paths".into(),
        ));
    }
    let diff_paths = git_text(
        repository,
        &[
            "diff-tree",
            "--no-commit-id",
            "--name-only",
            "-r",
            "--no-renames",
            &baseline_tree,
            tree,
        ],
        65_536,
    )?;
    let names: BTreeSet<_> = diff_paths.lines().map(str::to_owned).collect();
    if names != path_map.keys().cloned().collect() {
        return Err(CampaignError::Subject(
            "candidate diff path scope differs".into(),
        ));
    }
    let patch = git(
        repository,
        &[
            "diff-tree",
            "--binary",
            "--no-renames",
            "--no-ext-diff",
            &baseline_tree,
            tree,
        ],
        4_194_304,
    )?;
    if raw_sha256(&patch) != candidate["task_patch_digest"] {
        return Err(CampaignError::Subject(
            "candidate task patch differs".into(),
        ));
    }
    Ok(())
}

fn verify_profile(candidate: &Value, profile: &Value) -> Result<()> {
    exact(
        profile,
        &[
            "schema_version",
            "analyzer",
            "subject",
            "subject_digest",
            "observations",
            "coverage",
            "limitations",
            "provenance",
            "profile_digest",
        ],
        "profile",
    )?;
    if profile["schema_version"] != 2
        || profile["analyzer"]
            != json!({"name":"work-engine-deterministic-baseline","version":"2","source_sha256":ANALYZER_SOURCE_SHA256,"checkpoint_validator_sha256":CHECKPOINT_SOURCE_SHA256})
    {
        return Err(CampaignError::Subject(
            "profile analyzer differs from trusted producer".into(),
        ));
    }
    let subject = &profile["subject"];
    exact(
        subject,
        &[
            "schema_version",
            "construction_method",
            "evidence_cutoff",
            "repository",
            "checkpoint",
            "limitations",
        ],
        "profile subject",
    )?;
    let mut checkpoint = candidate.clone();
    checkpoint.as_object_mut().unwrap().remove("repository");
    checkpoint.as_object_mut().unwrap().remove("limitations");
    if subject["schema_version"] != 2
        || subject["construction_method"] != "slice_checkpoint_candidate_receipt"
        || subject["checkpoint"] != checkpoint
        || subject["repository"] != candidate["repository"]
        || subject["evidence_cutoff"] != candidate["created_at"]
    {
        return Err(CampaignError::Subject(
            "profile subject differs from candidate".into(),
        ));
    }
    if !string_array(&subject["limitations"])
        || subject["limitations"] != candidate["limitations"]
        || subject["repository"].as_str().is_none_or(str::is_empty)
        || subject["evidence_cutoff"]
            .as_str()
            .is_none_or(str::is_empty)
    {
        return Err(CampaignError::Subject(
            "derived subject field types or limitations differ".into(),
        ));
    }
    if historical_digest(subject)? != profile["subject_digest"] {
        return Err(CampaignError::Subject(
            "profile subject digest differs".into(),
        ));
    }
    let provenance = &profile["provenance"];
    exact(
        provenance,
        &["producer", "derivation_sources"],
        "profile provenance",
    )?;
    if provenance["producer"] != profile["analyzer"] {
        return Err(CampaignError::Subject("profile producer differs".into()));
    }
    let sources = &provenance["derivation_sources"];
    exact(
        sources,
        &[
            "checkpoint_subject",
            "repository_trees",
            "checkpoint_validator",
            "git_runtime",
            "python_runtime",
            "structural_graph",
            "invariant_catalog",
            "classifier",
        ],
        "derivation sources",
    )?;
    if sources["checkpoint_subject"]
        != json!({"use_state":"used","identity":{"subject_digest":profile["subject_digest"]}})
        || sources["repository_trees"]
            != json!({"use_state":"used","identity":{"baseline_tree_oid":candidate["baseline_tree_oid"],"result_tree_oid":candidate["checkpoint_tree_oid"],"task_patch_digest":candidate["task_patch_digest"]}})
        || sources["checkpoint_validator"]
            != json!({"use_state":"used","identity":{"sha256":CHECKPOINT_SOURCE_SHA256}})
        || sources["git_runtime"]
            != json!({"use_state":"used","identity":{"executable_sha256":GIT_EXE_SHA256,"version":"git version 2.53.0"}})
        || sources["python_runtime"]
            != json!({"use_state":"used","identity":{"executable_sha256":PYTHON_EXE_SHA256,"implementation":"CPython","version":"3.14.4"}})
    {
        return Err(CampaignError::Subject("profile provenance differs".into()));
    }
    for key in ["structural_graph", "invariant_catalog", "classifier"] {
        if sources[key] != json!({"use_state":"not_used","reason":"deferred_by_profile_scope"}) {
            return Err(CampaignError::Subject(
                "unsupported profile derivation source".into(),
            ));
        }
    }
    let mut unsigned = profile.clone();
    unsigned.as_object_mut().unwrap().remove("profile_digest");
    if historical_digest(&unsigned)? != profile["profile_digest"] {
        return Err(CampaignError::Subject("profile digest differs".into()));
    }
    verify_observations(
        &profile["observations"],
        &profile["coverage"],
        &candidate["paths"],
    )?;
    if !string_array(&profile["limitations"]) {
        return Err(CampaignError::Subject("profile limitations invalid".into()));
    }
    Ok(())
}

fn observed<'a>(observations: &'a Value, key: &str) -> Result<&'a Value> {
    let item = &observations[key];
    exact(item, &["state", "value"], key)?;
    if item["state"] != "observed" {
        return Err(CampaignError::Subject(format!("{key} is not observed")));
    }
    Ok(&item["value"])
}
fn nonnegative(value: &Value) -> bool {
    value.as_u64().is_some()
}
fn string_array(value: &Value) -> bool {
    value
        .as_array()
        .is_some_and(|items| items.iter().all(Value::is_string))
}
fn verify_observations(observations: &Value, coverage: &Value, manifest: &Value) -> Result<()> {
    exact(
        observations,
        &[
            "files",
            "file_count",
            "line_totals",
            "hunk_count",
            "file_categories",
            "test_file_count",
            "documentation_file_count",
            "configuration_file_count",
            "changed_symbols",
            "module_distribution",
        ],
        "observations",
    )?;
    let files = array(observed(observations, "files")?, "files")?;
    let paths = array(manifest, "manifest")?
        .iter()
        .map(|entry| str_at(entry, "path").map(str::to_owned))
        .collect::<Result<Vec<_>>>()?;
    let mut sorted = paths.clone();
    sorted.sort();
    if files
        .iter()
        .map(|row| str_at(row, "path").map(str::to_owned))
        .collect::<Result<Vec<_>>>()?
        != sorted
    {
        return Err(CampaignError::Subject("profile file paths differ".into()));
    }
    let mut categories = BTreeMap::<String, u64>::new();
    let mut modules = BTreeMap::<String, u64>::new();
    let binary = files.iter().any(|row| row["binary"] == Value::Bool(true));
    let mut additions = 0u64;
    let mut deletions = 0u64;
    for row in files {
        exact(
            row,
            &["path", "category", "additions", "deletions", "binary"],
            "profile file",
        )?;
        let category = str_at(row, "category")?;
        if !["test", "documentation", "configuration", "source", "other"].contains(&category) {
            return Err(CampaignError::Subject("file category invalid".into()));
        }
        let is_binary = row["binary"]
            .as_bool()
            .ok_or_else(|| CampaignError::Subject("binary flag invalid".into()))?;
        if is_binary {
            if !row["additions"].is_null() || !row["deletions"].is_null() {
                return Err(CampaignError::Subject(
                    "binary line count fabricated".into(),
                ));
            }
        } else {
            if !nonnegative(&row["additions"]) || !nonnegative(&row["deletions"]) {
                return Err(CampaignError::Subject("file line count invalid".into()));
            }
            if !binary {
                additions = additions
                    .checked_add(row["additions"].as_u64().unwrap())
                    .ok_or_else(|| {
                        CampaignError::Subject("text additions exceed supported total".into())
                    })?;
                deletions = deletions
                    .checked_add(row["deletions"].as_u64().unwrap())
                    .ok_or_else(|| {
                        CampaignError::Subject("text deletions exceed supported total".into())
                    })?;
            }
        }
        *categories.entry(category.into()).or_default() += 1;
        let path = str_at(row, "path")?;
        let module = path.split_once('/').map(|(part, _)| part).unwrap_or(".");
        *modules.entry(module.into()).or_default() += 1;
    }
    if observed(observations, "file_count")? != files.len()
        || !nonnegative(observed(observations, "hunk_count")?)
    {
        return Err(CampaignError::Subject("file or hunk count invalid".into()));
    }
    for (key, category) in [
        ("test_file_count", "test"),
        ("documentation_file_count", "documentation"),
        ("configuration_file_count", "configuration"),
    ] {
        if observed(observations, key)? != categories.get(category).copied().unwrap_or(0) {
            return Err(CampaignError::Subject(format!("{key} differs")));
        }
    }
    if observed(observations, "file_categories")? != &json!(categories)
        || observed(observations, "module_distribution")? != &json!(modules)
    {
        return Err(CampaignError::Subject(
            "profile category/module counts differ".into(),
        ));
    }
    let totals = &observations["line_totals"];
    if binary {
        if totals
            != &json!({"state":"unsupported","reason":"binary diff has no textual line totals"})
        {
            return Err(CampaignError::Subject("binary line totals invalid".into()));
        }
    } else if totals
        != &json!({"state":"observed","value":{"additions":additions,"deletions":deletions}})
    {
        return Err(CampaignError::Subject("text line totals invalid".into()));
    }
    let symbols = array(
        observed(observations, "changed_symbols")?,
        "changed symbols",
    )?;
    if symbols.len() != sorted.len() {
        return Err(CampaignError::Subject("symbol path count differs".into()));
    }
    for (entry, path) in symbols.iter().zip(sorted.iter()) {
        exact(entry, &["path", "measurement"], "symbol entry")?;
        if entry["path"] != *path {
            return Err(CampaignError::Subject("symbol path differs".into()));
        }
        let measure = &entry["measurement"];
        let state = str_at(measure, "state")?;
        match state {
            "observed" => {
                exact(measure, &["state", "value"], "symbol measure")?;
                for change in array(&measure["value"], "symbol changes")? {
                    exact(change, &["name", "change"], "symbol change")?;
                    if str_at(change, "name")?.is_empty()
                        || !["added", "deleted", "modified"].contains(&str_at(change, "change")?)
                    {
                        return Err(CampaignError::Subject("symbol value invalid".into()));
                    }
                }
            }
            "unsupported" | "failed" => {
                exact(measure, &["state", "reason"], "symbol measure")?;
                if str_at(measure, "reason")?.is_empty() {
                    return Err(CampaignError::Subject("symbol reason empty".into()));
                }
            }
            _ => return Err(CampaignError::Subject("symbol state invalid".into())),
        }
    }
    if coverage
        != &json!({"manifest_paths":sorted.len(),"profiled_paths":files.len(),"path_scope":"attributed checkpoint manifest"})
    {
        return Err(CampaignError::Subject("coverage differs".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn fixture(file: &str) -> Value {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(file);
        serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
    }

    #[test]
    fn sc0_profile_and_observation_vectors() {
        let handoff = fixture("subject-handoff.json");
        let base_candidate = fixture(
            "vectors/4a2c4f1872f6dfe8de275181f08b1e4d8c33a437c2519cd8b9ce8b3c9b56a5cd.json",
        );
        for item in handoff["vectors"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|item| item["kind"] == "profile")
        {
            let profile = fixture(item["artifact"]["file"].as_str().unwrap());
            let mut candidate = profile["subject"]["checkpoint"].clone();
            if let Some(candidate_map) = candidate.as_object_mut() {
                candidate_map.insert(
                    "repository".into(),
                    profile["subject"]["repository"].clone(),
                );
                candidate_map.insert("limitations".into(), json!([]));
            }
            if item["case"].as_str().unwrap().contains("cross_binding") {
                candidate = base_candidate.clone();
            }
            let accepted = verify_profile(&candidate, &profile).is_ok();
            assert_eq!(
                accepted,
                item["sc1_supported_profile_expected_accept"]
                    .as_bool()
                    .unwrap(),
                "{}",
                item["case"]
            );
        }
    }
}
