//! Subject materialization (operator decision D3/OQ-2, 2026-11-17): only the exact frozen subject
//! commit's diff and changed files, from git objects, with verified commit/tree/patch identity,
//! the requirements-to-subject binding, input-size enforcement and the secret rule set.
//!
//! `git` is invoked for read-only object queries only (`rev-parse`, `rev-list`, `diff-tree`,
//! `cat-file`); the repository and its working tree are never written.

use review_episode_core::implementation_review_v1::SubjectParts;
use serde::Serialize;
use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use work_engine_review_preparation::Subject;

use crate::{Refusal, secrets, sha256_hex};

/// Host snapshot bounds (`work-engine-manual-host/src/pi_coding_analysis.rs:23-24`, path length
/// `:40`, checked at admission `:35-49`).
pub const SNAPSHOT_BYTES: u64 = 1 << 20;
pub const SNAPSHOT_FILES: usize = 256;
pub const MAX_PATH_BYTES: usize = 256;
/// The Host refuses a request whose SDK context exceeds this (`pi_coding_analysis.rs:16`).
pub const MAX_CONTEXT_BYTES: u64 = 400_000;

pub const REVIEW_PATCH: &str = ".review/patch.diff";
pub const REVIEW_SUBJECT: &str = ".review/subject.json";

/// The snapshot: post-change changed files at repository paths, `.review/patch.diff` (exactly
/// the bytes hashed for `patchIdentity`) and `.review/subject.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub parent: String,
    pub members: BTreeMap<String, Vec<u8>>,
}

impl Snapshot {
    pub fn bytes(&self) -> u64 {
        self.members.values().map(|b| b.len() as u64).sum()
    }
    /// Path → SHA-256 of the member bytes (what `snapshot_list` reports).
    pub fn manifest(&self) -> BTreeMap<String, String> {
        self.members
            .iter()
            .map(|(p, b)| (p.clone(), sha256_hex(b)))
            .collect()
    }
    /// Writes every member under `root`, which must not exist.
    pub fn write(&self, root: &Path) -> Result<(), Refusal> {
        std::fs::create_dir(root)?;
        for (path, bytes) in &self.members {
            let target = root.join(path);
            if let Some(dir) = target.parent() {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&target)
                .and_then(|mut f| std::io::Write::write_all(&mut f, bytes))?;
        }
        Ok(())
    }
}

/// Environment variables that would redirect git away from `--repo` (git sets `GIT_DIR` for hooks).
pub const GIT_REDIRECTS: [&str; 5] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
];

// Bound the bytes captured from Git, not Git's own computation or wall time.
// Every caller supplies a fixed limit or its remaining snapshot budget.
fn git(repo: &Path, args: &[&str], max_bytes: u64) -> Result<Vec<u8>, Refusal> {
    let mut command = Command::new("git");
    for var in GIT_REDIRECTS {
        command.env_remove(var);
    }
    let git_error = |e: std::io::Error| Refusal::Git(format!("git {}: {e}", args.join(" ")));
    let mut child = command
        .env("GIT_NO_REPLACE_OBJECTS", "1")
        .arg("-C")
        .arg(repo)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(git_error)?;
    let mut bytes = Vec::new();
    let read = child
        .stdout
        .take()
        .expect("piped Git stdout")
        .take(max_bytes + 1)
        .read_to_end(&mut bytes);
    let oversized = bytes.len() as u64 > max_bytes;
    if read.is_err() || oversized {
        // Stop the producer rather than drain an inadmissible result. It may
        // have already exited; still reap it before returning a refusal.
        let _ = child.kill();
    }
    let status = child.wait().map_err(git_error)?;
    read.map_err(git_error)?;
    if oversized {
        return Err(Refusal::SnapshotBound(format!(
            "git {} output exceeds {max_bytes} bytes",
            args[0]
        )));
    }
    if !status.success() {
        return Err(Refusal::Git(format!(
            "git {} exited {status}",
            args.join(" ")
        )));
    }
    Ok(bytes)
}

fn git_line(repo: &Path, args: &[&str]) -> Result<String, Refusal> {
    // rev-parse and rev-list here return at most a commit line for a valid subject.
    let bytes = git(repo, args, 4_096)?;
    String::from_utf8(bytes)
        .map(|s| s.trim_end().to_owned())
        .map_err(|_| Refusal::Git(format!("git {}: non-UTF-8 output", args.join(" "))))
}

