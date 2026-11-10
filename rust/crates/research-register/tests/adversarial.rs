//! Adversarial qualification (R-1a hazards plus R-1b.1 additions). Failure messages are prefixed:
//! SECURITY = a security property was violated (foreign or substituted content delivered, a
//! command ran, a required refusal did not happen, history reported as complete when it is not);
//! BEHAVIOR = a safe but wrong outcome (refused where the true objects should be read, wrong
//! category); CONTROL = the fixture failed to make the hazard live under the git CLI.

mod support;

use research_register::{Ancestry, Error, GitObjects, PinnedSnapshot};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use support::{Fx, listing, shas, write};

type Listing = BTreeMap<String, (String, String)>;
type Shas = BTreeMap<String, String>;

enum Expect {
    Truth(Listing, Shas),
    Refuse(&'static str),
}

fn truth(fx: &Fx, repo: &Path, commit: &str) -> Expect {
    Expect::Truth(
        fx.truth(repo, commit, "reg"),
        fx.truth_sha(repo, commit, "reg"),
    )
}

fn read(repo: &Path, rev: &str, reg: &str) -> Result<PinnedSnapshot, Error> {
    PinnedSnapshot::build(GitObjects::open(repo)?, rev, reg)
}

fn judge(fx: &Fx, what: &str, result: Result<PinnedSnapshot, Error>, expect: &Expect) {
    let hits = fx.take_hits();
    assert!(
        hits.is_empty(),
        "SECURITY: {what}: commands executed during the read: {hits:?}"
    );
    match (result, expect) {
        (Ok(s), Expect::Truth(t, sha)) => {
            assert_eq!(
                &listing(&s),
                t,
                "SECURITY: {what}: delivered entries differ from the commit's true objects"
            );
            assert_eq!(
                &shas(&s),
                sha,
                "SECURITY: {what}: delivered file bytes differ from the true objects"
            );
        }
        (Ok(_), Expect::Refuse(cat)) => {
            panic!("SECURITY: {what}: expected refusal ({cat}) but a snapshot was delivered")
        }
        (Err(e), Expect::Truth(..)) => {
            panic!("BEHAVIOR: {what}: expected the true objects, got refusal: {e}")
        }
        (Err(e), Expect::Refuse(cat)) => assert_eq!(
            e.category(),
            *cat,
            "BEHAVIOR: {what}: wrong refusal category: {e}"
        ),
    }
}

/// Run the CLI with a fully controlled environment and judge its JSON like `judge`.
fn judge_cli(fx: &Fx, what: &str, repo: &Path, rev: &str, env: &[(&str, String)], expect: &Expect) {
    let mut c = std::process::Command::new(env!("CARGO_BIN_EXE_work-engine-register"));
    c.args(["snapshot", "--at", rev, "--register", "reg", "--repo"])
        .arg(repo)
        .env_clear()
        .env("PATH", "/usr/bin:/bin");
    for (k, v) in env {
        c.env(k, v);
    }
    let out = c.output().unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let hits = fx.take_hits();
    assert!(
        hits.is_empty(),
        "SECURITY: {what}: commands executed during the read: {hits:?}"
    );
    match (v["outcome"].as_str().unwrap(), expect) {
        ("ok", Expect::Truth(t, sha)) => {
            let mut got = Listing::new();
            let mut got_sha = Shas::new();
            for e in v["entries"].as_array().unwrap() {
                let p = e["path"].as_str().unwrap().to_string();
                got.insert(
                    p.clone(),
                    (
                        e["kind"].as_str().unwrap().into(),
                        e["id"].as_str().unwrap().into(),
                    ),
                );
                if let Some(h) = e["sha256"].as_str() {
                    got_sha.insert(p, h.into());
                }
            }
            assert_eq!(
                &got, t,
                "SECURITY: {what}: delivered entries differ from the commit's true objects"
            );
            assert_eq!(
                &got_sha, sha,
                "SECURITY: {what}: delivered file bytes differ from the true objects"
            );
        }
        ("ok", Expect::Refuse(cat)) => {
            panic!("SECURITY: {what}: expected refusal ({cat}) but a snapshot was delivered")
        }
        (other, Expect::Truth(..)) => panic!(
            "BEHAVIOR: {what}: expected the true objects, got {other}: {}",
            v["error"]
        ),
        (other, Expect::Refuse(cat)) => assert_eq!(
            other, *cat,
            "BEHAVIOR: {what}: wrong refusal category: {}",
            v["error"]
        ),
    }
}

fn none(_: &Fx, _: &Path) {}

fn unhex(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

fn writable(p: &Path) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o644)).unwrap();
}

#[test]
fn behavior_packed_objects() {
    let fx = Fx::new("packed");
    let (r, c1, _) = fx.base("repo", &none);
    let t = truth(&fx, &r, &c1);
    fx.git(&r, &["gc", "-q"]);
    fx.git(&r, &["prune"]);
    judge(&fx, "packed", read(&r, &c1, "reg"), &t);
}

#[test]
fn security_working_tree_index_and_symlinked_register_path_are_never_read() {
    let fx = Fx::new("worktree");
    let (r, c1, _) = fx.base("repo", &none);
    let t = truth(&fx, &r, &c1);
    write(&r.join("reg/decisions.json"), "{\"dirty\": 1}\n");
    fx.git(&r, &["add", "reg/decisions.json"]);
    write(&r.join("reg/events.jsonl"), "junk\n");
    write(&r.join("reg/new.json"), "{}\n");
    judge(
        &fx,
        "dirty working tree and index",
        read(&r, &c1, "reg"),
        &t,
    );
    let elsewhere = fx.root.join("elsewhere");
    write(&elsewhere.join("decisions.json"), "{\"elsewhere\": 1}\n");
    std::fs::remove_dir_all(r.join("reg")).unwrap();
    std::os::unix::fs::symlink(&elsewhere, r.join("reg")).unwrap();
    judge(
        &fx,
        "working-tree register path is a symlink",
        read(&r, &c1, "reg"),
        &t,
    );
}

