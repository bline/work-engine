//! Entirely synthetic immutable Git fixtures. No live authorization or scientific result.
mod support;
use research_register::use_bundle::*;
use research_register::{GitObjects, Outcome, PinnedSnapshot, validate};
use serde_json::{Value, json};
use std::path::PathBuf;
use support::{Fx, write};

struct Fixture {
    fx: Fx,
    repo: PathBuf,
    source: String,
    docs: Vec<(String, Value)>,
    request: String,
    app: String,
    original_ids: std::collections::BTreeMap<String, String>,
}
fn bytes(v: &Value) -> Vec<u8> {
    let mut b = serde_json::to_vec(v).unwrap();
    b.push(b'\n');
    b
}
impl Fixture {
    fn new() -> Self {
        let fx = Fx::new("use-bundle");
        let repo = fx.root.join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        fx.git(&repo, &["init", "-q", "-b", "main"]);
        write(&repo.join("policy.md"), "# policy\nsynthetic\n");
        fx.git(&repo, &["add", "."]);
        fx.git(&repo, &["commit", "-qm", "target"]);
        let target_commit = fx.git(&repo, &["rev-parse", "HEAD"]);
        let blob = fx.git(&repo, &["rev-parse", "HEAD:policy.md"]);
        let p = json!({"path":"policy.md","commit":target_commit,"blob":blob});
        for (path, v) in [
            (
                "questions.json",
                json!([{"id":"Q1","text":"question","area":"test"}]),
            ),
            (
                "decisions.json",
                json!([{"decision_id":"d-one","document":"policy.md","anchor":"policy","revision":format!("git:{target_commit}"),"owner":"synthetic owner","findings":["alpha#one"]}]),
            ),
            (
                "studies/alpha.json",
                json!({"id":"alpha","title":"synthetic","kind":"probe","questions":["Q1"],"lifecycle":[{"date":"2026-01-01","state":"concluded","basis":p}],"stop_reason_class":null,"findings":[{"id":"alpha#one","claim":"synthetic claim","value":1,"n":1,"basis":"measured","scope":"test","caveats":[],"sources":[p]}],"sources":[],"aliases":[]}),
            ),
        ] {
            write(&repo.join(format!("reg/{path}")), &v.to_string());
        }
        for p in [
            "events.jsonl",
            "judgments.jsonl",
            "applicability.jsonl",
            "materialized.jsonl",
        ] {
            write(&repo.join(format!("reg/{p}")), "");
        }
        fx.git(&repo, &["add", "."]);
        fx.git(&repo, &["commit", "-qm", "source register"]);
        let source = fx.git(&repo, &["rev-parse", "HEAD"]);
        let mut f = Self {
            fx,
            repo,
            source,
            docs: Vec::new(),
            request: String::new(),
            app: String::new(),
            original_ids: std::collections::BTreeMap::new(),
        };
        let evidence = f.add(
            "evidence",
            json!({"kind":"evidence","source":{"kind":"git","pin":f.pin(&f.source,"policy.md")}}),
        );
        let assertion=f.add("assertion",json!({"kind":"evidence","source":{"kind":"assertion","locator":"synthetic://authority","asserted_sha256":null,"limitation":"publisher assertion, unauthenticated"}}));
        let subject=f.add("subject",json!({"kind":"subject","finding":"alpha#one","study":f.pin(&f.source,"reg/studies/alpha.json"),"evidence":[evidence]}));
        let qualification=f.add("qualification",json!({"kind":"qualification","assessor":"synthetic assessor","purpose":"test","disposition":"unestablished","reviewer":"synthetic reviewer","exposure_boundary":"asserted only","capability_evidence":[assertion],"authorization_evidence":[assertion],"evidence":[evidence]}));
        let mut facets = serde_json::Map::new();
        for name in [
            "provenance",
            "method_reliability",
            "consistency",
            "reproducibility",
            "context_transfer",
            "inferential_strength",
            "completeness",
            "temporal_relevance",
        ] {
            facets.insert(
                name.into(),
                json!({"state":"unknown","reason":"synthetic uncertainty","evidence":[evidence]}),
            );
        }
        facets["completeness"] =
            json!({"state":"not_assessed","reason":"not examined","evidence":[]});
        f.add("assessment",json!({"kind":"assessment","subject":subject,"assessor":"synthetic author","as_of_context":"fixture context","scope":"test only","limitations":["no live result"],"evidence":[evidence],"facets":facets,"predecessors":[],"corrections":[],"competes_with":[],"qualification":[qualification]}));
        f.request=f.add("request",json!({"kind":"request","request_id":"test.audit","decision":"d-one","decision_revision":format!("git:{target_commit}"),"target":f.pin(&f.source,"policy.md"),"selector":"heading policy","purpose":"synthetic audit","alternatives":["defer"],"conditions":["read only"],"operating_context":"model and host unknown","owner":"caller-built owner string","acceptance":["preserve unknowns"]}));
        f.app=f.add("applicability",json!({"kind":"applicability","subject":subject,"request":f.request,"decision":"d-one","decision_revision":format!("git:{target_commit}"),"relation":"not_established","bounds":"synthetic audit only","owner":"caller-built owner string","authorization_evidence":[assertion]}));
        f
    }
    fn add(&mut self, name: &str, record: Value) -> String {
        let v = json!({"schema":VERSION,"record":record});
        let id = revision(&bytes(&v));
        self.original_ids.insert(format!("{name}.json"), id.clone());
        self.docs.push((format!("{name}.json"), v));
        id
    }
    fn pin(&self, c: &str, p: &str) -> BlobPin {
        let exact = self
            .fx
            .cmd(&self.repo, &["show", &format!("{c}:{p}")], &[])
            .output()
            .unwrap()
            .stdout;
        BlobPin {
            commit: c.into(),
            path: p.into(),
            blob: self.fx.git(&self.repo, &["rev-parse", &format!("{c}:{p}")]),
            sha256: revision(&exact),
        }
    }
    fn manifest(&self) -> Value {
        json!({"schema":VERSION,"source":{"commit":self.source,"register_path":"reg","register_tree":self.fx.git(&self.repo,&["rev-parse",&format!("{}:reg",self.source)])},"inputs":[],"request":self.request,"applicability":[self.app]})
    }
    fn publish(&self, mut m: Value) -> BlobPin {
        // Rebind dependent digests after one deliberate fixture mutation, so a negative case
        // reaches its own invariant rather than failing accidentally on an obsolete link.
        fn replace(v: &mut Value, map: &std::collections::BTreeMap<String, String>) {
            match v {
                Value::String(s) => {
                    if let Some(new) = map.get(s) {
                        *s = new.clone();
                    }
                }
                Value::Array(a) => {
                    for v in a {
                        replace(v, map);
                    }
                }
                Value::Object(o) => {
                    for v in o.values_mut() {
                        replace(v, map);
                    }
                }
                _ => {}
            }
        }
        let mut docs = self.docs.clone();
        let mut prior = self.original_ids.clone();
        for _ in 0..docs.len() + 1 {
            let current: std::collections::BTreeMap<_, _> = docs
                .iter()
                .map(|(p, v)| (p.clone(), revision(&bytes(v))))
                .collect();
            let changes = prior
                .iter()
                .filter_map(|(p, old)| {
                    current
                        .get(p)
                        .filter(|new| *new != old)
                        .map(|new| (old.clone(), new.clone()))
                })
                .collect();
            for (_, v) in &mut docs {
                replace(v, &changes);
            }
            replace(&mut m, &changes);
            prior = current;
        }
        for (p, v) in &docs {
            std::fs::create_dir_all(self.repo.join("bundle")).unwrap();
            std::fs::write(self.repo.join(format!("bundle/{p}")), bytes(v)).unwrap();
        }
        self.fx.git(&self.repo, &["add", "."]);
        self.fx.git(
            &self.repo,
            &["commit", "-q", "--allow-empty", "-m", "inputs"],
        );
        let c = self.fx.git(&self.repo, &["rev-parse", "HEAD"]);
        m["inputs"] = json!(
            docs.iter()
                .map(|(p, _)| {
                    let pin = self.pin(&c, &format!("bundle/{p}"));
                    json!({"path":p,"blob":pin.blob,"sha256":pin.sha256})
                })
                .collect::<Vec<_>>()
        );
        self.publish_raw("bundle/manifest.json", &bytes(&m))
    }
    fn publish_raw(&self, path: &str, b: &[u8]) -> BlobPin {
        std::fs::create_dir_all(self.repo.join(path).parent().unwrap()).unwrap();
        std::fs::write(self.repo.join(path), b).unwrap();
        self.fx.git(&self.repo, &["add", "."]);
        self.fx.git(
            &self.repo,
            &["commit", "-q", "--allow-empty", "-m", "publish"],
        );
        self.pin(&self.fx.git(&self.repo, &["rev-parse", "HEAD"]), path)
    }
    fn run(&self, pin: BlobPin) -> Result<ValidUseBundle, UseError> {
        let s = PinnedSnapshot::build(GitObjects::open(&self.repo).unwrap(), &self.source, "reg")
            .unwrap();
        let Outcome::Valid(v) = validate(&s).unwrap() else {
            panic!("fixture register invalid: {:?}", validate(&s).unwrap())
        };
        validate_use_bundle(&s, &v, pin)
    }
}

