//! Rule fixtures for the parse layer and typed decoding (Git-free rules). The base fixture is a
//! valid register; each rejecting case changes one thing and must report exactly one violation
//! with the expected rule id and location.

use research_register::{RuleId, Violation, decode};
use serde_json::{Value, json};
use std::collections::BTreeMap;

type Reg = BTreeMap<String, Value>;

const BLOB: &str = "0123456789abcdef0123456789abcdef01234567";
const SHA: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn pin() -> Value {
    json!({"path": "docs/a.md", "commit": "0123456", "blob": BLOB, "lines": "1-2"})
}

fn finding(id: &str) -> Value {
    json!({"id": id, "claim": "c", "value": 3, "n": 4, "basis": "measured", "scope": "s",
           "caveats": ["k"], "sources": [pin()]})
}

fn study(id: &str, finding_id: &str, alias: &str) -> Value {
    json!({"id": id, "title": "T", "kind": "probe", "questions": ["Q1"],
           "lifecycle": [{"date": "2026-01-02", "state": "concluded", "basis": pin()}],
           "stop_reason_class": null, "findings": [finding(finding_id)],
           "sources": [{"location": "~/x.txt", "sha256": null, "custody": "off_repo",
                        "status": "unverified", "note": "n"}],
           "aliases": [alias]})
}

/// A valid register. JSON Lines files are arrays of their records.
fn base() -> Reg {
    let mut r = Reg::new();
    r.insert(
        "questions.json".into(),
        json!([{"id": "Q1", "text": "Does it work?", "area": "engine"}]),
    );
    r.insert(
        "studies/alpha.json".into(),
        study("alpha", "alpha#one", "a-one"),
    );
    r.insert(
        "studies/beta.json".into(),
        study("beta", "beta#two", "b-one"),
    );
    r.insert(
        "decisions.json".into(),
        json!([
            {"decision_id": "d.one", "document": "docs/a.md", "anchor": "A", "revision": "git:0123456789",
             "owner": "o", "findings": ["alpha#one"], "lines": "3",
             "revisions": [{"revision": "git:abcdef0", "anchor": "A0", "superseded_at": "2026-01-01", "reason": "r"}],
             "historical": {"reason": "r", "marked_by": "m", "marked_at": "2026-01-03", "superseded_by": "d-two"}},
            {"decision_id": "d-two", "document": "docs/b.md", "anchor": "B", "revision": "git:0123456",
             "owner": "o", "findings": [],
             "evidence_state": {"state": "none_found", "corpus": {"id": "c", "sha256": SHA},
                                "search_scope": "s", "coverage": "c", "assessor": "a", "date": "2024-02-29"}},
            {"decision_id": "d-three", "document": "docs/c.md", "anchor": "C", "revision": "git:0123456",
             "owner": "o", "findings": [], "evidence_state": {"state": "none_linked"}}
        ]),
    );
    r.insert(
        "events.jsonl".into(),
        json!([{"event_id": "e-one", "kind": "correction", "date": "2026-01-04", "subject_finding": "alpha#one",
                "by_finding": null, "basis": "b", "source": pin()}]),
    );
    r.insert(
        "judgments.jsonl".into(),
        json!([{"finding": "alpha#one", "axis": "standing", "value": "qualified", "basis": "b",
                "assessor": "a", "date": "2026-01-05"}]),
    );
    r.insert(
        "applicability.jsonl".into(),
        json!([
            {"applicability_id": "ap-one", "finding": "alpha#one", "decision_id": "d.one",
             "decision_revision": "git:0123456789", "relation": "supports", "rationale": "r", "evidence": [pin()],
             "proposed_by": "p", "proposed_at": "2026-01-06",
             "authorization": {"status": "authorized", "owner": "o", "at": "2026-01-07"}},
            {"applicability_id": "ap-two", "finding": "beta#two", "decision_id": "d-two",
             "decision_revision": "git:0123456", "relation": "not_established", "rationale": "r",
             "evidence": [pin()], "proposed_by": "p", "proposed_at": "2026-01-06",
             "authorization": {"status": "pending", "owner": "o"}}
        ]),
    );
    r.insert(
        "materialized.jsonl".into(),
        json!([{"finding": "alpha#one", "label": "L", "claim_id": "c1", "revision_id": "r1", "revision_sha256": "x"}]),
    );
    r
}

