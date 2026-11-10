//! Whole-register validation against real Git repositories (stage B rules: R-01, R-09, R-15,
//! R-17's revision match, and the Git-backed R-10, R-12, R-13, R-14), the F-4 reading outcomes
//! (G2-4), A5 and the `validate` command. The base register is valid; each rejecting case changes
//! one thing and must report exactly one violation with the expected rule id and location.

mod support;

use research_register::{Error, GitObjects, Outcome, PinnedSnapshot, Violation, validate};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use support::{Fx, write};

type Reg = BTreeMap<String, Value>;
const REG: &str = "metrics/register";

/// Commits and object ids of the fixture history: A -> M -> B on main, X on a side branch off A.
struct Ids {
    a: String,
    m: String,
    b: String,
    x: String,
    /// docs/a.md at A (5 lines) and at B (6 lines).
    a_at_a: String,
    /// docs/b.md (3 lines, unchanged).
    b_md: String,
    /// docs/l.md, a symbolic link to a.md (A onwards).
    link: String,
    /// The docs tree at A.
    docs_at_a: String,
}

fn history(fx: &Fx, name: &str) -> (PathBuf, Ids) {
    let r = fx.root.join(name);
    std::fs::create_dir_all(&r).unwrap();
    fx.git(&r, &["init", "-q", "-b", "main"]);
    write(&r.join("docs/a.md"), "1\n2\n3\n4\n5\n");
    write(&r.join("docs/b.md"), "x\ny\nz\n");
    std::os::unix::fs::symlink("a.md", r.join("docs/l.md")).unwrap();
    fx.git(&r, &["add", "-A"]);
    fx.git(&r, &["commit", "-q", "-m", "A"]);
    let a = fx.git(&r, &["rev-parse", "HEAD"]);
    fx.git(&r, &["checkout", "-q", "-b", "side"]);
    write(&r.join("docs/a.md"), "side\n");
    fx.git(&r, &["commit", "-q", "-am", "X"]);
    let x = fx.git(&r, &["rev-parse", "HEAD"]);
    fx.git(&r, &["checkout", "-q", "main"]);
    write(&r.join("other.txt"), "m\n");
    fx.git(&r, &["add", "-A"]);
    fx.git(&r, &["commit", "-q", "-m", "M"]);
    let m = fx.git(&r, &["rev-parse", "HEAD"]);
    write(&r.join("docs/a.md"), "1\n2\n3\n4\n5\n6\n");
    fx.git(&r, &["commit", "-q", "-am", "B"]);
    let b = fx.git(&r, &["rev-parse", "HEAD"]);
    let ids = Ids {
        a_at_a: fx.git(&r, &["rev-parse", &format!("{a}:docs/a.md")]),
        b_md: fx.git(&r, &["rev-parse", &format!("{a}:docs/b.md")]),
        link: fx.git(&r, &["rev-parse", &format!("{a}:docs/l.md")]),
        docs_at_a: fx.git(&r, &["rev-parse", &format!("{a}:docs")]),
        a,
        m,
        b,
        x,
    };
    (r, ids)
}

fn pin(i: &Ids) -> Value {
    json!({"path": "docs/b.md", "commit": &i.a[..7], "blob": i.b_md, "lines": "1-3"})
}

