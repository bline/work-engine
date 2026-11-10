mod support;

use research_register::{
    Ancestry, BadRequest, Error, GitObjects, NoRegister, PinnedSnapshot, Prefix,
};
use support::{Fx, listing, write};

fn snapshot(repo: &std::path::Path, rev: &str) -> Result<PinnedSnapshot, Error> {
    PinnedSnapshot::build(GitObjects::open(repo)?, rev, "reg")
}

#[test]
fn behavior_snapshot_matches_git_listing_by_id_branch_and_head() {
    let fx = Fx::new("basic");
    let (r, c1, c2) = fx.base("repo", &|_, _| {});
    let s1 = snapshot(&r, &c1).unwrap();
    assert_eq!(s1.commit().to_string(), c1);
    assert_eq!(
        s1.register_tree().to_string(),
        fx.tree_id(&r, &format!("{c1}:reg"))
    );
    assert_eq!(
        s1.root_tree().to_string(),
        fx.tree_id(&r, &format!("{c1}^{{tree}}"))
    );
    assert_eq!(listing(&s1), fx.truth(&r, &c1, "reg"));
    assert_eq!(
        s1.entry("decisions.json").unwrap().bytes().unwrap(),
        b"{\"v\": 1}\n"
    );
    assert!(
        s1.entry("studies/s2.json").is_none(),
        "C1 must not contain C2-only content"
    );
    for rev in ["refs/heads/main", "HEAD"] {
        let s = snapshot(&r, rev).unwrap();
        assert_eq!(s.commit().to_string(), c2);
        assert_eq!(listing(&s), fx.truth(&r, &c2, "reg"));
    }
}

#[test]
fn behavior_cli_output_is_deterministic() {
    let fx = Fx::new("cli");
    let (r, c1, _) = fx.base("repo", &|_, _| {});
    let run = |args: &[&str]| {
        std::process::Command::new(env!("CARGO_BIN_EXE_work-engine-register"))
            .args(args)
            .output()
            .unwrap()
    };
    let repo = r.display().to_string();
    let a = run(&[
        "snapshot",
        "--at",
        &c1,
        "--register",
        "reg",
        "--repo",
        &repo,
    ]);
    let b = run(&[
        "snapshot",
        "--repo",
        &repo,
        "--register",
        "reg",
        "--at",
        &c1,
    ]);
    assert_eq!(a.status.code(), Some(0));
    assert_eq!(a.stdout, b.stdout);
    let v: serde_json::Value = serde_json::from_slice(&a.stdout).unwrap();
    assert_eq!(v["commit"], c1.as_str());
    assert_eq!(v["outcome"], "ok");
    assert_eq!(
        v["entries"].as_array().unwrap().len(),
        fx.truth(&r, &c1, "reg").len()
    );
    let bad = run(&[
        "snapshot",
        "--at",
        &c1[..12],
        "--register",
        "reg",
        "--repo",
        &repo,
    ]);
    assert_eq!(bad.status.code(), Some(3));
    let missing = run(&[
        "snapshot",
        "--at",
        &c1,
        "--register",
        "nope",
        "--repo",
        &repo,
    ]);
    assert_eq!(missing.status.code(), Some(4));
    let v: serde_json::Value = serde_json::from_slice(&missing.stdout).unwrap();
    assert_eq!(v["outcome"], "no_register_at_commit");
    assert_eq!(run(&["snapshot"]).status.code(), Some(2));
}

#[test]
fn behavior_bad_requests_and_missing_register() {
    let fx = Fx::new("bad");
    let (r, c1, _) = fx.base("repo", &|_, _| {});
    let blob = fx.git(&r, &["rev-parse", &format!("{c1}:other.txt")]);
    for rev in [&c1[..12], "main", "refs/heads/nope", blob.as_str()] {
        assert!(
            matches!(
                snapshot(&r, rev),
                Err(Error::BadRequest(BadRequest::Revision { .. }))
            ),
            "{rev}"
        );
    }
    let objects = || GitObjects::open(&r).unwrap();
    assert!(matches!(
        PinnedSnapshot::build(objects(), &c1, "nope"),
        Err(Error::NoRegister(NoRegister::Absent { .. }))
    ));
    assert!(matches!(
        PinnedSnapshot::build(objects(), &c1, "other.txt"),
        Err(Error::NoRegister(NoRegister::NotATree { .. }))
    ));
    assert!(matches!(
        PinnedSnapshot::build(objects(), &c1, "../reg"),
        Err(Error::BadRequest(BadRequest::Path { .. }))
    ));
}

#[test]
fn behavior_prefix_resolution_unique_missing_ambiguous() {
    let fx = Fx::new("prefix");
    let (r, c1, _) = fx.base("repo", &|_, _| {});
    let dir = fx.root.join("blobs");
    let paths: Vec<String> = (0..1500)
        .map(|i| {
            let p = dir.join(format!("{i}"));
            write(&p, &format!("blob {i}\n"));
            p.display().to_string()
        })
        .collect();
    let ids = fx.git_with(
        &r,
        &["hash-object", "-w", "--stdin-paths"],
        Some(paths.join("\n").as_bytes()),
        &[],
    );
    let mut by_prefix = std::collections::HashMap::<&str, usize>::new();
    for id in ids.lines() {
        *by_prefix.entry(&id[..4]).or_default() += 1;
    }
    let shared = by_prefix
        .iter()
        .find(|(_, n)| **n > 1)
        .map(|(p, _)| p.to_string())
        .expect("a 4-hex collision among 1500 blobs");
    let o = GitObjects::open(&r).unwrap();
    assert_eq!(o.resolve_prefix(&shared).unwrap(), Prefix::Ambiguous);
    assert_eq!(
        o.resolve_prefix(&c1[..12]).unwrap(),
        Prefix::Unique(o.resolve(&c1).unwrap())
    );
    assert_eq!(
        o.resolve_prefix("0000000000000000000000000000000000000000")
            .unwrap(),
        Prefix::Missing
    );
    assert!(o.resolve_prefix("abc").is_err());
    assert!(o.resolve_prefix("ABCD").is_err());
}

#[test]
fn behavior_ancestry_answers() {
    let fx = Fx::new("ancestry");
    let (r, c1, c2) = fx.base("repo", &|_, _| {});
    let o = GitObjects::open(&r).unwrap();
    let (c1, c2) = (o.resolve(&c1).unwrap(), o.resolve(&c2).unwrap());
    assert_eq!(o.is_ancestor(c1, c2).unwrap(), Ancestry::Yes);
    assert_eq!(o.is_ancestor(c2, c1).unwrap(), Ancestry::No);
    assert_eq!(o.is_ancestor(c2, c2).unwrap(), Ancestry::Yes);
}
