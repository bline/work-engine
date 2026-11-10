//! Test-only fixture builders. These use the git CLI to construct repositories; the crate itself
//! never runs git.
#![allow(dead_code)]

use research_register::PinnedSnapshot;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

pub struct Fx {
    pub root: PathBuf,
    pub home: PathBuf,
    pub bin: PathBuf,
    pub hits: PathBuf,
}

pub fn write(path: &Path, content: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content).unwrap();
}

impl Fx {
    pub fn new(name: &str) -> Fx {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("research-register")
            .join(format!("{name}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (home, bin) = (root.join("home"), root.join("bin"));
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&bin).unwrap();
        let hits = root.join("marker-hits");
        Fx { root, home, bin, hits }
    }

    pub fn cmd(&self, dir: &Path, args: &[&str], env: &[(&str, &str)]) -> Command {
        let mut c = Command::new("git");
        c.arg("-C").arg(dir).args(args).env_clear()
            .env("PATH", "/usr/bin:/bin").env("HOME", &self.home).env("GIT_CONFIG_NOSYSTEM", "1").env("LC_ALL", "C")
            .env("GIT_AUTHOR_NAME", "t").env("GIT_AUTHOR_EMAIL", "t@x").env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@x").env("GIT_AUTHOR_DATE", "2026-01-01T00:00:00Z")
            .env("GIT_COMMITTER_DATE", "2026-01-01T00:00:00Z");
        for (k, v) in env {
            c.env(k, v);
        }
        c
    }

    /// Run git and require success; returns trimmed stdout.
    pub fn git(&self, dir: &Path, args: &[&str]) -> String {
        self.git_with(dir, args, None, &[])
    }

    pub fn git_with(&self, dir: &Path, args: &[&str], input: Option<&[u8]>, env: &[(&str, &str)]) -> String {
        use std::io::Write;
        let mut c = self.cmd(dir, args, env);
        c.stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped());
        let mut child = c.spawn().expect("git runs");
        if let Some(bytes) = input {
            child.stdin.take().unwrap().write_all(bytes).unwrap();
        } else {
            drop(child.stdin.take());
        }
        let out = child.wait_with_output().unwrap();
        assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
        String::from_utf8(out.stdout).unwrap().trim().to_string()
    }

