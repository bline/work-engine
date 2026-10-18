//! Exact, read-only Git source for the S5 independent-review contract revision.
//! The fixed OIDs are case admission, not selectors supplied by a caller.

use super::{Result, hex, sha, string};
use serde_json::{Value, json};
use sha1::Sha1;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const COMMIT: &str = "cc4b644ff102c3eb8fa46de01ff6a73fa171d575";
const PARENT: &str = "2d99c5b63e983d9e6e0c9e5e2fc4a436eb21e6aa";
const TREE: &str = "f532363bab18f65d6b0381fb4dc1ebeb61ccc591";
const SCRIPT_PATH: &str = "skills/independent-review-state/scripts/independent_review_state.py";
const SCRIPT_BEFORE: &str = "55f1854fc1beea4c90550c9c31dde8333d7907b7";
const SCRIPT_AFTER: &str = "da91b16e8e20edc04f60d8d4d5cbfa8e8567d2e1";
const SCRIPT_BEFORE_SHA: &str = "42892a365846770a3eb604091460fd2112217f33d54f91b99b250e42fcbf96cc";
const SCRIPT_AFTER_SHA: &str = "44968e3ac7b4be38758a654c646969c804865deb107409c7abdb1bfb6496bdf4";
const SKILL_PATH: &str = "skills/independent-review-state/SKILL.md";
const SKILL_BEFORE: &str = "f765e7e6b71e529e2266db2eee12d055412c3f3c";
const SKILL_AFTER: &str = "79ffec8899892dee1906ff27bf7f5b3b83d1b6b4";
const SKILL_BEFORE_SHA: &str = "661fa7a945d84a78c020c1da4ef2f14d336effc1ad9406a02f05630ead9de652";
const SKILL_AFTER_SHA: &str = "f786fefdec815522958dfdec2105e36e30bdea27e2e9910d0e7ea5fb8c7aca5f";
const GIT_OUTPUT_CAP: usize = 1_100_000;
const GIT_OBJECT_CAP: usize = 1_048_576;

fn validate_descriptor(grant: &Value) -> Result<()> {
    let descriptor = grant
        .get("repair_contract")
        .and_then(Value::as_object)
        .ok_or("repair contract descriptor missing")?;
    let expected = [
        ("commit_oid", COMMIT),
        ("parent_oid", PARENT),
        ("tree_oid", TREE),
        ("script_path", SCRIPT_PATH),
        ("script_before_blob_oid", SCRIPT_BEFORE),
        ("script_after_blob_oid", SCRIPT_AFTER),
        ("script_before_sha256", SCRIPT_BEFORE_SHA),
        ("script_after_sha256", SCRIPT_AFTER_SHA),
        ("skill_path", SKILL_PATH),
        ("skill_before_blob_oid", SKILL_BEFORE),
        ("skill_after_blob_oid", SKILL_AFTER),
        ("skill_before_sha256", SKILL_BEFORE_SHA),
        ("skill_after_sha256", SKILL_AFTER_SHA),
    ];
    if descriptor.len() != expected.len()
        || expected
            .iter()
            .any(|(key, value)| descriptor.get(*key).and_then(Value::as_str) != Some(*value))
    {
        return Err("repair contract descriptor differs from fixed S5 source".into());
    }
    Ok(())
}

fn bounded_git(
    repository: &Path,
    args: &[&str],
    input: Option<&[u8]>,
    deadline: Instant,
) -> Result<Vec<u8>> {
    if Instant::now() >= deadline {
        return Err("repair Git read deadline expired".into());
    }
    let mut child = Command::new("git")
        .arg("-C")
        .arg(repository)
        .args(args)
        .env("GIT_NO_REPLACE_OBJECTS", "1")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let stdout = child.stdout.take().ok_or("repair Git stdout absent")?;
    let stderr = child.stderr.take().ok_or("repair Git stderr absent")?;
    let out_reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout
            .take((GIT_OUTPUT_CAP + 1) as u64)
            .read_to_end(&mut bytes)
            .map(|_| bytes)
    });
    let err_reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.take(8193).read_to_end(&mut bytes).map(|_| bytes)
    });
    if let Some(bytes) = input {
        let mut stdin = child.stdin.take().ok_or("repair Git stdin absent")?;
        if let Err(err) = stdin.write_all(bytes) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(err.to_string());
        }
    }
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err("repair Git read deadline expired".into());
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    let out = out_reader
        .join()
        .map_err(|_| "repair Git reader panicked")?
        .map_err(|e| e.to_string())?;
    let err = err_reader
        .join()
        .map_err(|_| "repair Git error reader panicked")?
        .map_err(|e| e.to_string())?;
    if !status.success() || out.len() > GIT_OUTPUT_CAP || !err.is_empty() {
        return Err("bounded repair Git source read failed".into());
    }
    Ok(out)
}

