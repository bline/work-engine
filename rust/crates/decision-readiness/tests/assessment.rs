//! `check --kind assessment` (contract 6; AC1, AC4 fixture form). The tool validates
//! assessments and never writes one: every assessment here is built by the test.

mod support;

use serde_json::{Value, json};
use std::path::Path;
use support::*;

/// An assessment that agrees with `evaluation` under `profile`: statuses copied, attributed
/// requirements assessed indeterminate with a reason and evidence, the derived verdict, and the
/// freshness basis copied from the evaluation. `purpose` is fixture.
fn honest(profile: &Path, evaluation: &Path) -> Value {
    let p = read_json(profile);
    let e = read_json(evaluation);
    let mut entries = Vec::new();
    let (mut missing, mut indeterminate) = (Vec::new(), Vec::new());
    for r in p["requirements"].as_array().unwrap() {
        let id = r["id"].as_str().unwrap();
        let from = e["requirements"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["id"] == id)
            .map(|x| x["status"].clone())
            .unwrap();
        let attributed = r["class"] == "attributed";
        let assessed = if attributed {
            json!("indeterminate")
        } else {
            from.clone()
        };
        match assessed.as_str().unwrap() {
            "unmet" => missing.push(id.to_string()),
            "indeterminate" => indeterminate.push(id.to_string()),
            _ => {}
        }
        entries.push(json!({
            "id": id, "evaluation_status": from, "assessed_status": assessed,
            "basis": "evaluation",
            "reason": if attributed { "judgment pending" } else { "" },
            "evidence": if attributed { json!(["notes/a.md"]) } else { json!([]) }
        }));
    }
    let verdict = if !missing.is_empty() {
        "blocked"
    } else if !indeterminate.is_empty() {
        "uncertain"
    } else {
        "ready"
    };
    json!({
        "schema": "decision-readiness-assessment-v1",
        "purpose": "fixture",
        "assessment_id": "fixture-1",
        "issued_at": "2026-10-21T12:01:00Z",
        "assessor": {"name": "tester", "role": p["assessor_roles"][0], "session": "s",
                     "model_requested": "m", "model_observed": "unknown", "authority_ref": "packet-1"},
        "decision": p["decision"],
        "profile": {"id": p["profile_id"], "version": p["version"], "sha256": file_sha(profile)},
        "evaluation": {"path": s(evaluation), "sha256": file_sha(evaluation)},
        "verdict": verdict,
        "requirements": entries,
        "missing": missing,
        "indeterminate": indeterminate,
        "profile_gaps": [],
        "evidence": [],
        "freshness_basis": {
            "observation": {
                "battery_file_sha256": e["battery"]["file_sha256"],
                "battery_sha256": e["battery"]["battery_sha256"],
                "observation_sha256": e["observation"]["sha256"],
                "started_at": e["observation"]["started_at"],
                "finished_at": e["observation"]["finished_at"]
            },
            "claim_reliance": [],
            "note": "fixture"
        },
        "limits": []
    })
}

fn assess(dir: &Path, tag: &str, profile: &Path, evaluation: &Path, doc: &Value) -> Run {
    let path = dir.join(format!("{tag}.assessment.json"));
    write_json(&path, doc);
    cli(&[
        "check",
        "--kind",
        "assessment",
        &s(&path),
        "--evaluation",
        &s(evaluation),
        "--profile",
        &s(profile),
    ])
}

struct Fixture {
    case: Case,
    evaluation: std::path::PathBuf,
    doc: Value,
}

fn fixture(name: &str, met: bool) -> Fixture {
    let case = if met { met_case(name) } else { base_case(name) };
    let observed = observe(&case, "run");
    let evaluation = evaluation_file(&case, &observed, "e.json");
    // AC5 (a): the evaluation every assessment fixture starts from has no verdict key anywhere.
    assert!(!has_key_deep(&read_json(&evaluation), "verdict"));
    let doc = honest(&case.profile_path(), &evaluation);
    Fixture {
        case,
        evaluation,
        doc,
    }
}

impl Fixture {
    fn run(&self, tag: &str, doc: &Value) -> Run {
        assess(
            &self.case.dir,
            tag,
            &self.case.profile_path(),
            &self.evaluation,
            doc,
        )
    }
    fn codes_after(&self, tag: &str, mutate: impl Fn(&mut Value)) -> Vec<String> {
        let mut doc = self.doc.clone();
        mutate(&mut doc);
        let run = self.run(tag, &doc);
        assert_eq!(run.code, 1, "{tag}: {}", run.stdout);
        let mut codes = run.codes();
        codes.sort();
        codes
    }
}

