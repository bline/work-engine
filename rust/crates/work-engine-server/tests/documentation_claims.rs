#[path = "support/documentation_claims.rs"]
mod documentation_claims;
use claim_evidence::{OpenFiles, RepositoryReadStartup, RepositoryReader, codec::canonical_json};
use documentation_claims::{load, write_json};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
fn inventory(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(base: &Path, path: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        if path.is_dir() {
            for item in fs::read_dir(path).unwrap() {
                visit(base, &item.unwrap().path(), out);
            }
        } else {
            out.insert(
                path.strip_prefix(base).unwrap().to_path_buf(),
                fs::read(path).unwrap(),
            );
        }
    }
    let mut result = BTreeMap::new();
    visit(root, root, &mut result);
    result
}
fn reader(state: &Value) -> RepositoryReader {
    let startup = load(Path::new(state["startup"].as_str().unwrap())).unwrap();
    RepositoryReader::open(RepositoryReadStartup {
        files: OpenFiles {
            root: startup["root"].as_str().unwrap().into(),
            config: startup["config"].as_str().unwrap().into(),
            source: startup["source"].as_str().unwrap().into(),
            expected_config_sha256: startup["expected_config_sha256"].as_str().unwrap().into(),
            expected_source_sha256: startup["expected_source_sha256"].as_str().unwrap().into(),
        },
        repository_id: startup["repository_id"].as_str().unwrap().into(),
        decision_scope: startup["decision_scope"].as_str().unwrap().into(),
    })
    .unwrap()
}
fn resolve(state: &Value, exact: &Value) -> Value {
    serde_json::from_str(
        &canonical_json(
            &reader(state)
                .resolve(
                    exact["claim_id"].as_str().unwrap(),
                    exact["revision_id"].as_str().unwrap(),
                    exact["revision_sha256"].as_str().unwrap(),
                )
                .unwrap(),
        )
        .unwrap(),
    )
    .unwrap()
}
#[test]
#[ignore = "requires explicitly pinned CBM, claims host/server/capture binaries and starts only its own private CBM daemon"]
fn real_documentation_claims_survive_capture_only_changes_and_manual_successor() {
    let directory = tempfile::Builder::new()
        .prefix("we-doc-claims-test-")
        .tempdir_in("/tmp")
        .unwrap()
        .keep();
    let manifest = directory.join("state.json");
    struct Cleanup(PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            if self.0.exists() {
                let _ = documentation_claims::cleanup(&self.0);
            }
        }
    }
    let _cleanup = Cleanup(manifest.clone());
    let mut state = documentation_claims::prepare(&manifest).unwrap();
    println!("qualification_manifest={}", manifest.display());
    let baseline = state["current_capture"].clone();
    assert_eq!(state["stages"][0]["observation"]["stdout"], "10");
    let captured = load(Path::new(
        state["stages"][0]["capture_full"].as_str().unwrap(),
    ))
    .unwrap();
    let mentions = captured["capture"]["observations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["request"]["tool"] == "query_graph")
        .unwrap();
    assert!(
        mentions["response"].as_str().unwrap().contains("balance"),
        "{mentions}"
    );
    let exact = state["initial_exact"].clone();
    let root = PathBuf::from(state["root"].as_str().unwrap());
    let canonical_before = inventory(&root);
    let mut captures = std::collections::BTreeSet::new();
    captures.insert(
        baseline["reference"]["integrity_sha256"]
            .as_str()
            .unwrap()
            .to_owned(),
    );
    for (case, output) in [("code", "5"), ("docs", "10"), ("unresolved", "10")] {
        state = documentation_claims::advance(&manifest, case).unwrap();
        let stage = state["stages"].as_array().unwrap().last().unwrap();
        assert_eq!(stage["observation"]["stdout"], output);
        assert_eq!(
            canonical_before,
            inventory(&root),
            "advance must not publish or rely"
        );
        assert_eq!(
            resolve(&state, &exact)["items"][0]["revision_sha256"],
            exact["revision_sha256"]
        );
        let sha = stage["capture"]["reference"]["integrity_sha256"]
            .as_str()
            .unwrap();
        assert!(captures.insert(sha.to_owned()));
        let artifact = repository_evidence_cbm::read_artifact(
            Path::new(state["artifacts"].as_str().unwrap()),
            baseline["reference"]["integrity_sha256"].as_str().unwrap(),
            state["repository_id"].as_str().unwrap(),
        )
        .unwrap();
        let currency = repository_evidence_cbm::source_currency(
            &artifact,
            Path::new(state["repository"].as_str().unwrap()),
        )
        .unwrap();
        let anchors = currency["anchors"].as_array().unwrap();
        let anchor_state = |name: &str| {
            anchors.iter().find(|a| a["path"] == name).unwrap()["state"]
                .as_str()
                .unwrap()
        };
        assert_eq!(
            anchor_state("src/ledger.rs"),
            if case == "code" {
                "bytes_changed"
            } else {
                "bytes_match"
            }
        );
        assert_eq!(
            anchor_state("docs/ledger.md"),
            if case == "code" {
                "bytes_match"
            } else {
                "bytes_changed"
            }
        );
        assert_eq!(currency["applicability"], "not_assessed");
        write_json(
            &PathBuf::from(stage["directory"].as_str().unwrap()).join("baseline-currency.json"),
            &currency,
        )
        .unwrap();
        if case == "unresolved" {
            let captured = load(Path::new(stage["capture_full"].as_str().unwrap())).unwrap();
            let observation = captured["capture"]["observations"]
                .as_array()
                .unwrap()
                .iter()
                .find(|o| o["request"]["tool"] == "detect_changes")
                .unwrap();
            let response: Value =
                serde_json::from_str(observation["response"].as_str().unwrap()).unwrap();
            let evidence = &response["result"]["structuredContent"];
            assert_eq!(evidence["references"][0]["status"], "missing");
            assert_eq!(
                evidence["references"][0]["reference"],
                "src/ledger.rs::missing_balance"
            );
            assert_eq!(evidence["has_more"], false);
            assert_eq!(evidence["total_relation"], "eq");
        }
    }
    // Labelled fixture judgment exercises publication mechanics; it is not a
    // replacement for Root's real model consumer reassessment.
    let mut revision = state["initial_revision_payload"].clone();
    revision["proposition"] = json!(
        "Fixture-producer judgment: sample output remains10; the documentation target is unresolved, so navigation support is insufficient."
    );
    revision["evidence_references"] = json!([state["current_capture"]["reference"]]);
    revision["support_qualification"] = json!("deterministic fixture-producer judgment only");
    let request = json!({"schema_version":1,"operation_id":"fixture-manual-successor","action":"publish_revision","profile":"repository-fact-v1","expected_state":exact["revision_id"],"payload":{"claim_id":exact["claim_id"],"revision":revision}});
    let receipt = documentation_claims::publish(
        &state,
        "execute",
        Some(&request),
        &directory.join("manual-successor"),
    )
    .unwrap();
    assert_eq!(receipt["status"], "applied", "{receipt}");
    let old = resolve(&state, &exact);
    assert_eq!(old["items"][0]["revision_sha256"], exact["revision_sha256"]);
    assert_eq!(
        old["items"][0]["revision"]["proposition"],
        state["initial_revision_payload"]["proposition"]
    );
    let successor = json!({"claim_id":exact["claim_id"],"revision_id":receipt["result"]["record"]["id"],"revision_sha256":receipt["result"]["sha256"]});
    assert_eq!(
        resolve(&state, &successor)["items"][0]["revision"]["support_qualification"],
        "deterministic fixture-producer judgment only"
    );
    documentation_claims::cleanup(&manifest).unwrap();
}

