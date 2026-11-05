//! Actual-process tests for `work-engine-claims-host reassess`.
//!
//! The claims root, grants and trusted startup follow `Fixture::new` in
//! `repository_host.rs`. CBM captures are hand-authored, schema-valid fixtures
//! (the adapter's `verify` admits them) whose documentation rows copy the real
//! `detect_changes` row shape observed from the pinned CBM; no CBM process runs.
//! Every judgment text here is fixture-authored, not a model assessment.
use claim_evidence::sha256_bytes;
use repository_evidence_cbm::{publish_artifact, sha256, verify};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

const REPO: &str = "repo-a";

struct Fixture {
    _temp: tempfile::TempDir,
    base: PathBuf,
    root: PathBuf,
    startup: PathBuf,
    sha: String,
    checkout: PathBuf,
    artifacts: PathBuf,
    binding: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let root = base.join("root");
        let source = base.join("source.json");
        let config = base.join("config.json");
        let startup = base.join("startup.json");
        let source_value = json!({"schema_version":3,"owner":"trusted-fixture-issuer","reference":"fixture:authority","revision":"1","freshness":"pinned","profile":"repository-claims-v1","repository_id":REPO,"actors":["producer"],"decision_scopes":["scope"],"grant_ids":["grant"]});
        fs::write(&source, serde_json::to_vec(&source_value).unwrap()).unwrap();
        let source_sha = sha256_bytes(&fs::read(&source).unwrap());
        let config_value = json!({"schema_version":3,"profile":"repository-claims-v1","root_id":"fixture-root","repository_id":REPO,"grants":[{"schema_version":1,"grant_id":"grant","actor":"producer","profile":"repository-fact-v1","permissions":["create_claim","publish_revision","record_reliance"],"decision_scope":"scope","authority_reference":{"owner":"trusted-fixture-issuer","reference":"fixture:authority","revision":"1","freshness":"pinned","integrity_sha256":source_sha,"status":"verified"}}]});
        fs::write(&config, serde_json::to_vec(&config_value).unwrap()).unwrap();
        let config_sha = sha256_bytes(&fs::read(&config).unwrap());
        let startup_value = json!({"schema_version":1,"root":root,"config":config,"source":source,"expected_config_sha256":config_sha,"expected_source_sha256":source_sha,"repository_id":REPO,"decision_scope":"scope","actor":"producer","grant_id":"grant","allowed_actions":["create_claim","publish_revision","record_reliance"]});
        fs::write(&startup, serde_json::to_vec(&startup_value).unwrap()).unwrap();
        let sha = sha256_bytes(&fs::read(&startup).unwrap());
        let checkout = base.join("checkout");
        let artifacts = base.join("artifacts");
        fs::create_dir_all(checkout.join("src")).unwrap();
        fs::create_dir_all(checkout.join("docs")).unwrap();
        fs::create_dir(&artifacts).unwrap();
        let binding = base.join("binding.json");
        fs::write(
            &binding,
            serde_json::to_vec(&json!({"repository_id":REPO,"repository":checkout,"project":"project-a","executable":"/nonexistent/cbm","executable_sha256":"a".repeat(64)})).unwrap(),
        )
        .unwrap();
        let this = Self {
            _temp: temp,
            base,
            root,
            startup,
            sha,
            checkout,
            artifacts,
            binding,
        };
        let out = this.host(&["initialize"], None).unwrap();
        assert_eq!(out["status"], "initialized");
        this
    }
    fn host(&self, action: &[&str], stdin: Option<&Value>) -> Result<Value, String> {
        self.host_raw(
            action[0],
            &[&self.startup.to_string_lossy(), &self.sha],
            &action[1..],
            stdin,
        )
    }
    fn host_raw(
        &self,
        action: &str,
        head: &[&str],
        tail: &[&str],
        stdin: Option<&Value>,
    ) -> Result<Value, String> {
        self.host_bytes(
            action,
            head,
            tail,
            stdin.map(|v| serde_json::to_vec(v).unwrap()),
        )
    }
    fn host_bytes(
        &self,
        action: &str,
        head: &[&str],
        tail: &[&str],
        stdin: Option<Vec<u8>>,
    ) -> Result<Value, String> {
        let mut child = Command::new(env!("CARGO_BIN_EXE_work-engine-claims-host"))
            .arg(action)
            .args(head)
            .args(tail)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut pipe = child.stdin.take().unwrap();
        if let Some(bytes) = stdin {
            let _ = pipe.write_all(&bytes);
        }
        drop(pipe);
        let output = child.wait_with_output().unwrap();
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).into());
        }
        serde_json::from_slice(&output.stdout)
            .map_err(|e| format!("{e}: {}", String::from_utf8_lossy(&output.stdout)))
    }
    fn reassess_args(&self, selections: Vec<Value>) -> Result<Value, String> {
        self.reassess_request(&json!({"schema_version":1,"selections":selections}))
    }
    fn reassess_request(&self, request: &Value) -> Result<Value, String> {
        self.host_raw(
            "reassess",
            &[&self.startup.to_string_lossy(), &self.sha],
            &[
                "--binding",
                &self.binding.to_string_lossy(),
                "--artifacts",
                &self.artifacts.to_string_lossy(),
            ],
            Some(request),
        )
    }
    fn write(&self, path: &str, text: &str) {
        fs::write(self.checkout.join(path), text).unwrap();
    }
    /// A schema-valid capture over the checkout's current bytes at `paths`.
    fn capture(&self, repo: &str, label: &str, paths: &[&str], pages: Option<Vec<Value>>) -> Value {
        let generation = format!("generation-{label}");
        let envelope = |tool: &str, arguments: Value, result: Value| json!({"request":{"tool":tool,"arguments":arguments},"response":json!({"content":[{"type":"text","text":result.to_string()}],"structuredContent":result,"isError":false}).to_string()});
        let graph = |data: Value| json!({"store_generation":generation,"snapshot_consistent":true,"data":data});
        let mut observations = vec![envelope(
            "get_code_snippet",
            json!({"project":"project-a","format":"json","snapshot":true}),
            graph(json!({"content_verified":true,"source":"fixture"})),
        )];
        for page in pages.unwrap_or_default() {
            let mut page = page;
            page["store_generation"] = json!(generation);
            let args =
                json!({"project":"project-a","format":"json","scope":"doc_references","limit":1});
            observations.push(envelope("detect_changes", args, page));
        }
        let query = envelope(
            "query_graph",
            json!({"project":"project-a","format":"json","snapshot":true}),
            graph(json!({"rows":[]})),
        )["response"]
            .clone();
        let anchors: Vec<Value> = paths
            .iter()
            .map(|p| {
                let content = fs::read_to_string(self.checkout.join(p)).unwrap();
                json!({"path":p,"sha256":sha256(content.as_bytes()),"content":content})
            })
            .collect();
        json!({"schema_version":1,"repository_id":repo,"project":"project-a","executable_sha256":"a".repeat(64),
            "initialization":json!({"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05","serverInfo":{"name":"codebase-memory-mcp","version":"fixture"}}}).to_string(),
            "project_binding":{"repository_path":"/fixture/repository","before":query,"index_status":json!({"content":[{"type":"text","text":json!({"project":"project-a","root_path":"/fixture/repository"}).to_string()}],"structuredContent":{"project":"project-a","root_path":"/fixture/repository"},"isError":false}).to_string(),"after":query},
            "store_generation":generation,"anchors":anchors,"observations":observations,
            "limitations":["Fixture capture; attributed observations only; applicability not assessed."]})
    }
    /// Publishes a capture in custody and returns its evidence reference.
    fn publish(&self, value: &Value, repo: &str) -> Value {
        let bytes = serde_json::to_vec(value).unwrap();
        let artifact = verify(&bytes, &sha256(&bytes), repo).unwrap();
        publish_artifact(&self.artifacts, &artifact).unwrap();
        artifact.reference()
    }
    fn simple(&self, label: &str, paths: &[&str], pages: Option<Vec<Value>>) -> Value {
        self.publish(&self.capture(REPO, label, paths, pages), REPO)
    }
    fn claim_request(&self, key: &str, references: Vec<Value>) -> Value {
        self.claim_request_with(key, references, None, vec![])
    }
    /// A claim whose subject baseline and sensitivity references are chosen
    /// independently of its ordinary evidence. `baseline: None` copies the first
    /// ordinary reference, as the older fixtures did.
    fn claim_request_with(
        &self,
        key: &str,
        references: Vec<Value>,
        baseline: Option<Value>,
        sensitivity: Vec<Value>,
    ) -> Value {
        let revision = json!({"proposition":"Fixture-authored bounded assertion.","support_qualification":"fixture judgment, not a model assessment","assumptions":[],"limitations":["Synthetic fixture."],"confidence":{"label":"bounded"},"evidence_references":references,"sensitivity_references":sensitivity,"evidence_mode":"cbm_capture","judgment_kind":"repository_fact","decision_scope":"scope","profile_payload":{"repository_id":REPO,"assertion_key":key,"assertion_scope":"fixture assertion"},"reopening_conditions":["new source evidence"],"tombstone":false});
        let baseline = baseline.unwrap_or_else(|| revision["evidence_references"].get(0).cloned().unwrap_or_else(|| json!({"owner":"capture","reference":"fixture:evidence","revision":"1","integrity_sha256":"0".repeat(64),"freshness":"exact","status":"unavailable"})));
        json!({"schema_version":1,"operation_id":format!("create-{key}"),"action":"create_claim","profile":"repository-fact-v1","expected_state":null,"payload":{"subject":{"namespace":format!("repository-fact:{REPO}"),"subject_kind":"repository_fact","stable_subject_id":key,"evidence_baseline":baseline,"content_set":["src/lib.rs"]},"statement_identity":key,"initial_revision":revision}})
    }
    /// Publishes a claim (via the existing producer path); returns its receipt.
    fn claim(&self, key: &str, references: Vec<Value>) -> Value {
        self.claim_with(key, references, None, vec![])
    }
    fn claim_with(
        &self,
        key: &str,
        references: Vec<Value>,
        baseline: Option<Value>,
        sensitivity: Vec<Value>,
    ) -> Value {
        let request = self.claim_request_with(key, references, baseline, sensitivity);
        let receipt = self.host(&["execute"], Some(&request)).unwrap();
        assert_eq!(receipt["status"], "applied", "{receipt}");
        receipt
    }
    fn successor(
        &self,
        create: &Value,
        receipt: &Value,
        op: &str,
        references: Vec<Value>,
    ) -> Value {
        let mut revision = create["payload"]["initial_revision"].clone();
        revision["proposition"] = json!("Fixture-authored successor assertion.");
        revision["evidence_references"] = json!(references);
        let request = json!({"schema_version":1,"operation_id":op,"action":"publish_revision","profile":"repository-fact-v1","expected_state":receipt["result"]["record"]["id"],"payload":{"claim_id":receipt["result"]["record"]["claim_id"],"revision":revision}});
        let out = self.host(&["execute"], Some(&request)).unwrap();
        assert_eq!(out["status"], "applied", "{out}");
        out
    }
}

