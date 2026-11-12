//! `anchors` on synthetic repositories: every class, the rename interim, symbolic links, the
//! final-newline rule, the identity refusal, an unreadable object at the target, a revision
//! commit that becomes unavailable after validation and the CLI.

mod support;

use research_register::anchor::AnchorClass;
use research_register::{
    AnchorError, AnchorReport, CannotRead, Error, GitObjects, Outcome, PinnedSnapshot,
    ValidRegister, anchors, validate,
};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use support::{Fx, write};

const REG: &str = "metrics/register";

/// (name, content at the revision, content at the target or None for removed, lines)
type Doc = (
    &'static str,
    &'static str,
    Option<&'static str>,
    Option<&'static str>,
);

const DOCS: &[Doc] = &[
    (
        "u.md",
        "# T\na\nb\nc\n",
        Some("# T\na\nb\nc\n"),
        Some("2-3"),
    ),
    (
        "m.md",
        "# T\na\nb\nc\n",
        Some("# T\nnew\na\nb\nc\n"),
        Some("3"),
    ),
    (
        "ms.md",
        "# S1\nx\n# S2\nkeep\n",
        Some("# S1\nx\n# S2\n## S3\nkeep\n"),
        Some("4"),
    ),
    ("rv.txt", "A\nX\nA\n", Some("A\n"), Some("3")),
    ("mod.txt", "a\nb\nc\nd\ne\n", Some("a\nZ\ne\n"), Some("2-4")),
    ("amb.txt", "p\nq\nr\n", Some("q\np\nq\nr\n"), Some("2")),
    ("abs.txt", "a\nb\nc\nd\n", Some("a\nd\n"), Some("2-3")),
    ("wu.txt", "w\n", Some("w\n"), None),
    ("wm.txt", "w\n", Some("w2\n"), None),
    ("gone.txt", "g\nh\n", None, Some("2")),
    ("gone2.txt", "g\n", None, None),
    ("fn.txt", "a\nb", Some("a\nb\n"), Some("2")),
    ("tree.txt", "t\n", None, Some("1")),
    ("link.txt", "x\ny\n", None, Some("2")),
];

struct Fixture {
    fx: Fx,
    repo: PathBuf,
    rev: String,
    target: String,
}

fn blob(fx: &Fx, repo: &Path, spec: &str) -> String {
    fx.git(repo, &["rev-parse", spec])
}

fn register(rev: &str, pin_blob: &str, docs: &[Doc]) -> Vec<(String, String)> {
    let pin = json!({"path": "u.md", "commit": &rev[..7], "blob": pin_blob});
    let decisions: Vec<Value> = docs
        .iter()
        .map(|(name, _, _, lines)| {
            let mut d = json!({"decision_id": format!("d-{}", name.replace('.', "-")),
                "document": name, "anchor": "x", "revision": format!("git:{}", &rev[..10]),
                "owner": "o", "findings": ["alpha#one"]});
            if let Some(l) = lines {
                d["lines"] = json!(l);
            }
            d
        })
        .collect();
    let study = json!({"id": "alpha", "title": "T", "kind": "probe", "questions": ["Q1"],
        "lifecycle": [{"date": "2026-01-02", "state": "concluded", "basis": pin}],
        "stop_reason_class": null,
        "findings": [{"id": "alpha#one", "claim": "c", "value": 1, "n": 2, "basis": "measured",
                      "scope": "s", "caveats": [], "sources": [pin]}],
        "sources": [], "aliases": []});
    vec![
        (
            "questions.json".into(),
            json!([{"id": "Q1", "text": "t", "area": "a"}]).to_string(),
        ),
        ("studies/alpha.json".into(), study.to_string()),
        ("decisions.json".into(), Value::Array(decisions).to_string()),
        ("events.jsonl".into(), String::new()),
        ("judgments.jsonl".into(), String::new()),
        ("applicability.jsonl".into(), String::new()),
        ("materialized.jsonl".into(), String::new()),
    ]
}