#[test]
fn security_replace_refs_do_not_change_what_is_read() {
    for what in ["blob", "tree", "commit"] {
        let fx = Fx::new(&format!("replace-{what}"));
        let (r, c1, c2) = fx.base("repo", &none);
        let t = truth(&fx, &r, &c1);
        let (old, new) = match what {
            "blob" => (
                fx.git(&r, &["rev-parse", &format!("{c1}:reg/decisions.json")]),
                fx.git_with(
                    &r,
                    &["hash-object", "-w", "--stdin"],
                    Some(b"{\"forged\": true}\n"),
                    &[],
                ),
            ),
            "tree" => (
                fx.git(&r, &["rev-parse", &format!("{c1}:reg")]),
                fx.git(&r, &["rev-parse", &format!("{c2}:reg")]),
            ),
            _ => (c1.clone(), c2.clone()),
        };
        fx.git(&r, &["replace", "-f", &old, &new]);
        let forged = fx.git(&r, &["cat-file", "-p", &format!("{c1}:reg/decisions.json")]);
        assert_ne!(
            forged, "{\"v\": 1}",
            "CONTROL: replace-{what}: git CLI does not substitute"
        );
        judge(
            &fx,
            &format!("replace ref on {what}"),
            read(&r, &c1, "reg"),
            &t,
        );
        for setting in ["true", "false"] {
            fx.git(&r, &["config", "core.useReplaceRefs", setting]);
            judge(
                &fx,
                &format!("replace ref on {what}, repository core.useReplaceRefs={setting}"),
                read(&r, &c1, "reg"),
                &t,
            );
        }
    }
}

#[test]
fn security_swapped_loose_object_file_is_refused() {
    let fx = Fx::new("swap");
    let (r, c1, _) = fx.base("repo", &none);
    let target = fx.git(&r, &["rev-parse", &format!("{c1}:reg/decisions.json")]);
    let donor = fx.git(&r, &["rev-parse", &format!("{c1}:other.txt")]);
    let path = fx.loose(&r, &target);
    std::fs::remove_file(&path).unwrap();
    std::fs::copy(fx.loose(&r, &donor), &path).unwrap();
    let control = fx.git_bytes(&r, &["cat-file", "blob", &target]);
    assert_eq!(
        control, b"x\n",
        "CONTROL: the swapped file is served for the target id"
    );
    judge(
        &fx,
        "loose object file swapped for another object's",
        read(&r, &c1, "reg"),
        &Expect::Refuse("cannot_read"),
    );
}

#[test]
fn security_object_type_mismatch_is_refused() {
    let fx = Fx::new("type");
    let (r, c1, _) = fx.base("repo", &none);
    let some_tree = fx.git(&r, &["rev-parse", &format!("{c1}:reg/studies")]);
    let some_blob = fx.git(&r, &["rev-parse", &format!("{c1}:other.txt")]);
    let entry = |mode: &str, name: &str, id: &str| {
        [format!("{mode} {name}\0").into_bytes(), unhex(id)].concat()
    };
    let literal_tree = |bytes: Vec<u8>| {
        fx.git_with(
            &r,
            &["hash-object", "-t", "tree", "-w", "--literally", "--stdin"],
            Some(&bytes),
            &[],
        )
    };
    let reg = literal_tree(
        [
            entry("100644", "bad.json", &some_tree),
            entry("100644", "ok.json", &some_blob),
        ]
        .concat(),
    );
    let root = literal_tree(entry("40000", "reg", &reg));
    let commit = fx.git(&r, &["commit-tree", &root, "-m", "type mismatch"]);
    judge(
        &fx,
        "tree entry says file but names a tree",
        read(&r, &commit, "reg"),
        &Expect::Refuse("cannot_read"),
    );
}

#[test]
fn security_grafts_and_forged_commit_graph_change_neither_content_nor_ancestry() {
    let fx = Fx::new("history");
    let (r, c1, c2) = fx.base("repo", &none);
    let (t1, t2) = (truth(&fx, &r, &c1), truth(&fx, &r, &c2));
    std::fs::write(r.join(".git/info/grafts"), format!("{c2}\n")).unwrap();
    let parents = fx.git(&r, &["rev-list", "--parents", "-n1", &c2]);
    assert_eq!(
        parents, c2,
        "CONTROL: grafts do not rewrite parents under the git CLI"
    );
    judge(&fx, "grafts", read(&r, &c2, "reg"), &t2);
    let o = GitObjects::open(&r).unwrap();
    let (anc, desc) = (o.resolve(&c1).unwrap(), o.resolve(&c2).unwrap());
    assert_eq!(
        o.is_ancestor(anc, desc).unwrap(),
        Ancestry::Yes,
        "SECURITY: grafts changed ancestry"
    );
    std::fs::remove_file(r.join(".git/info/grafts")).unwrap();

    fx.git(&r, &["commit-graph", "write", "--reachable"]);
    let p = r.join(".git/objects/info/commit-graph");
    let mut b = std::fs::read(&p).unwrap();
    assert_eq!(&b[..4], b"CGPH");
    let mut table = std::collections::HashMap::new();
    for i in 0..=(b[6] as usize) {
        let off = 8 + 12 * i;
        table.insert(
            b[off..off + 4].to_vec(),
            u64::from_be_bytes(b[off + 4..off + 12].try_into().unwrap()) as usize,
        );
    }
    let (fanout, oidl, cdat) = (
        table[&b"OIDF".to_vec()],
        table[&b"OIDL".to_vec()],
        table[&b"CDAT".to_vec()],
    );
    let n = u32::from_be_bytes(b[fanout + 1020..fanout + 1024].try_into().unwrap()) as usize;
    let position = |hex: &str| {
        (0..n)
            .find(|i| b[oidl + 20 * i..oidl + 20 * i + 20] == unhex(hex)[..])
            .unwrap()
    };
    let (i1, i2) = (position(&c1), position(&c2));
    let c2_tree = fx.git(&r, &["rev-parse", &format!("{c2}^{{tree}}")]);
    b[cdat + 36 * i1..cdat + 36 * i1 + 20].copy_from_slice(&unhex(&c2_tree));
    b[cdat + 36 * i2 + 20..cdat + 36 * i2 + 24].copy_from_slice(&0x7000_0000u32.to_be_bytes());
    writable(&p);
    std::fs::write(&p, &b).unwrap();
    assert_eq!(
        fx.git(&r, &["log", "-1", "--format=%T", &c1]),
        c2_tree,
        "CONTROL: forged commit-graph tree not used by git log"
    );
    judge(&fx, "forged commit-graph", read(&r, &c1, "reg"), &t1);
    let o = GitObjects::open(&r).unwrap();
    assert_eq!(
        o.is_ancestor(anc, desc).unwrap(),
        Ancestry::Yes,
        "SECURITY: forged commit-graph changed ancestry"
    );
}