fn object(
    repository: &Path,
    oid: &str,
    kind: &str,
    format: &str,
    deadline: Instant,
) -> Result<Vec<u8>> {
    let oid_bytes = match format {
        "sha1" => 20,
        "sha256" => 32,
        _ => return Err("Git object format unsupported".into()),
    };
    if oid.len() != oid_bytes * 2
        || !oid
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err("repair Git OID invalid".into());
    }
    let response = bounded_git(
        repository,
        &["cat-file", "--batch"],
        Some(format!("{oid}\n").as_bytes()),
        deadline,
    )?;
    let newline = response
        .iter()
        .position(|b| *b == b'\n')
        .ok_or("repair Git batch header missing")?;
    let header = std::str::from_utf8(&response[..newline]).map_err(|e| e.to_string())?;
    let mut fields = header.split(' ');
    if fields.next() != Some(oid) || fields.next() != Some(kind) {
        return Err("repair Git object type or identity differs".into());
    }
    let size = fields
        .next()
        .ok_or("repair Git object size absent")?
        .parse::<usize>()
        .map_err(|e| e.to_string())?;
    if fields.next().is_some()
        || size > GIT_OBJECT_CAP
        || response.len() != newline + 1 + size + 1
        || response.last() != Some(&b'\n')
    {
        return Err("repair Git object size or frame invalid".into());
    }
    let bytes = response[newline + 1..newline + 1 + size].to_vec();
    let mut framed = format!("{kind} {size}\0").into_bytes();
    framed.extend_from_slice(&bytes);
    let actual = match format {
        "sha1" => hex(&Sha1::digest(&framed)),
        "sha256" => hex(&Sha256::digest(&framed)),
        _ => unreachable!(),
    };
    if actual != oid {
        return Err("raw repair Git object identity differs".into());
    }
    Ok(bytes)
}

fn commit_tree<'a>(commit: &'a [u8], expected_parent: Option<&str>) -> Result<&'a str> {
    let header = std::str::from_utf8(commit)
        .map_err(|e| e.to_string())?
        .split_once("\n\n")
        .ok_or("repair Git commit header invalid")?
        .0;
    let mut trees = header.lines().filter_map(|line| line.strip_prefix("tree "));
    let tree = trees.next().ok_or("repair Git commit tree absent")?;
    if trees.next().is_some() {
        return Err("repair Git commit tree repeated".into());
    }
    if let Some(expected) = expected_parent {
        let mut parents = header
            .lines()
            .filter_map(|line| line.strip_prefix("parent "));
        if parents.next() != Some(expected) || parents.next().is_some() {
            return Err("repair Git commit parent differs".into());
        }
    }
    Ok(tree)
}

fn tree_entry(tree: &[u8], name: &str, format: &str) -> Result<(String, String)> {
    let oid_len = if format == "sha1" { 20 } else { 32 };
    let mut rest = tree;
    let mut found = None;
    while !rest.is_empty() {
        let zero = rest
            .iter()
            .position(|b| *b == 0)
            .ok_or("repair Git tree entry invalid")?;
        let header = std::str::from_utf8(&rest[..zero]).map_err(|e| e.to_string())?;
        let (mode, entry_name) = header
            .split_once(' ')
            .ok_or("repair Git tree mode absent")?;
        if rest.len() < zero + 1 + oid_len {
            return Err("repair Git tree OID truncated".into());
        }
        if entry_name == name {
            if found.is_some() {
                return Err("repair Git tree path duplicated".into());
            }
            found = Some((mode.to_string(), hex(&rest[zero + 1..zero + 1 + oid_len])));
        }
        rest = &rest[zero + 1 + oid_len..];
    }
    found.ok_or_else(|| "repair Git tree path absent".into())
}

fn path_blob(
    repository: &Path,
    root_oid: &str,
    path: &str,
    expected_blob: &str,
    expected_sha: &str,
    format: &str,
    deadline: Instant,
) -> Result<()> {
    let mut oid = root_oid.to_string();
    let mut parts = path.split('/').peekable();
    while let Some(part) = parts.next() {
        let tree = object(repository, &oid, "tree", format, deadline)?;
        let (mode, next_oid) = tree_entry(&tree, part, format)?;
        if parts.peek().is_some() {
            if mode != "40000" {
                return Err("repair Git path is not a tree".into());
            }
        } else {
            if mode != "100644" || next_oid != expected_blob {
                return Err("repair Git blob path, mode or identity differs".into());
            }
            let blob = object(repository, &next_oid, "blob", format, deadline)?;
            if sha(&blob) != expected_sha {
                return Err("repair Git blob digest differs".into());
            }
            return Ok(());
        }
        oid = next_oid;
    }
    Err("repair Git path empty".into())
}