/// A valid register over the fixture history.
fn base(i: &Ids) -> Reg {
    let p = pin(i);
    let mut r = Reg::new();
    r.insert(
        "questions.json".into(),
        json!([{"id": "Q1", "text": "t", "area": "a"}]),
    );
    r.insert(
        "studies/alpha.json".into(),
        json!({"id": "alpha", "title": "T", "kind": "probe", "questions": ["Q1"],
               "lifecycle": [{"date": "2026-01-02", "state": "concluded", "basis": p}],
               "stop_reason_class": null,
               "findings": [{"id": "alpha#one", "claim": "c", "value": 1, "n": 2, "basis": "measured",
                             "scope": "s", "caveats": [], "sources": [p]}],
               "sources": [], "aliases": []}),
    );
    r.insert(
        "decisions.json".into(),
        json!([
            {"decision_id": "d-one", "document": "docs/a.md", "anchor": "A", "revision": format!("git:{}", &i.b[..10]),
             "owner": "o", "findings": ["alpha#one"], "lines": "1-6",
             "revisions": [{"revision": format!("git:{}", &i.a[..7]), "anchor": "A0",
                            "superseded_at": "2026-01-01", "reason": "r"}]},
            {"decision_id": "d-two", "document": "docs/b.md", "anchor": "B", "revision": format!("git:{}", i.a),
             "owner": "o", "findings": ["alpha#one"],
             "historical": {"reason": "r", "marked_by": "m", "marked_at": "2026-01-03", "superseded_by": "d-one"}}
        ]),
    );
    r.insert(
        "events.jsonl".into(),
        json!([{"event_id": "e-one", "kind": "correction", "date": "2026-01-04", "subject_finding": "alpha#one",
                "by_finding": "alpha#one", "basis": "b", "source": p}]),
    );
    r.insert(
        "judgments.jsonl".into(),
        json!([{"finding": "alpha#one", "axis": "standing", "value": "qualified", "basis": "b",
                "assessor": "a", "date": "2026-01-05"}]),
    );
    r.insert(
        "applicability.jsonl".into(),
        json!([
            {"applicability_id": "ap-one", "finding": "alpha#one", "decision_id": "d-one",
             "decision_revision": format!("git:{}", &i.b[..10]), "relation": "supports", "rationale": "r",
             "evidence": [p], "proposed_by": "p", "proposed_at": "2026-01-06",
             "authorization": {"status": "authorized", "owner": "o", "at": "2026-01-07"}},
            {"applicability_id": "ap-two", "finding": "alpha#one", "decision_id": "d-two",
             "decision_revision": format!("git:{}", i.a), "relation": "constrains", "rationale": "r",
             "evidence": [p], "proposed_by": "p", "proposed_at": "2026-01-06",
             "authorization": {"status": "pending", "owner": "o"}}
        ]),
    );
    r.insert(
        "materialized.jsonl".into(),
        json!([{"finding": "alpha#one", "claim_id": "c", "revision_id": "r", "revision_sha256": "s"}]),
    );
    r
}

fn set(reg: &mut Reg, file: &str, pointer: &str, value: Value) {
    let slot = reg
        .get_mut(file)
        .unwrap()
        .pointer_mut(pointer)
        .expect("pointer exists");
    *slot = value;
}

/// Replace the register directory with `reg`, run `extra` on it, commit, return the commit.
fn commit(fx: &Fx, repo: &Path, reg: &Reg, extra: &dyn Fn(&Path)) -> String {
    let dir = repo.join(REG);
    let _ = std::fs::remove_dir_all(&dir);
    for (path, v) in reg {
        let text = if path.ends_with(".jsonl") {
            let lines: Vec<String> = v.as_array().unwrap().iter().map(Value::to_string).collect();
            lines.join("\n") + "\n"
        } else {
            v.to_string()
        };
        write(&dir.join(path), &text);
    }
    extra(&dir);
    fx.git(repo, &["add", "-A"]);
    fx.git(repo, &["commit", "-q", "--allow-empty", "-m", "register"]);
    fx.git(repo, &["rev-parse", "HEAD"])
}

fn run(repo: &Path, commit: &str) -> Result<Outcome, Error> {
    let snapshot = PinnedSnapshot::build(GitObjects::open(repo)?, commit, REG)?;
    validate(&snapshot)
}

fn violations(repo: &Path, commit: &str) -> Vec<Violation> {
    match run(repo, commit).expect("register is readable") {
        Outcome::Valid(_) => Vec::new(),
        Outcome::Invalid(v) => v,
    }
}