/// Revision commit R with every document, then target commit C with the target contents and the
/// register (whose decisions are pinned at R).
fn fixture(name: &str) -> Fixture {
    let fx = Fx::new(name);
    let repo = fx.root.join("r");
    std::fs::create_dir_all(&repo).unwrap();
    fx.git(&repo, &["init", "-q", "-b", "main"]);
    for (doc, at_rev, _, _) in DOCS {
        write(&repo.join(doc), at_rev);
    }
    fx.git(&repo, &["add", "-A"]);
    fx.git(&repo, &["commit", "-q", "-m", "rev"]);
    let rev = fx.git(&repo, &["rev-parse", "HEAD"]);
    for (doc, _, at_target, _) in DOCS {
        let p = repo.join(doc);
        std::fs::remove_file(&p).unwrap();
        if let Some(t) = at_target {
            write(&p, t);
        }
    }
    write(&repo.join("tree.txt/inner"), "i\n");
    std::os::unix::fs::symlink("y", repo.join("link.txt")).unwrap();
    let pin_blob = blob(&fx, &repo, &format!("{rev}:u.md"));
    for (path, text) in register(&rev, &pin_blob, DOCS) {
        write(&repo.join(REG).join(path), &text);
    }
    fx.git(&repo, &["add", "-A"]);
    fx.git(&repo, &["commit", "-q", "-m", "target"]);
    let target = fx.git(&repo, &["rev-parse", "HEAD"]);
    Fixture {
        fx,
        repo,
        rev,
        target,
    }
}

fn valid_at(repo: &Path, commit: &str) -> (PinnedSnapshot, Box<ValidRegister>) {
    let snapshot = PinnedSnapshot::build(GitObjects::open(repo).unwrap(), commit, REG).unwrap();
    match validate(&snapshot).unwrap() {
        Outcome::Valid(v) => (snapshot, v),
        Outcome::Invalid(v) => panic!("fixture register invalid: {v:?}"),
    }
}

fn run(repo: &Path, commit: &str) -> Result<AnchorReport, AnchorError> {
    let (snapshot, valid) = valid_at(repo, commit);
    anchors(&valid, &snapshot)
}

fn entry<'r>(report: &'r AnchorReport, doc: &str) -> &'r Value {
    let id = format!("d-{}", doc.replace('.', "-"));
    report
        .entries
        .iter()
        .find(|e| e.decision_id == id)
        .unwrap_or_else(|| panic!("{id}"))
        .json()
}