pub fn inspect(grant: &Value) -> Result<Value> {
    validate_descriptor(grant)?;
    let repository = Path::new(string(grant, "repository")?);
    if !repository.is_absolute() {
        return Err("repair Git repository is not absolute".into());
    }
    let deadline = Instant::now() + Duration::from_secs(25);
    let format_bytes = bounded_git(
        repository,
        &["rev-parse", "--show-object-format=storage"],
        None,
        deadline,
    )?;
    let format = std::str::from_utf8(&format_bytes)
        .map_err(|e| e.to_string())?
        .trim();
    if format != "sha1" && format != "sha256" {
        return Err("Git object format unsupported".into());
    }
    let child = object(repository, COMMIT, "commit", format, deadline)?;
    let root = commit_tree(&child, Some(PARENT))?;
    if root != TREE {
        return Err("repair Git commit tree differs".into());
    }
    let parent = object(repository, PARENT, "commit", format, deadline)?;
    let parent_root = commit_tree(&parent, None)?;
    path_blob(
        repository,
        parent_root,
        SCRIPT_PATH,
        SCRIPT_BEFORE,
        SCRIPT_BEFORE_SHA,
        format,
        deadline,
    )?;
    path_blob(
        repository,
        root,
        SCRIPT_PATH,
        SCRIPT_AFTER,
        SCRIPT_AFTER_SHA,
        format,
        deadline,
    )?;
    path_blob(
        repository,
        parent_root,
        SKILL_PATH,
        SKILL_BEFORE,
        SKILL_BEFORE_SHA,
        format,
        deadline,
    )?;
    path_blob(
        repository,
        root,
        SKILL_PATH,
        SKILL_AFTER,
        SKILL_AFTER_SHA,
        format,
        deadline,
    )?;
    Ok(json!({
        "source_owner":"independent-review-state",
        "source_class":"immutable_contract_source_revision",
        "confidentiality":"private_local",
        "completeness":"selected_commit_and_parent_only",
        "commit_oid":COMMIT,
        "parent_oid":PARENT,
        "tree_oid":TREE,
        "script_path":SCRIPT_PATH,
        "script_before_blob_oid":SCRIPT_BEFORE,
        "script_after_blob_oid":SCRIPT_AFTER,
        "script_before_sha256":SCRIPT_BEFORE_SHA,
        "script_after_sha256":SCRIPT_AFTER_SHA,
        "skill_path":SKILL_PATH,
        "skill_before_blob_oid":SKILL_BEFORE,
        "skill_after_blob_oid":SKILL_AFTER,
        "skill_before_sha256":SKILL_BEFORE_SHA,
        "skill_after_sha256":SKILL_AFTER_SHA,
        "before_rule":"remediation_subject_requires_remediation_phase",
        "after_rule":"active_reported_or_remediation_can_record_exact_later_remediation_subject;reported_is_not_retirement",
        "source_limit":"Immutable source-contract change; no S5 deployment, use, replacement cause or savings proof"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn descriptor(repository: &Path) -> Value {
        json!({
            "repository":repository,
            "repair_contract":{
                "commit_oid":COMMIT,"parent_oid":PARENT,"tree_oid":TREE,
                "script_path":SCRIPT_PATH,
                "script_before_blob_oid":SCRIPT_BEFORE,"script_after_blob_oid":SCRIPT_AFTER,
                "script_before_sha256":SCRIPT_BEFORE_SHA,"script_after_sha256":SCRIPT_AFTER_SHA,
                "skill_path":SKILL_PATH,
                "skill_before_blob_oid":SKILL_BEFORE,"skill_after_blob_oid":SKILL_AFTER,
                "skill_before_sha256":SKILL_BEFORE_SHA,"skill_after_sha256":SKILL_AFTER_SHA
            }
        })
    }

    #[test]
    fn exact_source_is_admitted_without_raw_script_and_descriptor_is_closed() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let grant = descriptor(&repo);
        let admitted = inspect(&grant).expect("fixed historical owner bytes");
        assert_eq!(admitted["commit_oid"], COMMIT);
        assert_eq!(
            admitted["after_rule"],
            "active_reported_or_remediation_can_record_exact_later_remediation_subject;reported_is_not_retirement"
        );
        assert!(admitted.get("script").is_none());

        for key in [
            "commit_oid",
            "parent_oid",
            "tree_oid",
            "script_path",
            "script_before_blob_oid",
            "script_after_sha256",
            "skill_path",
            "skill_after_blob_oid",
            "skill_before_sha256",
        ] {
            let mut wrong = grant.clone();
            wrong["repair_contract"][key] = json!("wrong");
            assert!(inspect(&wrong).is_err(), "accepted altered {key}");
        }
        let mut extra = grant.clone();
        extra["repair_contract"]["extra"] = json!("no");
        assert!(inspect(&extra).is_err());
        let mut missing = grant.clone();
        missing["repair_contract"]
            .as_object_mut()
            .unwrap()
            .remove("skill_path");
        assert!(inspect(&missing).is_err());
    }

    fn git(repo: &Path, args: &[&str], input: Option<&[u8]>) -> Vec<u8> {
        let mut child = Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(args)
            .stdin(if input.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        if let Some(bytes) = input {
            child.stdin.take().unwrap().write_all(bytes).unwrap();
        }
        let out = child.wait_with_output().unwrap();
        assert!(
            out.status.success(),
            "Git fixture command failed: {:?}",
            String::from_utf8_lossy(&out.stderr)
        );
        out.stdout
    }

    #[test]
    fn raw_object_formats_replace_fence_and_tree_modes() {
        let base = Path::new(
            "/home/bline/.local/state/work-engine/ui0-s5-repair-contract-20261018/tests/repair-contract",
        );
        fs::create_dir_all(base).unwrap();
        for format in ["sha1", "sha256"] {
            let repo = base.join(format!(
                "{}-{}-{}",
                format,
                std::process::id(),
                std::thread::current().name().unwrap_or("test")
            ));
            fs::create_dir(&repo).unwrap();
            git(
                &repo,
                &["init", "--quiet", &format!("--object-format={format}")],
                None,
            );
            let original = b"original owner bytes\n";
            let replacement = b"replacement owner bytes\n";
            let oid = String::from_utf8(git(
                &repo,
                &["hash-object", "-w", "--stdin"],
                Some(original),
            ))
            .unwrap()
            .trim()
            .to_string();
            let replacement_oid = String::from_utf8(git(
                &repo,
                &["hash-object", "-w", "--stdin"],
                Some(replacement),
            ))
            .unwrap()
            .trim()
            .to_string();
            git(
                &repo,
                &[
                    "update-ref",
                    &format!("refs/replace/{oid}"),
                    &replacement_oid,
                ],
                None,
            );
            let deadline = Instant::now() + Duration::from_secs(10);
            assert_eq!(
                object(&repo, &oid, "blob", format, deadline).unwrap(),
                original
            );
            assert!(object(&repo, &oid, "tree", format, deadline).is_err());
            assert!(object(&repo, &"0".repeat(oid.len()), "blob", format, deadline).is_err());
            let tree_input = format!("100644 blob {oid}\tsample.txt\n");
            let root = String::from_utf8(git(&repo, &["mktree"], Some(tree_input.as_bytes())))
                .unwrap()
                .trim()
                .to_string();
            path_blob(
                &repo,
                &root,
                "sample.txt",
                &oid,
                &sha(original),
                format,
                deadline,
            )
            .unwrap();
            assert!(
                path_blob(
                    &repo,
                    &root,
                    "absent.txt",
                    &oid,
                    &sha(original),
                    format,
                    deadline
                )
                .is_err()
            );
            assert!(
                path_blob(
                    &repo,
                    &root,
                    "sample.txt",
                    &replacement_oid,
                    &sha(original),
                    format,
                    deadline
                )
                .is_err()
            );
            assert!(
                path_blob(
                    &repo,
                    &root,
                    "sample.txt",
                    &oid,
                    &sha(replacement),
                    format,
                    deadline
                )
                .is_err()
            );
            let executable = format!("100755 blob {oid}\tsample.txt\n");
            let executable_root =
                String::from_utf8(git(&repo, &["mktree"], Some(executable.as_bytes())))
                    .unwrap()
                    .trim()
                    .to_string();
            assert!(
                path_blob(
                    &repo,
                    &executable_root,
                    "sample.txt",
                    &oid,
                    &sha(original),
                    format,
                    deadline
                )
                .is_err()
            );
            fs::remove_dir_all(repo).unwrap();
        }
    }
}