fn page(rows: Vec<Value>) -> Value {
    json!({"scope":"doc_references","semantics":"referenced_targets_for_review","coverage":"indexed_references_only","total":rows.len(),"total_relation":"eq","returned":rows.len(),"has_more":false,"truncated":false,"next_cursor":null,"references":rows})
}
/// First page of a three-row result whose continuation was never captured.
fn partial_page(rows: Vec<Value>) -> Value {
    json!({"scope":"doc_references","semantics":"referenced_targets_for_review","coverage":"indexed_references_only","total":rows.len()+2,"total_relation":"eq","returned":rows.len(),"has_more":true,"truncated":false,"next_cursor":"cursor-1","references":rows})
}
fn row(document: &str, status: &str, reference: &str) -> Value {
    json!({"document":document,"section":"fixture.section","heading_line":1,"target":if status=="resolved" {"fixture.target"} else {""},"file":"","label":"","status":status,"reference":reference,"change_mapping":"hunk"})
}
fn selection(receipt: &Value, new: Option<&str>) -> Value {
    json!({"claim_id":receipt["result"]["record"]["claim_id"],"revision_id":receipt["result"]["record"]["id"],"revision_sha256":receipt["result"]["sha256"],"new_capture_sha256":new})
}
fn sha_of(reference: &Value) -> String {
    reference["integrity_sha256"].as_str().unwrap().to_owned()
}
fn codes(item: &Value, list: &str) -> Vec<String> {
    item[list]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["code"].as_str().unwrap().to_owned())
        .collect()
}
fn snapshot(path: &Path) -> BTreeMap<PathBuf, (u32, u64, u64, Vec<u8>)> {
    fn visit(base: &Path, path: &Path, out: &mut BTreeMap<PathBuf, (u32, u64, u64, Vec<u8>)>) {
        let m = fs::symlink_metadata(path).unwrap();
        out.insert(
            path.strip_prefix(base).unwrap().into(),
            (
                m.permissions().mode(),
                m.dev(),
                m.ino(),
                if m.is_file() {
                    fs::read(path).unwrap()
                } else {
                    Vec::new()
                },
            ),
        );
        if m.is_dir() {
            for entry in fs::read_dir(path).unwrap() {
                visit(base, &entry.unwrap().path(), out);
            }
        }
    }
    let mut out = BTreeMap::new();
    visit(path, path, &mut out);
    out
}
fn standard(f: &Fixture) {
    f.write("src/lib.rs", "pub fn balance() -> i32 { 10 }\n");
    f.write("docs/notes.md", "The balance is ten.\n");
    f.write("docs/guide.md", "See balance.\n");
}

