use crate::codec::raw_sha256;
use crate::contract::{AttributedManifest, BlobSide, GitOid, PublicationCandidate};
use crate::{Error, Result};
use std::collections::BTreeMap;
use std::io::Write;
use std::os::fd::RawFd;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub fn command(repo: &Path, args: &[&str], input: Option<&[u8]>) -> Result<Vec<u8>> {
    command_inner(repo, args, input, None)
}
pub fn command_guarded(
    repo: &Path,
    args: &[&str],
    input: Option<&[u8]>,
    guard_fd: RawFd,
) -> Result<Vec<u8>> {
    command_inner(repo, args, input, Some(guard_fd))
}
fn controlled(cmd: &mut Command) {
    cmd.env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LC_ALL", "C")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("HOME", "/")
        .env("GIT_NO_REPLACE_OBJECTS", "1")
        .arg("--no-replace-objects");
}
// Git must never interpret a canonical repository's config as an untrusted
// executable grant (filters, fsmonitor, hooks, external diff, includes, etc.).
fn safe_config(repo: &Path) -> Result<()> {
    let dot = repo.join(".git");
    let mut dir = if dot.is_file() {
        let text = std::fs::read_to_string(&dot).map_err(|e| Error::Io(e.to_string()))?;
        let value = text
            .strip_prefix("gitdir: ")
            .ok_or_else(|| Error::Integrity("invalid worktree link".into()))?
            .trim();
        repo.join(value)
            .canonicalize()
            .map_err(|e| Error::Io(e.to_string()))?
    } else if dot.is_dir() {
        dot
    } else {
        repo.into()
    };
    let common = dir.join("commondir");
    if common.exists() {
        let value = std::fs::read_to_string(common).map_err(|e| Error::Io(e.to_string()))?;
        dir = dir
            .join(value.trim())
            .canonicalize()
            .map_err(|e| Error::Io(e.to_string()))?;
    }
    let path = dir.join("config");
    if std::fs::symlink_metadata(&path)
        .map_err(|e| Error::Io(e.to_string()))?
        .file_type()
        .is_symlink()
    {
        return Err(Error::Integrity("Git config symlink refused".into()));
    }
    let text = std::fs::read_to_string(path).map_err(|e| Error::Io(e.to_string()))?;
    let mut core = false;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') {
            core = line.eq_ignore_ascii_case("[core]");
            if !core {
                return Err(Error::Integrity(
                    "executable-capable Git configuration refused".into(),
                ));
            }
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| Error::Integrity("unsupported Git config syntax".into()))?;
        let key = key.trim().to_ascii_lowercase();
        let value = value.trim();
        if !core
            || !matches!(
                key.as_str(),
                "repositoryformatversion" | "filemode" | "bare" | "logallrefupdates"
            )
            || !matches!(value, "0" | "true" | "false")
        {
            return Err(Error::Integrity(
                "unsupported Git configuration refused".into(),
            ));
        }
    }
    if dir.join("config.worktree").exists() {
        return Err(Error::Integrity("worktree config refused".into()));
    }
    Ok(())
}
fn retain_guard(cmd: &mut Command, guard_fd: Option<RawFd>) {
    if let Some(fd) = guard_fd {
        // The child holds the same open file description and flock until its
        // topology mutation exits, even if the caller is killed.
        unsafe {
            cmd.pre_exec(move || {
                let inherited = libc::fcntl(fd, libc::F_DUPFD, 100);
                if inherited < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                if libc::fcntl(inherited, libc::F_SETFD, 0) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }
}
fn command_inner(
    repo: &Path,
    args: &[&str],
    input: Option<&[u8]>,
    guard_fd: Option<RawFd>,
) -> Result<Vec<u8>> {
    #[cfg(feature = "test-faults")]
    let program = if guard_fd.is_some() {
        std::env::var("WORKSPACE_PUBLICATION_GUARDED_GIT_FIXTURE")
            .unwrap_or_else(|_| "/usr/bin/git".into())
    } else {
        "/usr/bin/git".into()
    };
    #[cfg(not(feature = "test-faults"))]
    let program = "/usr/bin/git";
    safe_config(repo)?;
    let mut cmd = Command::new(program);
    controlled(&mut cmd);
    crate::custody::trusted_program("/usr/bin/git", crate::custody::GIT_SHA256)?;
    cmd.args([
        "-c",
        "core.hooksPath=/dev/null",
        "-c",
        "core.fsmonitor=false",
        "-c",
        "core.attributesFile=/dev/null",
        "-c",
        "protocol.ext.allow=never",
    ]);
    cmd.arg("-C")
        .arg(repo)
        .args(args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    retain_guard(&mut cmd, guard_fd);
    let mut child = cmd.spawn().map_err(|e| Error::Io(e.to_string()))?;
    if let Some(data) = input {
        child
            .stdin
            .take()
            .unwrap()
            .write_all(data)
            .map_err(|e| Error::Io(e.to_string()))?;
    }
    let out = child
        .wait_with_output()
        .map_err(|e| Error::Io(e.to_string()))?;
    if !out.status.success() {
        return Err(Error::Conflict(format!(
            "git {:?}: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        )));
    }
    Ok(out.stdout)
}

fn text(repo: &Path, args: &[&str]) -> Result<String> {
    String::from_utf8(command(repo, args, None)?)
        .map(|s| s.trim().into())
        .map_err(|e| Error::Integrity(e.to_string()))
}
pub fn oid(hex: String) -> Result<GitOid> {
    if hex.len() != 40
        || !hex
            .bytes()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    {
        return Err(Error::Invalid("W1 supports SHA-1 Git OIDs only".into()));
    }
    Ok(GitOid {
        algorithm: "sha1".into(),
        hex,
    })
}
pub fn check_oid(oid: &GitOid) -> Result<()> {
    if *oid == self::oid(oid.hex.clone())? {
        Ok(())
    } else {
        Err(Error::Invalid("Git OID algorithm".into()))
    }
}
pub fn common_dir(repo: &Path) -> Result<PathBuf> {
    let dir = text(
        repo,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?;
    std::fs::canonicalize(dir).map_err(|e| Error::Io(e.to_string()))
}
pub fn repository_id(repo: &Path) -> Result<String> {
    Ok(raw_sha256(common_dir(repo)?.to_string_lossy().as_bytes()))
}
pub fn ref_oid(repo: &Path, name: &str) -> Result<GitOid> {
    if !name.starts_with("refs/heads/") || name.contains("..") || name.contains(' ') {
        return Err(Error::Invalid("canonical ref name".into()));
    }
    oid(text(repo, &["rev-parse", "--verify", name])?)
}
pub fn commit_tree(repo: &Path, commit: &GitOid) -> Result<GitOid> {
    check_oid(commit)?;
    oid(text(
        repo,
        &["rev-parse", "--verify", &format!("{}^{{tree}}", commit.hex)],
    )?)
}
pub fn commit_parent(repo: &Path, commit: &GitOid) -> Result<GitOid> {
    check_oid(commit)?;
    let line = text(repo, &["show", "-s", "--format=%P", &commit.hex])?;
    if line.split_whitespace().count() != 1 {
        return Err(Error::Invalid("checkpoint must have one parent".into()));
    }
    oid(line)
}
pub fn commit_message(repo: &Path, commit: &GitOid) -> Result<Vec<u8>> {
    check_oid(commit)?;
    let raw = command(repo, &["cat-file", "commit", &commit.hex], None)?;
    let split = raw
        .windows(2)
        .position(|w| w == b"\n\n")
        .ok_or_else(|| Error::Integrity("commit header framing".into()))?;
    Ok(raw[split + 2..].to_vec())
}
pub fn blob_bytes(repo: &Path, blob: &GitOid) -> Result<Vec<u8>> {
    check_oid(blob)?;
    if text(repo, &["cat-file", "-t", &blob.hex])? != "blob" {
        return Err(Error::Invalid("manifest object is not blob".into()));
    }
    command(repo, &["cat-file", "blob", &blob.hex], None)
}
fn tree_map(repo: &Path, tree: &GitOid) -> Result<BTreeMap<String, (u32, GitOid)>> {
    check_oid(tree)?;
    let data = command(repo, &["ls-tree", "-r", "-z", &tree.hex], None)?;
    let mut result = BTreeMap::new();
    for item in data.split(|b| *b == 0).filter(|v| !v.is_empty()) {
        let (meta, path) = SplitOnce::split_once(item, |b| *b == b'\t')
            .ok_or_else(|| Error::Integrity("ls-tree framing".into()))?;
        let words: Vec<_> = meta.split(|b| *b == b' ').collect();
        if words.len() != 3 || words[1] != b"blob" {
            return Err(Error::Invalid(
                "submodules/trees unsupported in manifest".into(),
            ));
        }
        let mode = u32::from_str_radix(
            std::str::from_utf8(words[0]).map_err(|e| Error::Integrity(e.to_string()))?,
            8,
        )
        .map_err(|e| Error::Integrity(e.to_string()))?;
        let id = oid(std::str::from_utf8(words[2])
            .map_err(|e| Error::Integrity(e.to_string()))?
            .into())?;
        let path = std::str::from_utf8(path)
            .map_err(|e| Error::Invalid(e.to_string()))?
            .to_string();
        if path.starts_with('/') || path.contains("../") || path.contains('\n') {
            return Err(Error::Invalid("unsafe tree path".into()));
        }
        result.insert(path, (mode, id));
    }
    Ok(result)
}
trait SplitOnce {
    fn split_once(&self, pred: impl FnMut(&u8) -> bool) -> Option<(&[u8], &[u8])>;
}
impl SplitOnce for [u8] {
    fn split_once(&self, mut pred: impl FnMut(&u8) -> bool) -> Option<(&[u8], &[u8])> {
        let i = self.iter().position(&mut pred)?;
        Some((&self[..i], &self[i + 1..]))
    }
}
fn side(repo: &Path, pair: Option<&(u32, GitOid)>) -> Result<Option<BlobSide>> {
    pair.map(|(mode, blob)| {
        Ok(BlobSide {
            mode: *mode,
            blob: blob.clone(),
            raw_sha256: raw_sha256(&blob_bytes(repo, blob)?),
        })
    })
    .transpose()
}
pub fn verify_manifest(
    repo: &Path,
    before: &GitOid,
    after: &GitOid,
    manifest: &AttributedManifest,
) -> Result<()> {
    let old = tree_map(repo, before)?;
    let new = tree_map(repo, after)?;
    let mut changed = BTreeMap::new();
    for path in old.keys().chain(new.keys()) {
        if old.get(path) == new.get(path) {
            continue;
        }
        changed.insert(
            path.clone(),
            (side(repo, old.get(path))?, side(repo, new.get(path))?),
        );
    }
    if changed.len() != manifest.entries.len() {
        return Err(Error::Conflict(
            "manifest omits or adds changed paths".into(),
        ));
    }
    let mut last = "";
    for e in &manifest.entries {
        if e.path.as_str() <= last
            || e.task_attribution.trim().is_empty()
            || e.attribution_evidence.sha256.len() != 64
        {
            return Err(Error::Invalid("manifest order/attribution".into()));
        }
        let Some(pair) = changed.get(&e.path) else {
            return Err(Error::Conflict("manifest path differs".into()));
        };
        if pair.0 != e.before || pair.1 != e.after {
            return Err(Error::Conflict(format!(
                "manifest blob/mode differs at {}",
                e.path
            )));
        }
        last = &e.path;
    }
    Ok(())
}
pub fn verify_candidate(repo: &Path, candidate: &PublicationCandidate) -> Result<()> {
    if commit_tree(repo, &candidate.checkpoint_commit)? != candidate.checkpoint_tree
        || commit_parent(repo, &candidate.checkpoint_commit)? != candidate.checkpoint_parent
    {
        return Err(Error::Conflict("checkpoint object binding".into()));
    }
    let parent_tree = commit_tree(repo, &candidate.checkpoint_parent)?;
    verify_manifest(
        repo,
        &parent_tree,
        &candidate.checkpoint_tree,
        &candidate.manifest,
    )
}
pub fn integrate(
    repo: &Path,
    artifact: &Path,
    parent: &GitOid,
    candidate: &PublicationCandidate,
    guard_fd: RawFd,
) -> Result<GitOid> {
    if artifact.exists() {
        return Err(Error::Conflict("operation artifact already exists".into()));
    }
    let target = artifact
        .to_str()
        .ok_or_else(|| Error::Invalid("artifact path encoding".into()))?;
    command_guarded(
        repo,
        &["worktree", "add", "--detach", target, &parent.hex],
        None,
        guard_fd,
    )?;
    let patch = command(
        repo,
        &[
            "diff",
            "--binary",
            "--no-renames",
            &candidate.checkpoint_parent.hex,
            &candidate.checkpoint_commit.hex,
        ],
        None,
    )?;
    command_guarded(
        artifact,
        &["apply", "--index", "--3way", "-"],
        Some(&patch),
        guard_fd,
    )?;
    oid(
        String::from_utf8(command_guarded(artifact, &["write-tree"], None, guard_fd)?)
            .map_err(|e| Error::Integrity(e.to_string()))?
            .trim()
            .into(),
    )
}
pub fn create_commit(
    repo: &Path,
    tree: &GitOid,
    parent: &GitOid,
    message: &str,
    guard_fd: RawFd,
) -> Result<GitOid> {
    safe_config(repo)?;
    #[cfg(feature = "test-faults")]
    let program = std::env::var("WORKSPACE_PUBLICATION_GUARDED_GIT_FIXTURE")
        .unwrap_or_else(|_| "/usr/bin/git".into());
    #[cfg(not(feature = "test-faults"))]
    let program = "/usr/bin/git";
    let mut cmd = Command::new(program);
    controlled(&mut cmd);
    crate::custody::trusted_program("/usr/bin/git", crate::custody::GIT_SHA256)?;
    retain_guard(&mut cmd, Some(guard_fd));
    let mut child = cmd
        .args([
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.attributesFile=/dev/null",
            "-c",
            "protocol.ext.allow=never",
        ])
        .arg("-C")
        .arg(repo)
        .args(["commit-tree", &tree.hex, "-p", &parent.hex])
        .env("GIT_AUTHOR_NAME", "Work Engine")
        .env("GIT_AUTHOR_EMAIL", "work-engine@localhost")
        .env("GIT_COMMITTER_NAME", "Work Engine")
        .env("GIT_COMMITTER_EMAIL", "work-engine@localhost")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| Error::Io(e.to_string()))?;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(message.as_bytes())
        .map_err(|e| Error::Io(e.to_string()))?;
    let out = child
        .wait_with_output()
        .map_err(|e| Error::Io(e.to_string()))?;
    if !out.status.success() {
        return Err(Error::Io(String::from_utf8_lossy(&out.stderr).into()));
    }
    let data = out.stdout;
    oid(String::from_utf8(data)
        .map_err(|e| Error::Io(e.to_string()))?
        .trim()
        .into())
}
pub fn compare_and_swap(repo: &Path, name: &str, old: &GitOid, new: &GitOid) -> Result<()> {
    check_oid(old)?;
    check_oid(new)?;
    safe_config(repo)?;
    // The ref transaction is in-process, so child Command::env_clear() does
    // not protect it. Bind this open to the explicit path and repository-local
    // configuration, then clear even a locally configured ref namespace.
    let mut repository = gix::open_opts(repo, gix::open::Options::isolated())
        .map_err(|e| Error::Io(e.to_string()))?;
    repository.clear_namespace();
    let old_id = old
        .hex
        .parse::<gix::hash::ObjectId>()
        .map_err(|e| Error::Invalid(e.to_string()))?;
    let new_id = new
        .hex
        .parse::<gix::hash::ObjectId>()
        .map_err(|e| Error::Invalid(e.to_string()))?;
    repository
        .reference(
            name,
            new_id,
            gix::refs::transaction::PreviousValue::MustExistAndMatch(gix::refs::Target::Object(
                old_id,
            )),
            "work-engine canonical publication",
        )
        .map_err(|e| Error::Conflict(format!("expected-old ref transaction: {e}")))?;
    Ok(())
}

/// Refuse checkout transforms before treating validator input as the actual tree.
pub fn verify_worktree(repo: &Path, artifact: &Path, tree: &GitOid) -> Result<()> {
    use std::io::Read;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    let entries = command(repo, &["ls-tree", "-rz", "--full-tree", &tree.hex], None)?;
    for entry in entries.split(|b| *b == 0).filter(|e| !e.is_empty()) {
        let tab = entry
            .iter()
            .position(|b| *b == b'\t')
            .ok_or_else(|| Error::Integrity("tree entry format".into()))?;
        let header =
            std::str::from_utf8(&entry[..tab]).map_err(|e| Error::Integrity(e.to_string()))?;
        let parts: Vec<_> = header.split(' ').collect();
        let path =
            std::str::from_utf8(&entry[tab + 1..]).map_err(|e| Error::Integrity(e.to_string()))?;
        if parts.len() != 3
            || parts[1] != "blob"
            || Path::new(path)
                .components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            return Err(Error::Integrity("unsupported tree entry".into()));
        }
        let file = artifact.join(path);
        let metadata = std::fs::symlink_metadata(&file).map_err(|e| Error::Io(e.to_string()))?;
        let actual = match parts[0] {
            "120000" if metadata.file_type().is_symlink() => std::fs::read_link(&file)
                .map_err(|e| Error::Io(e.to_string()))?
                .as_os_str()
                .as_bytes()
                .to_vec(),
            "100644" | "100755" if metadata.is_file() => {
                if (metadata.mode() & 0o111 != 0) != (parts[0] == "100755") {
                    return Err(Error::Integrity("checkout executable mode differs".into()));
                }
                let mut bytes = Vec::new();
                std::fs::OpenOptions::new()
                    .read(true)
                    .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
                    .open(&file)
                    .and_then(|mut f| f.read_to_end(&mut bytes))
                    .map_err(|e| Error::Io(e.to_string()))?;
                bytes
            }
            _ => return Err(Error::Integrity("checkout entry type differs".into())),
        };
        if actual != command(repo, &["cat-file", "blob", parts[2]], None)? {
            return Err(Error::Integrity(
                "validator checkout bytes differ from integrated Git tree".into(),
            ));
        }
    }
    Ok(())
}