#[test]
fn security_missing_history_is_cannot_establish_not_no() {
    let fx = Fx::new("missing-parent");
    let (r, c1, c2) = fx.base("repo", &none);
    write(&r.join("reg/decisions.json"), "{\"v\": 3}\n");
    fx.git(&r, &["commit", "-qam", "c3"]);
    let c3 = fx.git(&r, &["rev-parse", "HEAD"]);
    let tree = fx.git(&r, &["rev-parse", &format!("{c1}^{{tree}}")]);
    let unrelated = fx.git(&r, &["commit-tree", &tree, "-m", "unrelated"]);
    std::fs::remove_file(fx.loose(&r, &c1)).unwrap();
    let o = GitObjects::open(&r).unwrap();
    let id = |h: &str| {
        o.resolve_prefix(h)
            .map(|p| match p {
                research_register::Prefix::Unique(i) => i,
                other => panic!("{other:?}"),
            })
            .unwrap()
    };
    let (c2, c3, unrelated) = (id(&c2), id(&c3), id(&unrelated));
    assert_eq!(o.is_ancestor(c2, c3).unwrap(), Ancestry::Yes);
    assert_eq!(
        o.is_ancestor(unrelated, c3).unwrap(),
        Ancestry::CannotEstablish,
        "SECURITY: incomplete history reported as definite"
    );
    assert_eq!(o.is_ancestor(c3, unrelated).unwrap(), Ancestry::No);
}

#[test]
fn security_shallow_repository_is_refused() {
    let fx = Fx::new("shallow");
    let (src, _, _) = fx.base("src", &none);
    let dst = fx.root.join("shallow");
    fx.git(
        &fx.root,
        &[
            "clone",
            "-q",
            "--depth",
            "1",
            &format!("file://{}", src.display()),
            dst.to_str().unwrap(),
        ],
    );
    judge(
        &fx,
        "shallow clone",
        read(&dst, "HEAD", "reg"),
        &Expect::Refuse("unsupported"),
    );
}

#[test]
fn security_partial_clone_missing_blob_is_never_fetched() {
    let fx = Fx::new("partial");
    let (src, c1, _) = fx.base("src", &none);
    fx.git(&src, &["config", "uploadpack.allowFilter", "true"]);
    let dst = fx.root.join("partial");
    fx.git(
        &fx.root,
        &[
            "clone",
            "-q",
            "--filter=blob:none",
            "--no-checkout",
            &format!("file://{}", src.display()),
            dst.to_str().unwrap(),
        ],
    );
    let exec_path = fx.git(&dst, &["--exec-path"]);
    let marker = fx.marker("upload-pack", Some(&format!("{exec_path}/git-upload-pack")));
    fx.git(&dst, &["config", "remote.origin.uploadpack", &marker]);
    judge(
        &fx,
        "partial clone, missing blob",
        read(&dst, &c1, "reg"),
        &Expect::Refuse("cannot_read"),
    );
    let _ = fx.git_try(
        &dst,
        &["cat-file", "-p", &format!("{c1}:reg/decisions.json")],
        &[],
    );
    assert!(
        fx.take_hits().contains(&"upload-pack".to_string()),
        "CONTROL: git CLI did not lazily fetch"
    );
}

#[test]
fn security_hostile_repository_config_runs_nothing() {
    let fx = Fx::new("hostile-config");
    let (r, c1, _) = fx.base("repo", &|_, r| {
        write(&r.join(".gitattributes"), "*.json filter=evil diff=evil\n")
    });
    let t = truth(&fx, &r, &c1);
    let hooks = fx.root.join("hooks");
    std::fs::create_dir_all(&hooks).unwrap();
    for hook in [
        "post-checkout",
        "pre-commit",
        "post-index-change",
        "reference-transaction",
    ] {
        std::fs::copy(fx.marker(&format!("hook-{hook}"), None), hooks.join(hook)).unwrap();
    }
    let include = fx.root.join("included.cfg");
    std::fs::write(
        &include,
        format!("[core]\n\tpager = {}\n", fx.marker("pager", None)),
    )
    .unwrap();
    let settings = [
        ("core.fsmonitor", fx.marker("fsmonitor", None)),
        ("core.hooksPath", hooks.display().to_string()),
        ("filter.evil.smudge", fx.marker("smudge", None)),
        ("filter.evil.clean", fx.marker("clean", None)),
        ("filter.evil.process", fx.marker("process", None)),
        ("diff.evil.textconv", fx.marker("textconv", None)),
        ("core.sshCommand", fx.marker("ssh", None)),
        ("core.alternateRefsCommand", fx.marker("altrefs", None)),
        ("credential.helper", fx.marker("cred", None)),
        ("include.path", include.display().to_string()),
        ("gpg.program", fx.marker("gpg", None)),
        ("core.editor", fx.marker("editor", None)),
    ];
    for (k, v) in &settings {
        fx.git(&r, &["config", k, v]);
    }
    let _ = fx.git_try(&r, &["status"], &[]);
    assert!(
        !fx.take_hits().is_empty(),
        "CONTROL: git status ran no configured command"
    );
    judge(&fx, "hostile repository config", read(&r, &c1, "reg"), &t);
}