#[test]
fn every_class_and_the_interim() {
    let f = fixture("anchors-classes");
    let report = run(&f.repo, &f.target).unwrap();
    assert_eq!(report.pinned_commit.to_string(), f.target);
    assert_eq!(report.entries.len(), DOCS.len());
    let ids: Vec<&str> = report
        .entries
        .iter()
        .map(|e| e.decision_id.as_str())
        .collect();
    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(ids, sorted, "entries sorted by decision_id");
    let class = |doc: &str| entry(&report, doc)["class"].clone();
    let expect = [
        ("u.md", json!("unchanged")),
        ("m.md", json!("moved")),
        ("ms.md", json!("moved")),
        ("rv.txt", json!("relocated_verbatim")),
        ("mod.txt", json!("modified")),
        ("amb.txt", json!("ambiguous")),
        ("abs.txt", json!("absent")),
        ("wu.txt", json!("unchanged")),
        ("wm.txt", json!("modified")),
        ("gone.txt", Value::Null),
        ("gone2.txt", Value::Null),
        ("fn.txt", json!("unchanged")),
        ("tree.txt", Value::Null),
        ("link.txt", json!("moved")),
    ];
    for (doc, want) in &expect {
        assert_eq!(&class(doc), want, "{doc}: {:#}", entry(&report, doc));
    }
    for e in &report.entries {
        let j = e.json();
        assert_eq!(j["revision_commit"], json!(f.rev));
        assert_eq!(j["revision_is_ancestor_of_target"], json!(true));
        assert_eq!(e.class.map(AnchorClass::as_str), j["class"].as_str());
    }

    // unchanged: after lines, no context, no review.
    let u = entry(&report, "u.md");
    assert_eq!(u["after"], json!({"lines": "2-3"}));
    assert_eq!(u["context_before"], Value::Null);
    assert_eq!(u["lines"], json!("2-3"));
    assert_eq!(u["before"]["lines"], json!("2-3"));
    let sha = format!("{:x}", <sha2::Sha256 as sha2::Digest>::digest(b"a\nb\n"));
    assert_eq!(u["before"]["anchored_text_sha256"], json!(sha));

    // moved in the same section: no review; both chains reported.
    let m = entry(&report, "m.md");
    assert_eq!(m["after"], json!({"lines": "4"}));
    assert_eq!(m["owner_review_required"], json!(false));
    assert_eq!(
        m["context_after"]["section_chain"],
        json!([{"level": 1, "line": 1, "text": "# T"}])
    );
    assert_eq!(m["context_after"]["before"], json!(["# T", "new", "a"]));
    assert_eq!(m["context_after"]["after"], json!(["c"]));

    // moved across sections: owner review for section identity.
    let ms = entry(&report, "ms.md");
    assert_eq!(ms["owner_review_required"], json!(true));
    assert_eq!(
        ms["owner_review_reasons"],
        json!(["section_identity_changed"])
    );
    assert_eq!(
        ms["context_after"]["section_chain"],
        json!([{"level": 1, "line": 3, "text": "# S2"}, {"level": 2, "line": 4, "text": "## S3"}])
    );

    // relocated_verbatim (git keeps line 1 of [A,X,A] against [A]); not Markdown.
    let rv = entry(&report, "rv.txt");
    assert_eq!(rv["owner_review_reasons"], json!(["relocated_verbatim"]));
    assert_eq!(rv["after"], json!({"lines": "1"}));
    assert_eq!(
        rv["context_before"]["section_chain"],
        json!("not applicable (not a Markdown document)")
    );
    assert_eq!(
        rv["context_before"]["heading"],
        json!("not applicable (not a Markdown document)")
    );

    // modified: three lines replaced by one, clamped to the replacement.
    let md = entry(&report, "mod.txt");
    assert_eq!(
        md["candidates"],
        json!([{"lines": "2", "basis": "line tracking through the diff; candidate only, not chosen"}])
    );
    assert_eq!(md["after"], Value::Null);

    // ambiguous: every occurrence, none chosen.
    let amb = entry(&report, "amb.txt");
    assert_eq!(
        amb["candidates"],
        json!([{"lines": "1", "basis": "exact anchored text occurs here"},
               {"lines": "3", "basis": "exact anchored text occurs here"}])
    );

    // absent: the range was deleted, the document remains.
    let abs = entry(&report, "abs.txt");
    assert_eq!(abs["candidates"], json!([]));
    assert_eq!(abs["document_at_target"]["path"], json!("abs.txt"));

    // whole documents by blob.
    let wu = entry(&report, "wu.txt");
    let wu_blob = blob(&f.fx, &f.repo, &format!("{}:wu.txt", f.target));
    assert_eq!(wu["after"], json!({"document_blob": wu_blob}));
    assert_eq!(wu["lines"], Value::Null);
    assert!(wu["before"].get("lines").is_none());
    let wm = entry(&report, "wm.txt");
    assert_eq!(
        wm["candidates"],
        json!([{"document": "wm.txt", "basis": "whole document differs; candidate only"}])
    );

    // the rename interim, ranged and whole, removed or replaced by a tree.
    for doc in ["gone.txt", "gone2.txt", "tree.txt"] {
        let g = entry(&report, doc);
        assert_eq!(
            g["error"],
            json!({"kind": "not_established", "reason": research_register::anchor::RENAME_INTERIM}),
            "{doc}"
        );
        assert_eq!(g["document_at_target"], Value::Null);
        assert_eq!(g["after"], Value::Null);
        assert_eq!(g["owner_review_required"], json!(false));
        let rev_blob = blob(&f.fx, &f.repo, &format!("{}:{doc}", f.rev));
        assert_eq!(g["before"]["document_blob"], json!(rev_blob));
    }
    assert_eq!(entry(&report, "gone.txt")["before"]["lines"], json!("2"));

    // a symbolic link at the target counts as the document (its blob is the link text).
    let link = entry(&report, "link.txt");
    let link_blob = blob(&f.fx, &f.repo, &format!("{}:link.txt", f.target));
    assert_eq!(link["document_at_target"]["blob"], json!(link_blob));
    assert_eq!(link["after"], json!({"lines": "1"}));

    // a final newline added to the last anchored line is no change.
    assert_eq!(entry(&report, "fn.txt")["after"], json!({"lines": "2"}));

    // deterministic JSON
    let again = run(&f.repo, &f.target).unwrap();
    assert_eq!(report.to_json().to_string(), again.to_json().to_string());
    assert_eq!(
        report.to_json()["statement"],
        json!(research_register::anchor::STATEMENT)
    );
}

