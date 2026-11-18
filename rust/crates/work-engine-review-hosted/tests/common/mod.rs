#![allow(dead_code)]
//! Test support: the frozen subject (strings only) and synthetic git repositories created in a
//! temporary directory. No test touches a real repository.

use std::path::{Path, PathBuf};
use std::process::Command;
use work_engine_review_hosted::sha256_hex;
use work_engine_review_preparation::Subject;

/// The frozen vertical's subject (`work-engine-review-delivery/tests/common/mod.rs:73-80`).
pub fn frozen_subject() -> Subject {
    Subject {
        commit: "96808de3d565a6c4b0211e8dc263cb7343514d74".to_owned(),
        tree: "9c5d94ba059ea5b4ff497a0540f6b8a7476e4474".to_owned(),
        patch_identity: "5212141d54e001b1fa85f7660fbf9605a957cf11a7636636dc03d11dded58c51"
            .to_owned(),
    }
}

/// `git` without the variables that would redirect it to another repository (review P3-3).
pub fn git_command() -> Command {
    let mut command = Command::new("git");
    for var in work_engine_review_hosted::subject::GIT_REDIRECTS {
        command.env_remove(var);
    }
    command
}

pub fn git(repo: &Path, args: &[&str]) -> String {
    let out = git_command()
        .arg("-C")
        .arg(repo)
        .args([
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim_end().to_owned()
}

/// A synthetic repository with a baseline commit and a candidate commit on top.
pub struct Repo {
    pub dir: tempfile::TempDir,
    pub subject: Subject,
    pub parent: String,
}

impl Repo {
    pub fn path(&self) -> PathBuf {
        self.dir.path().to_owned()
    }
}

fn write(repo: &Path, path: &str, bytes: &[u8]) {
    let target = repo.join(path);
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    std::fs::write(target, bytes).unwrap();
}

/// Baseline: `a.txt`, `c.txt`. Candidate: modifies `a.txt`, deletes `c.txt`, adds every
/// `(path, bytes)` of `added`.
pub fn repo_with(added: &[(&str, &[u8])]) -> Repo {
    repo_deleting(&[], added)
}

/// As [`repo_with`], and every `(path, bytes)` of `deleted` exists in the baseline only.
pub fn repo_deleting(deleted: &[(&str, &[u8])], added: &[(&str, &[u8])]) -> Repo {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    git(r, &["init", "--quiet"]);
    write(r, "a.txt", b"alpha\n");
    write(r, "c.txt", b"gamma\n");
    for (path, bytes) in deleted {
        write(r, path, bytes);
    }
    git(r, &["add", "-A"]);
    git(r, &["commit", "--quiet", "-m", "baseline"]);
    let parent = git(r, &["rev-parse", "HEAD"]);
    write(r, "a.txt", b"alpha two\n");
    std::fs::remove_file(r.join("c.txt")).unwrap();
    for (path, _) in deleted {
        std::fs::remove_file(r.join(path)).unwrap();
    }
    for (path, bytes) in added {
        write(r, path, bytes);
    }
    git(r, &["add", "-A"]);
    git(r, &["commit", "--quiet", "-m", "candidate"]);
    let commit = git(r, &["rev-parse", "HEAD"]);
    let tree = git(r, &["rev-parse", "HEAD^{tree}"]);
    let patch = git_command()
        .arg("-C")
        .arg(r)
        .args([
            "diff-tree",
            "--binary",
            "--no-renames",
            "--no-ext-diff",
            &format!("{parent}^{{tree}}"),
            &tree,
        ])
        .output()
        .unwrap()
        .stdout;
    Repo {
        subject: Subject {
            commit,
            tree,
            patch_identity: sha256_hex(&patch),
        },
        parent,
        dir,
    }
}

pub fn repo() -> Repo {
    repo_with(&[("src/b.rs", b"pub fn b() -> u32 {\n    2\n}\n")])
}