#[test]
fn security_environment_cannot_change_what_is_read() {
    let fx = Fx::new("env");
    let (r, c1, _) = fx.base("repo", &none);
    let t = truth(&fx, &r, &c1);
    let (other, _, _) = fx.base("other", &none);
    let forged = fx.git_with(
        &r,
        &["hash-object", "-w", "--stdin"],
        Some(b"{\"forged-by-env\": true}\n"),
        &[],
    );
    let decisions = fx.git(&r, &["rev-parse", &format!("{c1}:reg/decisions.json")]);
    fx.git(
        &r,
        &["update-ref", &format!("refs/evil/{decisions}"), &forged],
    );
    let cfg = fx.root.join("hostile.cfg");
    std::fs::write(
        &cfg,
        format!(
            "[core]\n\tfsmonitor = {}\n\tuseReplaceRefs = true\n",
            fx.marker("env-fsmonitor", None)
        ),
    )
    .unwrap();
    let home = fx.root.join("hostile-home");
    std::fs::create_dir_all(home.join(".config/git")).unwrap();
    std::fs::copy(&cfg, home.join(".gitconfig")).unwrap();
    std::fs::copy(&cfg, home.join(".config/git/config")).unwrap();
    let og = other.join(".git");
    let s = |p: &Path| p.display().to_string();
    let env: Vec<(&str, String)> = vec![
        ("GIT_DIR", s(&og)),
        ("GIT_WORK_TREE", s(&other)),
        ("GIT_COMMON_DIR", s(&og)),
        ("GIT_OBJECT_DIRECTORY", s(&og.join("objects"))),
        ("GIT_ALTERNATE_OBJECT_DIRECTORIES", s(&og.join("objects"))),
        ("GIT_REPLACE_REF_BASE", "refs/evil/".into()),
        ("GIT_CONFIG_GLOBAL", s(&cfg)),
        ("GIT_CONFIG_SYSTEM", s(&cfg)),
        ("HOME", s(&home)),
        ("XDG_CONFIG_HOME", s(&home.join(".config"))),
        ("GIT_CONFIG_COUNT", "1".into()),
        ("GIT_CONFIG_KEY_0", "core.useReplaceRefs".into()),
        ("GIT_CONFIG_VALUE_0", "true".into()),
        (
            "GIT_CONFIG_PARAMETERS",
            "'core.usereplacerefs'='true'".into(),
        ),
        ("GIT_NAMESPACE", "evil".into()),
        ("GIT_GRAFT_FILE", s(&fx.root.join("nope"))),
        ("GIT_SHALLOW_FILE", s(&fx.root.join("nope"))),
    ];
    let control_env: Vec<(&str, &str)> = vec![("GIT_REPLACE_REF_BASE", "refs/evil/")];
    let forged_view = fx.git_with(
        &r,
        &["cat-file", "-p", &format!("{c1}:reg/decisions.json")],
        None,
        &control_env,
    );
    assert!(
        forged_view.contains("forged-by-env"),
        "CONTROL: git CLI ignores the environment's replace base"
    );
    judge_cli(&fx, "hostile environment", &r, &c1, &env, &t);
}

#[test]
fn security_environment_alternate_cannot_supply_a_missing_object() {
    let fx = Fx::new("env-alternate");
    let (r, c1, _) = fx.base("repo", &none);
    let store = fx.root.join("store");
    std::fs::create_dir_all(&store).unwrap();
    let events = fx.git(&r, &["rev-parse", &format!("{c1}:reg/events.jsonl")]);
    let loose = fx.loose(&r, &events);
    let kept = store.join(&events[..2]);
    std::fs::create_dir_all(&kept).unwrap();
    std::fs::copy(&loose, kept.join(&events[2..])).unwrap();
    std::fs::remove_file(&loose).unwrap();
    let env = vec![
        (
            "GIT_ALTERNATE_OBJECT_DIRECTORIES",
            store.display().to_string(),
        ),
        ("HOME", fx.home.display().to_string()),
    ];
    let control = fx.git_with(
        &r,
        &["cat-file", "-p", &events],
        None,
        &[("GIT_ALTERNATE_OBJECT_DIRECTORIES", store.to_str().unwrap())],
    );
    assert_eq!(
        control, "{\"e\": 1}",
        "CONTROL: git CLI does not use the environment alternate"
    );
    judge_cli(
        &fx,
        "environment alternate supplies a missing object",
        &r,
        &c1,
        &env,
        &Expect::Refuse("cannot_read"),
    );
}

#[test]
fn security_missing_object_is_cannot_read() {
    let fx = Fx::new("missing");
    let (r, c1, _) = fx.base("repo", &none);
    let events = fx.git(&r, &["rev-parse", &format!("{c1}:reg/events.jsonl")]);
    std::fs::remove_file(fx.loose(&r, &events)).unwrap();
    judge(
        &fx,
        "missing loose object",
        read(&r, &c1, "reg"),
        &Expect::Refuse("cannot_read"),
    );
    let absent = "0123456789012345678901234567890123456789";
    judge(
        &fx,
        "unavailable commit id",
        read(&r, absent, "reg"),
        &Expect::Refuse("cannot_read"),
    );
}

#[test]
fn security_links_and_gitlinks_are_recorded_not_read() {
    let fx = Fx::new("links");
    let (r, c1, _) = fx.base("repo", &|fx, r| {
        std::os::unix::fs::symlink("../other.txt", r.join("reg/link.json")).unwrap();
        std::os::unix::fs::symlink("reg", r.join("reg2")).unwrap();
        std::fs::create_dir_all(r.join("reg/sub")).unwrap();
        fx.git(r, &["add", "-A"]);
        fx.git(
            r,
            &[
                "update-index",
                "--add",
                "--cacheinfo",
                &format!("160000,{},reg/sub", "1".repeat(40)),
            ],
        );
    });
    let t = truth(&fx, &r, &c1);
    let s = read(&r, &c1, "reg");
    if let Ok(s) = &s {
        for name in ["link.json", "sub"] {
            let e = s.entry(name).expect("entry recorded");
            assert!(
                e.bytes().is_none(),
                "SECURITY: {name} was read through as file content"
            );
        }
    }
    judge(&fx, "symlink and gitlink entries", s, &t);
    judge(
        &fx,
        "register path is a symlink in the tree",
        read(&r, &c1, "reg2"),
        &Expect::Refuse("no_register_at_commit"),
    );
}