#[test]
fn snapshot_of_another_commit_is_refused() {
    let f = fixture("anchors-identity");
    f.fx.git(
        &f.repo,
        &["commit", "-q", "--allow-empty", "-m", "same tree"],
    );
    let other = f.fx.git(&f.repo, &["rev-parse", "HEAD"]);
    let (_, valid) = valid_at(&f.repo, &f.target);
    let (snapshot, _) = valid_at(&f.repo, &other);
    assert_eq!(snapshot.register_tree(), valid.register_tree());
    let err = anchors(&valid, &snapshot).unwrap_err();
    assert_eq!(err.category(), "bad_request", "{err}");
}

#[test]
fn unreadable_object_at_the_target_is_cannot_read() {
    let f = fixture("anchors-unreadable");
    let id = blob(&f.fx, &f.repo, &format!("{}:wm.txt", f.target));
    let (snapshot, valid) = valid_at(&f.repo, &f.target);
    std::fs::remove_file(f.fx.loose(&f.repo, &id)).unwrap();
    let err = anchors(&valid, &snapshot).unwrap_err();
    assert_eq!(err.category(), "cannot_read", "{err}");
    assert!(matches!(err, AnchorError::Read(_)));
}

/// F-01: a decision's revision commit that disappears from the object store after validation is a
/// reading failure (F-4 `cannot_read`, as validation itself reports a missing commit), not an
/// invariant failure, and no report is produced.
#[test]
fn revision_commit_unavailable_after_validation_is_cannot_read() {
    let f = fixture("anchors-revision-gone");
    // Unchanged success: the same register at the same commit anchors every decision.
    let report = run(&f.repo, &f.target).unwrap();
    assert_eq!(report.entries.len(), DOCS.len());
    // A fresh snapshot and validation, then the revision commit object is removed.
    let (snapshot, valid) = valid_at(&f.repo, &f.target);
    let loose = f.fx.loose(&f.repo, &f.rev);
    assert!(loose.is_file(), "the revision commit is a loose object");
    std::fs::remove_file(loose).unwrap();
    let err = anchors(&valid, &snapshot).unwrap_err();
    assert_eq!(err.category(), "cannot_read", "{err}");
    match &err {
        AnchorError::Read(Error::CannotRead(CannotRead::ObjectUnavailable { id })) => {
            assert_eq!(id, &f.rev[..10], "{err}");
        }
        other => panic!("expected ObjectUnavailable for the revision, got {other:?}"),
    }
    // Validation of the same snapshot now reports the same F-4 class.
    let e = validate(&snapshot).unwrap_err();
    assert_eq!(e.category(), "cannot_read", "{e}");
}

#[test]
fn anchors_command() {
    let f = fixture("anchors-cli");
    let exe = env!("CARGO_BIN_EXE_work-engine-register");
    let run = |at: &str| {
        let out = std::process::Command::new(exe)
            .args(["anchors", "--at", at, "--repo"])
            .arg(&f.repo)
            .env_clear()
            .output()
            .unwrap();
        let v: Value = serde_json::from_slice(&out.stdout).unwrap();
        (out.status.code(), v)
    };
    let (code, v) = run(&f.target);
    assert_eq!(code, Some(0), "{v}");
    assert_eq!(v["outcome"], json!("ok"));
    assert_eq!(v["schema"], json!("research_register_anchors_v1"));
    assert_eq!(v["entries"].as_array().unwrap().len(), DOCS.len());
    // An invalid register prints its violations and exits 1, as `validate` does.
    write(
        &f.repo.join(REG).join("decisions.json"),
        "[{\"decision_id\": \"bad\"}]",
    );
    f.fx.git(&f.repo, &["commit", "-q", "-am", "invalid"]);
    let (code, v) = run("HEAD");
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(v["outcome"], json!("invalid"));
    assert!(!v["violations"].as_array().unwrap().is_empty());
    // A revision that is not a commit is bad_request (exit 3).
    let (code, v) = run(&f.rev[..0]);
    assert_eq!(code, Some(3), "{v}");
}