fn full_hex(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

/// The bytes `patchIdentity` hashes: `git diff-tree --binary --no-renames --no-ext-diff
/// <parent>^{tree} <tree>` (see
/// `app-server/tests/supervisor-native-review-hosting-integration.test.mjs:255-257`).
/// Refuses if the patch alone would exceed the snapshot byte limit.
pub fn patch_bytes(repo: &Path, parent: &str, tree: &str) -> Result<Vec<u8>, Refusal> {
    git(
        repo,
        &[
            "diff-tree",
            "--binary",
            "--no-renames",
            "--no-ext-diff",
            &format!("{parent}^{{tree}}"),
            tree,
        ],
        SNAPSHOT_BYTES,
    )
}

#[derive(Serialize)]
struct SubjectFile<'a> {
    commit: &'a str,
    tree: &'a str,
    #[serde(rename = "patchIdentity")]
    patch_identity: &'a str,
    parent: &'a str,
    members: Vec<SubjectMember>,
}

#[derive(Serialize)]
struct SubjectMember {
    path: String,
    bytes: u64,
    sha256: String,
}

/// Verifies the subject's identity and binding, then reads the snapshot. Refuses on: a commit
/// that is not a full id, a merge or root commit, a tree mismatch, a recomputed `patchIdentity`
/// mismatch, a `scope_digest` that is not the digest of the verified three-field subject,
/// a non-regular, binary or non-UTF-8
/// member, a member under `.review/`, an exceeded Host bound, or any secret-rule hit.
pub fn materialize(
    repo: &Path,
    expected: &Subject,
    scope_digest: &str,
) -> Result<Snapshot, Refusal> {
    if !full_hex(&expected.commit, 40) || !full_hex(&expected.tree, 40) {
        return Err(Refusal::Git(
            "commit and tree must be full lowercase object ids".into(),
        ));
    }
    let commit = &expected.commit;
    if git_line(
        repo,
        &["rev-parse", "--verify", &format!("{commit}^{{commit}}")],
    )? != *commit
    {
        return Err(Refusal::Git(format!("{commit} is not a commit")));
    }
    let tree = git_line(
        repo,
        &["rev-parse", "--verify", &format!("{commit}^{{tree}}")],
    )?;
    if tree != expected.tree {
        return Err(Refusal::TreeMismatch {
            expected: expected.tree.clone(),
            actual: tree,
        });
    }
    let parents = git_line(repo, &["rev-list", "--parents", "-n", "1", commit])?;
    let ids: Vec<&str> = parents.split_whitespace().collect();
    if ids.len() != 2 || ids[0] != commit {
        return Err(Refusal::ParentShape(commit.clone()));
    }
    let parent = ids[1].to_owned();
    let patch = patch_bytes(repo, &parent, &tree)?;
    let patch_identity = sha256_hex(&patch);
    if patch_identity != expected.patch_identity {
        return Err(Refusal::PatchIdentityMismatch {
            expected: expected.patch_identity.clone(),
            actual: patch_identity,
        });
    }
    let exact_scope = SubjectParts {
        commit,
        tree: &tree,
        patch_identity: &patch_identity,
    }
    .digest();
    if scope_digest != exact_scope {
        return Err(Refusal::ScopeDigestMismatch {
            expected: scope_digest.to_owned(),
            actual: exact_scope,
        });
    }

    // Changed entries, first parent to commit: ":<src mode> <dst mode> <src> <dst> <status>\0<path>\0".
    // Allow extra metadata bytes for the full object IDs even for tiny patches
    // (e.g. mode changes). The patch and raw list cover the same changed paths.
    const RAW_DIFF_BYTES: u64 = SNAPSHOT_BYTES * 4;
    let raw = git(
        repo,
        &[
            "diff-tree",
            "-r",
            "-z",
            "--no-renames",
            "--raw",
            &parent,
            commit,
        ],
        RAW_DIFF_BYTES,
    )?;
    let mut fields = raw.split(|b| *b == 0).filter(|f| !f.is_empty());
    let mut members = BTreeMap::new();
    let mut remaining = SNAPSHOT_BYTES - patch.len() as u64;
    while let Some(head) = fields.next() {
        let head = std::str::from_utf8(head).map_err(|_| Refusal::Git("raw diff header".into()))?;
        let path_bytes = fields
            .next()
            .ok_or_else(|| Refusal::Git("raw diff entry without a path".into()))?;
        let path = std::str::from_utf8(path_bytes)
            .map_err(|_| Refusal::Member {
                path: String::from_utf8_lossy(path_bytes).into_owned(),
                why: "path is not UTF-8".into(),
            })?
            .to_owned();
        let parts: Vec<&str> = head.trim_start_matches(':').split(' ').collect();
        let [_, dst_mode, _, dst, status] = parts[..] else {
            return Err(Refusal::Git(format!("raw diff header for {path}")));
        };
        match status {
            "D" => continue,
            "A" | "M" | "T" => {}
            other => {
                return Err(Refusal::Member {
                    path,
                    why: format!("unexpected change status {other}"),
                });
            }
        }
        if !matches!(dst_mode, "100644" | "100755") {
            return Err(Refusal::Member {
                path,
                why: format!("not a regular file (mode {dst_mode})"),
            });
        }
        if path.starts_with(".review/") {
            return Err(Refusal::Member {
                path,
                why: "collides with the .review/ layout".into(),
            });
        }
        // Reserve member slots for the patch and the generated subject index.
        if members.len() >= SNAPSHOT_FILES - 2 {
            return Err(Refusal::SnapshotBound(format!(
                "more than {SNAPSHOT_FILES} members including review metadata"
            )));
        }
        let bytes = git(repo, &["cat-file", "blob", dst], remaining)?;
        remaining -= bytes.len() as u64;
        members.insert(path, bytes);
    }
    members.insert(REVIEW_PATCH.to_owned(), patch);
    let listed = members
        .iter()
        .map(|(path, bytes)| SubjectMember {
            path: path.clone(),
            bytes: bytes.len() as u64,
            sha256: sha256_hex(bytes),
        })
        .collect();
    let mut subject_file = serde_json::to_vec_pretty(&SubjectFile {
        commit,
        tree: &tree,
        patch_identity: &patch_identity,
        parent: &parent,
        members: listed,
    })
    .map_err(|e| Refusal::Io(e.to_string()))?;
    subject_file.push(b'\n');
    members.insert(REVIEW_SUBJECT.to_owned(), subject_file);

    let snapshot = Snapshot { parent, members };
    check_members(&snapshot)?;
    Ok(snapshot)
}