#[test]
fn behavior_linked_worktree_and_annotated_tag() {
    let fx = Fx::new("refs");
    let (r, c1, _) = fx.base("repo", &none);
    let t = truth(&fx, &r, &c1);
    let wt: PathBuf = fx.root.join("linked");
    fx.git(
        &r,
        &[
            "worktree",
            "add",
            "-q",
            "--detach",
            wt.to_str().unwrap(),
            &c1,
        ],
    );
    judge(&fx, "linked worktree", read(&wt, "HEAD", "reg"), &t);
    fx.git(&r, &["tag", "-a", "t1", "-m", "x", &c1]);
    judge(&fx, "annotated tag", read(&r, "refs/tags/t1", "reg"), &t);
}

#[test]
fn security_sha256_repository_is_refused_as_unsupported() {
    let fx = Fx::new("sha256");
    let r = fx.root.join("repo");
    std::fs::create_dir_all(&r).unwrap();
    fx.git(&r, &["init", "-q", "--object-format=sha256", "-b", "main"]);
    write(&r.join("reg/decisions.json"), "{}\n");
    fx.git(&r, &["add", "-A"]);
    fx.git(&r, &["commit", "-q", "-m", "c1"]);
    let result = read(&r, "HEAD", "reg");
    assert!(
        matches!(
            &result,
            Err(Error::Unsupported(
                research_register::Unsupported::ObjectFormat { .. }
            ))
        ),
        "BEHAVIOR: sha256 refusal reason: {:?}",
        result.as_ref().err()
    );
    judge(
        &fx,
        "sha256 repository",
        result,
        &Expect::Refuse("unsupported"),
    );
}

// ---- A9 repairs (R-1b.2 repair commit) ----

/// A literal tree entry: mode, name, object id.
type Entry<'a> = (&'a str, &'a str, &'a str);

fn literal_tree(fx: &Fx, r: &Path, entries: &[Entry]) -> String {
    let mut bytes = Vec::new();
    for (mode, name, id) in entries {
        bytes.extend_from_slice(format!("{mode} {name}\0").as_bytes());
        bytes.extend_from_slice(gix::ObjectId::from_hex(id.as_bytes()).unwrap().as_bytes());
    }
    fx.git_with(
        r,
        &["hash-object", "-t", "tree", "-w", "--literally", "--stdin"],
        Some(&bytes),
        &[],
    )
}

fn commit_with_register(fx: &Fx, r: &Path, reg_tree: &str) -> String {
    let root = fx.git_with(
        r,
        &["mktree"],
        Some(format!("040000 tree {reg_tree}\treg\n").as_bytes()),
        &[],
    );
    fx.git(r, &["commit-tree", &root, "-m", "malformed"])
}

fn set_config_text(r: &Path, edit: &dyn Fn(String) -> String) {
    let p = r.join(".git/config");
    let t = edit(std::fs::read_to_string(&p).unwrap());
    std::fs::write(&p, t).unwrap();
}

#[test]
fn security_ancestry_yes_requires_a_verified_commit() {
    let fx = Fx::new("ancestry-self");
    let (r, c1, _) = fx.base("repo", &none);
    let o = GitObjects::open(&r).unwrap();
    let id = |h: &str| gix::ObjectId::from_hex(h.as_bytes()).unwrap();
    assert!(
        matches!(o.is_ancestor(id(&c1), id(&c1)), Ok(Ancestry::Yes)),
        "BEHAVIOR: a commit is its own ancestor"
    );
    let missing = id("0123456789abcdef0123456789abcdef01234567");
    assert!(
        matches!(
            o.is_ancestor(missing, missing),
            Ok(Ancestry::CannotEstablish)
        ),
        "SECURITY: ancestry of an absent object must be CannotEstablish"
    );
    let blob = id(&fx.git(&r, &["rev-parse", &format!("{c1}:reg/decisions.json")]));
    assert!(
        matches!(o.is_ancestor(blob, blob), Err(Error::CannotRead(_))),
        "SECURITY: ancestry asserted for a blob"
    );
}

#[test]
fn behavior_invalid_reference_name_is_bad_request() {
    let fx = Fx::new("bad-refname");
    let (r, _, _) = fx.base("repo", &none);
    for rev in ["refs/heads/main~1", "refs/heads/x..y", "refs/heads/ma:in"] {
        judge(
            &fx,
            &format!("revision {rev}"),
            read(&r, rev, "reg"),
            &Expect::Refuse("bad_request"),
        );
    }
}

#[test]
fn behavior_repository_format_follows_git_config_grammar() {
    let fx = Fx::new("format-grammar");
    let r = fx.root.join("sha256");
    std::fs::create_dir_all(&r).unwrap();
    fx.git(&r, &["init", "-q", "--object-format=sha256", "-b", "main"]);
    write(&r.join("reg/decisions.json"), "{}\n");
    fx.git(&r, &["add", "-A"]);
    fx.git(&r, &["commit", "-q", "-m", "c1"]);
    set_config_text(&r, &|t| {
        t.replace(
            "[extensions]\n\tobjectformat = sha256",
            "[extensions] objectformat = sha256",
        )
    });
    assert_eq!(
        fx.git(&r, &["rev-parse", "--show-object-format"]),
        "sha256",
        "CONTROL: git reads the one-line declaration"
    );
    judge(
        &fx,
        "sha256 declared on one line",
        read(&r, "HEAD", "reg"),
        &Expect::Refuse("unsupported"),
    );

    let (r, c1, _) = fx.base("sha1-comment", &none);
    let t = truth(&fx, &r, &c1);
    set_config_text(&r, &|t| {
        t.replace("repositoryformatversion = 0", "repositoryformatversion = 1")
            + "[extensions]\n\tobjectformat = sha1 ; comment\n"
    });
    assert_eq!(
        fx.git(&r, &["rev-parse", "--show-object-format"]),
        "sha1",
        "CONTROL: git reads sha1 despite the comment"
    );
    judge(&fx, "sha1 with an inline comment", read(&r, &c1, "reg"), &t);
}