/// Only the required revision is packed; target, both trees, and every blob stay loose.
#[test]
fn packed_revision_disappears_without_removing_target_inputs() {
    packed_loss(false);
}
#[test]
fn bounded_packed_revision_disappears_after_validation() {
    packed_loss(true);
}
fn packed_loss(bounded: bool) {
    let f = fixture("anchors-packed-revision-gone");
    let trees: Vec<_> = [&f.rev, &f.target]
        .iter()
        .map(|rev| {
            f.fx.git(&f.repo, &["rev-parse", &format!("{rev}^{{tree}}")])
        })
        .collect();
    let blobs: Vec<_> = [&f.rev, &f.target]
        .iter()
        .flat_map(|rev| {
            f.fx.git(
                &f.repo,
                &["ls-tree", "-r", "--format=%(objecttype) %(objectname)", rev],
            )
            .lines()
            .filter_map(|line| line.strip_prefix("blob ").map(str::to_owned))
            .collect::<Vec<_>>()
        })
        .collect();
    let pack = f.repo.join(".git/objects/pack/revision");
    let hash = f.fx.git_with(
        &f.repo,
        &["pack-objects", pack.to_str().unwrap()],
        Some(format!("{}\n", f.rev).as_bytes()),
        &[],
    );
    let pack_path = PathBuf::from(format!("{}-{hash}.pack", pack.display()));
    let idx_path = PathBuf::from(format!("{}-{hash}.idx", pack.display()));
    let listing =
        f.fx.git(&f.repo, &["verify-pack", "-v", idx_path.to_str().unwrap()]);
    let object_rows: Vec<_> = listing.lines().filter(|l| l.starts_with(&f.rev)).collect();
    assert_eq!(object_rows.len(), 1);
    assert_eq!(
        listing
            .lines()
            .filter(|l| l.split_whitespace().next().is_some_and(|s| s.len() == 40))
            .count(),
        1
    );
    std::fs::remove_file(f.fx.loose(&f.repo, &f.rev)).unwrap();
    let (snapshot, valid) = if bounded {
        valid_at_bounded(&f.repo, &f.target)
    } else {
        valid_at(&f.repo, &f.target)
    };
    assert_eq!(
        anchors(&valid, &snapshot).unwrap().entries.len(),
        DOCS.len()
    );
    // Move just this single-object pack outside the object store.
    let saved_pack = f.fx.root.join("saved.pack");
    let saved_idx = f.fx.root.join("saved.idx");
    std::fs::rename(&pack_path, &saved_pack).unwrap();
    std::fs::rename(&idx_path, &saved_idx).unwrap();
    let objects = GitObjects::open(&f.repo).unwrap();
    let target = objects.resolve(&f.target).unwrap();
    objects.commit(target).unwrap();
    for tree in &trees {
        assert!(f.fx.loose(&f.repo, tree).is_file());
        objects
            .tree(gix::hash::ObjectId::from_hex(tree.as_bytes()).unwrap())
            .unwrap();
    }
    for blob in &blobs {
        assert!(f.fx.loose(&f.repo, blob).is_file());
        objects
            .blob(gix::hash::ObjectId::from_hex(blob.as_bytes()).unwrap())
            .unwrap();
    }
    let err = anchors(&valid, &snapshot).unwrap_err();
    assert_eq!(err.category(), "cannot_read", "{err}");
    assert!(
        matches!(&err, AnchorError::Read(Error::CannotRead(CannotRead::ObjectUnavailable { id })) if id == &f.rev[..10] || id == &f.rev),
        "{err}"
    );
    std::fs::rename(saved_pack, pack_path).unwrap();
    std::fs::rename(saved_idx, idx_path).unwrap();
    assert_eq!(
        anchors(&valid, &snapshot).unwrap().entries.len(),
        DOCS.len()
    );
    assert_eq!(run(&f.repo, &f.target).unwrap().entries.len(), DOCS.len());
}

