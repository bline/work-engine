//! Explicit local qualification: real CBM, trusted native publisher, and two
//! fresh read-only MCP server processes. No model/provider or production root.
use repository_evidence_cbm::{
    Binding, CaptureRequest, ToolRequest, capture, publish_artifact, sha256,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    io::{BufRead, BufReader, Write},
    net::TcpListener,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

fn private(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}
fn write_json(path: &Path, value: &Value) -> String {
    let bytes = serde_json::to_vec(value).unwrap();
    private(path, &bytes);
    sha256(&bytes)
}
fn command(binary: &Path, args: &[&str], cwd: &Path) -> Value {
    let output = Command::new(binary)
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}: {}",
        binary.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn publish(
    host: &Path,
    startup: &Path,
    startup_sha: &str,
    action: &str,
    request: Option<&Value>,
) -> Value {
    let mut child = Command::new(host)
        .arg(action)
        .arg(startup)
        .arg(startup_sha)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    if let Some(request) = request {
        child
            .stdin
            .take()
            .unwrap()
            .write_all(&serde_json::to_vec(request).unwrap())
            .unwrap();
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
struct Client {
    child: Option<Child>,
    input: Option<ChildStdin>,
    output: BufReader<ChildStdout>,
    sequence: u64,
}
impl Client {
    fn start(config: &Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_work-engine-server"))
            .arg(config)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let input = child.stdin.take();
        let output = BufReader::new(child.stdout.take().unwrap());
        let mut client = Self {
            child: Some(child),
            input,
            output,
            sequence: 0,
        };
        assert!(client.rpc("initialize",json!({"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"repository-fact-fixture","version":"1"}}))["result"]["serverInfo"].is_object());
        client
    }
    fn rpc(&mut self, method: &str, params: Value) -> Value {
        self.sequence += 1;
        let id = self.sequence;
        let input = self.input.as_mut().unwrap();
        writeln!(
            input,
            "{}",
            json!({"jsonrpc":"2.0","id":id,"method":method,"params":params})
        )
        .unwrap();
        input.flush().unwrap();
        let mut line = String::new();
        assert!(
            self.output.read_line(&mut line).unwrap() > 0,
            "server closed before response"
        );
        let result: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(result["id"], id);
        assert!(result.get("error").is_none(), "{result}");
        result
    }
    fn tool(&mut self, name: &str, args: Value) -> Value {
        self.rpc("tools/call", json!({"name":name,"arguments":args}))["result"].clone()
    }
    fn close(mut self) {
        self.input.take();
        let output = self.child.take().unwrap().wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
impl Drop for Client {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            self.input.take();
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
fn inventory(root: &Path) -> BTreeMap<PathBuf, (u32, Vec<u8>)> {
    fn walk(root: &Path, path: &Path, out: &mut BTreeMap<PathBuf, (u32, Vec<u8>)>) {
        let m = fs::symlink_metadata(path).unwrap();
        out.insert(
            path.strip_prefix(root).unwrap().into(),
            (
                m.permissions().mode(),
                if m.is_file() {
                    fs::read(path).unwrap()
                } else {
                    Vec::new()
                },
            ),
        );
        if m.is_dir() {
            for e in fs::read_dir(path).unwrap() {
                walk(root, &e.unwrap().path(), out);
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}
fn projection(result: &Value) -> Value {
    assert_eq!(result["isError"], false, "{result}");
    let owner = &result["structuredContent"]["owner_result"];
    assert_eq!(owner["encoding"], "claim-evidence-canonical-json-v1");
    serde_json::from_str(owner["canonical_projection"].as_str().unwrap()).unwrap()
}

fn complete_projection(client: &mut Client, name: &str, mut args: Value) -> (String, usize) {
    let mut bytes = String::new();
    let mut expected_sha = None;
    let mut offset = 0;
    let mut pages = 0;
    loop {
        let result = client.tool(name, args.clone());
        assert_eq!(result["isError"], false, "{result}");
        assert!(result.to_string().len() < 1024 * 1024);
        let owner = &result["structuredContent"]["owner_result"];
        let sha = owner["projection_sha256"].as_str().unwrap();
        if let Some(expected) = &expected_sha {
            assert_eq!(sha, expected);
        } else {
            expected_sha = Some(sha.to_owned());
        }
        pages += 1;
        if let Some(complete) = owner["canonical_projection"].as_str() {
            assert_eq!(pages, 1);
            bytes.push_str(complete);
            break;
        }
        assert_eq!(owner["offset"], offset);
        bytes.push_str(owner["canonical_projection_page"].as_str().unwrap());
        if owner["has_more"] == false {
            assert_eq!(bytes.len(), owner["total_bytes"].as_u64().unwrap() as usize);
            break;
        }
        let next = owner["next_offset"].as_u64().unwrap() as usize;
        assert!(next > offset);
        offset = next;
        args["projection_page"] = json!({"offset":offset,"sha256":sha,"max_bytes":32768});
    }
    assert_eq!(sha256(bytes.as_bytes()), expected_sha.unwrap());
    (bytes, pages)
}

#[test]
#[ignore = "requires explicitly selected real CBM/claims-host binaries and private CBM cache/runtime; run through the qualification command"]
fn real_cbm_fact_survives_fresh_mcp_clients_with_exact_reliance() {
    let proof = PathBuf::from(std::env::var("WORK_ENGINE_CLAIMS_PROOF_DIR").unwrap())
        .canonicalize()
        .unwrap();
    assert_eq!(
        PathBuf::from(std::env::var("CBM_RUNTIME_DIR").unwrap()),
        proof.join("runtime")
    );
    assert_eq!(
        PathBuf::from(std::env::var("CBM_CACHE_DIR").unwrap()),
        proof.join("cache")
    );
    let cbm = PathBuf::from(std::env::var("WORK_ENGINE_CBM_BINARY").unwrap())
        .canonicalize()
        .unwrap();
    let host = PathBuf::from(std::env::var("WORK_ENGINE_CLAIMS_HOST").unwrap())
        .canonicalize()
        .unwrap();
    struct Cleanup {
        cbm: PathBuf,
        proof: PathBuf,
    }
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let result = Command::new(&self.cbm)
                .args(["daemon", "stop"])
                .output()
                .unwrap();
            private(&self.proof.join("daemon-cleanup.json"),&serde_json::to_vec(&json!({"success":result.status.success(),"stdout":String::from_utf8_lossy(&result.stdout),"stderr":String::from_utf8_lossy(&result.stderr)})).unwrap());
        }
    }
    let _cleanup = Cleanup {
        cbm: cbm.clone(),
        proof: proof.clone(),
    };
    let run = proof.join(format!(
        "run-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&run).unwrap();
    fs::set_permissions(&run, fs::Permissions::from_mode(0o700)).unwrap();
    let repository = run.join("repository");
    fs::create_dir(&repository).unwrap();
    fs::create_dir(repository.join("src")).unwrap();
    fs::create_dir(repository.join("docs")).unwrap();
    let source = "def post_entry(ledger, amount):\n    ledger.append(amount)\n    return balance(ledger)\n\n\ndef balance(ledger):\n    return sum(ledger)\n";
    let docs = "Preamble uses `src/ledger.py::balance`.\n\n# Notes\n\n## Posting\n\nPosting uses `src/ledger.py::post_entry`.\n\n## Unchanged\n\nUnchanged uses `src/ledger.py::balance`.\n";
    fs::write(repository.join("src/ledger.py"), source).unwrap();
    fs::write(repository.join("docs/notes.md"), docs).unwrap();
    for args in [
        vec!["init", "-q"],
        vec!["add", "src/ledger.py", "docs/notes.md"],
        vec![
            "-c",
            "user.name=Claims Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-qm",
            "fixture baseline",
        ],
    ] {
        assert!(
            Command::new("git")
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .args(args)
                .current_dir(&repository)
                .status()
                .unwrap()
                .success()
        );
    }
    fs::write(
        repository.join("docs/notes.md"),
        docs.replace("Preamble uses", "Preamble carefully uses")
            .replace("Posting uses", "Posting always uses"),
    )
    .unwrap();
    let project = "repository-facts-fixture";
    let index = json!({"repo_path":repository,"name":project,"mode":"full","persistence":false})
        .to_string();
    let indexed = command(
        &cbm,
        &["cli", "--quiet", "--json", "index_repository", &index],
        &repository,
    );
    assert_eq!(indexed["isError"], false, "{indexed}");
    let binding = Binding {
        repository_id: "fixture-repository".into(),
        repository: repository.clone(),
        project: project.into(),
        executable: cbm.clone(),
        executable_sha256: repository_evidence_cbm::executable_digest(&cbm).unwrap(),
    };
    let requests = CaptureRequest {
        source_paths: vec!["src/ledger.py".into(), "docs/notes.md".into()],
        tools: vec![
            ToolRequest {
                tool: "query_graph".into(),
                arguments: json!({"query":"MATCH (n:Function) RETURN n.name LIMIT 5"}),
            },
            ToolRequest {
                tool: "search_graph".into(),
                arguments: json!({"name_pattern":"^post_entry$"}),
            },
            ToolRequest {
                tool: "trace_path".into(),
                arguments: json!({"function_name":"post_entry","direction":"outbound","depth":1}),
            },
            ToolRequest {
                tool: "get_code_snippet".into(),
                arguments: json!({"qualified_name":"post_entry","source_mode":"full"}),
            },
            ToolRequest {
                tool: "check_index_coverage".into(),
                arguments: json!({"paths":["src/ledger.py","docs/notes.md"],"diagnostics":"none"}),
            },
            ToolRequest {
                tool: "search_code".into(),
                arguments: json!({"pattern":"return","file_pattern":"src/ledger.py","source_mode":"full"}),
            },
            ToolRequest {
                tool: "detect_changes".into(),
                arguments: json!({"scope":"doc_references","limit":1}),
            },
        ],
    };
    let artifact = capture(&binding, &requests).unwrap();
    assert_eq!(
        artifact.capture().documentation_status().unwrap()["pagination"],
        "complete"
    );
    assert_eq!(
        artifact.capture().documentation_status().unwrap()["rows"],
        2
    );
    assert_eq!(artifact.capture().observations().len(), 8);
    let artifacts = run.join("artifacts");
    fs::create_dir(&artifacts).unwrap();
    publish_artifact(&artifacts, &artifact).unwrap();
    write_json(
        &run.join("capture-binding.json"),
        &serde_json::to_value(&binding).unwrap(),
    );
    write_json(
        &run.join("capture-request.json"),
        &serde_json::to_value(&requests).unwrap(),
    );
    let reference = artifact.reference();
    let source_path = run.join("authority.json");
    let config_path = run.join("claims-config.json");
    let startup_path = run.join("producer-startup.json");
    let claims_root = run.join("claims");
    let authority = json!({"schema_version":3,"owner":"trusted-local-fixture-issuer","reference":"fixture:authority","revision":"1","freshness":"pinned fixture revision","profile":"repository-claims-v1","repository_id":"fixture-repository","actors":["producer"],"decision_scopes":["fixture-scope"],"grant_ids":["fixture-grant"]});
    let authority_sha = write_json(&source_path, &authority);
    let grants = json!({"schema_version":3,"profile":"repository-claims-v1","root_id":"fixture-claims-root","repository_id":"fixture-repository","grants":[{"schema_version":1,"grant_id":"fixture-grant","actor":"producer","profile":"repository-fact-v1","permissions":["create_claim","publish_revision","record_reliance"],"decision_scope":"fixture-scope","authority_reference":{"owner":"trusted-local-fixture-issuer","reference":"fixture:authority","revision":"1","freshness":"pinned fixture revision","integrity_sha256":authority_sha,"status":"verified"}}]});
    let config_sha = write_json(&config_path, &grants);
    let startup = json!({"schema_version":1,"root":claims_root,"config":config_path,"source":source_path,"expected_config_sha256":config_sha,"expected_source_sha256":authority_sha,"repository_id":"fixture-repository","decision_scope":"fixture-scope","actor":"producer","grant_id":"fixture-grant","allowed_actions":["create_claim","publish_revision","record_reliance"]});
    let startup_sha = write_json(&startup_path, &startup);
    assert_eq!(
        publish(&host, &startup_path, &startup_sha, "initialize", None)["status"],
        "initialized"
    );
    let key = "ledger-post-entry";
    let revision = json!({"proposition":"Ledger post_entry appends the amount and returns balance in the captured source.","support_qualification":"attributed observation of captured fixture source","assumptions":[],"limitations":artifact.capture().limitations(),"confidence":{"label":"bounded source observation"},"evidence_references":[reference],"sensitivity_references":[],"evidence_mode":"cbm_capture","judgment_kind":"repository_fact","decision_scope":"fixture-scope","profile_payload":{"repository_id":"fixture-repository","assertion_key":key,"assertion_scope":"captured ledger post_entry source behavior"},"reopening_conditions":["new source evidence"],"tombstone":false});
    let create = json!({"schema_version":1,"operation_id":"create-ledger-fact","action":"create_claim","profile":"repository-fact-v1","expected_state":null,"payload":{"subject":{"namespace":"repository-fact:fixture-repository","subject_kind":"repository_fact","stable_subject_id":key,"evidence_baseline":artifact.reference(),"content_set":["src/ledger.py","docs/notes.md"]},"statement_identity":key,"initial_revision":revision}});
    let receipt = publish(&host, &startup_path, &startup_sha, "execute", Some(&create));
    assert_eq!(receipt["status"], "applied", "{receipt}");
    assert_eq!(
        publish(&host, &startup_path, &startup_sha, "execute", Some(&create))["status"],
        "replayed"
    );
    write_json(&run.join("fact-operation.json"), &create);
    write_json(&run.join("fact-receipt.json"), &receipt);
    let read_profile = run.join("claims-read.json");
    let mut read = startup.clone();
    for key in ["actor", "grant_id", "allowed_actions"] {
        read.as_object_mut().unwrap().remove(key);
    }
    // Historical reads are authorized by stable repository identity even when
    // the deployment now selects another CBM project label.
    read["evidence"] = json!({"artifacts":artifacts,"repository":repository,"project":format!("{project}-relocated")});
    write_json(&read_profile, &read);
    let owner = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../skills/session-events/scripts/session_events.py")
        .canonicalize()
        .unwrap();
    let owner_bytes = fs::read(&owner).unwrap();
    let query_profile = run.join("session-query.json");
    write_json(
        &query_profile,
        &json!({"version":1,"owner":owner,"owner_sha256":sha256(&owner_bytes),"codex_home":run.join("codex"),"claude_home":run.join("claude"),"state_dir":run.join("session-state"),"bridge":null,"context":null}),
    );
    let credential = run.join("credential");
    private(&credential, b"repository-facts-fixture-token-32-characters");
    let socket = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = socket.local_addr().unwrap();
    drop(socket);
    let server_config = run.join("server.json");
    let server = json!({"version":1,"listen":address.to_string(),"mcp":true,"query_profile":query_profile,"credential_file":credential,"repository_claims_profile":read_profile,"max_connections":4,"max_queries":2,"request_timeout_seconds":40});
    write_json(&server_config, &server);
    let exact = json!({"claim_id":receipt["result"]["record"]["claim_id"],"revision_id":receipt["result"]["record"]["id"],"revision_sha256":receipt["result"]["sha256"]});
    let before = inventory(&claims_root);
    let artifact_before = inventory(&artifacts);
    let mut first = Client::start(&server_config);
    let tools = first.rpc("tools/list", json!({}));
    let names: Vec<_> = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"resolve_repository_fact"));
    assert!(
        !names
            .iter()
            .any(|n| n.contains("publish") || n.contains("initialize"))
    );
    let resolved = projection(&first.tool("resolve_repository_fact", exact.clone()));
    assert_eq!(
        resolved["items"][0]["revision_sha256"],
        exact["revision_sha256"]
    );
    assert_eq!(resolved["items"][0]["applicability"], "not_assessed");
    assert_eq!(
        resolved["items"][0]["revision"]["evidence_references"][0],
        artifact.reference()
    );
    let discovered = projection(&first.tool(
        "discover_repository_facts",
        json!({"query":"Ledger","limit":1}),
    ));
    assert_eq!(discovered["items"].as_array().unwrap().len(), 1);
    assert_eq!(
        first.tool("publish_repository_fact", create.clone())["isError"],
        true
    );
    assert_eq!(first.tool("resolve_repository_fact",json!({"claim_id":exact["claim_id"],"revision_id":exact["revision_id"],"revision_sha256":exact["revision_sha256"],"root":"/untrusted"}))["isError"],true);
    let mut collected = String::new();
    let mut offset = 0;
    loop {
        let result = first.tool(
            "read_repository_evidence",
            json!({"sha256":artifact.sha256(),"offset":offset,"max_bytes":97}),
        );
        assert_eq!(result["isError"], false, "{result}");
        let page = &result["structuredContent"]["owner_result"];
        collected.push_str(page["utf8"].as_str().unwrap());
        if page["has_more"] == false {
            break;
        }
        offset = page["next_offset"].as_u64().unwrap() as usize;
    }
    assert_eq!(collected.as_bytes(), artifact.bytes());
    first.close();
    assert_eq!(before, inventory(&claims_root));
    assert_eq!(artifact_before, inventory(&artifacts));
    let rely = json!({"schema_version":1,"operation_id":"use-ledger-fact","action":"record_reliance","profile":"repository-fact-v1","expected_state":null,"payload":{"consumer":"fresh-mcp-client","consumer_revision":"fixture-consumption-1","decision_scope":"fixture-scope","claim_revision_id":exact["revision_id"],"claim_revision_sha256":exact["revision_sha256"],"state":"active","predecessor_reliance":null}});
    let reliance = publish(&host, &startup_path, &startup_sha, "execute", Some(&rely));
    assert_eq!(reliance["status"], "applied", "{reliance}");
    write_json(&run.join("reliance-receipt.json"), &reliance);
    let mut second = Client::start(&server_config);
    let persisted = projection(&second.tool("resolve_repository_fact", exact.clone()));
    assert_eq!(
        persisted["items"][0]["revision_sha256"],
        exact["revision_sha256"]
    );
    assert!(
        !persisted["items"][0]["reliances"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    fs::write(
        repository.join("src/ledger.py"),
        source.replace("sum(ledger)", "max(ledger)"),
    )
    .unwrap();
    let currency = second.tool(
        "check_repository_evidence_currency",
        json!({"sha256":artifact.sha256()}),
    );
    assert_eq!(
        currency["structuredContent"]["owner_result"]["anchors"][0]["state"],
        "bytes_changed"
    );
    assert_eq!(
        currency["structuredContent"]["owner_result"]["applicability"],
        "not_assessed"
    );
    let historical = projection(&second.tool("resolve_repository_fact", exact));
    assert_eq!(
        historical["items"][0]["revision_sha256"],
        receipt["result"]["sha256"]
    );

    let mut large = create.clone();
    large["operation_id"] = json!("create-large-fact");
    large["payload"]["subject"]["stable_subject_id"] = json!("large-fact");
    large["payload"]["statement_identity"] = json!("large-fact");
    large["payload"]["initial_revision"]["profile_payload"]["assertion_key"] = json!("large-fact");
    large["payload"]["initial_revision"]["proposition"] =
        json!(format!("Large fact {}", "x".repeat(600 * 1024)));
    assert!(serde_json::to_vec(&large).unwrap().len() < 1024 * 1024);
    let large_receipt = publish(&host, &startup_path, &startup_sha, "execute", Some(&large));
    assert_eq!(large_receipt["status"], "applied");
    let large_exact = json!({"claim_id":large_receipt["result"]["record"]["claim_id"],"revision_id":large_receipt["result"]["record"]["id"],"revision_sha256":large_receipt["result"]["sha256"]});
    let (large_bytes, large_pages) =
        complete_projection(&mut second, "resolve_repository_fact", large_exact.clone());
    assert!(large_bytes.len() > 1024 * 1024);
    assert!(large_pages > 1);
    let native = || {
        claim_evidence::RepositoryReader::open(claim_evidence::RepositoryReadStartup {
            files: claim_evidence::OpenFiles {
                root: claims_root.clone(),
                config: config_path.clone(),
                source: source_path.clone(),
                expected_config_sha256: config_sha.clone(),
                expected_source_sha256: authority_sha.clone(),
            },
            repository_id: "fixture-repository".into(),
            decision_scope: "fixture-scope".into(),
        })
        .unwrap()
    };
    let native_large = native()
        .resolve(
            large_exact["claim_id"].as_str().unwrap(),
            large_exact["revision_id"].as_str().unwrap(),
            large_exact["revision_sha256"].as_str().unwrap(),
        )
        .unwrap();
    assert_eq!(
        large_bytes,
        claim_evidence::codec::canonical_json(&native_large).unwrap()
    );
    let (discovery_bytes, discovery_pages) = complete_projection(
        &mut second,
        "discover_repository_facts",
        json!({"query":"Large fact","limit":1}),
    );
    assert!(discovery_pages > 1);
    assert_eq!(
        discovery_bytes,
        claim_evidence::codec::canonical_json(&native().discover("Large fact", 1, None).unwrap())
            .unwrap()
    );

    // Later valid reliance growth must remain readable, even when the admitted
    // revision itself was small. Reject a stale page after the first new record.
    let historical_exact = json!({"claim_id":receipt["result"]["record"]["claim_id"],"revision_id":receipt["result"]["record"]["id"],"revision_sha256":receipt["result"]["sha256"]});
    let mut stale_args = historical_exact.clone();
    stale_args["projection_page"] = json!({"offset":0,"max_bytes":1024});
    let initial_page = second.tool("resolve_repository_fact", stale_args.clone());
    let initial_page = &initial_page["structuredContent"]["owner_result"];
    stale_args["projection_page"] = json!({"offset":initial_page["next_offset"],"sha256":initial_page["projection_sha256"],"max_bytes":1024});
    for index in 0..8 {
        let mut grown = rely.clone();
        grown["operation_id"] = json!(format!("additional-reliance-{index}"));
        grown["payload"]["consumer"] = json!(format!("consumer-{index}-{}", "c".repeat(64 * 1024)));
        assert_eq!(
            publish(&host, &startup_path, &startup_sha, "execute", Some(&grown))["status"],
            "applied"
        );
    }
    let stale = second.tool("resolve_repository_fact", stale_args);
    assert_eq!(
        stale["structuredContent"]["owner_error"]["code"],
        "claim_projection_changed"
    );
    let (grown_bytes, reliance_pages) = complete_projection(
        &mut second,
        "resolve_repository_fact",
        historical_exact.clone(),
    );
    assert!(grown_bytes.len() > 1024 * 1024);
    assert!(reliance_pages > 1);
    let native_grown = native()
        .resolve(
            historical_exact["claim_id"].as_str().unwrap(),
            historical_exact["revision_id"].as_str().unwrap(),
            historical_exact["revision_sha256"].as_str().unwrap(),
        )
        .unwrap();
    assert_eq!(
        grown_bytes,
        claim_evidence::codec::canonical_json(&native_grown).unwrap()
    );
    second.close();
    write_json(
        &run.join("RESULT.json"),
        &json!({"status":"passed","cbm_executable_sha256":binding.executable_sha256,"capture_sha256":artifact.sha256(),"store_generation":artifact.capture().store_generation(),"documentation":artifact.capture().documentation_status().unwrap(),"fact_receipt":receipt,"reliance_receipt":reliance,"fresh_mcp_clients":2,"large_fact_projection_pages":large_pages,"large_discovery_projection_pages":discovery_pages,"grown_reliance_projection_pages":reliance_pages,"stale_projection_page_rejected":true,"historical_project_label_changed":true,"production_roots_changed":false,"semantic_applicability":"not_assessed"}),
    );
    println!("qualification={}", run.display());
}