#[test]
fn behavior_unsupported_repository_formats_are_refused_as_unsupported() {
    let fx = Fx::new("formats");
    let r = fx.root.join("reftable");
    std::fs::create_dir_all(&r).unwrap();
    fx.git(&r, &["init", "-q", "--ref-format=reftable", "-b", "main"]);
    write(&r.join("reg/decisions.json"), "{}\n");
    fx.git(&r, &["add", "-A"]);
    fx.git(&r, &["commit", "-q", "-m", "c1"]);
    assert_eq!(
        fx.git(&r, &["rev-parse", "--show-ref-format"]),
        "reftable",
        "CONTROL: reftable repository"
    );
    for rev in ["refs/heads/main", "HEAD"] {
        judge(
            &fx,
            &format!("reftable {rev}"),
            read(&r, rev, "reg"),
            &Expect::Refuse("unsupported"),
        );
    }
    for (name, edit) in [
        (
            "unknown extension at version 1",
            &(|t: String| {
                t.replace("repositoryformatversion = 0", "repositoryformatversion = 1")
                    + "[extensions]\n\tbogusext = 1\n"
            }) as &dyn Fn(String) -> String,
        ),
        ("format version 2", &|t: String| {
            t.replace("repositoryformatversion = 0", "repositoryformatversion = 2")
        }),
    ] {
        let (r, c1, _) = fx.base(&name.replace(' ', "-"), &none);
        set_config_text(&r, edit);
        assert!(
            !fx.git_try(&r, &["rev-parse", "HEAD"], &[]).0,
            "CONTROL: git accepts the repository with {name}"
        );
        judge(
            &fx,
            name,
            read(&r, &c1, "reg"),
            &Expect::Refuse("unsupported"),
        );
    }
}

#[test]
fn security_malformed_register_tree_entries_are_cannot_read() {
    let fx = Fx::new("malformed-tree");
    let (r, _, _) = fx.base("repo", &none);
    let b1 = fx.git_with(
        &r,
        &["hash-object", "-w", "--stdin"],
        Some(b"{\"v\": 1}\n"),
        &[],
    );
    let b2 = fx.git_with(
        &r,
        &["hash-object", "-w", "--stdin"],
        Some(b"{\"v\": 2}\n"),
        &[],
    );
    let cases: [(&str, Vec<Entry>); 2] = [
        (
            "duplicate entry names",
            vec![("100644", "a.json", &b1), ("100644", "a.json", &b2)],
        ),
        (
            "entry name containing '/'",
            vec![("100644", "a/b.json", &b1)],
        ),
    ];
    for (what, entries) in cases {
        let tree = literal_tree(&fx, &r, &entries);
        let c = commit_with_register(&fx, &r, &tree);
        if what.starts_with("duplicate") {
            let out = fx
                .cmd(&r, &["fsck", "--no-dangling"], &[])
                .output()
                .unwrap();
            assert!(
                String::from_utf8_lossy(&out.stderr).contains("duplicateEntries"),
                "CONTROL: git fsck does not report the duplicate"
            );
            let o = GitObjects::open(&r).unwrap();
            let cid = gix::ObjectId::from_hex(c.as_bytes()).unwrap();
            assert!(
                matches!(o.blob_at(cid, "reg/a.json"), Err(Error::CannotRead(_))),
                "SECURITY: blob_at read one of two duplicate entries"
            );
        }
        judge(
            &fx,
            what,
            read(&r, &c, "reg"),
            &Expect::Refuse("cannot_read"),
        );
    }
}

/// R-1b.4 item 1 (owner decision D-NAMES): empty, `.`, `..` and `.git`-equivalent entry names
/// are refused when a tree is parsed, before any path is formed, at the register root, nested,
/// and above the register path; look-alike names are read normally.
#[test]
fn security_unsafe_tree_entry_names_are_cannot_read() {
    let fx = Fx::new("unsafe-names");
    let (r, _, _) = fx.base("repo", &none);
    let blob = |text: &str| {
        fx.git_with(
            &r,
            &["hash-object", "-w", "--stdin"],
            Some(text.as_bytes()),
            &[],
        )
    };
    let (b1, b2) = (blob("{\"v\": 1}\n"), blob("{\"v\": 2}\n"));
    let refuse = |what: &str, commit: &str| {
        judge(
            &fx,
            what,
            read(&r, commit, "reg"),
            &Expect::Refuse("cannot_read"),
        )
    };
    let unsafe_names = [
        "",
        ".",
        "..",
        ".git",
        ".GIT",
        "git~1",
        ".git.",
        ".git ",
        ".g\u{200c}it",
        "a\\.git",
    ];
    for name in unsafe_names {
        for mode in ["100644", "040000"] {
            let id = if mode == "100644" {
                b2.clone()
            } else {
                literal_tree(&fx, &r, &[("100644", "decisions.json", &b2)])
            };
            // At the register root, beside an ordinary file.
            let reg = literal_tree(
                &fx,
                &r,
                &[("100644", "decisions.json", &b1), (mode, name, &id)],
            );
            refuse(
                &format!("{name:?} ({mode}) at the register root"),
                &commit_with_register(&fx, &r, &reg),
            );
            // Nested one level down.
            let sub = literal_tree(&fx, &r, &[(mode, name, &id)]);
            let reg = literal_tree(
                &fx,
                &r,
                &[("100644", "decisions.json", &b1), ("040000", "sub", &sub)],
            );
            refuse(
                &format!("{name:?} ({mode}) nested"),
                &commit_with_register(&fx, &r, &reg),
            );
        }
    }
    // Fable E1/E3 and the coordinator's f1 case: an empty-named subtree whose children would
    // become register-root keys, replacing (both orders) or supplying decisions.json.
    let inner = literal_tree(&fx, &r, &[("100644", "decisions.json", &b2)]);
    let (b1s, inner) = (b1.as_str(), inner.as_str());
    for (what, entries) in [
        (
            "empty-named subtree after decisions.json",
            vec![("100644", "decisions.json", b1s), ("040000", "", inner)],
        ),
        (
            "empty-named subtree before decisions.json",
            vec![("040000", "", inner), ("100644", "decisions.json", b1s)],
        ),
        (
            "empty-named subtree supplies decisions.json",
            vec![("100644", "events.jsonl", b1s), ("040000", "", inner)],
        ),
    ] {
        let reg = literal_tree(&fx, &r, &entries);
        refuse(what, &commit_with_register(&fx, &r, &reg));
    }
    // Fable E4: an unsafe name above the register path (in the commit's root tree).
    let reg = literal_tree(&fx, &r, &[("100644", "decisions.json", &b1)]);
    for name in ["", ".git"] {
        let root = literal_tree(&fx, &r, &[("040000", name, &reg), ("040000", "reg", &reg)]);
        let c = fx.git(&r, &["commit-tree", &root, "-m", "unsafe root"]);
        refuse(&format!("{name:?} in the root tree"), &c);
    }
    // Controls: names that only look like the refused ones are ordinary entries.
    let entries: Vec<Entry> = [".gitx", "git", "git~2", ".git~1", "x.git", "..."]
        .into_iter()
        .map(|n| ("100644", n, b1.as_str()))
        .collect();
    let reg = literal_tree(&fx, &r, &entries);
    let s = read(&r, &commit_with_register(&fx, &r, &reg), "reg")
        .expect("BEHAVIOR: look-alike names are refused");
    assert_eq!(
        s.entries().keys().cloned().collect::<Vec<_>>(),
        vec!["...", ".gitx", ".git~1", "git", "git~2", "x.git"],
        "BEHAVIOR: look-alike names"
    );
}

