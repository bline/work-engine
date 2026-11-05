//! Claims-only transport using real server processes and native trusted-host fixtures.
use claim_evidence::{
    RepositoryApplication,
    codec::{canonical_json, parse_json},
    sha256_bytes,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    io::{BufRead, BufReader, Write},
    os::{
        fd::AsRawFd,
        unix::fs::{MetadataExt, PermissionsExt},
    },
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};
use work_engine_claims_host::TrustedAdmission;
fn private(path: &Path, value: &Value) -> String {
    let bytes = serde_json::to_vec(value).unwrap();
    fs::write(path, &bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    sha256_bytes(&bytes)
}
struct Fixture {
    _temp: tempfile::TempDir,
    base: PathBuf,
    root: PathBuf,
    profile: PathBuf,
    read: Value,
    exact: Value,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let root = base.join("root");
        let source = base.join("source.json");
        let config = base.join("config.json");
        let startup = base.join("startup.json");
        let profile = base.join("read.json");
        let source_sha = private(
            &source,
            &json!({"schema_version":3,"owner":"trusted-fixture-issuer","reference":"fixture:authority","revision":"1","freshness":"pinned","profile":"repository-claims-v1","repository_id":"repo","actors":["producer"],"decision_scopes":["scope"],"grant_ids":["grant"]}),
        );
        let config_sha = private(
            &config,
            &json!({"schema_version":3,"profile":"repository-claims-v1","root_id":"fixture-root","repository_id":"repo","grants":[{"schema_version":1,"grant_id":"grant","actor":"producer","profile":"repository-fact-v1","permissions":["create_claim"],"decision_scope":"scope","authority_reference":{"owner":"trusted-fixture-issuer","reference":"fixture:authority","revision":"1","freshness":"pinned","integrity_sha256":source_sha,"status":"verified"}}]}),
        );
        let read = json!({"schema_version":1,"root":root,"config":config,"source":source,"expected_config_sha256":config_sha,"expected_source_sha256":source_sha,"repository_id":"repo","decision_scope":"scope"});
        let mut producer = read.clone();
        producer["actor"] = json!("producer");
        producer["grant_id"] = json!("grant");
        producer["allowed_actions"] = json!(["create_claim"]);
        let sha = private(&startup, &producer);
        let port = TrustedAdmission::load(&startup, &sha).unwrap();
        let native_startup = port.startup().producer().unwrap();
        let mut app = RepositoryApplication::initialize(native_startup, port).unwrap();
        let reference = json!({"owner":"capture","reference":"fixture:evidence","revision":"1","integrity_sha256":"0".repeat(64),"freshness":"exact","status":"unavailable"});
        let request = json!({"schema_version":1,"operation_id":"create","action":"create_claim","profile":"repository-fact-v1","expected_state":null,"payload":{"subject":{"namespace":"repository-fact:repo","subject_kind":"repository_fact","stable_subject_id":"assertion","evidence_baseline":reference,"content_set":["src/symbol.rs"]},"statement_identity":"assertion","initial_revision":{"proposition":"The source declares this bounded assertion.","support_qualification":"attributed fixture judgment","assumptions":[],"limitations":["Synthetic fixture"],"confidence":{"label":"bounded"},"evidence_references":[reference],"sensitivity_references":[],"evidence_mode":"capture","judgment_kind":"repository_fact","decision_scope":"scope","profile_payload":{"repository_id":"repo","assertion_key":"assertion","assertion_scope":"bounded assertion about symbol"},"reopening_conditions":["new source evidence"],"tombstone":false}}});
        let receipt = app
            .execute(&parse_json(&request.to_string()).unwrap())
            .unwrap();
        let receipt: Value = serde_json::from_str(&canonical_json(&receipt).unwrap()).unwrap();
        let exact = json!({"claim_id":receipt["result"]["record"]["claim_id"],"revision_id":receipt["result"]["record"]["id"],"revision_sha256":receipt["result"]["sha256"]});
        drop(app);
        private(&profile, &read);
        Self {
            _temp: temp,
            base,
            root,
            profile,
            read,
            exact,
        }
    }
}
struct Client {
    child: Option<Child>,
    input: Option<ChildStdin>,
    output: BufReader<ChildStdout>,
    sequence: u64,
}
impl Client {
    fn start(config: &Path) -> Self {
        let mut client = Self::connect(config);
        assert_eq!(client.rpc("initialize",json!({"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"repository-fact-fixture","version":"1"}}))["result"]["protocolVersion"], "2025-06-18");
        client
    }
    fn connect(config: &Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_work-engine-server"))
            .arg("claims-stdio")
            .arg(config)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let input = child.stdin.take();
        let output = BufReader::new(child.stdout.take().unwrap());
        Self {
            child: Some(child),
            input,
            output,
            sequence: 0,
        }
    }
    fn rpc(&mut self, method: &str, params: Value) -> Value {
        let result = self.rpc_response(method, params);
        assert!(result.get("error").is_none(), "{result}");
        result
    }
    fn rpc_response(&mut self, method: &str, params: Value) -> Value {
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
        let result = self.response();
        assert_eq!(result["id"], id);
        result
    }
    fn response(&mut self) -> Value {
        let mut poll = libc::pollfd {
            fd: self.output.get_ref().as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        assert!(
            unsafe { libc::poll(&mut poll, 1, 10_000) } > 0,
            "response deadline"
        );
        let mut line = String::new();
        assert!(
            self.output.read_line(&mut line).unwrap() > 0,
            "server closed before response"
        );
        serde_json::from_str(&line).unwrap()
    }
    fn tool(&mut self, name: &str, args: Value) -> Value {
        self.rpc("tools/call", json!({"name":name,"arguments":args}))["result"].clone()
    }
    fn close(mut self) {
        self.input.take();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while self.child.as_mut().unwrap().try_wait().unwrap().is_none() {
            assert!(
                std::time::Instant::now() < deadline,
                "EOF shutdown deadline"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
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
fn inventory(root: &Path) -> BTreeMap<PathBuf, (u32, u32, u32, Vec<u8>)> {
    fn walk(root: &Path, path: &Path, out: &mut BTreeMap<PathBuf, (u32, u32, u32, Vec<u8>)>) {
        let m = fs::symlink_metadata(path).unwrap();
        out.insert(
            path.strip_prefix(root).unwrap().into(),
            (
                m.permissions().mode(),
                m.uid(),
                m.gid(),
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

fn assert_no_listener(client: &Client) {
    let pid = client.child.as_ref().unwrap().id();
    let sockets: Vec<_> = fs::read_dir(format!("/proc/{pid}/fd"))
        .unwrap()
        .filter_map(|e| fs::read_link(e.ok()?.path()).ok())
        .collect();
    for family in ["tcp", "tcp6"] {
        for row in fs::read_to_string(format!("/proc/{pid}/net/{family}"))
            .unwrap()
            .lines()
            .skip(1)
        {
            let fields: Vec<_> = row.split_whitespace().collect();
            if fields[3] == "0A" {
                assert!(!sockets.contains(&PathBuf::from(format!("socket:[{}]", fields[9]))));
            }
        }
    }
}
#[test]
fn concurrent_claims_only_clients_read_persisted_facts_without_mutation() {
    let f = Fixture::new();
    let before = inventory(&f.base);
    let mut first = Client::start(&f.profile);
    let mut second = Client::start(&f.profile);
    let exclusive = fs::File::open(&f.profile).unwrap();
    assert_ne!(
        unsafe { libc::flock(exclusive.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
        0
    );
    for client in [&mut first, &mut second] {
        let tools = client.rpc("tools/list", json!({}))["result"]["tools"].clone();
        let names: Vec<_> = tools
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            ["discover_repository_facts", "resolve_repository_fact"]
        );
        let found =
            projection(&client.tool("discover_repository_facts", json!({"query":"assertion"})));
        assert!(
            found
                .to_string()
                .contains(f.exact["claim_id"].as_str().unwrap()),
            "{found}"
        );
        let resolved = projection(&client.tool("resolve_repository_fact", f.exact.clone()));
        assert!(
            resolved
                .to_string()
                .contains("The source declares this bounded assertion.")
        );
        for name in [
            "list_sessions",
            "read_session",
            "submit_session_request",
            "read_session_operation",
            "read_session_result",
            "create_claim",
            "publish_revision",
            "record_reliance",
            "read_repository_evidence",
            "check_repository_evidence_currency",
        ] {
            assert_eq!(client.tool(name, json!({}))["isError"], true, "{name}");
        }
        let bad = client.tool(
            "discover_repository_facts",
            json!({"query":"assertion","root":f.root}),
        );
        assert_eq!(bad["isError"], true);
        assert_no_listener(client);
    }
    assert_eq!(
        first.tool("resolve_repository_fact", f.exact.clone()),
        second.tool("resolve_repository_fact", f.exact.clone())
    );
    first.close();
    second.close();
    assert_eq!(before, inventory(&f.base));
}
fn refused(profile: &Path) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_work-engine-server"))
        .arg("claims-stdio")
        .arg(profile)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while child.try_wait().unwrap().is_none() {
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("startup deadline");
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        !output.status.success(),
        "startup accepted {}",
        fs::read_to_string(profile).unwrap_or_default()
    );
    assert!(output.stdout.is_empty());
}
#[test]
fn invalid_claims_startup_fails_closed_without_repair_or_creation() {
    let f = Fixture::new();
    for read in [
        json!({}),
        {
            let mut v = f.read.clone();
            v["repository_id"] = json!("wrong");
            v
        },
        {
            let mut v = f.read.clone();
            v["root"] = json!(f.base.join("missing"));
            v
        },
        {
            let mut v = f.read.clone();
            v["actor"] = json!("producer");
            v
        },
    ] {
        private(&f.profile, &read);
        let before = inventory(&f.base);
        refused(&f.profile);
        assert_eq!(before, inventory(&f.base));
    }
    private(&f.profile, &f.read);
    let journal = f.root.join("claim-evidence.sqlite3-journal");
    fs::write(&journal, b"recovery marker").unwrap();
    let before = inventory(&f.base);
    refused(&f.profile);
    assert_eq!(before, inventory(&f.base));
    fs::remove_file(journal).unwrap();
    fs::set_permissions(&f.profile, fs::Permissions::from_mode(0o644)).unwrap();
    refused(&f.profile);
    fs::set_permissions(&f.profile, fs::Permissions::from_mode(0o600)).unwrap();
    let link = f.base.join("link.json");
    std::os::unix::fs::symlink(&f.profile, &link).unwrap();
    refused(&link);
    refused(&f.base.join("absent.json"));
}

#[test]
fn selected_scope_never_exposes_another_scopes_fact() {
    let f = Fixture::new();
    let mut wrong = f.read.clone();
    wrong["decision_scope"] = json!("wrong");
    private(&f.profile, &wrong);
    let before = inventory(&f.base);
    let mut client = Client::start(&f.profile);
    let found = projection(&client.tool("discover_repository_facts", json!({"query":"assertion"})));
    assert!(
        !found
            .to_string()
            .contains(f.exact["claim_id"].as_str().unwrap()),
        "{found}"
    );
    assert_eq!(
        client.tool("resolve_repository_fact", f.exact.clone())["isError"],
        true
    );
    client.close();
    assert_eq!(before, inventory(&f.base));
}

#[test]
fn codex_metadata_is_transport_only_and_malformed_requests_stay_closed() {
    let f = Fixture::new();
    let mut wrong = f.read.clone();
    wrong["decision_scope"] = json!("wrong");
    private(&f.profile, &wrong);
    let before = inventory(&f.base);
    let mut client = Client::connect(&f.profile);
    let initialize = json!({"protocolVersion":"2025-06-18","capabilities":{"experimental":{"codex/auth-change":{}},"elicitation":{"form":{},"url":{}}},"clientInfo":{"name":"codex-mcp-client","title":"Codex","version":"0.166.0"}});
    for malformed in [
        Value::Null,
        json!([]),
        json!("metadata"),
        json!(1),
        json!(false),
    ] {
        let mut params = initialize.clone();
        params["_meta"] = malformed;
        assert_eq!(
            client.rpc_response("initialize", params)["error"]["code"],
            -32602
        );
    }
    let mut init = initialize;
    init["_meta"] = json!({"progressToken":0});
    assert_eq!(
        client.rpc("initialize", init)["result"]["protocolVersion"],
        "2025-06-18"
    );
    writeln!(
        client.input.as_mut().unwrap(),
        "{}",
        json!({"jsonrpc":"2.0","method":"notifications/initialized"})
    )
    .unwrap();
    let listed = client.rpc("tools/list", json!({"_meta":{"progressToken":0}}));
    assert_eq!(listed["result"]["tools"].as_array().unwrap().len(), 2);
    let metadata = json!({"progressToken":"read","root":f.root,"repository_id":"repo","decision_scope":"scope","actor":"producer","grant_id":"grant"});
    let found = client.rpc("tools/call",json!({"name":"discover_repository_facts","arguments":{"query":"assertion"},"_meta":metadata}));
    assert!(
        !projection(&found["result"])
            .to_string()
            .contains(f.exact["claim_id"].as_str().unwrap())
    );
    let refused = client.rpc(
        "tools/call",
        json!({"name":"resolve_repository_fact","arguments":f.exact,"_meta":metadata}),
    );
    assert_eq!(refused["result"]["isError"], true);
    for name in [
        "list_sessions",
        "read_session",
        "submit_session_request",
        "read_session_operation",
        "read_session_result",
        "create_claim",
        "publish_revision",
        "record_reliance",
    ] {
        let refused = client.rpc(
            "tools/call",
            json!({"name":name,"arguments":{},"_meta":metadata}),
        );
        assert_eq!(
            refused["result"]["structuredContent"]["error"]["code"],
            "invalid_request"
        );
    }
    for malformed in [
        Value::Null,
        json!([]),
        json!("metadata"),
        json!(1),
        json!(false),
    ] {
        assert_eq!(
            client.rpc_response("tools/list", json!({"_meta":malformed}))["error"]["code"],
            -32602
        );
        assert_eq!(client.rpc_response("tools/call",json!({"name":"discover_repository_facts","arguments":{"query":"assertion"},"_meta":malformed}))["error"]["code"], -32602);
    }
    assert_eq!(
        client.rpc_response("tools/list", json!({"_meta":{},"root":f.root}))["error"]["code"],
        -32602
    );
    assert_eq!(client.rpc_response("tools/call",json!({"name":"discover_repository_facts","arguments":{},"_meta":{},"decision_scope":"scope"}))["error"]["code"], -32602);
    assert_eq!(client.rpc("tools/call",json!({"name":"discover_repository_facts","arguments":{"query":"assertion","decision_scope":"scope"},"_meta":metadata}))["result"]["isError"], true);
    for raw in [
        r#"{"jsonrpc":"2.0","id":999,"method":"tools/list","params":{"_meta":{},"_meta":{}}}"#,
        r#"{"jsonrpc":"2.0","id":999,"method":"tools/list","params":{"_meta":{"progressToken":1,"progressToken":2}}}"#,
    ] {
        writeln!(client.input.as_mut().unwrap(), "{raw}").unwrap();
        client.input.as_mut().unwrap().flush().unwrap();
        assert_eq!(client.response()["error"]["code"], -32700);
    }
    client.close();
    assert_eq!(before, inventory(&f.base));
    // The same Codex-shaped metadata also permits a read in the selected scope.
    private(&f.profile, &f.read);
    let before = inventory(&f.base);
    let mut client = Client::start(&f.profile);
    let resolved = client.rpc(
        "tools/call",
        json!({"name":"resolve_repository_fact","arguments":f.exact,"_meta":{"progressToken":1}}),
    );
    assert!(
        projection(&resolved["result"])
            .to_string()
            .contains("The source declares this bounded assertion.")
    );
    client.close();
    assert_eq!(before, inventory(&f.base));
}

#[test]
fn repository_read_tools_publish_accurate_capability_hints() {
    let f = Fixture::new();
    let artifacts = f.base.join("artifacts");
    fs::create_dir(&artifacts).unwrap();
    let mut read = f.read.clone();
    read["evidence"] = json!({"artifacts":artifacts,"repository":f.base,"project":"fixture"});
    private(&f.profile, &read);
    let before = inventory(&f.base);
    let mut client = Client::start(&f.profile);
    let listing = client.rpc("tools/list", json!({"_meta":{"progressToken":0}}));
    let tools = listing["result"]["tools"].as_array().unwrap();
    let names: Vec<_> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(
        names,
        [
            "discover_repository_facts",
            "resolve_repository_fact",
            "read_repository_evidence",
            "check_repository_evidence_currency"
        ]
    );
    for tool in tools {
        assert_eq!(
            tool["annotations"],
            json!({"readOnlyHint":true,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false}),
            "{}",
            tool["name"]
        );
    }
    client.close();
    assert_eq!(before, inventory(&f.base));
}