fn render(reg: &Reg) -> BTreeMap<String, Vec<u8>> {
    reg.iter()
        .map(|(path, v)| {
            let bytes = if path.ends_with(".jsonl") {
                let lines: Vec<String> = v
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|l| l.to_string())
                    .collect();
                (lines.join("\n") + "\n").into_bytes()
            } else {
                v.to_string().into_bytes()
            };
            (path.clone(), bytes)
        })
        .collect()
}

fn run(files: &BTreeMap<String, Vec<u8>>) -> Vec<Violation> {
    decode(files.iter().map(|(p, b)| (p.as_str(), b.as_slice()))).violations
}

fn set(reg: &mut Reg, file: &str, pointer: &str, value: Value) {
    let doc = reg.get_mut(file).unwrap();
    if let Some(slot) = doc.pointer_mut(pointer) {
        *slot = value;
        return;
    }
    let (parent, key) = pointer.rsplit_once('/').unwrap();
    let obj = doc
        .pointer_mut(parent)
        .unwrap_or_else(|| panic!("{file} {pointer}"));
    obj.as_object_mut().unwrap().insert(key.to_owned(), value);
}

fn remove(reg: &mut Reg, file: &str, pointer: &str) {
    let (parent, key) = pointer.rsplit_once('/').unwrap();
    let obj = reg.get_mut(file).unwrap().pointer_mut(parent).unwrap();
    obj.as_object_mut().unwrap().remove(key).unwrap();
}

fn push(reg: &mut Reg, file: &str, pointer: &str, value: Value) {
    reg.get_mut(file)
        .unwrap()
        .pointer_mut(pointer)
        .unwrap()
        .as_array_mut()
        .unwrap()
        .push(value);
}