#[test]
fn an_assessment_that_agrees_with_its_evaluation_is_accepted_and_echoes_purpose() {
    let f = fixture("assess-accept", true);
    let run = f.run("ok", &f.doc);
    assert_eq!(run.code, 0, "{}", run.stdout);
    let report = run.out_json();
    assert_eq!(
        (report["valid"].clone(), report["purpose"].clone()),
        (json!(true), json!("fixture"))
    );
    assert_eq!(f.doc["verdict"], "uncertain");
    // The purpose is echoed for each valid value and all rules apply to every purpose.
    for purpose in ["live", "demonstration"] {
        let mut doc = f.doc.clone();
        doc["purpose"] = json!(purpose);
        let report = f.run(purpose, &doc).out_json();
        assert_eq!(report["purpose"], purpose);
        assert_eq!(report["valid"], true);
    }
    // A document that fails the closed schema still echoes a valid purpose; an invalid one is not echoed.
    let mut doc = f.doc.clone();
    doc["accepted"] = json!(true);
    doc["purpose"] = json!("demonstration");
    let run = f.run("extra", &doc);
    assert_eq!(run.code, 1);
    assert_eq!(run.out_json()["purpose"], "demonstration");
    let mut doc = f.doc.clone();
    doc["purpose"] = json!("bogus");
    let run = f.run("bogus", &doc);
    assert_eq!(run.code, 1);
    assert!(run.out_json().get("purpose").is_none());
}

#[test]
fn rules_1_and_2_digests_decision_and_coverage() {
    let f = fixture("assess-rules-1-2", true);
    let one = |code: &str, mutate: &dyn Fn(&mut Value)| {
        assert_eq!(
            f.codes_after(code, mutate),
            vec![code.to_string()],
            "{code}"
        );
    };
    one("evaluation_digest_mismatch", &|v| {
        v["evaluation"]["sha256"] = json!("0".repeat(64))
    });
    one("decision_mismatch", &|v| {
        v["decision"]["id"] = json!("other")
    });
    one("requirement_missing", &|v| {
        v["requirements"].as_array_mut().unwrap().remove(0);
    });
    one("requirement_unknown", &|v| {
        let mut extra = v["requirements"][0].clone();
        extra["id"] = json!("ZZ");
        v["requirements"].as_array_mut().unwrap().push(extra);
    });
    one("requirement_duplicate", &|v| {
        let extra = v["requirements"][0].clone();
        v["requirements"].as_array_mut().unwrap().push(extra);
    });
    // A wrong profile digest or version also means the evaluation cites a different profile.
    let codes = f.codes_after("digest", |v| v["profile"]["sha256"] = json!("0".repeat(64)));
    assert_eq!(
        codes,
        ["evaluation_profile_mismatch", "profile_digest_mismatch"]
    );
    let codes = f.codes_after("version", |v| v["profile"]["version"] = json!(2));
    assert_eq!(
        codes,
        ["evaluation_profile_mismatch", "profile_version_mismatch"]
    );
    // A wrong profile id also means the evaluation cites a different profile.
    let codes = f.codes_after("id", |v| v["profile"]["id"] = json!("other"));
    assert_eq!(
        codes,
        ["evaluation_profile_mismatch", "profile_id_mismatch"]
    );
    let codes = f.codes_after("status", |v| {
        v["requirements"][0]["evaluation_status"] = json!("unmet")
    });
    assert!(
        codes.contains(&"evaluation_status_mismatch".to_string()),
        "{codes:?}"
    );
}