    /// Run git without requiring success; returns (success, stdout).
    pub fn git_try(&self, dir: &Path, args: &[&str], env: &[(&str, &str)]) -> (bool, String) {
        let out = self.cmd(dir, args, env).output().unwrap();
        (out.status.success(), String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    /// A repository with two commits. C1: reg/{decisions.json, events.jsonl, studies/s1.json},
    /// other.txt. C2 changes decisions.json and adds studies/s2.json. `extra` runs before the
    /// first commit's `git add`.
    pub fn base(&self, name: &str, extra: &dyn Fn(&Fx, &Path)) -> (PathBuf, String, String) {
        let r = self.root.join(name);
        std::fs::create_dir_all(&r).unwrap();
        self.git(&r, &["init", "-q", "-b", "main"]);
        write(&r.join("reg/decisions.json"), "{\"v\": 1}\n");
        write(&r.join("reg/events.jsonl"), "{\"e\": 1}\n");
        write(&r.join("reg/studies/s1.json"), "{\"s\": 1}\n");
        write(&r.join("other.txt"), "x\n");
        extra(self, &r);
        self.git(&r, &["add", "-A"]);
        self.git(&r, &["commit", "-q", "-m", "c1"]);
        let c1 = self.git(&r, &["rev-parse", "HEAD"]);
        write(&r.join("reg/decisions.json"), "{\"v\": 2}\n");
        write(&r.join("reg/studies/s2.json"), "{\"s\": 2}\n");
        self.git(&r, &["add", "-A"]);
        self.git(&r, &["commit", "-q", "-m", "c2"]);
        let c2 = self.git(&r, &["rev-parse", "HEAD"]);
        (r, c1, c2)
    }

    /// Ground truth from the git CLI with replacement disabled: path (relative to `reg`) to
    /// (kind, id). Call before applying a hazard.
    pub fn truth(&self, repo: &Path, commit: &str, reg: &str) -> BTreeMap<String, (String, String)> {
        let listing = self.git(repo, &["--no-replace-objects", "ls-tree", "-r", "-t", commit, "--", &format!("{reg}/")]);
        listing
            .lines()
            .filter(|line| line.split_once('\t').is_some_and(|(_, path)| path != reg))
            .map(|line| {
                let (meta, path) = line.split_once('\t').unwrap();
                let mut parts = meta.split_whitespace();
                let mode = parts.next().unwrap();
                let id = parts.nth(1).unwrap().to_string();
                let kind = match mode {
                    "040000" => "tree",
                    "100644" => "blob",
                    "100755" => "blob_executable",
                    "120000" => "link",
                    "160000" => "gitlink",
                    other => panic!("mode {other}"),
                };
                (path[reg.len() + 1..].to_string(), (kind.to_string(), id))
            })
            .collect()
    }

    pub fn tree_id(&self, repo: &Path, spec: &str) -> String {
        self.git(repo, &["--no-replace-objects", "rev-parse", spec])
    }

    /// A script that records its name when run, then optionally execs `passthru`.
    pub fn marker(&self, name: &str, passthru: Option<&str>) -> String {
        let p = self.bin.join(name);
        let tail = passthru.map(|t| format!("exec {t} \"$@\"\n")).unwrap_or_else(|| "exit 0\n".into());
        std::fs::write(&p, format!("#!/bin/sh\necho \"{name} $*\" >> {}\n{tail}", self.hits.display())).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        p.display().to_string()
    }

    /// Marker names recorded since the last call.
    pub fn take_hits(&self) -> Vec<String> {
        let text = std::fs::read_to_string(&self.hits).unwrap_or_default();
        let _ = std::fs::remove_file(&self.hits);
        let mut names: Vec<String> = text.lines().filter_map(|l| l.split_whitespace().next()).map(String::from).collect();
        names.sort();
        names.dedup();
        names
    }
}

/// The snapshot's entries in the same shape as [`Fx::truth`].
pub fn listing(s: &PinnedSnapshot) -> BTreeMap<String, (String, String)> {
    s.entries().iter().map(|(p, e)| (p.clone(), (e.kind().as_str().to_string(), e.id().to_string()))).collect()
}

impl Fx {
    /// Raw stdout of a successful git command.
    pub fn git_bytes(&self, dir: &Path, args: &[&str]) -> Vec<u8> {
        let out = self.cmd(dir, args, &[]).output().unwrap();
        assert!(out.status.success(), "git {args:?}");
        out.stdout
    }

    /// sha256 of every file entry's true content (replacement disabled). Call before a hazard.
    pub fn truth_sha(&self, repo: &Path, commit: &str, reg: &str) -> BTreeMap<String, String> {
        use sha2::{Digest, Sha256};
        self.truth(repo, commit, reg)
            .into_iter()
            .filter(|(_, (kind, _))| kind == "blob" || kind == "blob_executable")
            .map(|(path, (_, id))| {
                let bytes = self.git_bytes(repo, &["--no-replace-objects", "cat-file", "blob", &id]);
                (path, format!("{:x}", Sha256::digest(&bytes)))
            })
            .collect()
    }

    /// Path of a loose object file.
    pub fn loose(&self, repo: &Path, id: &str) -> PathBuf {
        repo.join(".git/objects").join(&id[..2]).join(&id[2..])
    }
}

pub fn shas(s: &PinnedSnapshot) -> BTreeMap<String, String> {
    s.entries().iter().filter_map(|(p, e)| e.sha256().map(|h| (p.clone(), h.to_string()))).collect()
}