type Expect = (&'static str, &'static str, Option<u64>, &'static str);

fn summary(v: &Violation) -> (String, String, Option<u64>, String) {
    (
        v.rule.as_str().to_owned(),
        v.location.file.clone(),
        v.location.line,
        v.location.pointer.clone(),
    )
}

fn check_one(
    name: &str,
    files: &BTreeMap<String, Vec<u8>>,
    expect: Expect,
    failures: &mut Vec<String>,
) {
    let got: Vec<_> = run(files).iter().map(summary).collect();
    let want = (
        expect.0.to_owned(),
        expect.1.to_owned(),
        expect.2,
        expect.3.to_owned(),
    );
    if got != vec![want.clone()] {
        failures.push(format!("{name}: want [{want:?}], got {got:?}"));
    }
}

#[test]
fn base_register_is_accepted() {
    let got = run(&render(&base()));
    assert!(got.is_empty(), "{got:#?}");
    let d = decode(
        render(&base())
            .iter()
            .map(|(p, b)| (p.as_str(), b.as_slice())),
    );
    assert_eq!(d.records.studies.len(), 2);
    assert_eq!(d.records.decisions.len(), 3);
    assert_eq!(d.records.applicability.len(), 2);
    assert_eq!(d.index.finding_ids.len(), 2);
    assert_eq!(d.index.decisions["d.one"].listed.len(), 1);
    // pins: 2 studies x (lifecycle + finding) + event + 2 applicability
    assert_eq!(d.pins.len(), 7);
}

#[test]
fn accepting_variants() {
    type Edit = fn(&mut Reg);
    let cases: Vec<(&str, Edit)> = vec![
        ("n text", |r| {
            set(r, "studies/alpha.json", "/findings/0/n", json!("about 3"))
        }),
        ("n null", |r| {
            set(r, "studies/alpha.json", "/findings/0/n", json!(null))
        }),
        ("n negative integer", |r| {
            set(r, "studies/alpha.json", "/findings/0/n", json!(-2))
        }),
        ("40-hex pin commit", |r| {
            set(
                r,
                "studies/alpha.json",
                "/findings/0/sources/0/commit",
                json!(BLOB),
            )
        }),
        ("single line", |r| {
            set(
                r,
                "studies/alpha.json",
                "/findings/0/sources/0/lines",
                json!("7"),
            )
        }),
        ("pin without lines", |r| {
            remove(r, "studies/alpha.json", "/findings/0/sources/0/lines")
        }),
        ("verified off-repo", |r| {
            set(r, "studies/alpha.json", "/sources/0/sha256", json!(SHA));
            set(
                r,
                "studies/alpha.json",
                "/sources/0/status",
                json!("verified"),
            );
        }),
        ("stop class", |r| {
            set(
                r,
                "studies/alpha.json",
                "/stop_reason_class",
                json!("quota"),
            )
        }),
        ("empty sources", |r| {
            set(r, "studies/alpha.json", "/sources", json!([]))
        }),
        ("leap day 2000", |r| {
            set(r, "judgments.jsonl", "/0/date", json!("2000-02-29"))
        }),
        ("corpus pin", |r| {
            remove(r, "decisions.json", "/1/evidence_state/corpus/sha256");
            set(r, "decisions.json", "/1/evidence_state/corpus/pin", pin());
        }),
        ("revision entry document and lines", |r| {
            set(
                r,
                "decisions.json",
                "/0/revisions/0/document",
                json!("docs/old.md"),
            );
            set(r, "decisions.json", "/0/revisions/0/lines", json!("2-4"));
        }),
        ("declined with at", |r| {
            set(
                r,
                "applicability.jsonl",
                "/1/authorization/status",
                json!("declined"),
            );
            set(
                r,
                "applicability.jsonl",
                "/1/authorization/at",
                json!("2026-02-01"),
            );
        }),
        ("unknown-type finding value", |r| {
            set(
                r,
                "studies/alpha.json",
                "/findings/0/value",
                json!({"a": [1, null]}),
            )
        }),
    ];
    let mut failures = Vec::new();
    for (name, edit) in cases {
        let mut r = base();
        edit(&mut r);
        let got: Vec<_> = run(&render(&r)).iter().map(summary).collect();
        if !got.is_empty() {
            failures.push(format!("{name}: {got:?}"));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn jsonl_blank_lines_and_crlf_are_accepted() {
    let mut files = render(&base());
    let ev = String::from_utf8(files["events.jsonl"].clone()).unwrap();
    files.insert(
        "events.jsonl".into(),
        format!("\n  \n{}\r\n\n", ev.trim_end()).into_bytes(),
    );
    assert!(run(&files).is_empty());
}

#[test]
fn rejecting_cases_report_one_violation() {
    type Edit = fn(&mut Reg);
    const A: &str = "studies/alpha.json";
    const D: &str = "decisions.json";
    const AP: &str = "applicability.jsonl";
    let cases: Vec<(&str, Edit, Expect)> = vec![
        // R-03 closed key sets
        (
            "unknown study key",
            |r| set(r, A, "/extra", json!(1)),
            ("R-03", A, None, "/extra"),
        ),
        (
            "missing judgment key",
            |r| remove(r, "judgments.jsonl", "/0/assessor"),
            ("R-03", "judgments.jsonl", Some(1), ""),
        ),
        (
            "unknown pin key",
            |r| set(r, A, "/findings/0/sources/0/sha", json!("x")),
            ("R-03", A, None, "/findings/0/sources/0/sha"),
        ),
        (
            "unknown historical key",
            |r| set(r, D, "/0/historical/when", json!("x")),
            ("R-03", D, None, "/0/historical/when"),
        ),
        (
            "unknown corpus key",
            |r| set(r, D, "/1/evidence_state/corpus/size", json!(1)),
            ("R-03", D, None, "/1/evidence_state/corpus/size"),
        ),
        (
            "unknown authorization key",
            |r| set(r, AP, "/0/authorization/by", json!("x")),
            ("R-03", AP, Some(1), "/authorization/by"),
        ),
        // R-04
        (
            "global usefulness key",
            |r| set(r, A, "/findings/0/Usefulness", json!("high")),
            ("R-04", A, None, "/findings/0/Usefulness"),
        ),
        (
            "global obsolete key",
            |r| set(r, D, "/0/obsolete", json!(true)),
            ("R-04", D, None, "/0/obsolete"),
        ),
        // R-05
        (
            "relation",
            |r| set(r, AP, "/0/relation", json!("helps")),
            ("R-05", AP, Some(1), "/relation"),
        ),
        (
            "axis value",
            |r| set(r, "judgments.jsonl", "/0/value", json!("high")),
            ("R-05", "judgments.jsonl", Some(1), "/value"),
        ),
        (
            "axis",
            |r| set(r, "judgments.jsonl", "/0/axis", json!("quality")),
            ("R-05", "judgments.jsonl", Some(1), "/axis"),
        ),
        (
            "off-repo status",
            |r| set(r, A, "/sources/0/status", json!("lost")),
            ("R-05", A, None, "/sources/0/status"),
        ),
        (
            "event kind",
            |r| set(r, "events.jsonl", "/0/kind", json!("note")),
            ("R-05", "events.jsonl", Some(1), "/kind"),
        ),
        (
            "evidence state",
            |r| set(r, D, "/2/evidence_state/state", json!("none")),
            ("R-05", D, None, "/2/evidence_state/state"),
        ),
        (
            "authorization status",
            |r| set(r, AP, "/1/authorization/status", json!("open")),
            ("R-05", AP, Some(2), "/authorization/status"),
        ),
        // R-05b
        (
            "study kind",
            |r| set(r, A, "/kind", json!("survey")),
            ("R-05b", A, None, "/kind"),
        ),
        (
            "lifecycle state",
            |r| set(r, A, "/lifecycle/0/state", json!("done")),
            ("R-05b", A, None, "/lifecycle/0/state"),
        ),
        (
            "finding basis",
            |r| set(r, A, "/findings/0/basis", json!("guessed")),
            ("R-05b", A, None, "/findings/0/basis"),
        ),
        (
            "stop class",
            |r| set(r, A, "/stop_reason_class", json!("bored")),
            ("R-05b", A, None, "/stop_reason_class"),
        ),
        // R-06
        (
            "question id",
            |r| set(r, "questions.json", "/0/id", json!("q1")),
            ("R-06", "questions.json", None, "/0/id"),
        ),
        (
            "duplicate question",
            |r| {
                push(
                    r,
                    "questions.json",
                    "",
                    json!({"id": "Q1", "text": "t", "area": "a"}),
                )
            },
            ("R-06", "questions.json", None, "/1/id"),
        ),
        (
            "study id not kebab",
            |r| set(r, A, "/id", json!("Alpha")),
            ("R-06", A, None, "/id"),
        ),
        (
            "finding under another study",
            |r| set(r, A, "/findings/0/id", json!("beta#one")),
            ("R-06", A, None, "/findings/0/id"),
        ),
        (
            "finding id format",
            |r| set(r, A, "/findings/0/id", json!("alpha-one")),
            ("R-06", A, None, "/findings/0/id"),
        ),
        (
            "duplicate finding",
            |r| push(r, A, "/findings", finding("alpha#one")),
            ("R-06", A, None, "/findings/1/id"),
        ),
        (
            "duplicate alias",
            |r| set(r, "studies/beta.json", "/aliases/0", json!("a-one")),
            ("R-06", "studies/beta.json", None, "/aliases/0"),
        ),
        (
            "decision id",
            |r| set(r, D, "/0/decision_id", json!("d_one")),
            ("R-06", D, None, "/0/decision_id"),
        ),
        (
            "duplicate decision",
            |r| set(r, D, "/2/decision_id", json!("d-two")),
            ("R-06", D, None, "/2/decision_id"),
        ),
        (
            "duplicate applicability",
            |r| set(r, AP, "/1/applicability_id", json!("ap-one")),
            ("R-06", AP, Some(2), "/applicability_id"),
        ),
        (
            "event id",
            |r| set(r, "events.jsonl", "/0/event_id", json!("e one")),
            ("R-06", "events.jsonl", Some(1), "/event_id"),
        ),
        // R-07
        (
            "blank title",
            |r| set(r, A, "/title", json!("  ")),
            ("R-07", A, None, "/title"),
        ),
        (
            "n float",
            |r| set(r, A, "/findings/0/n", json!(2.5)),
            ("R-07", A, None, "/findings/0/n"),
        ),
        (
            "n bool",
            |r| set(r, A, "/findings/0/n", json!(true)),
            ("R-07", A, None, "/findings/0/n"),
        ),
        (
            "caveat not text",
            |r| set(r, A, "/findings/0/caveats/0", json!("")),
            ("R-07", A, None, "/findings/0/caveats/0"),
        ),
        (
            "study findings empty",
            |r| set(r, A, "/findings", json!([])),
            ("R-07", A, None, "/findings"),
        ),
        (
            "finding sources empty",
            |r| set(r, A, "/findings/0/sources", json!([])),
            ("R-07", A, None, "/findings/0/sources"),
        ),
        (
            "lifecycle empty",
            |r| set(r, A, "/lifecycle", json!([])),
            ("R-07", A, None, "/lifecycle"),
        ),
        (
            "questions not a list",
            |r| set(r, A, "/questions", json!("Q1")),
            ("R-07", A, None, "/questions"),
        ),
        (
            "question ref not a string",
            |r| set(r, A, "/questions/0", json!(1)),
            ("R-07", A, None, "/questions/0"),
        ),
        (
            "questions.json not a list",
            |r| {
                r.insert("questions.json".into(), json!({}));
            },
            ("R-07", "questions.json", None, ""),
        ),
        (
            "event not an object",
            |r| set(r, "events.jsonl", "/0", json!([1])),
            ("R-07", "events.jsonl", Some(1), ""),
        ),
        (
            "decision owner blank",
            |r| set(r, D, "/0/owner", json!("")),
            ("R-07", D, None, "/0/owner"),
        ),
        (
            "revisions empty",
            |r| set(r, D, "/0/revisions", json!([])),
            ("R-07", D, None, "/0/revisions"),
        ),
        (
            "applicability evidence empty",
            |r| set(r, AP, "/0/evidence", json!([])),
            ("R-07", AP, Some(1), "/evidence"),
        ),
        (
            "materialized finding not a string",
            |r| set(r, "materialized.jsonl", "/0/finding", json!(1)),
            ("R-07", "materialized.jsonl", Some(1), "/finding"),
        ),
        // R-08 and F-3
        (
            "date shape",
            |r| set(r, "events.jsonl", "/0/date", json!("2026-1-04")),
            ("R-08", "events.jsonl", Some(1), "/date"),
        ),
        (
            "date not a string",
            |r| set(r, A, "/lifecycle/0/date", json!(20260102)),
            ("R-08", A, None, "/lifecycle/0/date"),
        ),
        (
            "historical date",
            |r| set(r, D, "/0/historical/marked_at", json!("03/01/2026")),
            ("R-08", D, None, "/0/historical/marked_at"),
        ),
        (
            "Feb 30",
            |r| set(r, "judgments.jsonl", "/0/date", json!("2026-02-30")),
            ("F-3", "judgments.jsonl", Some(1), "/date"),
        ),
        (
            "Feb 29 non-leap",
            |r| set(r, "judgments.jsonl", "/0/date", json!("2025-02-29")),
            ("F-3", "judgments.jsonl", Some(1), "/date"),
        ),
        (
            "Feb 29 1900",
            |r| set(r, "judgments.jsonl", "/0/date", json!("1900-02-29")),
            ("F-3", "judgments.jsonl", Some(1), "/date"),
        ),
        (
            "month 13",
            |r| set(r, AP, "/0/proposed_at", json!("2026-13-01")),
            ("F-3", AP, Some(1), "/proposed_at"),
        ),
        (
            "year 0",
            |r| set(r, D, "/0/revisions/0/superseded_at", json!("0000-01-01")),
            ("F-3", D, None, "/0/revisions/0/superseded_at"),
        ),
        // F-2
        (
            "null study",
            |r| {
                r.insert(A.into(), json!(null));
            },
            ("F-2", A, None, ""),
        ),
        // R-10 pin shape
        (
            "short blob",
            |r| set(r, A, "/findings/0/sources/0/blob", json!("0123456")),
            ("R-10", A, None, "/findings/0/sources/0/blob"),
        ),
        (
            "uppercase commit",
            |r| set(r, "events.jsonl", "/0/source/commit", json!("ABCDEF0")),
            ("R-10", "events.jsonl", Some(1), "/source/commit"),
        ),
        (
            "lines from zero",
            |r| set(r, A, "/findings/0/sources/0/lines", json!("0-3")),
            ("R-10", A, None, "/findings/0/sources/0/lines"),
        ),
        (
            "lines descending",
            |r| set(r, A, "/lifecycle/0/basis/lines", json!("3-2")),
            ("R-10", A, None, "/lifecycle/0/basis/lines"),
        ),
        (
            "lines number",
            |r| set(r, A, "/lifecycle/0/basis/lines", json!(3)),
            ("R-10", A, None, "/lifecycle/0/basis/lines"),
        ),
        (
            "source not an object",
            |r| set(r, A, "/sources/0", json!("docs/a.md")),
            ("R-10", A, None, "/sources/0"),
        ),
        (
            "pin path blank",
            |r| set(r, AP, "/0/evidence/0/path", json!("")),
            ("R-10", AP, Some(1), "/evidence/0/path"),
        ),
        // R-11
        (
            "custody",
            |r| set(r, A, "/sources/0/custody", json!("on_repo")),
            ("R-11", A, None, "/sources/0/custody"),
        ),
        (
            "verified without sha256",
            |r| set(r, A, "/sources/0/status", json!("verified")),
            ("R-11", A, None, "/sources/0"),
        ),
        (
            "off-repo sha256",
            |r| set(r, A, "/sources/0/sha256", json!("abc")),
            ("R-11", A, None, "/sources/0/sha256"),
        ),
        (
            "off-repo location blank",
            |r| set(r, A, "/sources/0/location", json!(" ")),
            ("R-11", A, None, "/sources/0/location"),
        ),
        // R-12, R-13
        (
            "decision revision",
            |r| set(r, D, "/0/revision", json!("0123456789")),
            ("R-12", D, None, "/0/revision"),
        ),
        (
            "decision revision short",
            |r| set(r, D, "/0/revision", json!("git:012345")),
            ("R-12", D, None, "/0/revision"),
        ),
        (
            "decision lines",
            |r| set(r, D, "/0/lines", json!("x")),
            ("R-12", D, None, "/0/lines"),
        ),
        (
            "revision entry",
            |r| set(r, D, "/0/revisions/0/revision", json!("git:xyz1234")),
            ("R-13", D, None, "/0/revisions/0/revision"),
        ),
        (
            "revision entry lines",
            |r| set(r, D, "/0/revisions/0/lines", json!("2-")),
            ("R-13", D, None, "/0/revisions/0/lines"),
        ),
        // R-16
        (
            "findings [] without evidence_state",
            |r| set(r, D, "/0/findings", json!([])),
            ("R-16", D, None, "/0/findings"),
        ),
        (
            "evidence_state with findings (A7)",
            |r| set(r, D, "/2/findings", json!(["alpha#one"])),
            ("R-16", D, None, "/2/evidence_state"),
        ),
        (
            "none_found without corpus",
            |r| remove(r, D, "/1/evidence_state/corpus"),
            ("R-16", D, None, "/1/evidence_state"),
        ),
        (
            "none_found without coverage",
            |r| remove(r, D, "/1/evidence_state/coverage"),
            ("R-16", D, None, "/1/evidence_state"),
        ),
        (
            "corpus without sha256 or pin",
            |r| remove(r, D, "/1/evidence_state/corpus/sha256"),
            ("R-16", D, None, "/1/evidence_state/corpus"),
        ),
        (
            "corpus sha256",
            |r| set(r, D, "/1/evidence_state/corpus/sha256", json!("x")),
            ("R-16", D, None, "/1/evidence_state/corpus/sha256"),
        ),
        (
            "none_linked with coverage",
            |r| set(r, D, "/2/evidence_state/coverage", json!("c")),
            ("R-16", D, None, "/2/evidence_state/coverage"),
        ),
        (
            "evidence_state without state",
            |r| remove(r, D, "/2/evidence_state/state"),
            ("R-16", D, None, "/2/evidence_state"),
        ),
        // R-17
        (
            "decision_revision absent",
            |r| remove(r, AP, "/0/decision_revision"),
            ("R-17", AP, Some(1), ""),
        ),
        (
            "decision_revision null",
            |r| set(r, AP, "/0/decision_revision", json!(null)),
            ("R-17", AP, Some(1), ""),
        ),
        (
            "decision_revision format",
            |r| set(r, AP, "/0/decision_revision", json!("abc")),
            ("R-17", AP, Some(1), "/decision_revision"),
        ),
        (
            "authorization without status",
            |r| remove(r, AP, "/0/authorization/status"),
            ("R-17", AP, Some(1), ""),
        ),
        (
            "authorization absent",
            |r| remove(r, AP, "/1/authorization"),
            ("R-17", AP, Some(2), ""),
        ),
        (
            "authorized without at",
            |r| remove(r, AP, "/0/authorization/at"),
            ("R-17", AP, Some(1), "/authorization"),
        ),
        // R-18
        (
            "materialized unknown key",
            |r| set(r, "materialized.jsonl", "/0/extra", json!(1)),
            ("R-18", "materialized.jsonl", Some(1), "/extra"),
        ),
        (
            "materialized missing key",
            |r| remove(r, "materialized.jsonl", "/0/claim_id"),
            ("R-18", "materialized.jsonl", Some(1), ""),
        ),
    ];
    let mut failures = Vec::new();
    for (name, edit, expect) in cases {
        let mut r = base();
        edit(&mut r);
        check_one(name, &render(&r), expect, &mut failures);
    }
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn study_file_name_must_match_id() {
    let mut files = render(&base());
    let bytes = files.remove("studies/beta.json").unwrap();
    files.insert("studies/gamma.json".into(), bytes);
    let mut failures = Vec::new();
    check_one(
        "rename",
        &files,
        ("R-06", "studies/gamma.json", None, "/id"),
        &mut failures,
    );
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn only_direct_json_entries_are_study_files() {
    let mut files = render(&base());
    files.insert("studies/notes.txt".into(), b"not json".to_vec());
    files.insert("studies/sub/x.json".into(), b"not json".to_vec());
    assert!(run(&files).is_empty());
}

#[test]
fn parse_layer_rules() {
    let cases: Vec<(&str, &str, &[u8], Expect)> = vec![
        (
            "invalid UTF-8 on line 2",
            "events.jsonl",
            b"{}\n\xff\n",
            ("R-02", "events.jsonl", Some(2), ""),
        ),
        (
            "syntax error",
            "questions.json",
            b"[{]",
            ("R-02", "questions.json", Some(1), ""),
        ),
        (
            "truncated JSONL line",
            "judgments.jsonl",
            b"{\"finding\": \n",
            ("R-02", "judgments.jsonl", Some(1), ""),
        ),
        (
            "NaN text in a string is not F-6",
            "questions.json",
            b"[{\"id\": \"NaN\"",
            ("R-02", "questions.json", Some(1), ""),
        ),
        (
            "BOM",
            "decisions.json",
            b"\xef\xbb\xbf[]",
            ("R-02", "decisions.json", Some(1), ""),
        ),
        (
            "duplicate key in a study",
            "studies/alpha.json",
            b"{\"id\": \"alpha\",\n\"id\": \"alpha\"}",
            ("F-5", "studies/alpha.json", Some(2), ""),
        ),
        (
            "nested duplicate key",
            "events.jsonl",
            b"{\"a\": {\"b\": 1, \"b\": 1}}\n",
            ("F-5", "events.jsonl", Some(1), ""),
        ),
        (
            "NaN",
            "events.jsonl",
            b"{\"x\": NaN}\n",
            ("F-6", "events.jsonl", Some(1), ""),
        ),
        (
            "-Infinity",
            "applicability.jsonl",
            b"\n{\"x\": [-Infinity]}\n",
            ("F-6", "applicability.jsonl", Some(2), ""),
        ),
        (
            "Infinity",
            "decisions.json",
            b"[Infinity]",
            ("F-6", "decisions.json", Some(1), ""),
        ),
        (
            "overflow",
            "questions.json",
            b"[1e400]",
            ("F-6", "questions.json", Some(1), ""),
        ),
    ];
    let mut failures = Vec::new();
    for (name, file, bytes, expect) in cases {
        let mut files = render(&base());
        files.insert(file.into(), bytes.to_vec());
        check_one(name, &files, expect, &mut failures);
    }
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn rule_ids_are_stable() {
    let ids: Vec<&str> = [
        RuleId::R01,
        RuleId::R02,
        RuleId::F5,
        RuleId::F6,
        RuleId::R03,
        RuleId::R04,
        RuleId::R05,
        RuleId::R05b,
        RuleId::R06,
        RuleId::R07,
        RuleId::R08,
        RuleId::F3,
        RuleId::F2,
        RuleId::R09,
        RuleId::R10,
        RuleId::R11,
        RuleId::R12,
        RuleId::R13,
        RuleId::R14,
        RuleId::R15,
        RuleId::R16,
        RuleId::R17,
        RuleId::R18,
    ]
    .iter()
    .map(|r| r.as_str())
    .collect();
    assert_eq!(
        ids.join(" "),
        "R-01 R-02 F-5 F-6 R-03 R-04 R-05 R-05b R-06 R-07 R-08 F-3 F-2 R-09 R-10 R-11 R-12 R-13 R-14 R-15 R-16 R-17 R-18"
    );
}

/// Decodes a register directory named by `RESEARCH_REGISTER_DIR` (for qualification runs against
/// extracted historical registers; ignored by default).
#[test]
#[ignore]
fn decode_register_dir_from_env() {
    let dir = std::path::PathBuf::from(
        std::env::var("RESEARCH_REGISTER_DIR").expect("RESEARCH_REGISTER_DIR"),
    );
    let mut files = BTreeMap::new();
    for name in [
        "questions.json",
        "events.jsonl",
        "judgments.jsonl",
        "decisions.json",
        "applicability.jsonl",
        "materialized.jsonl",
    ] {
        if let Ok(b) = std::fs::read(dir.join(name)) {
            files.insert(name.to_owned(), b);
        }
    }
    for entry in std::fs::read_dir(dir.join("studies")).unwrap() {
        let entry = entry.unwrap();
        files.insert(
            format!("studies/{}", entry.file_name().to_str().unwrap()),
            std::fs::read(entry.path()).unwrap(),
        );
    }
    let d = decode(files.iter().map(|(p, b)| (p.as_str(), b.as_slice())));
    println!(
        "files {} studies {} decisions {} pins {}",
        files.len(),
        d.records.studies.len(),
        d.records.decisions.len(),
        d.pins.len()
    );
    assert!(d.violations.is_empty(), "{:#?}", d.violations);
}

/// R-1b.4 item 5 (owner decision D-CF02): an R-07 integer is a JSON number literal without a
/// fraction or exponent, of any magnitude, kept losslessly as its decimal text.
#[test]
fn r07_integers_of_any_magnitude_are_kept_losslessly() {
    use research_register::Json;
    use research_register::model::SampleSize;
    let files = render(&base());
    let file = "studies/alpha.json";
    let study = String::from_utf8(files[file].clone()).unwrap();
    assert!(study.contains("\"n\":4") && study.contains("\"value\":3"));
    let with = |text: String| {
        let mut f = files.clone();
        f.insert(file.into(), text.into_bytes());
        f
    };
    let with_n = |n: &str| with(study.replace("\"n\":4", &format!("\"n\":{n}")));
    let decoded_n = |f: &BTreeMap<String, Vec<u8>>| {
        let d = decode(f.iter().map(|(p, b)| (p.as_str(), b.as_slice())));
        let finding = &d.records.studies[0].findings[0];
        let value = match &finding.value {
            Json::Number(v) => v.to_string(),
            other => format!("{other:?}"),
        };
        let n = match &finding.n {
            Some(SampleSize::Integer(v)) => v.to_string(),
            other => format!("{other:?}"),
        };
        (d.violations, n, value)
    };
    let nines = "9".repeat(400);
    for n in [
        "18446744073709551615",
        "18446744073709551616",
        "-9223372036854775808",
        "-9223372036854775809",
        "1234567890123456789012345678901234567890",
        nines.as_str(),
        "-0",
        "0",
    ] {
        let (v, got, _) = decoded_n(&with_n(n));
        assert!(
            v.is_empty(),
            "{n}: {:?}",
            v.iter().map(summary).collect::<Vec<_>>()
        );
        assert_eq!(got, n, "n is kept as written");
    }
    let mut failures = Vec::new();
    for n in [
        "1.0", "1e3", "1E3", "1e+3", "-0.0", "0.5", "1e-400", "10E-1",
    ] {
        check_one(
            n,
            &with_n(n),
            ("R-07", file, None, "/findings/0/n"),
            &mut failures,
        );
    }
    // Not JSON numbers: still parse errors (R-02), never integers.
    for n in [
        "01", "-01", "1.", ".5", "+1", "-", "1e", "1e+", "0x10", "1_0",
    ] {
        check_one(n, &with_n(n), ("R-02", file, Some(1), ""), &mut failures);
    }
    // A float literal that overflows f64 is still F-6; a long integer is not.
    check_one(
        "1e400",
        &with_n("1e400"),
        ("F-6", file, Some(1), ""),
        &mut failures,
    );
    assert!(failures.is_empty(), "{failures:#?}");
    // Digits, signs, exponents, quotes and escapes inside strings (keys and values) before and
    // around the numbers do not shift the pairing of numbers with their literals.
    let tricky = study
        .replace(
            "\"claim\":\"c\"",
            r#""claim":"\"1\" -2 3e5 \\","1e9 \"x":"7","z":"\\\"""#,
        )
        .replace("\"n\":4", "\"n\":123456789012345678901234567890");
    let (v, n, value) = decoded_n(&with(tricky));
    assert_eq!(
        v.iter().map(summary).collect::<Vec<_>>(),
        vec![
            (
                "R-03".into(),
                file.into(),
                None,
                "/findings/0/1e9 \"x".into()
            ),
            ("R-03".into(), file.into(), None, "/findings/0/z".into()),
        ],
        "only the two extra keys are reported"
    );
    assert_eq!(
        (n.as_str(), value.as_str()),
        ("123456789012345678901234567890", "3")
    );
}