#[test]
fn code_only_change_nominates_through_anchor_currency_with_zero_documentation_rows() {
    let f = Fixture::new();
    standard(&f);
    let reference = f.simple(
        "a",
        &["src/lib.rs", "docs/notes.md"],
        Some(vec![page(vec![])]),
    );
    let claim = f.claim("k", vec![reference.clone()]);
    let request = vec![selection(&claim, None)];

    let unchanged = f.reassess_args(request.clone()).unwrap();
    let item = &unchanged["items"][0];
    assert_eq!(item["disposition"], "no_nomination_observed");
    assert_eq!(item["claim"]["applicability"], "not_assessed");
    assert_eq!(unchanged["observation_only"], true);
    assert_eq!(
        unchanged["completeness"]["scope"],
        "explicit_selections_only"
    );
    assert_eq!(unchanged["completeness"]["complete"], true);
    assert_eq!(
        item["evidence"][0]["capture"]["source_currency"]["all_anchors_bytes_match"],
        true
    );

    f.write("src/lib.rs", "pub fn balance() -> i32 { 5 }\n");
    let changed = f.reassess_args(request).unwrap();
    let item = &changed["items"][0];
    assert_eq!(item["disposition"], "review_nominated");
    assert_eq!(codes(item, "reasons"), ["anchor_bytes_changed"]);
    assert_eq!(item["reasons"][0]["detail"]["path"], "src/lib.rs");
    assert_eq!(item["reasons"][0]["capture_sha256"], sha_of(&reference));
    let documentation = &item["evidence"][0]["capture"]["documentation"];
    assert_eq!(documentation["rows"], 0, "{documentation}");
    assert_eq!(documentation["pagination"], "complete");
    assert_eq!(
        item["evidence"][0]["capture"]["source_currency"]["anchors"][1]["state"],
        "bytes_match"
    );
    assert_eq!(changed["completeness"]["review_nominated"], 1);
    let currency = &item["evidence"][0]["capture"]["source_currency"];
    assert_eq!(currency["anchors_total"], 2);
    assert_eq!(currency["anchors_bytes_match"], 1);
}

#[test]
fn large_anchor_sets_are_bounded_with_attention_first_and_counts_for_all() {
    let f = Fixture::new();
    standard(&f);
    let names: Vec<String> = (0..130).map(|i| format!("src/f{i:03}.rs")).collect();
    for name in &names {
        f.write(name, "fn f() {}\n");
    }
    let paths: Vec<&str> = names.iter().map(String::as_str).collect();
    let reference = f.simple("big", &paths, Some(vec![page(vec![])]));
    let claim = f.claim("k", vec![reference]);
    f.write("src/f129.rs", "fn f() { changed }\n");
    let out = f.reassess_args(vec![selection(&claim, None)]).unwrap();
    let currency = &out["items"][0]["evidence"][0]["capture"]["source_currency"];
    assert_eq!(currency["anchors_total"], 130);
    assert_eq!(currency["anchors_bytes_match"], 129);
    assert_eq!(currency["omitted_anchors"], 30);
    assert_eq!(currency["anchors"].as_array().unwrap().len(), 100);
    assert_eq!(currency["anchors"][0]["path"], "src/f129.rs");
    assert_eq!(currency["anchors"][0]["state"], "bytes_changed");
    assert_eq!(currency["all_anchors_bytes_match"], false);
    assert_eq!(codes(&out["items"][0], "reasons"), ["anchor_bytes_changed"]);
}