#[test]
fn security_namespace_settings_cannot_redirect_a_reference() {
    let fx = Fx::new("namespace");
    let (r, c1, c2) = fx.base("repo", &none);
    let t = truth(&fx, &r, &c2);
    fx.git(
        &r,
        &["update-ref", "refs/namespaces/ns/refs/heads/main", &c1],
    );
    assert_eq!(
        fx.git(&r, &["rev-parse", "refs/namespaces/ns/refs/heads/main"]),
        c1,
        "CONTROL: the namespaced ref names another commit"
    );
    let home = fx.root.join("ns-home");
    write(
        &home.join(".gitconfig"),
        "[gitoxide \"core\"]\n\trefsNamespace = ns\n",
    );
    judge_cli(
        &fx,
        "GIT_NAMESPACE",
        &r,
        "refs/heads/main",
        &[("GIT_NAMESPACE", "ns".into())],
        &t,
    );
    judge_cli(
        &fx,
        "user config refsNamespace",
        &r,
        "refs/heads/main",
        &[("HOME", home.display().to_string())],
        &t,
    );
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        let target = to.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            copy_dir(&e.path(), &target);
        } else {
            std::fs::copy(e.path(), target).unwrap();
        }
    }
}

const UNSUPPORTED_FORMATS: [(&str, &str); 4] = [
    (
        "unknown extension",
        "[core]\n\trepositoryformatversion = 1\n[extensions]\n\tbogusext = 1\n",
    ),
    (
        "reftable",
        "[core]\n\trepositoryformatversion = 1\n[extensions]\n\trefstorage = reftable\n",
    ),
    (
        "format version 2",
        "[core]\n\trepositoryformatversion = 2\n",
    ),
    (
        "sha256",
        "[core]\n\trepositoryformatversion = 1\n[extensions]\n\tobjectformat = sha256\n",
    ),
];

/// R-1b.4 item 7 (Fable F-4): the format check reads the configuration of the repository gix
/// actually opens, in every layout, so a decoy `.git/` or an odd gitfile cannot hide it.
#[test]
fn security_repository_format_is_checked_where_gix_opens() {
    let fx = Fx::new("format-layouts");
    for (k, (name, config)) in UNSUPPORTED_FORMATS.iter().enumerate() {
        let (r, c1, _) = fx.base(&format!("repo-{k}"), &none);
        std::fs::write(r.join(".git/config"), config).unwrap();
        let expect = |what: &str, path: &Path| {
            let res = GitObjects::open(path).and_then(|o| PinnedSnapshot::build(o, &c1, "reg"));
            judge(
                &fx,
                &format!("{what}, {name}"),
                res,
                &Expect::Refuse("unsupported"),
            );
        };
        expect("worktree path", &r);
        // D1: the path is itself a git directory and holds a non-repository .git/ of its own.
        let bare = fx.root.join(format!("d1-{k}"));
        copy_dir(&r.join(".git"), &bare);
        expect("git directory path", &bare);
        std::fs::create_dir_all(bare.join(".git")).unwrap();
        expect("git directory path with an empty .git/ inside", &bare);
        write(
            &bare.join(".git/config"),
            "[core]\n\trepositoryformatversion = 0\n",
        );
        expect("git directory path with a decoy .git/config", &bare);
        // D2: the path names W/.git, which holds a decoy .git/ of its own.
        std::fs::create_dir_all(r.join(".git/.git")).unwrap();
        expect("W/.git with W/.git/.git/", &r.join(".git"));
        // D3: a gitfile whose target keeps a leading space ("gitdir:  realgit").
        let w = fx.root.join(format!("d3-{k}"));
        copy_dir(&r.join(".git"), &w.join(" realgit"));
        std::fs::write(w.join(".git"), "gitdir:  realgit\n").unwrap();
        expect("gitfile with two spaces", &w);
    }
}