#[test]
fn inherited_git_selectors_cannot_mutate_external_repository() {
    const CHILD: &str = "WORK_ENGINE_DOCUMENTATION_GIT_TEST_STATE";
    if let Some(selected) = std::env::var_os(CHILD) {
        let state = load(Path::new(&selected)).unwrap();
        documentation_claims::initialize_git(&state, Path::new(state["stage"].as_str().unwrap()))
            .unwrap();
        return;
    }
    fn clean_git(cwd: &Path, args: &[&str]) -> Vec<u8> {
        let mut command = std::process::Command::new("git");
        for (name, _) in std::env::vars_os() {
            if name.as_encoded_bytes().starts_with(b"GIT_") {
                command.env_remove(name);
            }
        }
        let output = command
            .current_dir(cwd)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        output.stdout
    }
    let temp = tempfile::tempdir().unwrap();
    let sentinel = temp.path().join("external");
    fs::create_dir(&sentinel).unwrap();
    fs::write(
        sentinel.join("sentinel.txt"),
        b"external worktree must remain unchanged",
    )
    .unwrap();
    clean_git(&sentinel, &["init", "-q"]);
    clean_git(&sentinel, &["add", "sentinel.txt"]);
    clean_git(
        &sentinel,
        &[
            "-c",
            "user.name=Sentinel",
            "-c",
            "user.email=sentinel@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-qm",
            "sentinel",
        ],
    );
    let separate_index = sentinel.join("external-index");
    fs::copy(sentinel.join(".git/index"), &separate_index).unwrap();
    let external_head = clean_git(&sentinel, &["rev-parse", "HEAD"]);
    let before = inventory(&sentinel);
    let fixture = temp.path().join("fixture");
    for directory in ["", "src", "docs", "runtime", "cache", "tmp", "stage"] {
        fs::create_dir(fixture.join(directory)).unwrap();
    }
    fs::write(fixture.join("src/ledger.rs"), "pub fn balance() {}\n").unwrap();
    fs::write(fixture.join("docs/ledger.md"), "fixture only\n").unwrap();
    fs::write(fixture.join("runtime-observation.json"), "{}\n").unwrap();
    let state_path = temp.path().join("child-state.json");
    write_json(&state_path, &json!({"repository":fixture,"runtime":fixture.join("runtime"),"cache":fixture.join("cache"),"tmp":fixture.join("tmp"),"stage":fixture.join("stage")})).unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "inherited_git_selectors_cannot_mutate_external_repository",
            "--nocapture",
        ])
        .env(CHILD, &state_path)
        .env("GIT_DIR", sentinel.join(".git"))
        .env("GIT_WORK_TREE", &sentinel)
        .env("GIT_INDEX_FILE", &separate_index)
        .env("GIT_COMMON_DIR", sentinel.join(".git"))
        .env("GIT_OBJECT_DIRECTORY", sentinel.join(".git/objects"))
        .env("GIT_CONFIG_COUNT", "2")
        .env("GIT_CONFIG_KEY_0", "core.worktree")
        .env("GIT_CONFIG_VALUE_0", &sentinel)
        .env("GIT_CONFIG_KEY_1", "commit.gpgsign")
        .env("GIT_CONFIG_VALUE_1", "true")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(before, inventory(&sentinel));
    assert_eq!(external_head, clean_git(&sentinel, &["rev-parse", "HEAD"]));
    assert!(fixture.join(".git/index").is_file());
    let tree = String::from_utf8(clean_git(
        &fixture,
        &["ls-tree", "-r", "--name-only", "HEAD"],
    ))
    .unwrap();
    assert_eq!(
        tree.lines().collect::<Vec<_>>(),
        [
            "docs/ledger.md",
            "runtime-observation.json",
            "src/ledger.rs"
        ]
    );
    assert!(!clean_git(&fixture, &["rev-parse", "HEAD"]).is_empty());
}
#[test]
fn observation_and_cleanup_failures_both_remain_visible() {
    let error = documentation_claims::finish_observation(
        Err("capture refused".into()),
        Err("stop spawn failed".into()),
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("capture refused"));
    assert!(error.contains("unresolved private daemon cleanup: stop spawn failed"));
}