/// The second valid object is genuinely absent during validation and exposed afterwards.
/// Frozen payloads avoid any collision search in CI.
#[test]
fn revision_prefix_becomes_ambiguous_after_validation() {
    late_ambiguity(false);
}
#[test]
fn bounded_revision_prefix_becomes_ambiguous_in_late_pack() {
    late_ambiguity(true);
}
fn late_ambiguity(bounded: bool) {
    let pair: Value =
        serde_json::from_str(include_str!("fixtures/revision-prefix-collision.json")).unwrap();
    let fx = Fx::new("anchors-prefix-collision");
    let repo = fx.root.join("r");
    std::fs::create_dir_all(&repo).unwrap();
    fx.git(&repo, &["init", "-q", "-b", "main"]);
    write(&repo.join("u.md"), "# T\na\nb\nc\n");
    fx.git(&repo, &["add", "u.md"]);
    assert_eq!(
        fx.git(&repo, &["write-tree"]),
        pair["tree"].as_str().unwrap()
    );
    let commits = pair["commits"].as_array().unwrap();
    let write_commit = |i: usize| {
        fx.git_with(
            &repo,
            &["hash-object", "-t", "commit", "-w", "--stdin"],
            Some(commits[i]["body"].as_str().unwrap().as_bytes()),
            &[],
        )
    };
    let first = write_commit(0);
    assert_eq!(first, commits[0]["id"].as_str().unwrap());
    fx.git(&repo, &["update-ref", "refs/heads/main", &first]);
    let pin_blob = blob(&fx, &repo, &format!("{first}:u.md"));
    for (path, text) in register(&first, &pin_blob, &DOCS[..1]) {
        // Decision and study prefixes must both be the accepted seven digits.
        write(
            &repo.join(REG).join(path),
            &text.replace(&first[..10], &first[..7]),
        );
    }
    fx.git(&repo, &["add", "-A"]);
    fx.git(&repo, &["commit", "-q", "-m", "target"]);
    let target = fx.git(&repo, &["rev-parse", "HEAD"]);
    let (snapshot, valid) = if bounded {
        valid_at_bounded(&repo, &target)
    } else {
        valid_at(&repo, &target)
    };
    assert_eq!(anchors(&valid, &snapshot).unwrap().entries.len(), 1);
    let second = write_commit(1);
    assert_eq!(second, commits[1]["id"].as_str().unwrap());
    let mut late_paths = Vec::new();
    if bounded {
        let prefix = repo.join(".git/objects/pack/late");
        let hash = fx.git_with(
            &repo,
            &["pack-objects", prefix.to_str().unwrap()],
            Some(format!("{second}\n").as_bytes()),
            &[],
        );
        for ext in ["pack", "idx"] {
            late_paths.push(std::path::PathBuf::from(format!(
                "{}-{hash}.{ext}",
                prefix.display()
            )));
        }
        std::fs::remove_file(fx.loose(&repo, &second)).unwrap();
    }
    let objects = GitObjects::open(&repo).unwrap();
    objects.commit(objects.resolve(&second).unwrap()).unwrap();
    assert!(matches!(
        objects
            .resolve_prefix(pair["prefix"].as_str().unwrap())
            .unwrap(),
        research_register::Prefix::Ambiguous
    ));
    let err = anchors(&valid, &snapshot).unwrap_err();
    assert_eq!(err.category(), "invariant_failure", "{err}");
    if bounded {
        for path in late_paths {
            std::fs::remove_file(path).unwrap();
        }
    } else {
        std::fs::remove_file(fx.loose(&repo, &second)).unwrap();
    }
    assert_eq!(anchors(&valid, &snapshot).unwrap().entries.len(), 1);
    assert_eq!(run(&repo, &target).unwrap().entries.len(), 1);
}

fn valid_at_bounded(repo: &Path, commit: &str) -> (PinnedSnapshot, Box<ValidRegister>) {
    let snapshot = PinnedSnapshot::build(
        GitObjects::open_bounded(
            repo,
            research_register::bounded_read::ReadProfile::default(),
        )
        .unwrap(),
        commit,
        REG,
    )
    .unwrap();
    let Outcome::Valid(valid) = validate(&snapshot).unwrap() else {
        panic!("bounded fixture invalid")
    };
    (snapshot, valid)
}