/// Member form, Host bounds and the secret rule set.
pub fn check_members(snapshot: &Snapshot) -> Result<(), Refusal> {
    let mut hits = Vec::new();
    // Changed files first, so a refusal names the source member rather than the patch.
    let (review, changed): (Vec<_>, Vec<_>) = snapshot
        .members
        .iter()
        .partition(|(path, _)| path.starts_with(".review/"));
    for (path, bytes) in changed.into_iter().chain(review) {
        if path
            .split('/')
            .any(|c| c.is_empty() || c == "." || c == ".." || c == ".git")
        {
            return Err(Refusal::Member {
                path: path.clone(),
                why: "empty, `.`, `..` or `.git` path component".into(),
            });
        }
        if path.len() > MAX_PATH_BYTES {
            return Err(Refusal::Member {
                path: path.clone(),
                why: format!("path longer than {MAX_PATH_BYTES} bytes"),
            });
        }
        let text = std::str::from_utf8(bytes)
            .ok()
            .filter(|t| !t.contains('\0'))
            .ok_or_else(|| Refusal::Member {
                path: path.clone(),
                why: "binary or non-UTF-8 content".into(),
            })?;
        hits.extend(secrets::scan(path, text));
    }
    if snapshot.members.len() > SNAPSHOT_FILES || snapshot.bytes() > SNAPSHOT_BYTES {
        return Err(Refusal::SnapshotBound(format!(
            "{} members, {} bytes (bounds {SNAPSHOT_FILES} members, {SNAPSHOT_BYTES} bytes)",
            snapshot.members.len(),
            snapshot.bytes()
        )));
    }
    if !hits.is_empty() {
        return Err(Refusal::Secret(hits));
    }
    Ok(())
}

/// One `snapshot_read` returns at most this many bytes (`pi_coding_analysis.rs:22`, `:139-142`).
pub const PAGE_BYTES: u64 = 24_576;

/// Pages needed to read `bytes` in full.
pub fn pages(bytes: u64) -> u64 {
    bytes.div_ceil(PAGE_BYTES)
}

/// Review P3-6: the Host counts every snapshot delivery against the read limit
/// (`pi_coding_analysis.rs:10`), so reading `.review/patch.diff` in full takes one
/// `snapshot_list` plus its pages. Refuse when that exceeds `max_reads`; returns the pages needed
/// to read every member (recorded, not enforced).
pub fn check_read_budget(snapshot: &Snapshot, max_reads: u32) -> Result<u64, Refusal> {
    let patch = snapshot
        .members
        .get(REVIEW_PATCH)
        .map_or(0, |b| b.len() as u64);
    let needed = 1 + pages(patch);
    if needed > u64::from(max_reads) {
        return Err(Refusal::ReadBudget { needed, max_reads });
    }
    Ok(snapshot
        .members
        .values()
        .map(|b| pages(b.len() as u64))
        .sum())
}

/// P2-4: every request carries the task and, once read, the snapshot (each change twice: file and
/// patch). Refuse when snapshot bytes × 1.25 + task bytes > 400 000 (ratio 1.25 from run set a3,
/// n=2, `records/dev-vertical-h2-route-20261117/README.md:98-99`).
pub fn check_context_fit(snapshot_bytes: u64, task_bytes: usize) -> Result<(), Refusal> {
    let weighted = snapshot_bytes
        .saturating_mul(5)
        .saturating_add((task_bytes as u64).saturating_mul(4));
    if weighted > MAX_CONTEXT_BYTES * 4 {
        return Err(Refusal::ContextBound {
            snapshot_bytes,
            task_bytes,
        });
    }
    Ok(())
}