#[test]
fn upstream_only_retains_exact_bytes_and_uncertainty_without_authority() {
    let f = Fixture::new();
    let pin = f.publish(f.manifest());
    let b = f.run(pin.clone()).unwrap();
    assert_eq!(revision(b.manifest_bytes()), pin.sha256);
    assert_eq!(b.records().len(), 7);
    assert_eq!(b.source_files().len(), 7);
    for (path, pin) in b.source_files() {
        let exact =
            f.fx.cmd(&f.repo, &["show", &format!("{}:{path}", f.source)], &[])
                .output()
                .unwrap()
                .stdout;
        assert_eq!(b.retained_git_bytes()[&pin.sha256], exact);
    }
    let assessment = b
        .records()
        .values()
        .find_map(|r| {
            if let Record::Assessment { facets, .. } = r.record() {
                Some(facets)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(assessment.provenance.state, FacetState::Unknown);
    assert_eq!(assessment.completeness.state, FacetState::NotAssessed);
    write(&f.repo.join("policy.md"), "uncommitted changed claim");
    std::fs::remove_dir_all(f.repo.join("bundle")).unwrap();
    assert!(f.run(pin).is_ok());
}

#[test]
fn changed_claim_under_same_id_cannot_reuse_subject() {
    let mut f = Fixture::new();
    let old = f.publish(f.manifest());
    assert!(f.run(old.clone()).is_ok());
    let p = f.repo.join("reg/studies/alpha.json");
    let mut study: Value = serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
    study["findings"][0]["claim"] = json!("changed");
    write(&p, &study.to_string());
    f.fx.git(&f.repo, &["add", "."]);
    f.fx.git(&f.repo, &["commit", "-qm", "changed claim"]);
    let old_source = f.source.clone();
    f.source = f.fx.git(&f.repo, &["rev-parse", "HEAD"]);
    let target = f.pin(&f.source, "policy.md");
    // Refresh every unrelated source-bound locator. Only the study blob/digest deliberately
    // remain stale; the finding ID itself is unchanged in the valid new register.
    for (name, doc) in &mut f.docs {
        match name.as_str() {
            "request.json" => doc["record"]["target"] = json!(target),
            "evidence.json" => doc["record"]["source"]["pin"] = json!(target),
            "subject.json" => doc["record"]["study"]["commit"] = json!(f.source),
            _ => {}
        }
    }
    assert!(matches!(
        f.run(f.publish(f.manifest())),
        Err(UseError::Invalid(detail)) if detail == "pin mismatch: reg/studies/alpha.json"
    ));
    // Repair just the stale study identity: the new bundle must now validate.
    let corrected = f.pin(&f.source, "reg/studies/alpha.json");
    f.docs
        .iter_mut()
        .find(|(p, _)| p == "subject.json")
        .unwrap()
        .1["record"]["study"] = json!(corrected);
    assert!(f.run(f.publish(f.manifest())).is_ok());
    f.source = old_source;
    assert!(f.run(old).is_ok());
}

#[test]
fn request_scope_digest_and_decision_revision_are_bound() {
    for changed_revision in [false, true] {
        let mut f = Fixture::new();
        let r = &mut f
            .docs
            .iter_mut()
            .find(|(p, _)| p == "request.json")
            .unwrap()
            .1["record"];
        r[if changed_revision {
            "decision_revision"
        } else {
            "conditions"
        }] = if changed_revision {
            json!("git:0000000000000000000000000000000000000000")
        } else {
            json!(["write policy"])
        };
        if !changed_revision {
            // Keep the old applicability/request digests deliberately: a scope edit is a new
            // request and cannot inherit an old exact-byte disposition.
            f.original_ids.clear();
        }
        assert!(f.run(f.publish(f.manifest())).is_err());
    }
}

#[test]
fn corrections_keep_old_bundle_and_conflict_representable() {
    let mut f = Fixture::new();
    let old = f.publish(f.manifest());
    let original = f
        .docs
        .iter()
        .find(|(p, _)| p == "assessment.json")
        .unwrap()
        .1
        .clone();
    let old_id = revision(&bytes(&original));
    let mut r = original["record"].clone();
    r["predecessors"] = json!([old_id]);
    r["competes_with"] = json!([old_id]);
    r["facets"]["provenance"]["state"] = json!("contested");
    f.add("assessment-correction", r);
    let new = f.publish(f.manifest());
    assert!(f.run(old).is_ok());
    assert_eq!(
        f.run(new)
            .unwrap()
            .records()
            .values()
            .filter(|r| r.record().kind() == "assessment")
            .count(),
        2
    );
}

#[test]
fn malformed_versions_and_duplicate_json_at_any_depth_fail() {
    let f = Fixture::new();
    let valid = f.publish(f.manifest());
    assert!(f.run(valid.clone()).is_ok());
    let original = std::fs::read(f.repo.join("bundle/manifest.json")).unwrap();
    let original_text = String::from_utf8(original.clone()).unwrap();

    let future = original_text.replacen(VERSION, "future", 1);
    let pin = f.publish_raw("bundle/future.json", future.as_bytes());
    assert!(matches!(f.run(pin),Err(UseError::Unsupported(schema)) if schema=="future"));

    for (name, from, to) in [
        (
            "duplicate-top",
            format!("\"schema\":\"{VERSION}\""),
            format!("\"schema\":\"{VERSION}\",\"schema\":\"{VERSION}\""),
        ),
        (
            "duplicate-nested",
            "\"register_path\":\"reg\"".into(),
            "\"register_path\":\"reg\",\"register_path\":\"reg\"".into(),
        ),
    ] {
        assert!(original_text.contains(&from));
        let raw = original_text.replacen(&from, &to, 1);
        let pin = f.publish_raw(&format!("bundle/{name}.json"), raw.as_bytes());
        assert!(matches!(f.run(pin),Err(UseError::Invalid(detail))
            if detail.starts_with("strict JSON:") && detail.contains("duplicate object key")));
    }
    // Break only the closing delimiter of a complete manifest.
    let mut malformed = original.clone();
    let end = malformed.iter().rposition(|b| *b == b'}').unwrap();
    malformed.remove(end);
    let pin = f.publish_raw("bundle/malformed.json", &malformed);
    assert!(matches!(f.run(pin),Err(UseError::Invalid(detail))
        if detail.starts_with("strict JSON:") && detail.contains("EOF")));

    // Corrupt UTF-8 inside the existing schema string; all other fields remain present.
    let mut non_utf8 = original;
    let offset = original_text.find(VERSION).unwrap();
    non_utf8[offset] = 0xff;
    let pin = f.publish_raw("bundle/non-utf8.json", &non_utf8);
    assert!(matches!(f.run(pin),Err(UseError::Invalid(detail))
        if detail.starts_with("strict JSON:") && detail.contains("invalid UTF-8")));
    assert!(f.run(valid).is_ok());
}

#[test]
fn wrong_source_tree_target_and_evidence_pins_fail() {
    for case in 0..5 {
        let mut f = Fixture::new();
        let mut m = f.manifest();
        match case {
            0 => m["source"]["commit"] = json!("0".repeat(40)),
            1 => m["source"]["register_tree"] = json!("0".repeat(40)),
            2 => {
                f.docs
                    .iter_mut()
                    .find(|(p, _)| p == "request.json")
                    .unwrap()
                    .1["record"]["target"]["blob"] = json!("0".repeat(40))
            }
            3 => {
                f.docs
                    .iter_mut()
                    .find(|(p, _)| p == "evidence.json")
                    .unwrap()
                    .1["record"]["source"]["pin"]["sha256"] = json!("0".repeat(64))
            }
            _ => {
                f.docs
                    .iter_mut()
                    .find(|(p, _)| p == "subject.json")
                    .unwrap()
                    .1["record"]["study"]["blob"] = json!("0".repeat(40))
            }
        };
        assert!(f.run(f.publish(m)).is_err(), "case {case}");
    }
}

#[test]
fn missing_record_wrong_kind_duplicate_identity_and_interrupted_publication_fail() {
    for case in 0..4 {
        let mut f = Fixture::new();
        match case {
            0 => {
                f.docs.retain(|(p, _)| p != "evidence.json");
            }
            1 => {
                let wrong = f.request.clone();
                f.docs
                    .iter_mut()
                    .find(|(p, _)| p == "assessment.json")
                    .unwrap()
                    .1["record"]["subject"] = json!(wrong);
            }
            2 => {
                let mut dup = f.docs[0].clone();
                dup.0 = "duplicate.json".into();
                f.docs.push(dup);
            }
            _ => {}
        }
        let p = f.publish(f.manifest());
        if case == 3 {
            let mut m: Value = serde_json::from_slice(
                &std::fs::read(f.repo.join("bundle/manifest.json")).unwrap(),
            )
            .unwrap();
            m["inputs"][0]["path"] = json!("not-published.json");
            let p = f.publish_raw("bundle/interrupted.json", &bytes(&m));
            assert!(matches!(f.run(p), Err(UseError::Missing(_))));
        } else {
            assert!(f.run(p).is_err());
        }
    }
}

#[test]
fn escaping_paths_are_rejected() {
    let f = Fixture::new();
    let valid = f.publish(f.manifest());
    assert!(f.run(valid).is_ok());
    let raw = std::fs::read(f.repo.join("bundle/manifest.json")).unwrap();
    for path in ["../policy.md", "/absolute", "a//b", "a\\b", "."] {
        let mut m: Value = serde_json::from_slice(&raw).unwrap();
        m["inputs"][0]["path"] = json!(path);
        let pin = f.publish_raw("bundle/escape.json", &bytes(&m));
        assert!(
            matches!(f.run(pin),Err(UseError::Invalid(detail)) if detail==format!("unsafe relative path: {path}"))
        );
    }
}

#[test]
fn honestly_pinned_symlink_and_tree_inputs_are_rejected() {
    for is_tree in [false, true] {
        let f = Fixture::new();
        let valid = f.publish(f.manifest());
        assert!(f.run(valid.clone()).is_ok());
        let mut manifest: Value =
            serde_json::from_slice(&std::fs::read(f.repo.join("bundle/manifest.json")).unwrap())
                .unwrap();
        let path = f.repo.join("bundle/evidence.json");
        let record = std::fs::read(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        if is_tree {
            std::fs::create_dir(&path).unwrap();
            std::fs::write(path.join("record.json"), &record).unwrap();
        } else {
            // The link object itself contains the complete valid record bytes. If an unsafe
            // reader treated a link as a blob, the entire bundle could otherwise validate.
            std::os::unix::fs::symlink(std::str::from_utf8(&record).unwrap(), &path).unwrap();
        }
        f.fx.git(&f.repo, &["add", "."]);
        f.fx.git(&f.repo, &["commit", "-qm", "non-file input"]);
        let c = f.fx.git(&f.repo, &["rev-parse", "HEAD"]);
        let blob = f.fx.git(
            &f.repo,
            &["rev-parse", &format!("{c}:bundle/evidence.json")],
        );
        let kind = if is_tree { "tree" } else { "blob" };
        let output =
            f.fx.cmd(&f.repo, &["cat-file", kind, &blob], &[])
                .output()
                .unwrap();
        assert!(output.status.success());
        let digest = revision(&output.stdout);
        if !is_tree {
            assert_eq!(output.stdout, record);
        }
        let input = manifest["inputs"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|v| v["path"] == "evidence.json")
            .unwrap();
        input["blob"] = json!(blob);
        input["sha256"] = json!(digest);
        let pin = f.publish_raw("bundle/non-file.json", &bytes(&manifest));
        assert!(
            matches!(f.run(pin),Err(UseError::Invalid(detail)) if detail=="not a regular file: bundle/evidence.json")
        );
        // Restore only the entry's file kind and original bytes as a positive control.
        if is_tree {
            std::fs::remove_dir_all(&path).unwrap();
        } else {
            std::fs::remove_file(&path).unwrap();
        }
        std::fs::write(&path, &record).unwrap();
        assert!(f.run(f.publish(f.manifest())).is_ok());
        assert!(f.run(valid).is_ok());
    }
}

#[test]
fn passive_backward_references_preserve_source_execution_and_cannot_be_upstream() {
    let f = Fixture::new();
    let m = f.publish(f.manifest());
    let b = f.run(m.clone()).unwrap();
    let artifacts = f.publish_raw(
        "downstream/synthetic-artifacts.json",
        b"{\"synthetic\":true}\n",
    );
    let assessment = b
        .records()
        .iter()
        .find(|(_, r)| r.record().kind() == "assessment")
        .unwrap()
        .0
        .clone();
    let target = b
        .records()
        .values()
        .find_map(|r| {
            if let Record::Request { target, .. } = r.record() {
                Some(target.clone())
            } else {
                None
            }
        })
        .unwrap();
    let prepared = json!({"schema":VERSION,"record":{"kind":"prepared_basis","bundle":m,"request":f.request,"assessments":[assessment],"applicability":[f.app],"card":artifacts,"delivery":artifacts,"recommendation":artifacts,"context_check":artifacts,"policy":target}});
    let basis = f.publish_raw("downstream/basis.json", &bytes(&prepared));
    let adoption = json!({"schema":VERSION,"record":{"kind":"adoption","basis":basis,"preparation_revision":"synthetic-preparation-1","execution":"synthetic-execution-1","admission":artifacts,"grant":artifacts}});
    let adopted = f.publish_raw("downstream/adoption.json", &bytes(&adoption));
    let feedback = json!({"schema":VERSION,"record":{"kind":"feedback","adoption":adopted,"execution":"synthetic-execution-1","verification":artifacts,"costs":"unknown","failures":[],"unknowns":["no live result"],"candidate_status":"synthetic only"}});
    let p1 = f.publish_raw("downstream/feedback1.json", &bytes(&feedback));
    let p2 = f.publish_raw("downstream/feedback2.json", &bytes(&feedback));
    let objects = GitObjects::open(&f.repo).unwrap();
    for p in [&p1, &p2] {
        let r = read_passive(&objects, &b, p).unwrap();
        assert!(
            matches!(r.record(),PassiveRecord::Feedback{execution,..} if execution=="synthetic-execution-1")
        );
        assert!(r.retained().contains_key(&basis.sha256));
    }
    let mut wrong = feedback.clone();
    wrong["record"]["execution"] = json!("different execution");
    let p = f.publish_raw("downstream/wrong.json", &bytes(&wrong));
    assert!(matches!(
        read_passive(&objects, &b, &p),
        Err(UseError::Invalid(_))
    ));
    // Upstream decoding refuses the passive kind even with correct byte pins.
    let mut upstream: Value = serde_json::from_slice(b.manifest_bytes()).unwrap();
    std::fs::write(f.repo.join("bundle/passive.json"), bytes(&prepared)).unwrap();
    f.fx.git(&f.repo, &["add", "."]);
    f.fx.git(&f.repo, &["commit", "-qm", "passive input"]);
    let c = f.fx.git(&f.repo, &["rev-parse", "HEAD"]);
    let p = f.pin(&c, "bundle/passive.json");
    upstream["inputs"]
        .as_array_mut()
        .unwrap()
        .push(json!({"path":"passive.json","blob":p.blob,"sha256":p.sha256}));
    let p = f.publish_raw("bundle/rejected-passive.json", &bytes(&upstream));
    assert!(matches!(f.run(p), Err(UseError::Invalid(_))));
    // Wrong backward kind fails rather than accepting an adoption in place of a basis.
    let mut bad = adoption;
    bad["record"]["basis"] = json!(adopted);
    let p = f.publish_raw("downstream/wrong-kind.json", &bytes(&bad));
    assert!(matches!(
        read_passive(&objects, &b, &p),
        Err(UseError::Invalid(_))
    ));
}

#[test]
fn missing_git_object_is_unreadable_and_missing_path_is_missing() {
    let f = Fixture::new();
    let mut p = f.publish(f.manifest());
    p.commit = "0".repeat(40);
    assert!(matches!(f.run(p), Err(UseError::Unreadable(_))));
    let mut p = f.publish(f.manifest());
    p.path = "bundle/absent.json".into();
    assert!(matches!(f.run(p), Err(UseError::Missing(_))));
}

#[test]
fn source_snapshot_cannot_substitute_for_the_register_proof() {
    let f = Fixture::new();
    let pin = f.publish(f.manifest());
    let original =
        PinnedSnapshot::build(GitObjects::open(&f.repo).unwrap(), &f.source, "reg").unwrap();
    let Outcome::Valid(proof) = validate(&original).unwrap() else {
        panic!("invalid fixture")
    };
    let different =
        PinnedSnapshot::build(GitObjects::open(&f.repo).unwrap(), &pin.commit, "reg").unwrap();
    assert!(matches!(
        validate_use_bundle(&different, &proof, pin),
        Err(UseError::Invalid(_))
    ));
}

#[test]
fn required_qualification_and_well_formed_ids_are_not_optional() {
    for case in 0..3 {
        let mut f = Fixture::new();
        match case {
            0 => {
                f.docs
                    .iter_mut()
                    .find(|(p, _)| p == "assessment.json")
                    .unwrap()
                    .1["record"]["qualification"] = json!([])
            }
            1 => {
                f.docs
                    .iter_mut()
                    .find(|(p, _)| p == "request.json")
                    .unwrap()
                    .1["record"]["request_id"] = json!("bad id/escape")
            }
            _ => {
                f.docs
                    .iter_mut()
                    .find(|(p, _)| p == "subject.json")
                    .unwrap()
                    .1["record"]["evidence"] = json!(["malformed-digest"])
            }
        }
        assert!(matches!(
            f.run(f.publish(f.manifest())),
            Err(UseError::Invalid(_))
        ));
    }
}
