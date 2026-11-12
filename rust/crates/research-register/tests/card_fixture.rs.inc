//! Entirely synthetic immutable Git fixtures. No live authorization or scientific result.
#[path = "support/mod.rs"]
mod support;
use research_register::use_bundle::*;
use research_register::{GitObjects, Outcome, PinnedSnapshot, validate};
use serde_json::{Value, json};
use std::path::PathBuf;
use support::{Fx, write};

pub(crate) struct Fixture {
    pub(crate) fx: Fx,
    pub(crate) repo: PathBuf,
    pub(crate) source: String,
    pub(crate) register_path: String,
    pub(crate) docs: Vec<(String, Value)>,
    pub(crate) request: String,
    pub(crate) app: String,
    pub(crate) original_ids: std::collections::BTreeMap<String, String>,
}
fn bytes(v: &Value) -> Vec<u8> {
    let mut b = serde_json::to_vec(v).unwrap();
    b.push(b'\n');
    b
}
impl Fixture {
    pub(crate) fn new() -> Self {
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
            register_path: "reg".into(),
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
    pub(crate) fn add(&mut self, name: &str, record: Value) -> String {
        let v = json!({"schema":VERSION,"record":record});
        let id = revision(&bytes(&v));
        self.original_ids.insert(format!("{name}.json"), id.clone());
        self.docs.push((format!("{name}.json"), v));
        id
    }
    pub(crate) fn pin(&self, c: &str, p: &str) -> BlobPin {
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
    pub(crate) fn manifest(&self) -> Value {
        json!({"schema":VERSION,"source":{"commit":self.source,"register_path":self.register_path,"register_tree":self.fx.git(&self.repo,&["rev-parse",&format!("{}:{}",self.source,self.register_path)])},"inputs":[],"request":self.request,"applicability":[self.app]})
    }
    pub(crate) fn publish(&self, mut m: Value) -> BlobPin {
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
    pub(crate) fn publish_raw(&self, path: &str, b: &[u8]) -> BlobPin {
        std::fs::create_dir_all(self.repo.join(path).parent().unwrap()).unwrap();
        std::fs::write(self.repo.join(path), b).unwrap();
        self.fx.git(&self.repo, &["add", "."]);
        self.fx.git(
            &self.repo,
            &["commit", "-q", "--allow-empty", "-m", "publish"],
        );
        self.pin(&self.fx.git(&self.repo, &["rev-parse", "HEAD"]), path)
    }
    pub(crate) fn run(&self, pin: BlobPin) -> Result<ValidUseBundle, UseError> {
        let s = PinnedSnapshot::build(
            GitObjects::open(&self.repo).unwrap(),
            &self.source,
            &self.register_path,
        )
        .unwrap();
        let Outcome::Valid(v) = validate(&s).unwrap() else {
            panic!("fixture register invalid: {:?}", validate(&s).unwrap())
        };
        validate_use_bundle(&s, &v, pin)
    }
}