type Expected = (&'static str, &'static str, Option<u64>, &'static str);
type Mutation<'a> = Box<dyn Fn(&mut Reg) + 'a>;
type TreeEdit = Box<dyn Fn(&Path)>;

fn shown(v: &[Violation]) -> Vec<String> {
    v.iter()
        .map(|v| {
            format!(
                "{} {} {:?} {} ({})",
                v.rule, v.location.file, v.location.line, v.location.pointer, v.detail
            )
        })
        .collect()
}

fn assert_one(name: &str, v: &[Violation], (rule, file, line, pointer): Expected) {
    assert!(
        v.len() == 1,
        "{name}: expected exactly one violation, got {:#?}",
        shown(v)
    );
    let got = &v[0];
    assert_eq!(
        (
            got.rule.as_str(),
            got.location.file.as_str(),
            got.location.line,
            got.location.pointer.as_str()
        ),
        (rule, file, line, pointer),
        "{name}: {}",
        got.detail
    );
}

#[test]
fn base_register_is_valid_and_carries_its_records() {
    let fx = Fx::new("validate-base");
    let (repo, ids) = history(&fx, "r");
    let c = commit(&fx, &repo, &base(&ids), &|_| {});
    match run(&repo, &c).unwrap() {
        Outcome::Valid(v) => {
            assert_eq!(v.commit().to_string(), c);
            assert_eq!(v.register_path(), REG);
            assert_eq!(v.records().decisions.len(), 2);
            assert_eq!(v.records().applicability.len(), 2);
        }
        Outcome::Invalid(v) => panic!("base is invalid: {:#?}", shown(&v)),
    }
}

#[test]
fn accepting_variants() {
    let fx = Fx::new("validate-accept");
    let (repo, i) = history(&fx, "r");
    let cases: Vec<(&str, Mutation)> = vec![
        (
            "applicability names a listed revision (R-17)",
            Box::new(|r| {
                set(
                    r,
                    "applicability.jsonl",
                    "/0/decision_revision",
                    json!(format!("git:{}", &i.a[..7])),
                )
            }),
        ),
        (
            "pin to a symbolic link, whose object is a blob (R-10)",
            Box::new(|r| {
                set(
                    r,
                    "studies/alpha.json",
                    "/findings/0/sources/0",
                    json!({"path": "docs/l.md", "commit": i.a, "blob": i.link}),
                )
            }),
        ),
        (
            "full 40-hex pin commit and lines at the last line (R-10)",
            Box::new(|r| {
                set(
                    r,
                    "studies/alpha.json",
                    "/findings/0/sources/0",
                    json!({"path": "docs/b.md", "commit": i.a, "blob": i.b_md, "lines": "3"}),
                )
            }),
        ),
        (
            "two listed revisions in ancestry order (R-13, R-14)",
            Box::new(|r| {
                set(
                    r,
                    "decisions.json",
                    "/0/revisions",
                    json!([
                    {"revision": format!("git:{}", &i.a[..7]), "anchor": "A0", "superseded_at": "2026-01-01", "reason": "r"},
                    {"revision": format!("git:{}", &i.m[..8]), "anchor": "M0", "superseded_at": "2026-01-02", "reason": "r",
                     "document": "docs/b.md", "lines": "2-3"}]),
                )
            }),
        ),
        (
            "revision entry with its own document and in-range lines (R-13)",
            Box::new(|r| {
                set(
                    r,
                    "decisions.json",
                    "/0/revisions/0",
                    json!({"revision": format!("git:{}", &i.a[..7]), "anchor": "A0",
                    "superseded_at": "2026-01-01", "reason": "r", "lines": "5"}),
                )
            }),
        ),
        (
            "a non-JSON file under studies/ is not a study file (R-01)",
            Box::new(|r| {
                r.insert("studies/README".into(), json!("notes"));
            }),
        ),
    ];
    for (name, mutate) in cases {
        let mut reg = base(&i);
        mutate(&mut reg);
        let c = commit(&fx, &repo, &reg, &|_| {});
        let v = violations(&repo, &c);
        assert!(v.is_empty(), "{name}: {:#?}", shown(&v));
    }
}

#[test]
fn rejecting_cases_report_exactly_one_violation() {
    let fx = Fx::new("validate-reject");
    let (repo, i) = history(&fx, "r");
    let pin0 = "/findings/0/sources/0";
    type Case<'a> = (&'a str, Box<dyn Fn(&mut Reg) + 'a>, Expected);
    let cases: Vec<Case> = vec![
        // R-01 by removal; the other file-set cases need tree edits (below).
        (
            "materialized.jsonl absent",
            Box::new(|r| {
                r.remove("materialized.jsonl");
            }),
            ("R-01", "materialized.jsonl", None, ""),
        ),
        // R-09
        (
            "unknown question in a study",
            Box::new(|r| set(r, "studies/alpha.json", "/questions/0", json!("Q9"))),
            ("R-09", "studies/alpha.json", None, "/questions/0"),
        ),
        (
            "unknown finding in a decision",
            Box::new(|r| set(r, "decisions.json", "/0/findings/0", json!("alpha#nine"))),
            ("R-09", "decisions.json", None, "/0/findings/0"),
        ),
        (
            "unknown subject finding in an event",
            Box::new(|r| set(r, "events.jsonl", "/0/subject_finding", json!("alpha#nine"))),
            ("R-09", "events.jsonl", Some(1), "/subject_finding"),
        ),
        (
            "unknown by finding in an event",
            Box::new(|r| set(r, "events.jsonl", "/0/by_finding", json!("alpha#nine"))),
            ("R-09", "events.jsonl", Some(1), "/by_finding"),
        ),
        (
            "unknown finding in a judgment",
            Box::new(|r| set(r, "judgments.jsonl", "/0/finding", json!("alpha#nine"))),
            ("R-09", "judgments.jsonl", Some(1), "/finding"),
        ),
        (
            "unknown finding in applicability",
            Box::new(|r| set(r, "applicability.jsonl", "/1/finding", json!("alpha#nine"))),
            ("R-09", "applicability.jsonl", Some(2), "/finding"),
        ),
        (
            "unknown decision in applicability",
            Box::new(|r| set(r, "applicability.jsonl", "/1/decision_id", json!("d-nine"))),
            ("R-09", "applicability.jsonl", Some(2), "/decision_id"),
        ),
        (
            "unknown finding in a materialized row",
            Box::new(|r| set(r, "materialized.jsonl", "/0/finding", json!("alpha#nine"))),
            ("R-09", "materialized.jsonl", Some(1), "/finding"),
        ),
        (
            "superseded_by names no decision",
            Box::new(|r| {
                set(
                    r,
                    "decisions.json",
                    "/1/historical/superseded_by",
                    json!("d-nine"),
                )
            }),
            (
                "R-09",
                "decisions.json",
                None,
                "/1/historical/superseded_by",
            ),
        ),
        // R-15
        (
            "superseded_by names its own decision",
            Box::new(|r| {
                set(
                    r,
                    "decisions.json",
                    "/1/historical/superseded_by",
                    json!("d-two"),
                )
            }),
            (
                "R-15",
                "decisions.json",
                None,
                "/1/historical/superseded_by",
            ),
        ),
        // R-17
        (
            "decision_revision neither current nor listed",
            Box::new(|r| {
                set(
                    r,
                    "applicability.jsonl",
                    "/0/decision_revision",
                    json!(format!("git:{}", &i.m[..7])),
                )
            }),
            ("R-17", "applicability.jsonl", Some(1), "/decision_revision"),
        ),
        // R-10 (Git-backed)
        (
            "pin path absent at its commit",
            Box::new(|r| {
                set(
                    r,
                    "studies/alpha.json",
                    &format!("{pin0}/path"),
                    json!("docs/zz.md"),
                )
            }),
            ("R-10", "studies/alpha.json", None, pin0),
        ),
        (
            "pin path not a plain relative path",
            Box::new(|r| {
                set(
                    r,
                    "studies/alpha.json",
                    &format!("{pin0}/path"),
                    json!("docs/../docs/b.md"),
                )
            }),
            ("R-10", "studies/alpha.json", None, pin0),
        ),
        (
            "pin blob mismatch",
            Box::new(|r| {
                set(
                    r,
                    "studies/alpha.json",
                    &format!("{pin0}/blob"),
                    json!(i.a_at_a),
                )
            }),
            ("R-10", "studies/alpha.json", None, pin0),
        ),
        (
            "pin names a tree",
            Box::new(|r| {
                set(
                    r,
                    "studies/alpha.json",
                    pin0,
                    json!({"path": "docs", "commit": i.a, "blob": i.docs_at_a}),
                )
            }),
            ("R-10", "studies/alpha.json", None, pin0),
        ),
        (
            "pin lines beyond the file",
            Box::new(|r| {
                set(
                    r,
                    "studies/alpha.json",
                    &format!("{pin0}/lines"),
                    json!("2-4"),
                )
            }),
            (
                "R-10",
                "studies/alpha.json",
                None,
                "/findings/0/sources/0/lines",
            ),
        ),
        (
            "pin commit names a blob",
            Box::new(|r| {
                set(
                    r,
                    "studies/alpha.json",
                    &format!("{pin0}/commit"),
                    json!(i.b_md),
                )
            }),
            ("R-10", "studies/alpha.json", None, pin0),
        ),
        (
            "event source pin blob mismatch",
            Box::new(|r| set(r, "events.jsonl", "/0/source/blob", json!(i.a_at_a))),
            ("R-10", "events.jsonl", Some(1), "/source"),
        ),
        // R-12
        (
            "decision document absent at its revision",
            Box::new(|r| set(r, "decisions.json", "/1/document", json!("docs/zz.md"))),
            ("R-12", "decisions.json", None, "/1"),
        ),
        (
            "decision lines beyond the document",
            Box::new(|r| set(r, "decisions.json", "/0/lines", json!("6-7"))),
            ("R-12", "decisions.json", None, "/0/lines"),
        ),
        // R-13
        (
            "listed revision equals the current one",
            Box::new(|r| {
                set(
                    r,
                    "decisions.json",
                    "/0/revisions/0/revision",
                    json!(format!("git:{}", i.b)),
                )
            }),
            ("R-13", "decisions.json", None, "/0/revisions/0"),
        ),
        (
            "listed revision repeats another",
            Box::new(|r| {
                let e = r["decisions.json"]
                    .pointer("/0/revisions/0")
                    .unwrap()
                    .clone();
                let mut e2 = e.clone();
                e2["revision"] = json!(format!("git:{}", &i.a[..9]));
                set(r, "decisions.json", "/0/revisions", json!([e, e2]));
            }),
            ("R-13", "decisions.json", None, "/0/revisions/1"),
        ),
        (
            "listed revision's document absent",
            Box::new(|r| {
                set(
                    r,
                    "decisions.json",
                    "/0/revisions/0",
                    json!({"revision": format!("git:{}", &i.a[..7]), "anchor": "A0",
             "superseded_at": "2026-01-01", "reason": "r", "document": "docs/zz.md"}),
                )
            }),
            ("R-13", "decisions.json", None, "/0/revisions/0"),
        ),
        (
            "listed revision's lines beyond the inherited document",
            Box::new(|r| {
                set(
                    r,
                    "decisions.json",
                    "/0/revisions/0",
                    json!({"revision": format!("git:{}", &i.a[..7]), "anchor": "A0",
             "superseded_at": "2026-01-01", "reason": "r", "lines": "6"}),
                )
            }),
            ("R-13", "decisions.json", None, "/0/revisions/0/lines"),
        ),
        // R-14
        (
            "listed revision not an ancestor of the current one",
            Box::new(|r| {
                set(
                    r,
                    "decisions.json",
                    "/0/revisions/0/revision",
                    json!(format!("git:{}", &i.x[..7])),
                )
            }),
            ("R-14", "decisions.json", None, "/0/revisions/0"),
        ),
        (
            "listed revisions out of order",
            Box::new(|r| {
                set(
                    r,
                    "decisions.json",
                    "/0/revisions",
                    json!([
             {"revision": format!("git:{}", &i.m[..7]), "anchor": "M0", "superseded_at": "2026-01-02", "reason": "r"},
             {"revision": format!("git:{}", &i.a[..7]), "anchor": "A0", "superseded_at": "2026-01-01", "reason": "r"}]),
                )
            }),
            ("R-14", "decisions.json", None, "/0/revisions/0"),
        ),
    ];
    let n = cases.len();
    for (name, mutate, expected) in cases {
        let mut reg = base(&i);
        mutate(&mut reg);
        let c = commit(&fx, &repo, &reg, &|_| {});
        assert_one(name, &violations(&repo, &c), expected);
    }
    assert_eq!(n, 27);
}

#[test]
fn file_set_rules_from_the_tree() {
    use std::os::unix::ffi::OsStrExt;
    let fx = Fx::new("validate-files");
    let (repo, i) = history(&fx, "r");
    let link = |name: &'static str, target: &'static str| {
        move |dir: &Path| {
            let _ = std::fs::remove_file(dir.join(name));
            std::os::unix::fs::symlink(target, dir.join(name)).unwrap();
        }
    };
    let cases: Vec<(&str, TreeEdit, Expected)> = vec![
        (
            "judgments.jsonl is a link",
            Box::new(link("judgments.jsonl", "questions.json")),
            ("R-01", "judgments.jsonl", None, ""),
        ),
        (
            "a study file is a link",
            Box::new(link("studies/zeta.json", "alpha.json")),
            ("R-01", "studies/zeta.json", None, ""),
        ),
        (
            "a non-UTF-8 entry name (A6)",
            Box::new(|dir: &Path| {
                std::fs::write(dir.join(std::ffi::OsStr::from_bytes(b"n\xffe.txt")), "x").unwrap()
            }),
            ("R-01", "n\\xffe.txt", None, ""),
        ),
        (
            "questions.json is a directory",
            Box::new(|dir: &Path| {
                std::fs::remove_file(dir.join("questions.json")).unwrap();
                write(&dir.join("questions.json/x"), "x");
            }),
            ("R-01", "questions.json", None, ""),
        ),
    ];
    for (name, extra, expected) in cases {
        let mut reg = base(&i);
        if name.starts_with("questions") {
            // no question to resolve without questions.json
            set(&mut reg, "studies/alpha.json", "/questions", json!([]));
        }
        let c = commit(&fx, &repo, &reg, &*extra);
        assert_one(name, &violations(&repo, &c), expected);
    }
    // studies/ that is a file: R-01, plus the references that no longer resolve.
    let mut reg = base(&i);
    reg.remove("studies/alpha.json");
    let c = commit(&fx, &repo, &reg, &|dir| write(&dir.join("studies"), "x"));
    let v = violations(&repo, &c);
    assert_eq!(
        (v[0].rule.as_str(), v[0].location.file.as_str()),
        ("R-01", "studies/"),
        "{:#?}",
        shown(&v)
    );
}

fn category(r: Result<Outcome, Error>) -> String {
    match r {
        Ok(Outcome::Valid(_)) => "valid".into(),
        Ok(Outcome::Invalid(v)) => format!("invalid {:?}", shown(&v)),
        Err(e) => e.category().into(),
    }
}

#[test]
fn missing_objects_are_cannot_read_never_invalid() {
    let fx = Fx::new("validate-f4");
    let (repo, i) = history(&fx, "r");
    let absent = "1111111111111111111111111111111111111111";
    let cases: Vec<(&str, Mutation)> = vec![
        (
            "pin commit absent (full id)",
            Box::new(|r| {
                set(
                    r,
                    "studies/alpha.json",
                    "/findings/0/sources/0/commit",
                    json!(absent),
                )
            }),
        ),
        (
            "pin commit absent (abbreviated)",
            Box::new(|r| set(r, "events.jsonl", "/0/source/commit", json!(&absent[..7]))),
        ),
        (
            "decision revision absent",
            Box::new(|r| {
                set(
                    r,
                    "decisions.json",
                    "/1/revision",
                    json!(format!("git:{absent}")),
                );
                set(
                    r,
                    "applicability.jsonl",
                    "/1/decision_revision",
                    json!(format!("git:{absent}")),
                );
            }),
        ),
        (
            "listed revision absent",
            Box::new(|r| {
                set(
                    r,
                    "decisions.json",
                    "/0/revisions/0/revision",
                    json!(format!("git:{}", &absent[..8])),
                )
            }),
        ),
    ];
    for (name, mutate) in cases {
        let mut reg = base(&i);
        mutate(&mut reg);
        let c = commit(&fx, &repo, &reg, &|_| {});
        assert_eq!(category(run(&repo, &c)), "cannot_read", "{name}");
    }
    // An unrelated defect does not hide a missing object (decision pins are kept for every
    // decision with its required keys).
    let mut reg = base(&i);
    set(&mut reg, "decisions.json", "/0/owner", json!(""));
    set(
        &mut reg,
        "decisions.json",
        "/0/revisions/0/revision",
        json!(format!("git:{}", &absent[..8])),
    );
    let c = commit(&fx, &repo, &reg, &|_| {});
    assert_eq!(
        category(run(&repo, &c)),
        "cannot_read",
        "missing revision beside an R-07 defect"
    );

    // A register blob missing from the object store.
    let c = commit(&fx, &repo, &base(&i), &|_| {});
    let q = fx.git(&repo, &["rev-parse", &format!("{c}:{REG}/questions.json")]);
    std::fs::remove_file(fx.loose(&repo, &q)).unwrap();
    assert_eq!(
        category(run(&repo, &c)),
        "cannot_read",
        "register blob missing"
    );
}

/// R-1b.4 item 2 (owner decision D-R12): a decision's or listed revision's document must be a
/// regular file entry (100644 or 100755) backed by a verified blob, with or without `lines`. A
/// link, tree or gitlink is an R-12 (current) or R-13 (listed) violation decided by the entry's
/// mode; an unreadable blob of a regular document, or an unreadable tree on the way, is
/// `cannot_read`.
#[test]
fn decision_documents_must_be_regular_files() {
    let fx = Fx::new("validate-documents");
    let (repo, i) = history(&fx, "r");
    // G (child of B): t/ (a tree whose object is removed below), u/f.md under a tree that is
    // removed below, w.md (a blob removed below) and docs/sub, a gitlink.
    write(&repo.join("t/only-here.txt"), "t\n");
    write(&repo.join("u/f.md"), "u-only\n");
    write(&repo.join("w.md"), "w-only\n");
    fx.git(&repo, &["add", "-A"]);
    fx.git(
        &repo,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("160000,{},docs/sub", i.x),
        ],
    );
    fx.git(&repo, &["commit", "-q", "-m", "G"]);
    let g = fx.git(&repo, &["rev-parse", "HEAD"]);
    let t_tree = fx.git(&repo, &["rev-parse", &format!("{g}:t")]);
    let u_tree = fx.git(&repo, &["rev-parse", &format!("{g}:u")]);
    let w_blob = fx.git(&repo, &["rev-parse", &format!("{g}:w.md")]);
    // Later register commits must not contain (and so rewrite) the objects removed below.
    fx.git(&repo, &["rm", "-r", "-q", "t", "u", "w.md", "docs/sub"]);
    let current_at = |r: &mut Reg, doc: &str, lines: Option<&str>| {
        set(
            r,
            "decisions.json",
            "/1/revision",
            json!(format!("git:{g}")),
        );
        set(
            r,
            "applicability.jsonl",
            "/1/decision_revision",
            json!(format!("git:{g}")),
        );
        set(r, "decisions.json", "/1/document", json!(doc));
        if let Some(l) = lines {
            r.get_mut("decisions.json").unwrap()[1]["lines"] = json!(l);
        }
    };
    let listed = |r: &mut Reg, doc: &str, lines: Option<&str>| {
        let mut e = json!({"revision": format!("git:{}", &i.a[..7]), "anchor": "A0",
                           "superseded_at": "2026-01-01", "reason": "r", "document": doc});
        if let Some(l) = lines {
            e["lines"] = json!(l);
        }
        set(r, "decisions.json", "/0/revisions/0", e);
    };
    let r12 = ("R-12", "decisions.json", None, "/1");
    let r13 = ("R-13", "decisions.json", None, "/0/revisions/0");
    let cases: Vec<(&str, Mutation, Expected)> = vec![
        (
            "current document is a tree, with lines",
            Box::new(|r| current_at(r, "docs", Some("1-40"))),
            r12,
        ),
        (
            "current document is a tree, with lines far beyond any count",
            Box::new(|r| current_at(r, "docs", Some("1-999999"))),
            r12,
        ),
        (
            "current document is a tree, without lines",
            Box::new(|r| current_at(r, "docs", None)),
            r12,
        ),
        (
            "current document is a symbolic link, without lines",
            Box::new(|r| current_at(r, "docs/l.md", None)),
            r12,
        ),
        (
            "current document is a symbolic link, with in-range lines",
            Box::new(|r| current_at(r, "docs/l.md", Some("1"))),
            r12,
        ),
        (
            "current document is a gitlink",
            Box::new(|r| current_at(r, "docs/sub", None)),
            r12,
        ),
        (
            "current document is a tree whose object is absent (mode decides)",
            Box::new(|r| current_at(r, "t", None)),
            r12,
        ),
        (
            "listed document is a tree, with lines",
            Box::new(|r| listed(r, "docs", Some("1-40"))),
            r13,
        ),
        (
            "listed document is a tree, without lines",
            Box::new(|r| listed(r, "docs", None)),
            r13,
        ),
        (
            "listed document is a symbolic link",
            Box::new(|r| listed(r, "docs/l.md", None)),
            r13,
        ),
    ];
    std::fs::remove_file(fx.loose(&repo, &t_tree)).unwrap();
    for (name, mutate, expected) in cases {
        let mut reg = base(&i);
        mutate(&mut reg);
        let c = commit(&fx, &repo, &reg, &|_| {});
        assert_one(name, &violations(&repo, &c), expected);
    }
    // Regular documents are unchanged: in range is valid, beyond the blob is a violation.
    let mut reg = base(&i);
    current_at(&mut reg, "w.md", Some("1"));
    let c = commit(&fx, &repo, &reg, &|_| {});
    assert_eq!(category(run(&repo, &c)), "valid", "regular document");
    let mut reg = base(&i);
    current_at(&mut reg, "w.md", Some("1-2"));
    let c = commit(&fx, &repo, &reg, &|_| {});
    assert_one(
        "regular document, lines beyond it",
        &violations(&repo, &c),
        ("R-12", "decisions.json", None, "/1/lines"),
    );
    // Required objects that cannot be read stay `cannot_read`.
    std::fs::remove_file(fx.loose(&repo, &w_blob)).unwrap();
    std::fs::remove_file(fx.loose(&repo, &u_tree)).unwrap();
    for (name, doc) in [
        ("regular document's blob absent", "w.md"),
        ("tree on the way to the document absent", "u/f.md"),
    ] {
        let mut reg = base(&i);
        current_at(&mut reg, doc, None);
        let c = commit(&fx, &repo, &reg, &|_| {});
        assert_eq!(category(run(&repo, &c)), "cannot_read", "{name}");
    }
}