#[test]
fn rule_3_mechanical_requirements_may_be_downgraded_never_upgraded() {
    let f = fixture("assess-rule-3", true);
    // Downgrade met -> indeterminate with no basis, reason or evidence: three codes.
    let bare = |v: &mut Value| {
        v["requirements"][0]["assessed_status"] = json!("indeterminate");
        v["indeterminate"] = json!(["A-note", "R-file"]);
    };
    assert_eq!(
        f.codes_after("bare", bare),
        [
            "downgrade_basis_not_assessor",
            "downgrade_evidence_missing",
            "downgrade_reason_missing"
        ]
    );
    // The same downgrade with basis assessor, a reason and evidence is accepted.
    let mut doc = f.doc.clone();
    bare(&mut doc);
    doc["requirements"][0]["basis"] = json!("assessor");
    doc["requirements"][0]["reason"] = json!("the bytes were not the reviewed ones");
    doc["requirements"][0]["evidence"] = json!(["notes/b.md"]);
    assert_eq!(f.run("down1", &doc).code, 0);
    // Downgrade to unmet needs the verdict blocked (a gap-free blocked is derived) and the sets.
    doc["requirements"][0]["assessed_status"] = json!("unmet");
    doc["missing"] = json!(["R-file"]);
    doc["indeterminate"] = json!(["A-note"]);
    doc["verdict"] = json!("blocked");
    assert_eq!(f.run("down2", &doc).code, 0);
    // Equal status is accepted with either basis.
    let mut doc = f.doc.clone();
    doc["requirements"][0]["basis"] = json!("assessor");
    assert_eq!(f.run("equal", &doc).code, 0);
    // Upgrade: the evaluation has R-json unmet; assessing it met is rejected.
    let g = fixture("assess-rule-3-upgrade", false);
    assert_eq!(g.doc["verdict"], "blocked");
    let codes = g.codes_after("upgrade", |v| {
        v["requirements"][2]["assessed_status"] = json!("met");
        v["requirements"][2]["basis"] = json!("assessor");
        v["requirements"][2]["reason"] = json!("I looked");
        v["requirements"][2]["evidence"] = json!(["notes/c.md"]);
        v["missing"] = json!([]);
        v["verdict"] = json!("uncertain");
    });
    assert_eq!(codes, ["mechanical_upgrade"]);
}

#[test]
fn rule_4_attributed_requirements_need_a_reason_and_evidence() {
    let f = fixture("assess-rule-4", true);
    assert_eq!(
        f.codes_after("reason", |v| v["requirements"][7]["reason"] = json!("")),
        ["attributed_reason_missing"]
    );
    assert_eq!(
        f.codes_after("evidence", |v| v["requirements"][7]["evidence"] = json!([])),
        ["attributed_evidence_missing"]
    );
    // The assessor may set an attributed requirement met, with reason and evidence, and then `ready`.
    let mut doc = f.doc.clone();
    doc["requirements"][7]["assessed_status"] = json!("met");
    doc["requirements"][7]["basis"] = json!("assessor");
    doc["indeterminate"] = json!([]);
    doc["verdict"] = json!("ready");
    assert_eq!(f.run("ready", &doc).code, 0);
}