#[test]
fn documentation_change_and_unavailable_anchor_are_distinct_reasons() {
    let f = Fixture::new();
    standard(&f);
    let reference = f.simple(
        "a",
        &["src/lib.rs", "docs/notes.md"],
        Some(vec![page(vec![])]),
    );
    let claim = f.claim("k", vec![reference]);
    f.write("docs/notes.md", "The balance is five.\n");
    fs::remove_file(f.checkout.join("src/lib.rs")).unwrap();
    let out = f.reassess_args(vec![selection(&claim, None)]).unwrap();
    let item = &out["items"][0];
    assert_eq!(item["disposition"], "review_nominated");
    let by_code: BTreeMap<_, _> = item["reasons"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            (
                r["code"].as_str().unwrap(),
                r["detail"]["path"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        by_code,
        BTreeMap::from([
            ("anchor_bytes_changed", "docs/notes.md"),
            ("anchor_bytes_unavailable", "src/lib.rs")
        ])
    );
}

#[test]
fn unresolved_navigation_is_associated_only_to_anchored_documents() {
    let f = Fixture::new();
    standard(&f);
    // Rows in one capture cover an anchored doc, an unrelated doc and a row without a path.
    let rows = vec![
        row("docs/notes.md", "missing", "src/lib.rs::missing_balance"),
        row("docs/other.md", "missing", "src/lib.rs::elsewhere"),
        row("docs/guide.md", "resolved", ""),
        json!({"status":"missing","reference":"no-document-path"}),
        row("docs/guide.md", "stale-link", "odd"),
    ];
    let with_rows = f.simple(
        "rows",
        &["src/lib.rs", "docs/notes.md"],
        Some(vec![page(rows.clone())]),
    );
    let unrelated = f.simple(
        "unrelated",
        &["src/lib.rs", "docs/guide.md"],
        Some(vec![page(rows)]),
    );
    let a = f.claim("a", vec![with_rows.clone()]);
    let b = f.claim("b", vec![unrelated.clone()]);
    let out = f
        .reassess_args(vec![selection(&a, None), selection(&b, None)])
        .unwrap();
    assert_eq!(out["completeness"]["selected"], 2);

    let item_a = &out["items"][0];
    assert_eq!(item_a["disposition"], "review_nominated");
    assert_eq!(
        codes(item_a, "reasons"),
        ["documentation_references_unresolved"]
    );
    let documentation = &item_a["evidence"][0]["capture"]["documentation"];
    assert_eq!(documentation["associated_unresolved_rows"], 1);
    assert_eq!(documentation["other_document_rows"], 3);
    assert_eq!(documentation["other_document_unresolved_rows"], 1);
    assert_eq!(documentation["rows_without_document_path"], 1);
    let reported = &documentation["associated_rows"][0];
    assert_eq!(reported["row"]["reference"], "src/lib.rs::missing_balance");
    assert_eq!(reported["capture_sha256"], sha_of(&with_rows));
    assert_eq!(reported["observation_index"], 1);
    assert_eq!(reported["classification"], "unresolved");
    // The unrelated row without a path stays visible as unresolved association.
    assert!(
        codes(item_a, "uncertainties")
            .contains(&"documentation_row_association_unresolved".to_string())
    );

    // The other claim's anchor is docs/guide.md: its own docs are resolved/unknown,
    // and the unresolved row for docs/notes.md does not nominate it.
    let item_b = &out["items"][1];
    assert_eq!(item_b["reasons"].as_array().unwrap().len(), 0, "{item_b}");
    assert_eq!(item_b["disposition"], "uncertain");
    let documentation = &item_b["evidence"][0]["capture"]["documentation"];
    assert_eq!(documentation["associated_resolved_rows"], 1);
    assert_eq!(documentation["associated_unknown_status_rows"], 1);
    assert_eq!(documentation["other_document_unresolved_rows"], 2);
    assert!(
        codes(item_b, "uncertainties").contains(&"documentation_row_status_unknown".to_string())
    );
    let classes: Vec<_> = documentation["associated_rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["classification"].as_str().unwrap())
        .collect();
    assert_eq!(classes, ["unknown_status", "resolved"]);
}

#[test]
fn partial_and_not_requested_documentation_are_never_clear() {
    let f = Fixture::new();
    standard(&f);
    let partial = f.simple(
        "partial",
        &["src/lib.rs"],
        Some(vec![partial_page(vec![row(
            "docs/notes.md",
            "resolved",
            "",
        )])]),
    );
    let none = f.simple("none", &["src/lib.rs"], None);
    let a = f.claim("a", vec![partial]);
    let b = f.claim("b", vec![none]);
    let out = f
        .reassess_args(vec![selection(&a, None), selection(&b, None)])
        .unwrap();
    assert_eq!(out["items"][0]["disposition"], "uncertain");
    assert_eq!(
        codes(&out["items"][0], "uncertainties"),
        ["documentation_coverage_partial"]
    );
    assert_eq!(
        out["items"][0]["evidence"][0]["capture"]["documentation"]["pagination"],
        "partial"
    );
    assert_eq!(out["items"][1]["disposition"], "uncertain");
    assert_eq!(
        codes(&out["items"][1], "uncertainties"),
        ["documentation_not_requested"]
    );
    assert_eq!(
        out["items"][1]["evidence"][0]["capture"]["documentation"]["pagination"],
        "not_requested"
    );
    assert_eq!(
        out["completeness"]["complete"], true,
        "uncertain is assessed, not blocked"
    );
    assert_eq!(out["completeness"]["review_nominated"], 0);
}

#[test]
fn new_capture_difference_staleness_and_documentation_are_reported() {
    let f = Fixture::new();
    standard(&f);
    let old = f.simple(
        "old",
        &["src/lib.rs", "docs/notes.md"],
        Some(vec![page(vec![])]),
    );
    let claim = f.claim("k", vec![old.clone()]);
    f.write("src/lib.rs", "pub fn balance() -> i32 { 5 }\n");
    let new = f.simple(
        "new",
        &["src/lib.rs", "docs/notes.md", "docs/guide.md"],
        Some(vec![page(vec![row(
            "docs/notes.md",
            "missing",
            "src/lib.rs::gone",
        )])]),
    );
    let out = f
        .reassess_args(vec![selection(&claim, Some(&sha_of(&new)))])
        .unwrap();
    let item = &out["items"][0];
    assert_eq!(item["disposition"], "review_nominated");
    let reasons = codes(item, "reasons");
    for expected in [
        "new_capture_differs",
        "new_capture_unresolved_documentation_references",
    ] {
        assert!(reasons.contains(&expected.to_string()), "{reasons:?}");
    }
    assert!(
        reasons.contains(&"anchor_bytes_changed".to_string()),
        "old anchor changed: {reasons:?}"
    );
    let differs = item["reasons"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["code"] == "new_capture_differs")
        .unwrap();
    assert_eq!(
        differs["detail"]["anchors_changed"]["paths"],
        json!(["src/lib.rs"])
    );
    assert_eq!(
        differs["detail"]["anchors_added"]["paths"],
        json!(["docs/guide.md"])
    );
    assert_eq!(item["new_capture"]["role"], "new_capture");
    assert_eq!(
        item["new_capture"]["source_currency"]["all_anchors_bytes_match"],
        true
    );

    // The new capture is no longer current proof after the repository moves again.
    f.write("docs/guide.md", "Edited after the new capture.\n");
    let stale = f
        .reassess_args(vec![selection(&claim, Some(&sha_of(&new)))])
        .unwrap();
    let item = &stale["items"][0];
    assert_eq!(
        item["new_capture"]["source_currency"]["all_anchors_bytes_match"],
        false
    );
    assert!(codes(item, "uncertainties").contains(&"new_capture_not_current".to_string()));
    let anchors = item["new_capture"]["source_currency"]["anchors"]
        .as_array()
        .unwrap();
    assert_eq!(
        anchors
            .iter()
            .find(|a| a["path"] == "docs/guide.md")
            .unwrap()["state"],
        "bytes_changed"
    );

    // A new capture with incomplete documentation observations is itself a reason.
    let incomplete = f.simple("new2", &["src/lib.rs"], None);
    let out = f
        .reassess_args(vec![selection(&claim, Some(&sha_of(&incomplete)))])
        .unwrap();
    assert!(
        codes(&out["items"][0], "reasons")
            .contains(&"new_capture_documentation_incomplete".to_string())
    );
    // Supplying the selected evidence itself is noted, not treated as a difference.
    let same = f
        .reassess_args(vec![selection(&claim, Some(&sha_of(&old)))])
        .unwrap();
    assert!(!codes(&same["items"][0], "reasons").contains(&"new_capture_differs".to_string()));
    assert_eq!(same["items"][0]["notes"].as_array().unwrap().len(), 1);
}

#[test]
fn missing_foreign_corrupt_and_unsupported_evidence_block_visibly() {
    let f = Fixture::new();
    standard(&f);
    let good = f.simple("good", &["src/lib.rs"], Some(vec![page(vec![])]));
    let missing = json!({"owner":"repository-evidence-cbm","reference":format!("cbm-capture@{}","1".repeat(64)),"revision":"1".repeat(64),"integrity_sha256":"1".repeat(64),"freshness":"x","status":"verified"});
    let foreign = f.publish(
        &f.capture(
            "repo-b",
            "foreign",
            &["src/lib.rs"],
            Some(vec![page(vec![])]),
        ),
        "repo-b",
    );
    let corrupt_sha = "2".repeat(64);
    fs::write(
        f.artifacts.join(format!("{corrupt_sha}.json")),
        b"{\"schema_version\":1}",
    )
    .unwrap();
    let corrupt = json!({"owner":"repository-evidence-cbm","reference":format!("cbm-capture@{corrupt_sha}"),"revision":corrupt_sha,"integrity_sha256":corrupt_sha,"freshness":"x","status":"verified"});
    let unsupported = json!({"owner":"capture","reference":"fixture:evidence","revision":"1","integrity_sha256":"0".repeat(64),"freshness":"exact","status":"verified"});
    let malformed = json!({"owner":"repository-evidence-cbm","reference":"cbm-capture@not-a-digest","revision":"x","integrity_sha256":"3".repeat(64),"freshness":"x","status":"verified"});
    let mut selections = Vec::new();
    selections.push(selection(&f.claim("good", vec![good.clone()]), None));
    for (key, reference) in [
        ("missing", missing),
        ("foreign", foreign),
        ("corrupt", corrupt),
        ("unsupported", unsupported),
        ("malformed", malformed),
    ] {
        selections.push(selection(&f.claim(key, vec![reference]), None));
    }
    let out = f.reassess_args(selections).unwrap();
    assert_eq!(out["completeness"]["complete"], false);
    assert_eq!(out["completeness"]["selected"], 6);
    assert_eq!(out["completeness"]["assessed"], 1);
    assert_eq!(out["completeness"]["blocked"], 5);
    assert_eq!(out["items"][0]["disposition"], "no_nomination_observed");
    let expected = [
        "artifact_missing",
        "artifact_foreign_repository",
        "artifact_digest_mismatch",
        "unsupported_evidence_owner",
        "malformed_cbm_reference",
    ];
    for (index, code) in expected.iter().enumerate() {
        let item = &out["items"][index + 1];
        assert_eq!(item["status"], "blocked", "{item}");
        assert_eq!(item["disposition"], "blocked");
        // These fixtures reuse the one bad reference as ordinary evidence and as the
        // subject baseline, so both locations are classified and both block.
        assert_eq!(codes(item, "blocked"), [*code, *code], "{item}");
        assert_eq!(
            item["blocked"][0]["location"]["role"],
            "historical_baseline"
        );
        assert_eq!(
            item["blocked"][0]["location"]["field"],
            "subject.evidence_baseline"
        );
        assert_eq!(item["blocked"][1]["location"]["role"], "selected_evidence");
        assert_eq!(item["blocked"][1]["location"]["index"], 0);
        assert_eq!(item["reference_accounting"]["locations"], 2);
        assert_eq!(item["reference_accounting"]["blocked"], 2);
    }

    // One bad reference blocks its item while the other CBM reference is still assessed.
    let mixed = f.claim("mixed", vec![good.clone(), json!({"owner":"capture","reference":"other","revision":"1","integrity_sha256":"0".repeat(64),"freshness":"x","status":"verified"})]);
    f.write("src/lib.rs", "changed\n");
    let out = f.reassess_args(vec![selection(&mixed, None)]).unwrap();
    let item = &out["items"][0];
    assert_eq!(item["disposition"], "blocked");
    assert_eq!(item["evidence"].as_array().unwrap().len(), 2);
    assert_eq!(codes(item, "reasons"), ["anchor_bytes_changed"]);
    assert_eq!(out["completeness"]["complete"], false);

    // Requested new captures fail the same way instead of being dropped.
    let claim = f.claim("newbad", vec![good]);
    for bad in [
        "1".repeat(64),
        sha_of(&f.publish(
            &f.capture("repo-b", "foreign2", &["src/lib.rs"], None),
            "repo-b",
        )),
        corrupt_sha,
    ] {
        let out = f
            .reassess_args(vec![selection(&claim, Some(&bad))])
            .unwrap();
        assert_eq!(out["items"][0]["status"], "blocked");
        assert_eq!(out["completeness"]["complete"], false);
        assert_eq!(
            out["items"][0]["blocked"][0]["detail"]["role"],
            "new_capture"
        );
    }
}

#[test]
fn exact_historical_revisions_multiple_claims_and_multiple_evidence() {
    let f = Fixture::new();
    standard(&f);
    let first = f.simple("first", &["src/lib.rs"], Some(vec![page(vec![])]));
    let second = f.simple("second", &["docs/notes.md"], Some(vec![page(vec![])]));
    let create = f.claim_request("hist", vec![first.clone()]);
    let created = f.host(&["execute"], Some(&create)).unwrap();
    let successor = f.successor(
        &create,
        &created,
        "succ",
        vec![first.clone(), second.clone()],
    );
    let other = f.claim("other", vec![second.clone()]);
    f.write("docs/notes.md", "edited\n");

    let out = f
        .reassess_args(vec![
            selection(&created, None),
            selection(&successor, None),
            selection(&other, None),
        ])
        .unwrap();
    let old = &out["items"][0];
    assert_eq!(
        old["claim"]["revision_id"],
        created["result"]["record"]["id"]
    );
    assert_eq!(old["claim"]["current_head"], false);
    assert_eq!(
        old["claim"]["head_revision_ids"],
        json!([successor["result"]["record"]["id"]])
    );
    assert_eq!(
        old["evidence"].as_array().unwrap().len(),
        1,
        "exact old revision keeps its own evidence"
    );
    assert_eq!(old["disposition"], "no_nomination_observed");
    assert!(
        old["notes"][0]
            .as_str()
            .unwrap()
            .contains("not a current head")
    );

    let head = &out["items"][1];
    assert_eq!(head["claim"]["current_head"], true);
    assert_eq!(head["evidence"].as_array().unwrap().len(), 2);
    assert_eq!(codes(head, "reasons"), ["anchor_bytes_changed"]);
    assert_eq!(head["reasons"][0]["capture_sha256"], sha_of(&second));
    assert_eq!(
        head["claim"]["revision_sha256"],
        successor["result"]["sha256"]
    );
    assert_ne!(
        old["claim"]["revision_sha256"],
        head["claim"]["revision_sha256"]
    );
    for item in &out["items"].as_array().unwrap()[..2] {
        assert_eq!(
            item["projection"]["projection"]["freshness"],
            "current_relative_to_canonical_input"
        );
        assert!(
            item["projection"]["projection"]["canonical_input_sha256"]
                .as_str()
                .unwrap()
                .len()
                == 64
        );
        assert_eq!(item["claim"]["decision_scope"], "scope");
    }
    assert_eq!(out["items"][2]["disposition"], "review_nominated");
    // Accounting follows each exact revision's own declared locations.
    assert_eq!(
        old["reference_accounting"]["declared"]["evidence_references"],
        1
    );
    assert_eq!(
        head["reference_accounting"]["declared"]["evidence_references"],
        2
    );
    assert_eq!(head["reference_accounting"]["declared"]["baseline"], 1);
    assert_eq!(head["reference_accounting"]["locations"], 3);
    assert_eq!(head["reference_accounting"]["assessed"], 3);
}

#[test]
fn wrong_digest_claim_scope_and_binding_do_not_resolve_or_get_dropped() {
    let f = Fixture::new();
    standard(&f);
    let reference = f.simple("a", &["src/lib.rs"], Some(vec![page(vec![])]));
    let claim = f.claim("k", vec![reference]);
    let good = selection(&claim, None);
    let mut wrong_digest = good.clone();
    wrong_digest["revision_sha256"] = json!("9".repeat(64));
    let mut wrong_claim = good.clone();
    wrong_claim["claim_id"] = json!("not-the-claim");
    let mut wrong_revision = good.clone();
    wrong_revision["revision_id"] = json!("no-such-revision");
    let out = f
        .reassess_args(vec![good.clone(), wrong_claim, wrong_revision])
        .unwrap();
    assert_eq!(out["completeness"]["complete"], false);
    assert_eq!(out["completeness"]["selected"], 3);
    assert_eq!(out["items"][0]["status"], "assessed");
    // Same address with another digest is its own batch: one address, one selection.
    let digest = f.reassess_args(vec![wrong_digest]).unwrap();
    assert_eq!(digest["completeness"]["complete"], false);
    for item in out["items"].as_array().unwrap()[1..]
        .iter()
        .chain(digest["items"].as_array().unwrap())
    {
        assert_eq!(
            codes(item, "blocked"),
            ["claim_resolution_failed"],
            "{item}"
        );
        assert_eq!(item["claim"], Value::Null);
    }
    assert!(
        digest["items"][0]["blocked"][0]["detail"]["error"]
            .as_str()
            .unwrap()
            .contains("exact revision reference mismatch")
    );

    // A startup selecting another decision scope reads nothing of this scope.
    let mut startup: Value = serde_json::from_slice(&fs::read(&f.startup).unwrap()).unwrap();
    startup["decision_scope"] = json!("other-scope");
    let other = f.base.join("other-startup.json");
    fs::write(&other, serde_json::to_vec(&startup).unwrap()).unwrap();
    let other_sha = sha256_bytes(&fs::read(&other).unwrap());
    let request = json!({"schema_version":1,"selections":[good]});
    let out = f
        .host_raw(
            "reassess",
            &[&other.to_string_lossy(), &other_sha],
            &[
                "--binding",
                &f.binding.to_string_lossy(),
                "--artifacts",
                &f.artifacts.to_string_lossy(),
            ],
            Some(&request),
        )
        .unwrap();
    assert_eq!(out["decision_scope"], "other-scope");
    assert_eq!(
        out["items"][0]["blocked"][0]["detail"]["error"],
        "read decision scope mismatch"
    );
    assert_eq!(out["completeness"]["complete"], false);

    // A wrong startup digest never reaches the reader.
    let error = f
        .host_raw(
            "reassess",
            &[&f.startup.to_string_lossy(), &"0".repeat(64)],
            &[
                "--binding",
                &f.binding.to_string_lossy(),
                "--artifacts",
                &f.artifacts.to_string_lossy(),
            ],
            Some(&request),
        )
        .unwrap_err();
    assert!(error.contains("startup digest mismatch"), "{error}");

    // A binding for another repository ID is refused for the whole batch.
    let foreign = f.base.join("foreign-binding.json");
    fs::write(&foreign, serde_json::to_vec(&json!({"repository_id":"repo-b","repository":f.checkout,"project":"project-a","executable":"/x","executable_sha256":"a".repeat(64)})).unwrap()).unwrap();
    let error = f
        .host_raw(
            "reassess",
            &[&f.startup.to_string_lossy(), &f.sha],
            &[
                "--binding",
                &foreign.to_string_lossy(),
                "--artifacts",
                &f.artifacts.to_string_lossy(),
            ],
            Some(&request),
        )
        .unwrap_err();
    assert!(
        error.contains("binding repository_id does not match"),
        "{error}"
    );
}

#[test]
fn binding_project_name_does_not_authorize_or_gate_historical_captures() {
    let f = Fixture::new();
    standard(&f);
    let reference = f.simple("a", &["src/lib.rs"], Some(vec![page(vec![])]));
    let claim = f.claim("k", vec![reference]);
    // The capture's CBM project is project-a; today's binding names another project.
    let renamed: Value = json!({"repository_id":REPO,"repository":f.checkout,"project":"renamed-project","executable":"/x","executable_sha256":"b".repeat(64)});
    fs::write(&f.binding, serde_json::to_vec(&renamed).unwrap()).unwrap();
    let out = f.reassess_args(vec![selection(&claim, None)]).unwrap();
    assert_eq!(out["items"][0]["status"], "assessed");
    assert_eq!(out["items"][0]["disposition"], "no_nomination_observed");
    assert_eq!(
        out["source_checkout"]["binding_sha256"],
        sha256_bytes(&fs::read(&f.binding).unwrap())
    );
}

#[test]
fn request_and_invocation_bounds_reject_the_whole_batch() {
    let f = Fixture::new();
    standard(&f);
    let reference = f.simple("a", &["src/lib.rs"], Some(vec![page(vec![])]));
    let claim = f.claim("k", vec![reference]);
    let good = selection(&claim, None);
    let bad = |selections: Vec<Value>| f.reassess_args(selections).unwrap_err();
    assert!(bad(vec![]).contains("selections must number"));
    assert!(bad(vec![good.clone(); 2]).contains("duplicate selection"));
    assert!(bad(vec![good.clone(); 17]).contains("selections must number"));
    let mut malformed = good.clone();
    malformed["revision_sha256"] = json!("ABC");
    assert!(bad(vec![malformed]).contains("exact identifiers"));
    let mut extra = good.clone();
    extra["follow_head"] = json!(true);
    assert!(bad(vec![extra]).contains("follow_head"));
    assert!(
        f.reassess_request(&json!({"schema_version":2,"selections":[good.clone()]}))
            .unwrap_err()
            .contains("unsupported")
    );
    assert!(
        f.reassess_request(
            &json!({"schema_version":1,"selections":[good.clone()],"root":"/elsewhere"})
        )
        .is_err()
    );
    let one = serde_json::to_string(&json!([good.clone()])).unwrap();
    let duplicate_key =
        format!(r#"{{"schema_version":1,"selections":{one},"selections":{one}}}"#).into_bytes();
    let error = f
        .host_bytes(
            "reassess",
            &[&f.startup.to_string_lossy(), &f.sha],
            &[
                "--binding",
                &f.binding.to_string_lossy(),
                "--artifacts",
                &f.artifacts.to_string_lossy(),
            ],
            Some(duplicate_key),
        )
        .unwrap_err();
    assert!(error.contains("duplicate"), "{error}");
    let oversize = vec![b' '; 1024 * 1024 + 1];
    let error = f
        .host_bytes(
            "reassess",
            &[&f.startup.to_string_lossy(), &f.sha],
            &[
                "--binding",
                &f.binding.to_string_lossy(),
                "--artifacts",
                &f.artifacts.to_string_lossy(),
            ],
            Some(oversize),
        )
        .unwrap_err();
    assert!(error.contains("exceeds capacity"), "{error}");
    // Invocation shape: missing, duplicate and unknown flags; redirected artifact root.
    let startup = f.startup.to_string_lossy().into_owned();
    let binding = f.binding.to_string_lossy().into_owned();
    let artifacts = f.artifacts.to_string_lossy().into_owned();
    let request = json!({"schema_version":1,"selections":[good.clone()]});
    let (binding, artifacts) = (binding.as_str(), artifacts.as_str());
    for tail in [
        vec!["--binding", binding, "--binding", binding],
        vec!["--artifacts", artifacts, "--artifacts", artifacts],
        vec!["--binding", binding, "--other", artifacts],
    ] {
        assert!(
            f.host_raw("reassess", &[&startup, &f.sha], &tail, Some(&request))
                .unwrap_err()
                .contains("usage: work-engine-claims-host reassess")
        );
    }
    assert!(
        f.host_raw(
            "reassess",
            &[&startup, &f.sha],
            &["--binding", binding],
            Some(&request)
        )
        .unwrap_err()
        .contains("usage")
    );
    let link = f.base.join("artifact-link");
    std::os::unix::fs::symlink(&f.artifacts, &link).unwrap();
    let error = f
        .host_raw(
            "reassess",
            &[&startup, &f.sha],
            &["--binding", binding, "--artifacts", &link.to_string_lossy()],
            Some(&request),
        )
        .unwrap_err();
    assert!(error.contains("canonical absolute directory"), "{error}");
    // Existing syntax is unchanged: the old usage line still names the original four.
    let error = f
        .host_raw("bogus", &[&startup, &f.sha], &[], None)
        .unwrap_err();
    assert!(error.contains("usage: work-engine-claims-host initialize|execute|inspect|read TRUSTED_STARTUP EXPECTED_STARTUP_SHA256"));
}

#[test]
fn reassessment_changes_no_canonical_artifact_or_checkout_bytes() {
    let f = Fixture::new();
    standard(&f);
    let first = f.simple(
        "first",
        &["src/lib.rs", "docs/notes.md"],
        Some(vec![page(vec![row("docs/notes.md", "missing", "x")])]),
    );
    let new = f.simple("new", &["src/lib.rs", "docs/notes.md"], None);
    let claim = f.claim("k", vec![first]);
    f.write("src/lib.rs", "changed\n");
    let (root, artifacts, checkout) = (
        snapshot(&f.root),
        snapshot(&f.artifacts),
        snapshot(&f.checkout),
    );
    let before_reads = f
        .host(
            &["read"],
            Some(&json!({"action":"discover","query":"","limit":10,"cursor":null})),
        )
        .unwrap();
    let out = f
        .reassess_args(vec![selection(&claim, Some(&sha_of(&new)))])
        .unwrap();
    assert_eq!(out["items"][0]["disposition"], "review_nominated");
    let _ = f
        .reassess_args(vec![selection(&claim, Some(&"4".repeat(64)))])
        .unwrap();
    assert_eq!(
        root,
        snapshot(&f.root),
        "claims root (database, locks, journals) must be untouched"
    );
    assert_eq!(artifacts, snapshot(&f.artifacts));
    assert_eq!(checkout, snapshot(&f.checkout));
    let after_reads = f
        .host(
            &["read"],
            Some(&json!({"action":"discover","query":"","limit":10,"cursor":null})),
        )
        .unwrap();
    assert_eq!(before_reads, after_reads);
    assert_eq!(before_reads["items"].as_array().unwrap().len(), 1);
}

fn cbm_reference(sha: &str, status: &str) -> Value {
    json!({"owner":"repository-evidence-cbm","reference":format!("cbm-capture@{sha}"),"revision":sha,"integrity_sha256":sha,"freshness":"x","status":status})
}
fn location_of(finding: &Value) -> (String, Value, String) {
    let l = &finding["location"];
    (
        l["field"].as_str().unwrap().to_owned(),
        l["index"].clone(),
        l["role"].as_str().unwrap().to_owned(),
    )
}
/// (expected blocked code, reference) for every way a material reference can be bad.
fn bad_references(f: &Fixture, good: &Value) -> Vec<(&'static str, Value)> {
    let corrupt_sha = "2".repeat(64);
    fs::write(
        f.artifacts.join(format!("{corrupt_sha}.json")),
        b"{\"schema_version\":1}",
    )
    .unwrap();
    let foreign = f.publish(
        &f.capture(
            "repo-b",
            "foreign",
            &["src/lib.rs"],
            Some(vec![page(vec![])]),
        ),
        "repo-b",
    );
    vec![
        (
            "artifact_missing",
            cbm_reference(&"1".repeat(64), "verified"),
        ),
        ("artifact_foreign_repository", foreign),
        (
            "artifact_digest_mismatch",
            cbm_reference(&corrupt_sha, "verified"),
        ),
        (
            "unsupported_evidence_owner",
            json!({"owner":"capture","reference":"fixture:evidence","revision":"1","integrity_sha256":"0".repeat(64),"freshness":"exact","status":"verified"}),
        ),
        (
            "malformed_cbm_reference",
            json!({"owner":"repository-evidence-cbm","reference":"cbm-capture@not-a-digest","revision":"x","integrity_sha256":"3".repeat(64),"freshness":"x","status":"verified"}),
        ),
        (
            "evidence_reference_not_verified",
            cbm_reference(&sha_of(good), "unavailable"),
        ),
    ]
}

#[test]
fn sensitivity_changes_nominate_while_ordinary_evidence_is_unchanged() {
    let f = Fixture::new();
    standard(&f);
    let evidence = f.simple("ev", &["src/lib.rs"], Some(vec![page(vec![])]));
    let sensitivity = f.simple("sens", &["docs/notes.md"], Some(vec![page(vec![])]));
    assert_ne!(sha_of(&evidence), sha_of(&sensitivity));
    let claim = f.claim_with("k", vec![evidence.clone()], None, vec![sensitivity.clone()]);
    let request = vec![selection(&claim, None)];

    let clean = f.reassess_args(request.clone()).unwrap();
    let item = &clean["items"][0];
    assert_eq!(item["disposition"], "no_nomination_observed", "{item}");
    assert_eq!(clean["completeness"]["complete"], true);
    let accounting = &item["reference_accounting"];
    assert_eq!(
        accounting["declared"],
        json!({"baseline":1,"evidence_references":1,"sensitivity_references":1})
    );
    assert_eq!(
        (
            accounting["locations"].as_u64(),
            accounting["assessed"].as_u64()
        ),
        (Some(3), Some(3))
    );
    assert_eq!(
        clean["completeness"]["reference_locations"],
        json!({"classified":3,"assessed":3,"blocked":0})
    );
    let roles: Vec<(String, Value, String)> = accounting["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(location_of_entry)
        .collect();
    assert_eq!(
        roles,
        [
            (
                "subject.evidence_baseline".to_owned(),
                Value::Null,
                "historical_baseline".to_owned()
            ),
            (
                "revision.evidence_references".to_owned(),
                json!(0),
                "selected_evidence".to_owned()
            ),
            (
                "revision.sensitivity_references".to_owned(),
                json!(0),
                "current_sensitivity".to_owned()
            ),
        ]
    );
    assert_eq!(
        item["sensitivity"][0]["capture"]["role"],
        "current_sensitivity"
    );
    assert_eq!(item["sensitivity"][0]["index"], 0);
    assert_eq!(
        item["historical_baseline"]["capture"]["role"],
        "historical_baseline"
    );

    // Only the distinct sensitivity capture's anchor changes.
    f.write("docs/notes.md", "The balance is now five.\n");
    let changed = f.reassess_args(request.clone()).unwrap();
    let item = &changed["items"][0];
    assert_eq!(item["disposition"], "review_nominated", "{item}");
    assert_eq!(codes(item, "reasons"), ["anchor_bytes_changed"]);
    assert_eq!(item["reasons"][0]["capture_sha256"], sha_of(&sensitivity));
    assert_eq!(
        location_of(&item["reasons"][0]),
        (
            "revision.sensitivity_references".to_owned(),
            json!(0),
            "current_sensitivity".to_owned()
        )
    );
    assert_eq!(
        item["evidence"][0]["capture"]["source_currency"]["all_anchors_bytes_match"],
        true
    );
    assert!(
        item["historical_observations"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    // Unavailable sensitivity bytes nominate with their own reason.
    fs::remove_file(f.checkout.join("docs/notes.md")).unwrap();
    let gone = f.reassess_args(request).unwrap();
    let item = &gone["items"][0];
    assert_eq!(item["disposition"], "review_nominated", "{item}");
    assert_eq!(codes(item, "reasons"), ["anchor_bytes_unavailable"]);
    assert_eq!(location_of(&item["reasons"][0]).2, "current_sensitivity");
    assert_eq!(gone["completeness"]["complete"], true);
}

fn location_of_entry(entry: &Value) -> (String, Value, String) {
    (
        entry["field"].as_str().unwrap().to_owned(),
        entry["index"].clone(),
        entry["role"].as_str().unwrap().to_owned(),
    )
}

#[test]
fn repeated_capture_references_keep_every_field_index_and_role() {
    let f = Fixture::new();
    standard(&f);
    let shared = f.simple("shared", &["src/lib.rs"], Some(vec![page(vec![])]));
    let claim = f.claim_with(
        "rep",
        vec![shared.clone(), shared.clone()],
        None,
        vec![shared.clone()],
    );
    let request = vec![selection(&claim, None)];
    let clean = f.reassess_args(request.clone()).unwrap();
    let item = &clean["items"][0];
    assert_eq!(item["reference_accounting"]["locations"], 4, "{item}");
    assert_eq!(item["reference_accounting"]["assessed"], 4);
    assert_eq!(item["disposition"], "no_nomination_observed");

    f.write("src/lib.rs", "pub fn balance() -> i32 { 1 }\n");
    let out = f.reassess_args(request).unwrap();
    let item = &out["items"][0];
    assert_eq!(item["disposition"], "review_nominated");
    let mut seen: Vec<(String, Value, String)> = item["reasons"]
        .as_array()
        .unwrap()
        .iter()
        .map(location_of)
        .collect();
    seen.sort_by_key(|(field, index, _)| (field.clone(), index.to_string()));
    assert_eq!(
        seen,
        [
            (
                "revision.evidence_references".to_owned(),
                json!(0),
                "selected_evidence".to_owned()
            ),
            (
                "revision.evidence_references".to_owned(),
                json!(1),
                "selected_evidence".to_owned()
            ),
            (
                "revision.sensitivity_references".to_owned(),
                json!(0),
                "current_sensitivity".to_owned()
            ),
        ]
    );
    // The same capture as immutable subject baseline is history, not a fourth reason.
    assert_eq!(
        codes(item, "historical_observations"),
        ["anchor_bytes_changed"]
    );
    assert_eq!(
        location_of(&item["historical_observations"][0]).2,
        "historical_baseline"
    );
    assert_eq!(out["completeness"]["historical_observations"], 1);
}

#[test]
fn invalid_sensitivity_references_block_without_hiding_assessed_evidence() {
    let f = Fixture::new();
    standard(&f);
    let good = f.simple("good", &["src/lib.rs"], Some(vec![page(vec![])]));
    let mut selections = Vec::new();
    let bad = bad_references(&f, &good);
    for (index, (_, reference)) in bad.iter().enumerate() {
        let claim = f.claim_with(
            &format!("s{index}"),
            vec![good.clone()],
            None,
            vec![reference.clone()],
        );
        selections.push(selection(&claim, None));
    }
    let out = f.reassess_args(selections).unwrap();
    assert_eq!(out["completeness"]["complete"], false);
    assert_eq!(out["completeness"]["blocked"], bad.len());
    for (index, (code, _)) in bad.iter().enumerate() {
        let item = &out["items"][index];
        assert_eq!(item["disposition"], "blocked", "{code}: {item}");
        assert_eq!(codes(item, "blocked"), [*code], "{item}");
        assert_eq!(
            location_of(&item["blocked"][0]),
            (
                "revision.sensitivity_references".to_owned(),
                json!(0),
                "current_sensitivity".to_owned()
            )
        );
        // Ordinary evidence and baseline are still assessed; only the bad location blocks.
        assert_eq!(item["evidence"][0]["capture"]["role"], "selected_evidence");
        assert_eq!(item["reference_accounting"]["locations"], 3);
        assert_eq!(item["reference_accounting"]["assessed"], 2);
        assert_eq!(item["reference_accounting"]["blocked"], 1);
        assert_eq!(
            item["reference_accounting"]["entries"][2]["blocked_codes"],
            json!([code])
        );
        assert!(item["sensitivity"][0]["capture"].is_null());
    }
}

#[test]
fn invalid_baseline_references_block_and_survive_a_current_evidence_successor() {
    let f = Fixture::new();
    standard(&f);
    let good = f.simple("good", &["src/lib.rs"], Some(vec![page(vec![])]));
    let bad = bad_references(&f, &good);
    let mut selections = Vec::new();
    for (index, (_, reference)) in bad.iter().enumerate() {
        let claim = f.claim_with(
            &format!("b{index}"),
            vec![good.clone()],
            Some(reference.clone()),
            vec![],
        );
        selections.push(selection(&claim, None));
    }
    let out = f.reassess_args(selections).unwrap();
    assert_eq!(out["completeness"]["complete"], false);
    for (index, (code, _)) in bad.iter().enumerate() {
        let item = &out["items"][index];
        assert_eq!(item["disposition"], "blocked", "{code}: {item}");
        assert_eq!(codes(item, "blocked"), [*code], "{item}");
        assert_eq!(
            location_of(&item["blocked"][0]),
            (
                "subject.evidence_baseline".to_owned(),
                Value::Null,
                "historical_baseline".to_owned()
            )
        );
        assert_eq!(item["evidence"][0]["capture"]["role"], "selected_evidence");
        assert_eq!(item["reference_accounting"]["blocked"], 1);
    }

    // The baseline is immutable: a successor with fully current evidence still
    // cannot report a complete assessment while the baseline is unresolved.
    let create = f.claim_request_with(
        "succ-bad",
        vec![good.clone()],
        Some(bad[0].1.clone()),
        vec![],
    );
    let created = f.host(&["execute"], Some(&create)).unwrap();
    let fresh = f.simple("fresh", &["src/lib.rs"], Some(vec![page(vec![])]));
    let successor = f.successor(&create, &created, "succ-bad-2", vec![fresh]);
    let out = f.reassess_args(vec![selection(&successor, None)]).unwrap();
    let item = &out["items"][0];
    assert_eq!(item["disposition"], "blocked", "{item}");
    assert_eq!(item["claim"]["current_head"], true);
    assert_eq!(codes(item, "blocked"), ["artifact_missing"]);
    assert_eq!(out["completeness"]["complete"], false);
    assert!(item["reasons"].as_array().unwrap().is_empty());
}

#[test]
fn valid_old_baseline_with_changed_bytes_stays_historical_after_a_current_successor() {
    let f = Fixture::new();
    standard(&f);
    let old = f.simple("old", &["src/lib.rs"], Some(vec![page(vec![])]));
    let create = f.claim_request("hist2", vec![old.clone()]);
    let created = f.host(&["execute"], Some(&create)).unwrap();
    f.write("src/lib.rs", "pub fn balance() -> i32 { 7 }\n");
    let fresh = f.simple("fresh", &["src/lib.rs"], Some(vec![page(vec![])]));
    let successor = f.successor(&create, &created, "hist2-succ", vec![fresh.clone()]);

    let out = f
        .reassess_args(vec![selection(&successor, None), selection(&created, None)])
        .unwrap();
    // Refreshed current evidence: the old baseline's change is not re-nominated.
    let head = &out["items"][0];
    assert_eq!(head["claim"]["current_head"], true);
    assert_eq!(head["disposition"], "no_nomination_observed", "{head}");
    assert!(head["reasons"].as_array().unwrap().is_empty());
    assert_eq!(
        head["historical_baseline"]["capture"]["role"],
        "historical_baseline"
    );
    assert_eq!(
        head["historical_baseline"]["capture"]["capture_sha256"],
        sha_of(&old)
    );
    assert_eq!(
        head["historical_baseline"]["capture"]["source_currency"]["all_anchors_bytes_match"],
        false
    );
    assert_eq!(
        codes(head, "historical_observations"),
        ["anchor_bytes_changed"]
    );
    assert_eq!(
        location_of(&head["historical_observations"][0]).2,
        "historical_baseline"
    );
    assert_eq!(
        head["evidence"][0]["capture"]["source_currency"]["all_anchors_bytes_match"],
        true
    );
    assert_eq!(head["reference_accounting"]["assessed"], 2);

    // The exact old revision's own current evidence still nominates, as before.
    let old_item = &out["items"][1];
    assert_eq!(old_item["claim"]["current_head"], false);
    assert_eq!(old_item["disposition"], "review_nominated");
    assert_eq!(
        location_of(&old_item["reasons"][0]),
        (
            "revision.evidence_references".to_owned(),
            json!(0),
            "selected_evidence".to_owned()
        )
    );
    assert_eq!(out["completeness"]["complete"], true);
    assert_eq!(out["completeness"]["historical_observations"], 2);
}