#[test]
fn unanswerable_ancestry_is_cannot_read() {
    let fx = Fx::new("validate-ancestry");
    let (repo, i) = history(&fx, "r");
    let c = commit(&fx, &repo, &base(&i), &|_| {});
    assert_eq!(category(run(&repo, &c)), "valid");
    // The walk from B (current) to A (listed) needs M.
    std::fs::remove_file(fx.loose(&repo, &i.m)).unwrap();
    assert_eq!(category(run(&repo, &c)), "cannot_read");
}

/// A5: an abbreviation matching several objects is a violation of the pin's rule.
#[test]
fn ambiguous_abbreviated_pin_is_a_violation() {
    let fx = Fx::new("validate-a5");
    let (repo, i) = history(&fx, "r");
    // Enough blobs that two share a 7-hex prefix (deterministic contents, so deterministic ids).
    let mut stream = Vec::new();
    for k in 0..60_000u32 {
        let data = format!("filler {k}\n");
        stream.extend_from_slice(format!("blob\ndata {}\n{data}\n", data.len()).as_bytes());
    }
    fx.git_with(&repo, &["fast-import", "--quiet"], Some(&stream), &[]);
    let all = fx.git(
        &repo,
        &[
            "cat-file",
            "--batch-all-objects",
            "--batch-check=%(objectname)",
        ],
    );
    let mut ids: Vec<&str> = all.lines().collect();
    ids.sort_unstable();
    let prefix = ids
        .windows(2)
        .find(|w| w[0][..7] == w[1][..7])
        .map(|w| w[0][..7].to_owned())
        .expect("a shared 7-hex prefix among 60k objects");
    let mut reg = base(&i);
    set(
        &mut reg,
        "studies/alpha.json",
        "/findings/0/sources/0/commit",
        json!(prefix),
    );
    let c = commit(&fx, &repo, &reg, &|_| {});
    assert_one(
        "ambiguous pin",
        &violations(&repo, &c),
        ("R-10", "studies/alpha.json", None, "/findings/0/sources/0"),
    );
}