/// T2F-001: the tool never derives an attributed status beyond `indeterminate`, so an attributed
/// `met` labelled `basis: evaluation` is rejected; the same document with `basis: assessor` passes.
#[test]
fn an_attributed_status_not_derived_from_the_tool_cannot_carry_basis_evaluation() {
    let f = fixture("assess-basis", true);
    let profile = read_json(&f.case.profile_path());
    let attributed: Vec<usize> = profile["requirements"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .filter(|(_, r)| r["class"] == "attributed")
        .map(|(i, _)| i)
        .collect();
    assert!(!attributed.is_empty(), "{attributed:?}");
    // Fable's probe: a forged `ready` over attributed requirements set to met, basis evaluation.
    let forged = |basis: &str| {
        let mut doc = f.doc.clone();
        for &i in &attributed {
            doc["requirements"][i]["assessed_status"] = json!("met");
            doc["requirements"][i]["basis"] = json!(basis);
        }
        doc["indeterminate"] = json!([]);
        doc["verdict"] = json!("ready");
        doc
    };
    let run = f.run("basis-forged", &forged("evaluation"));
    assert_eq!(run.code, 1, "{}", run.stdout);
    assert_eq!(
        run.codes(),
        vec!["basis_inconsistent"; attributed.len()],
        "{}",
        run.stdout
    );
    // Control: the correct basis passes.
    let run = f.run("basis-control", &forged("assessor"));
    assert_eq!(run.code, 0, "{}", run.stdout);
    // An attributed requirement left indeterminate keeps basis evaluation without error.
    assert_eq!(f.run("basis-honest", &f.doc).code, 0);
}

#[test]
fn rule_5_verdict_consistency_and_conservative_verdicts_need_a_profile_gap() {
    let f = fixture("assess-rule-5", true);
    assert_eq!(
        f.codes_after("less", |v| v["verdict"] = json!("ready")),
        ["verdict_inconsistent"]
    );
    assert_eq!(
        f.codes_after("more", |v| v["verdict"] = json!("blocked")),
        ["verdict_inconsistent"]
    );
    let mut doc = f.doc.clone();
    doc["verdict"] = json!("blocked");
    doc["profile_gaps"] =
        json!([{"statement": "the profile lacks a requirement for X", "evidence": ["notes/g.md"]}]);
    assert_eq!(
        f.run("gap", &doc).code,
        0,
        "a named profile gap permits a more conservative verdict"
    );
    // `ready` with an unmet requirement is rejected.
    let g = fixture("assess-rule-5-unmet", false);
    assert_eq!(
        g.codes_after("unmet-ready", |v| v["verdict"] = json!("ready")),
        ["verdict_inconsistent"]
    );
}

#[test]
fn rules_6_and_7_sets_role_and_freshness_basis() {
    let f = fixture("assess-rules-6-7", true);
    assert_eq!(
        f.codes_after("missing", |v| v["missing"] = json!(["R-file"])),
        ["missing_set_mismatch"]
    );
    assert_eq!(
        f.codes_after("indet", |v| v["indeterminate"] = json!([])),
        ["indeterminate_set_mismatch"]
    );
    assert_eq!(
        f.codes_after("role", |v| v["assessor"]["role"] = json!("nobody")),
        ["assessor_role_not_allowed"]
    );
    assert_eq!(
        f.codes_after("auth", |v| v["assessor"]["authority_ref"] = json!("  ")),
        ["authority_ref_empty"]
    );
    for field in [
        "battery_file_sha256",
        "battery_sha256",
        "observation_sha256",
    ] {
        let codes = f.codes_after(field, |v| {
            v["freshness_basis"]["observation"][field] = json!("0".repeat(64))
        });
        assert_eq!(codes, ["freshness_basis_mismatch"], "{field}");
    }
    for field in ["started_at", "finished_at"] {
        let codes = f.codes_after(field, |v| {
            v["freshness_basis"]["observation"][field] = json!("2026-01-01T00:00:00Z")
        });
        assert_eq!(codes, ["freshness_basis_mismatch"], "{field}");
    }
}

#[test]
fn closed_schema_and_added_shape_rules() {
    let f = fixture("assess-shape", true);
    for (tag, mutate) in [
        (
            "accepted",
            Box::new(|v: &mut Value| v["accepted"] = json!(true)) as Box<dyn Fn(&mut Value)>,
        ),
        (
            "decision-outcome",
            Box::new(|v| v["decision"]["outcome"] = json!("accept")),
        ),
        (
            "transition",
            Box::new(|v| v["transition"] = json!("integrate")),
        ),
    ] {
        assert_eq!(f.codes_after(tag, mutate), ["schema"], "{tag}");
    }
    assert_eq!(
        f.codes_after("ts", |v| v["issued_at"] = json!("yesterday")),
        ["invalid_timestamp"]
    );
    assert_eq!(
        f.codes_after("name", |v| v["assessor"]["name"] = json!("")),
        ["empty_name"]
    );
    assert_eq!(
        f.codes_after("id", |v| v["assessment_id"] = json!("")),
        ["empty_name"]
    );
    assert_eq!(
        f.codes_after("path", |v| v["evaluation"]["path"] = json!("rel.json")),
        ["invalid_absolute_path"]
    );
    assert_eq!(
        f.codes_after("sha", |v| v["evidence"] =
            json!([{"path": "/abs/x", "sha256": "zz"}])),
        ["invalid_sha256"]
    );
    assert_eq!(
        f.codes_after("claim", |v| v["freshness_basis"]["claim_reliance"] =
            json!([{"claim_id": "", "revision": "r1"}])),
        ["claim_reliance_invalid"]
    );
    let mut doc = f.doc.clone();
    doc["issued_at"] = json!("2026-10-21T12:01:00.250Z");
    assert_eq!(
        f.run("frac", &doc).code,
        0,
        "fractional seconds are accepted"
    );
    // Unparseable inputs are usage errors.
    let bad = f.case.dir.join("bad.json");
    write(&bad, "{");
    let run = cli(&[
        "check",
        "--kind",
        "assessment",
        &s(&bad),
        "--evaluation",
        &s(&f.evaluation),
        "--profile",
        &s(&f.case.profile_path()),
    ]);
    assert_eq!(run.code, 2);
    let run = cli(&[
        "check",
        "--kind",
        "assessment",
        &s(&f.case.profile_path()),
        "--evaluation",
        &s(&bad),
        "--profile",
        &s(&f.case.profile_path()),
    ]);
    assert_eq!(run.code, 2);
}

/// AC4, fixture form: one battery result under two fixture profiles. A `purpose: fixture`
/// `ready` assessment is accepted under profile A and refused with `verdict_inconsistent` under B.
#[test]
fn same_evidence_different_verdicts_under_two_profiles() {
    let mut case = met_case("assess-ac4");
    let base = case.profile.clone();
    let reqs = base["requirements"].as_array().unwrap();
    let mut profile_a = base.clone();
    profile_a["profile_id"] = json!("fixture.a");
    profile_a["requirements"] = json!([reqs[0], reqs[1]]);
    let mut profile_b = profile_a.clone();
    profile_b["profile_id"] = json!("fixture.b");
    let wrong = json!({"id": "R-extra", "class": "mechanical", "statement": "a.txt has other bytes",
        "predicates": [{"kind": "file_sha256", "file": {"root": "cand", "path": "src/a.txt"}, "expected": "f".repeat(64)}]});
    profile_b["requirements"]
        .as_array_mut()
        .unwrap()
        .push(wrong);
    let mut paths = Vec::new();
    for (tag, profile) in [("a", &profile_a), ("b", &profile_b)] {
        let pp = case.dir.join(format!("profile-{tag}.json"));
        let sha = write_json(&pp, profile);
        let mut subject = case.subject.clone();
        subject["profile"] = json!({"id": profile["profile_id"], "version": 1, "sha256": sha});
        let sp = case.dir.join(format!("subject-{tag}.json"));
        write_json(&sp, &subject);
        paths.push((pp, sp));
    }
    case.profile = base;
    let plan = cli(&[
        "plan",
        "--profile",
        &s(&paths[0].0),
        "--subject",
        &s(&paths[0].1),
        "--profile",
        &s(&paths[1].0),
        "--subject",
        &s(&paths[1].1),
    ]);
    assert_eq!(plan.code, 0, "{}", plan.stderr);
    let plan_path = case.dir.join("two.plan.json");
    write(&plan_path, &plan.stdout);
    let observed = observe_plan(&case, "two", &plan_path);
    let mut evaluations = Vec::new();
    for (tag, (pp, sp)) in ["a", "b"].iter().zip(&paths) {
        let out = case.dir.join(format!("{tag}.evaluation.json"));
        let run = cli(&[
            "evaluate",
            "--profile",
            &s(pp),
            "--subject",
            &s(sp),
            "--run-plan",
            &s(&observed.plan),
            "--battery",
            &s(&observed.result),
            "--observation",
            &s(&observed.observation),
            "--evaluated-at",
            EVALUATED_AT,
            "--out",
            &s(&out),
        ]);
        assert_eq!(run.code, 0, "{tag}: {}", run.stderr);
        evaluations.push(out);
    }
    let (ea, eb) = (read_json(&evaluations[0]), read_json(&evaluations[1]));
    for evaluation in [&ea, &eb] {
        assert!(!has_key_deep(evaluation, "verdict"), "AC5 (a)");
    }
    assert_eq!(
        ea["battery"], eb["battery"],
        "one battery result under both profiles"
    );
    assert_eq!(
        (ea["counts"]["unmet"].clone(), eb["counts"]["unmet"].clone()),
        (json!(0), json!(1))
    );
    // The `ready` assessment is honest under A.
    let mut ready_a = honest(&paths[0].0, &evaluations[0]);
    ready_a["verdict"] = json!("ready");
    ready_a["requirements"][0]["reason"] = json!("");
    assert_eq!(ready_a["indeterminate"], json!([]));
    let run = assess(&case.dir, "a", &paths[0].0, &evaluations[0], &ready_a);
    assert_eq!(run.code, 0, "{}", run.stdout);
    assert_eq!(run.out_json()["purpose"], "fixture");
    // The same claim under B: the unmet requirement is assessed as it is, and `ready` is refused.
    let mut ready_b = honest(&paths[1].0, &evaluations[1]);
    assert_eq!(ready_b["verdict"], "blocked");
    ready_b["verdict"] = json!("ready");
    let run = assess(&case.dir, "b", &paths[1].0, &evaluations[1], &ready_b);
    assert_eq!(run.code, 1);
    assert_eq!(run.codes(), ["verdict_inconsistent"]);
    // And the honest verdict under B is accepted.
    let honest_b = honest(&paths[1].0, &evaluations[1]);
    assert_eq!(
        assess(&case.dir, "b2", &paths[1].0, &evaluations[1], &honest_b).code,
        0
    );
}

/// Errata E9 / S2-T1-001. An evaluation whose mechanical statuses are all `met` while its
/// predicates stay `unmet` fails `check --kind evaluation`; an assessment over it is refused too,
/// whatever verdict it claims, instead of being compared with statuses nothing validated.
#[test]
fn an_assessment_over_a_structurally_invalid_evaluation_is_refused_with_evaluation_invalid() {
    let mut case = base_case("assess-forged-evaluation");
    // Mechanical requirements only, so that `ready` is the honest verdict over the forgery.
    case.profile["requirements"].as_array_mut().unwrap().pop();
    case.write_documents();
    let observed = observe(&case, "run");
    let genuine = evaluation_file(&case, &observed, "genuine.json");
    let genuine_json = read_json(&genuine);
    assert!(!has_key_deep(&genuine_json, "verdict"), "AC5 (a)");
    assert_eq!(
        (
            genuine_json["counts"]["met"].clone(),
            genuine_json["counts"]["unmet"].clone()
        ),
        (json!(6), json!(1)),
        "the base case has exactly one unmet requirement, R-json"
    );

    // The forgery: every requirement status `met` and the counts recounted, predicates untouched.
    let mut forged_json = genuine_json.clone();
    for r in forged_json["requirements"].as_array_mut().unwrap() {
        r["status"] = json!("met");
    }
    forged_json["counts"] = json!({"met": 7, "unmet": 0, "indeterminate": 0});
    let unmet_predicates = forged_json["requirements"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|r| r["predicates"].as_array().unwrap())
        .filter(|p| p["status"] == "unmet")
        .count();
    assert_eq!(unmet_predicates, 1, "the predicates stay unmet");
    let forged = case.dir.join("forged.json");
    write_json(&forged, &forged_json);

    // The structural check alone rejects it.
    let run = cli(&["check", "--kind", "evaluation", &s(&forged)]);
    assert_eq!(run.code, 1, "{}", run.stdout);
    assert_eq!(run.codes(), ["status_inconsistent"]);

    // A `ready` assessment over it, internally consistent and digest-bound to the forgery.
    let ready = honest(&case.profile_path(), &forged);
    assert_eq!(ready["verdict"], "ready");
    let run = assess(&case.dir, "ready", &case.profile_path(), &forged, &ready);
    assert_eq!(run.code, 1, "{}", run.stdout);
    assert_eq!(run.codes(), ["evaluation_invalid", "status_inconsistent"]);
    assert_eq!(run.out_json()["valid"], false);
    // The code is the same for any verdict: the evaluation, not the assessment, is at fault.
    let mut blocked = ready.clone();
    blocked["verdict"] = json!("blocked");
    let run = assess(
        &case.dir,
        "blocked",
        &case.profile_path(),
        &forged,
        &blocked,
    );
    assert_eq!(run.code, 1, "{}", run.stdout);
    assert!(run.codes().contains(&"evaluation_invalid".to_string()));

    // Control: the same profile and the genuine evaluation. The honest assessment is accepted,
    // and the `ready` claim is refused for its own reason, not `evaluation_invalid`.
    let honest_doc = honest(&case.profile_path(), &genuine);
    assert_eq!(honest_doc["verdict"], "blocked");
    let run = assess(
        &case.dir,
        "honest",
        &case.profile_path(),
        &genuine,
        &honest_doc,
    );
    assert_eq!(run.code, 0, "{}", run.stdout);
    let mut claim = honest_doc.clone();
    claim["verdict"] = json!("ready");
    let run = assess(&case.dir, "claim", &case.profile_path(), &genuine, &claim);
    assert_eq!(run.code, 1);
    assert_eq!(run.codes(), ["verdict_inconsistent"]);
}

#[test]
fn an_evaluation_that_relabels_an_attributed_requirement_mechanical_is_rejected() {
    // EXT-002: the evaluation claims `class: mechanical, status: met` for a requirement the
    // profile declares attributed, and the assessment keeps `basis: evaluation` over `met`.
    let f = fixture("assess-class-forgery", true);
    let profile = read_json(&f.case.profile_path());
    let attributed: Vec<String> = profile["requirements"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["class"] == "attributed")
        .map(|r| r["id"].as_str().unwrap().to_string())
        .collect();
    assert!(!attributed.is_empty());

    let mut forged = read_json(&f.evaluation);
    let predicate = forged["requirements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["class"] == "mechanical")
        .unwrap()["predicates"][0]
        .clone();
    for r in forged["requirements"].as_array_mut().unwrap() {
        if attributed.contains(&r["id"].as_str().unwrap().to_string()) {
            r["class"] = json!("mechanical");
            r["status"] = json!("met");
            r.as_object_mut().unwrap().remove("reason");
            r["predicates"] = json!([predicate.clone()]);
        }
    }
    let met = forged["requirements"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["status"] == "met")
        .count();
    forged["counts"] = json!({"met": met, "unmet": 0, "indeterminate": 0});
    let forged_path = f.case.dir.join("forged-class.evaluation.json");
    write_json(&forged_path, &forged);
    // Without the profile the forged evaluation is structurally consistent: the class check
    // needs the profile, so it runs in the assessment check and with --rederive.
    assert_eq!(
        cli(&["check", "--kind", "evaluation", &s(&forged_path)]).code,
        0
    );

    let mut doc = f.doc.clone();
    doc["evaluation"] = json!({"path": s(&forged_path), "sha256": file_sha(&forged_path)});
    for entry in doc["requirements"].as_array_mut().unwrap() {
        if attributed.contains(&entry["id"].as_str().unwrap().to_string()) {
            entry["evaluation_status"] = json!("met");
            entry["assessed_status"] = json!("met");
            entry["basis"] = json!("evaluation");
        }
    }
    doc["indeterminate"] = json!([]);
    doc["verdict"] = json!("ready");
    let run = assess(
        &f.case.dir,
        "class-forgery",
        &f.case.profile_path(),
        &forged_path,
        &doc,
    );
    assert_eq!(run.code, 1, "{}", run.stdout);
    assert_eq!(
        run.codes(),
        vec!["evaluation_class_mismatch"; attributed.len()],
        "{}",
        run.stdout
    );

    // The structural evaluation check rejects it too once the profile is supplied.
    let run = cli(&[
        "check",
        "--kind",
        "evaluation",
        &s(&forged_path),
        "--rederive",
        "--profile",
        &s(&f.case.profile_path()),
        "--subject",
        &s(&f.case.subject_path()),
    ]);
    assert_eq!(run.code, 1, "{}", run.stdout);
    assert!(
        run.codes().iter().any(|c| c == "evaluation_class_mismatch"),
        "{}",
        run.stdout
    );

    // Control: the genuine evaluation and honest assessment pass.
    assert_eq!(f.run("class-control", &f.doc).code, 0);
}

#[test]
fn the_cited_evaluation_path_must_resolve_to_the_checked_bytes() {
    // EXT-004: `evaluation.path` is the pointer a reader follows.
    let f = fixture("assess-path", true);
    let with_path = |path: &str| {
        let mut doc = f.doc.clone();
        doc["evaluation"]["path"] = json!(path);
        doc
    };
    // A path that does not exist.
    let missing = s(&f.case.dir.join("does-not-exist.json"));
    let run = f.run("path-missing", &with_path(&missing));
    assert_eq!(run.code, 1, "{}", run.stdout);
    assert_eq!(run.codes(), ["evaluation_path_unresolved"]);
    // A path that resolves to a different file.
    let other = f.case.dir.join("other.json");
    write(&other, "{\"not\": \"the evaluation\"}\n");
    let run = f.run("path-other", &with_path(&s(&other)));
    assert_eq!(run.code, 1, "{}", run.stdout);
    assert_eq!(run.codes(), ["evaluation_path_mismatch"]);
    // A byte-identical copy at the cited path is accepted: the digest still binds the content.
    let copy = f.case.dir.join("copy.json");
    std::fs::copy(&f.evaluation, &copy).unwrap();
    assert_eq!(f.run("path-copy", &with_path(&s(&copy))).code, 0);
    // Control: the honest assessment cites the supplied file.
    assert_eq!(f.run("path-control", &f.doc).code, 0);
}