/// R-1b.4 item 6 (Fable F-2, F-3): repository-local `gitoxide.*` keys (which git ignores) are
/// refused, in `config` and in `config.worktree`; shallowness comes from the shallow file as git
/// reads it, never from configuration.
#[test]
fn security_repository_gitoxide_keys_and_shallow_file() {
    let fx = Fx::new("gitoxide-keys");
    let refuse = |what: &str, r: &Path, rev: &str, cat: &'static str| {
        judge(&fx, what, read(r, rev, "reg"), &Expect::Refuse(cat))
    };
    // C1
    let (r, c1, _) = fx.base("c1", &none);
    set_config_text(&r, &|t| t + "[gitoxide \"core\"]\n\trefsNamespace = ns\n");
    refuse("C1 local refsNamespace", &r, &c1, "unsupported");
    // C9 (worktree configuration)
    let (r, c1, _) = fx.base("c9", &none);
    set_config_text(&r, &|t| {
        t.replace("repositoryformatversion = 0", "repositoryformatversion = 1")
            + "[extensions]\n\tworktreeConfig = true\n"
    });
    write(
        &r.join(".git/config.worktree"),
        "[gitoxide \"core\"]\n\trefsNamespace = ns\n",
    );
    refuse("C9 config.worktree refsNamespace", &r, &c1, "unsupported");
    // C2: a real shallow file hidden by gitoxide.core.shallowFile, then the file alone.
    let (r, c1, c2) = fx.base("c2", &none);
    std::fs::write(r.join(".git/shallow"), format!("{c1}\n")).unwrap();
    assert_eq!(
        fx.git(&r, &["rev-parse", "--is-shallow-repository"]),
        "true",
        "CONTROL"
    );
    set_config_text(&r, &|t| {
        t + "[gitoxide \"core\"]\n\tshallowFile = no-such-file\n"
    });
    refuse(
        "C2 shallowFile hides the shallow file",
        &r,
        &c2,
        "unsupported",
    );
    set_config_text(&r, &|t| {
        t.replace("[gitoxide \"core\"]\n\tshallowFile = no-such-file\n", "")
    });
    refuse("shallow file", &r, &c2, "unsupported");
    std::fs::write(r.join(".git/shallow"), "").unwrap();
    assert_eq!(
        fx.git(&r, &["rev-parse", "--is-shallow-repository"]),
        "true",
        "CONTROL: git treats an empty shallow file as shallow"
    );
    refuse("empty shallow file", &r, &c2, "unsupported");
    std::fs::remove_file(r.join(".git/shallow")).unwrap();
    std::fs::create_dir(r.join(".git/shallow")).unwrap();
    refuse("shallow state unreadable", &r, &c2, "cannot_read");
    std::fs::remove_dir(r.join(".git/shallow")).unwrap();
    judge(&fx, "control", read(&r, &c2, "reg"), &truth(&fx, &r, &c2));
}

/// R-1b.4 item 7 (Fable F-5, F-6): only the exact reference name is resolved, and names over
/// the documented length limits are `bad_request` before any lookup.
#[test]
fn security_exact_reference_names_and_length_limits() {
    let fx = Fx::new("exact-refs");
    let (r, c1, c2) = fx.base("repo", &none);
    for d in [
        "heads/refs/heads/zzz",
        "tags/refs/heads/zzz",
        "remotes/refs/heads/zzz",
        "remotes/refs/heads/zzy/HEAD",
        "refs/remotes/refs/heads/zzy/HEAD",
        "refs/tags/refs/heads/zzx",
    ] {
        write(&r.join(".git").join(d), &format!("{c1}\n"));
    }
    let o = GitObjects::open(&r).unwrap();
    let category = |rev: &str| match o.resolve(rev) {
        Ok(id) => format!("ok {id}"),
        Err(e) => e.category().to_owned(),
    };
    for name in ["refs/heads/zzz", "refs/heads/zzy", "refs/heads/zzx"] {
        assert_eq!(
            category(name),
            "bad_request",
            "SECURITY: {name} resolved through a decoy"
        );
    }
    assert_eq!(category("refs/heads/main"), format!("ok {c2}"));
    let long = |n: usize| format!("refs/heads/{}", "a".repeat(n));
    assert_eq!(
        category(&long(255)),
        "bad_request",
        "at the component limit: does not exist"
    );
    assert_eq!(category(&long(256)), "bad_request");
    assert_eq!(category(&long(5000)), "bad_request");
    let deep = format!("refs/{}", vec!["b".repeat(200); 21].join("/"));
    assert!(deep.len() > 4096);
    assert_eq!(category(&deep), "bad_request");
}

/// R-1b.4 item 8 (Fable F-9, survivors S1-S3): an object file holding another object's content
/// is refused for commits, trees and tags, not only blobs.
#[test]
fn security_substituted_commit_tree_and_tag_objects_are_cannot_read() {
    let fx = Fx::new("substituted");
    let swap = |r: &Path, from: &str, onto: &str| {
        let (src, dst) = (fx.loose(r, from), fx.loose(r, onto));
        std::fs::remove_file(&dst).unwrap();
        std::fs::copy(src, dst).unwrap();
    };
    let (r, c1, c2) = fx.base("commit", &none);
    swap(&r, &c1, &c2);
    judge(
        &fx,
        "commit substituted",
        read(&r, &c2, "reg"),
        &Expect::Refuse("cannot_read"),
    );
    let (r, c1, c2) = fx.base("tree", &none);
    let (t1, t2) = (
        fx.tree_id(&r, &format!("{c1}^{{tree}}")),
        fx.tree_id(&r, &format!("{c2}^{{tree}}")),
    );
    assert!(t1 != t2 && t1 != c1, "CONTROL: two distinct root trees");
    swap(&r, &t1, &t2);
    judge(
        &fx,
        "root tree substituted",
        read(&r, &c2, "reg"),
        &Expect::Refuse("cannot_read"),
    );
    let (r, c1, c2) = fx.base("tag", &none);
    fx.git(&r, &["tag", "-a", "-m", "one", "t1", &c1]);
    fx.git(&r, &["tag", "-a", "-m", "two", "t2", &c2]);
    let (g1, g2) = (
        fx.git(&r, &["rev-parse", "refs/tags/t1"]),
        fx.git(&r, &["rev-parse", "refs/tags/t2"]),
    );
    swap(&r, &g1, &g2);
    judge(
        &fx,
        "tag substituted",
        read(&r, "refs/tags/t2", "reg"),
        &Expect::Refuse("cannot_read"),
    );
}

/// R-1b.4 item 8 (survivor S15): the register tree nesting limit, pinned at its value (32).
#[test]
fn security_tree_nesting_limit_boundary() {
    let fx = Fx::new("depth");
    let (r, _, _) = fx.base("repo", &none);
    let b = fx.git_with(&r, &["hash-object", "-w", "--stdin"], Some(b"x\n"), &[]);
    let chain = |levels: usize| {
        let mut t = literal_tree(&fx, &r, &[("100644", "f", &b)]);
        for _ in 0..levels {
            t = literal_tree(&fx, &r, &[("040000", "d", &t)]);
        }
        commit_with_register(&fx, &r, &t)
    };
    let s = read(&r, &chain(32), "reg").expect("BEHAVIOR: 32 levels are within the limit");
    assert!(s.entry(&format!("{}f", "d/".repeat(32))).is_some());
    judge(
        &fx,
        "33 levels",
        read(&r, &chain(33), "reg"),
        &Expect::Refuse("cannot_read"),
    );
}