#[test]
fn validate_command_outcomes_and_exit_codes() {
    let fx = Fx::new("validate-cli");
    let (repo, i) = history(&fx, "r");
    let exe = env!("CARGO_BIN_EXE_work-engine-register");
    let call = |at: &str| {
        let out = std::process::Command::new(exe)
            .args(["validate", "--at", at, "--repo"])
            .arg(&repo)
            .output()
            .unwrap();
        let doc: Value = serde_json::from_slice(&out.stdout).expect("one JSON line");
        (out.status.code().unwrap(), doc)
    };
    let valid = commit(&fx, &repo, &base(&i), &|_| {});
    let (code, doc) = call(&valid);
    assert_eq!((code, doc["outcome"].as_str()), (0, Some("valid")), "{doc}");
    assert_eq!(doc["schema"], "research_register_validation_v1");
    assert_eq!(doc["commit"], json!(valid));

    let mut reg = base(&i);
    set(
        &mut reg,
        "judgments.jsonl",
        "/0/finding",
        json!("alpha#nine"),
    );
    let invalid = commit(&fx, &repo, &reg, &|_| {});
    let (code, doc) = call(&invalid);
    assert_eq!(
        (code, doc["outcome"].as_str()),
        (1, Some("invalid")),
        "{doc}"
    );
    assert_eq!(
        doc["violations"],
        json!([{"rule": "R-09", "file": "judgments.jsonl", "line": 1, "pointer": "/finding",
                "detail": "finding \"alpha#nine\" does not exist"}])
    );

    let mut reg = base(&i);
    set(
        &mut reg,
        "events.jsonl",
        "/0/source/commit",
        json!("1111111111111111111111111111111111111111"),
    );
    let unreadable = commit(&fx, &repo, &reg, &|_| {});
    let (code, doc) = call(&unreadable);
    assert_eq!(
        (code, doc["outcome"].as_str()),
        (4, Some("cannot_read")),
        "{doc}"
    );

    let (code, doc) = call(&valid[..12]);
    assert_eq!(
        (code, doc["outcome"].as_str()),
        (3, Some("bad_request")),
        "{doc}"
    );

    let out = std::process::Command::new(exe)
        .args(["validate"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
}
