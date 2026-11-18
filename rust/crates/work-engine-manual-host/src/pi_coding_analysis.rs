//! Hosted analysis profile (`work-engine.pi-analysis-hosted.v1`): no shell,
//! tool container or workspace effect. The job's workspace is a read-only
//! snapshot pinned by its manifest; the Host serves it through two tools and
//! checks every member against the manifest before returning bytes. The
//! container never sees the snapshot directly.
use super::*;

/// The profile's tools, in the receiver's registration order.
pub const TOOLS: [&str; 2] = ["snapshot_list", "snapshot_read"];
/// Host snapshot deliveries per job (`Profile::max_tool_calls` upper bound).
pub const MAX_READS: u32 = 64;
pub const MAX_REQUESTS: u32 = 32;
/// Bytes of the SDK context per provider request. A `coding_provider_request`
/// frame and the gate's `pi.provider_request` ingress carry it twice, which
/// must fit the 1 MiB frame and the store's 1 MiB ingress bound.
pub const MAX_CONTEXT_BYTES: u64 = 400_000;
pub const MAX_REQUEST_BODY: usize = 600_000;
/// One container frame (either direction is checked where it is read).
pub const FRAME_BYTES: usize = 1 << 20;
/// The inference container's stdout for the whole job.
pub const OUTPUT_BYTES: u64 = 64 << 20;
pub const PAGE_BYTES: usize = 24_576;
const SNAPSHOT_BYTES: u64 = 1 << 20;
const SNAPSHOT_FILES: usize = 256;
const MAX_ARGUMENT_BYTES: usize = 1024;
const MAX_RESULT_BYTES: usize = 100_000;
/// The settled answer bound (the store's analysis grammar refuses longer).
pub const MAX_ANSWER_BYTES: usize = 65_536;
/// SHA-256 of the canonical (`serde_json`, sorted keys) `tools` array the
/// pinned receiver and SDK send. Any other tool surface is refused before a
/// provider byte is written.
pub const TOOLS_SHA256: &str = "f447e25baf341152fe46cd10c84b2f9cdcec9d80ada958c37b9ce593ca047fc4";

/// Admission check of the snapshot (the job workspace).
pub(super) fn validate_snapshot(job: &Job) -> Result<()> {
    job.workspace.verify()?;
    let entries = inventory(&job.workspace.root)?;
    let mut total = 0u64;
    for (path, entry) in &entries {
        if !matches!(entry, Entry::File { .. }) || path.len() > 256 {
            return refuse("analysis snapshot member is not a bounded regular file");
        }
        total += fs::metadata(job.workspace.root.join(path))?.len();
    }
    if entries.is_empty() || entries.len() > SNAPSHOT_FILES || total > SNAPSHOT_BYTES {
        return refuse("analysis snapshot bounds");
    }
    Ok(())
}

/// A final assistant answer beyond the settlement bound fails the job when
/// it is observed, not after the history round trip.
pub(super) fn check_answer_bound(event: &Value) -> Result<()> {
    let m = &event["message"];
    if event["type"] == "message_end" && m["role"] == "assistant" && m["stopReason"] == "stop" {
        let text: usize = m["content"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|c| c["text"].as_str())
            .map(str::len)
            .sum();
        if text > MAX_ANSWER_BYTES {
            return refuse("analysis answer exceeds the settlement bound");
        }
    }
    Ok(())
}
/// The request's tool surface: exactly the profile's function tools, with the
/// pinned specification digest.
pub(super) fn check_request_tools(tools: &Value) -> Result<()> {
    let Some(list) = tools.as_array() else {
        return refuse("analysis request tools absent");
    };
    let mut names: Vec<&str> = list.iter().filter_map(|t| t["name"].as_str()).collect();
    names.sort_unstable();
    let mut expected = TOOLS.to_vec();
    expected.sort_unstable();
    let actual = digest(&serde_json::to_vec(tools)?);
    if names != expected || list.iter().any(|t| t["type"] != "function") {
        return refuse("analysis request tool set differs from the profile");
    }
    if actual != TOOLS_SHA256 {
        return refuse(format!(
            "analysis request tool specification differs from the pinned digest (got {actual})"
        ));
    }
    Ok(())
}

/// SHA-256 of the canonical Messages `tools` array the pinned receiver,
/// extension and Claude Code send on the `claude-sdk` provider: Claude Code's
/// `DeferredToolPlaceholder` and the profile's tools as `mcp__pi__<name>`
/// (taken from the first offline run's refusal message, W4).
pub const CLAUDE_SDK_TOOLS_SHA256: &str =
    "4dc958d99fc79a7cc2736c0216fc83e592c8fce53e6dc5e302116f195fa0e503";
/// The Messages request's tool surface: exactly the profile's tools as the
/// plan's Pi provider names them, with the pinned specification digest.
pub(super) fn check_messages_tools(provider: &str, tools: &Value) -> Result<()> {
    let Some(list) = tools.as_array() else {
        return refuse("analysis request tools absent");
    };
    let (prefix, extra, pinned): (&str, &[&str], &str) = match provider {
        "claude-sdk" => (
            "mcp__pi__",
            &["DeferredToolPlaceholder"],
            CLAUDE_SDK_TOOLS_SHA256,
        ),
        _ => return refuse("analysis request provider unknown"),
    };
    let mut names: Vec<String> = list
        .iter()
        .filter_map(|t| t["name"].as_str().map(str::to_owned))
        .collect();
    names.sort_unstable();
    let mut expected: Vec<String> = TOOLS
        .iter()
        .map(|t| format!("{prefix}{t}"))
        .chain(extra.iter().map(|t| (*t).to_owned()))
        .collect();
    expected.sort_unstable();
    if names != expected || names.len() != list.len() {
        return refuse("analysis request tool set differs from the profile");
    }
    let actual = digest(&serde_json::to_vec(tools)?);
    if actual != pinned {
        return refuse(format!(
            "analysis request tool specification differs from the pinned digest (got {actual})"
        ));
    }
    Ok(())
}

/// One read-only tool call over the snapshot, or the error text returned to
/// the model.
fn serve(snapshot: &Tree, tool: &str, arguments: &Value) -> std::result::Result<Value, String> {
    let manifest: BTreeMap<String, Entry> = snapshot
        .manifest
        .verify()
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .ok_or("snapshot manifest unavailable")?;
    let args = arguments.as_object().ok_or("arguments must be an object")?;
    match tool {
        "snapshot_list" => {
            if !args.is_empty() {
                return Err("snapshot_list takes no arguments".into());
            }
            let mut members = vec![];
            for (path, entry) in &manifest {
                let Entry::File { sha256 } = entry else {
                    return Err("snapshot member is not a file".into());
                };
                let bytes = fs::metadata(snapshot.root.join(path))
                    .map_err(|_| "snapshot member unavailable")?
                    .len();
                members.push(json!({"path":path,"bytes":bytes,"sha256":sha256}));
            }
            Ok(json!({"members":members}))
        }
        "snapshot_read" => {
            if args
                .keys()
                .any(|k| !matches!(k.as_str(), "path" | "offset" | "max_bytes"))
            {
                return Err("snapshot_read takes path, offset and max_bytes".into());
            }
            let path = args
                .get("path")
                .and_then(Value::as_str)
                .ok_or("path is required")?;
            let Some(Entry::File { sha256 }) = manifest.get(path) else {
                return Err(format!("{path} is not a snapshot member"));
            };
            let offset = match args.get("offset") {
                None => 0,
                Some(v) => v.as_u64().ok_or("offset must be a non-negative integer")? as usize,
            };
            let max = match args.get("max_bytes") {
                None => PAGE_BYTES,
                Some(v) => v
                    .as_u64()
                    .filter(|n| (1..=PAGE_BYTES as u64).contains(n))
                    .ok_or(format!("max_bytes must be 1..={PAGE_BYTES}"))?
                    as usize,
            };
            let bytes =
                fs::read(snapshot.root.join(path)).map_err(|_| "snapshot member unavailable")?;
            if digest(&bytes) != *sha256 {
                return Err("snapshot member changed".into());
            }
            let text = std::str::from_utf8(&bytes).map_err(|_| "member is not UTF-8 text")?;
            if offset > text.len() || !text.is_char_boundary(offset) {
                return Err("offset is outside the member or inside a character".into());
            }
            let mut end = (offset + max).min(text.len());
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            Ok(
                json!({"path":path,"sha256":sha256,"total":text.len(),"offset":offset,
                "bytes":end - offset,"next_offset":if end < text.len() {json!(end)} else {Value::Null},
                "text":&text[offset..end]}),
            )
        }
        _ => Err(format!("unknown tool {tool}")),
    }
}

impl Host {
    pub(crate) fn analysis_profile(&self) -> bool {
        self.journal
            .job
            .coding
            .as_ref()
            .is_some_and(|p| is_analysis(&p.schema))
    }
    /// One Host-served snapshot call: durable intent, the read, a create-only
    /// retained result, the gate's delivery record, then the reply.
    pub(super) fn coding_snapshot_frame(&mut self, frame: &Value) -> Result<Vec<u8>> {
        if !self.analysis_profile() {
            return refuse("snapshot request outside the analysis profile");
        }
        if self.expired() || self.cancelled() {
            return refuse("snapshot admission expired/cancelled");
        }
        let id = frame["id"]
            .as_str()
            .filter(|s| !s.is_empty() && s.len() < 128)
            .ok_or_else(|| Error::Refused("snapshot id absent".into()))?
            .to_string();
        let tool = frame["tool"]
            .as_str()
            .filter(|t| TOOLS.contains(t))
            .ok_or_else(|| Error::Refused("unknown snapshot tool".into()))?
            .to_string();
        let arguments = frame["arguments"].clone();
        if !arguments.is_object() || arguments.to_string().len() > MAX_ARGUMENT_BYTES {
            return refuse("snapshot arguments bound");
        }
        let limit = self.journal.job.coding.as_ref().unwrap().max_tool_calls as usize;
        let state = self.journal.coding.as_mut().unwrap();
        if state.snapshot_requests.contains_key(&id)
            || state.sdk_refused_ids.contains(&id)
            || state.snapshot_requests.len() >= limit
        {
            return refuse("duplicate or excess snapshot request");
        }
        state.snapshot_requests.insert(id.clone(), None);
        state.snapshot_tools.insert(id.clone(), tool.clone());
        self.save()?; // intent precedes the read; None stays unknown
        let mut result = match serve(&self.journal.job.workspace, &tool, &arguments) {
            Ok(v) => json!({"type":"snapshot_result","id":id,"ok":true,"result":v}),
            Err(e) => json!({"type":"snapshot_result","id":id,"ok":false,"error":e}),
        };
        let mut bytes = serde_json::to_vec(&result)?;
        if bytes.len() > MAX_RESULT_BYTES {
            // A page whose encoding outgrows the frame is answered with an
            // error the model can act on (a smaller max_bytes).
            result = json!({"type":"snapshot_result","id":id,"ok":false,
                "error":"the encoded page exceeds the result bound; request fewer bytes"});
            bytes = serde_json::to_vec(&result)?;
        }
        let path = self
            .journal
            .job
            .state_dir
            .join(format!("coding-snapshot-{}.json", digest(id.as_bytes())));
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        let artifact = Artifact {
            sha256: digest(&bytes),
            path,
        };
        self.journal
            .coding
            .as_mut()
            .unwrap()
            .snapshot_requests
            .insert(id.clone(), Some(artifact.clone()));
        self.save()?;
        self.coding_gate
            .as_mut()
            .ok_or_else(|| Error::Refused("coding lifecycle gate missing".into()))?
            .snapshot_delivered(&id, &tool, &arguments, &result, &artifact)?;
        Ok(bytes)
    }
    /// A native completion of an analysis tool call: either exactly the Host
    /// delivery, or an SDK refusal that never reached the Host.
    pub(super) fn analysis_native_end(&mut self, event: &Value) -> Result<()> {
        let id = event["toolCallId"]
            .as_str()
            .ok_or_else(|| Error::Refused("native snapshot id absent".into()))?;
        let state = self.journal.coding.as_mut().unwrap();
        let Some(slot) = state.snapshot_requests.get(id) else {
            if event["isError"] != true || !state.sdk_refused_ids.insert(id.into()) {
                return refuse("native analysis completion without a Host delivery");
            }
            return Ok(());
        };
        let delivered: Option<Value> = slot
            .as_ref()
            .and_then(|a| a.verify().ok())
            .and_then(|b| serde_json::from_slice(&b).ok());
        let Some(delivered) = delivered else {
            return refuse("native snapshot completion mismatch");
        };
        let ok = delivered["ok"] == true;
        let expected = if ok {
            delivered["result"].clone()
        } else {
            json!({"error":delivered["error"]})
        };
        let native_text = event["result"]["content"][0]["text"]
            .as_str()
            .and_then(|t| serde_json::from_str::<Value>(t).ok());
        if event["isError"] != false
            || state.snapshot_tools.get(id).map(String::as_str) != event["toolName"].as_str()
            || event["result"]["content"].as_array().map(Vec::len) != Some(1)
            || native_text.as_ref() != Some(&expected)
            || event["result"]["details"]["hostReportedSuccess"] != json!(ok)
            || !state.native_snapshot_ids.insert(id.into())
        {
            return refuse("native snapshot completion mismatch");
        }
        Ok(())
    }
    /// The counts an analysis completion must report. On `claude-sdk` the
    /// receiver also reports requests its forwarder answered locally (outside
    /// a grant, a second request per grant, another path, or an admitted
    /// request whose Host relay failed before a head): none may occur in a
    /// completed job, since each is a request Claude Code made that no
    /// completed Host attempt accounts for.
    fn analysis_counts(&self) -> Value {
        let state = self.journal.coding.as_ref().unwrap();
        let claude = self
            .journal
            .job
            .coding
            .as_ref()
            .and_then(|p| p.provider.as_ref())
            .is_some_and(|p| p.provider == "claude-sdk");
        if claude {
            json!({"providerRequests":state.provider_requests.len(),"resets":0,"forwarderRefused":0})
        } else {
            json!({"providerRequests":state.provider_requests.len(),"resets":0})
        }
    }
    /// Completion of an analysis job: every Host delivery resolved and matched
    /// natively, every provider request settled, counts as reported.
    pub(super) fn analysis_complete(&self, frame: &Value) -> Result<()> {
        let state = self.journal.coding.as_ref().unwrap();
        if state.completed
            || state.snapshot_requests.values().any(Option::is_none)
            || state.native_snapshot_ids.len() != state.snapshot_requests.len()
            || !state.tool_requests.is_empty()
            || state.provider_failed
            || state.provider_requests.is_empty()
            || state
                .provider_requests
                .values()
                .any(|r| r.settled_microusd.is_none())
            || frame["counts"] != self.analysis_counts()
        {
            return refuse("analysis completion lacks settled requests or native tool evidence");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn snapshot(files: &[(&str, &[u8])]) -> (tempfile::TempDir, Tree) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("snap");
        for (path, bytes) in files {
            let p = root.join(path);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, bytes).unwrap();
        }
        let manifest = serde_json::to_vec(&inventory(&root).unwrap()).unwrap();
        let mpath = dir.path().join("manifest.json");
        fs::write(&mpath, &manifest).unwrap();
        let tree = Tree {
            root,
            manifest: Artifact {
                path: mpath,
                sha256: digest(&manifest),
            },
        };
        (dir, tree)
    }
    #[test]
    fn snapshot_tools_serve_only_manifest_members_in_bounded_pages() {
        let long = "é".repeat(PAGE_BYTES); // two bytes per character
        let (_dir, tree) = snapshot(&[("a.md", b"hello"), ("d/long.md", long.as_bytes())]);
        let list = serve(&tree, "snapshot_list", &json!({})).unwrap();
        assert_eq!(list["members"].as_array().unwrap().len(), 2);
        assert_eq!(list["members"][0]["path"], "a.md");
        assert_eq!(list["members"][0]["bytes"], 5);
        let page = serve(&tree, "snapshot_read", &json!({"path":"a.md"})).unwrap();
        assert_eq!(page["text"], "hello");
        assert!(page["next_offset"].is_null());
        // A page ends on a character boundary and names where the next starts.
        let first = serve(
            &tree,
            "snapshot_read",
            &json!({"path":"d/long.md","max_bytes":5}),
        )
        .unwrap();
        assert_eq!(first["bytes"], 4);
        assert_eq!(first["next_offset"], 4);
        let rest = serve(
            &tree,
            "snapshot_read",
            &json!({"path":"d/long.md","offset":PAGE_BYTES}),
        )
        .unwrap();
        assert_eq!(rest["bytes"], PAGE_BYTES);
        assert!(rest["next_offset"].is_null());
        for (tool, args) in [
            ("snapshot_read", json!({"path":"../etc/passwd"})),
            ("snapshot_read", json!({"path":"a.md","offset":1,"extra":1})),
            ("snapshot_read", json!({"path":"d/long.md","offset":1})),
            (
                "snapshot_read",
                json!({"path":"a.md","max_bytes":PAGE_BYTES + 1}),
            ),
            ("snapshot_read", json!({"path":"a.md","offset":6})),
            ("snapshot_list", json!({"path":"a.md"})),
            ("workspace_exec", json!({"command":"id"})),
        ] {
            assert!(serve(&tree, tool, &args).is_err(), "{tool} {args}");
        }
        // A member that changed after pinning is never served.
        fs::write(tree.root.join("a.md"), b"HELLO").unwrap();
        assert!(serve(&tree, "snapshot_read", &json!({"path":"a.md"})).is_err());
    }
    #[test]
    fn request_tools_must_be_exactly_the_pinned_function_tools() {
        let tool = |name: &str| json!({"type":"function","name":name,"parameters":{}});
        let err = |v: Value| match check_request_tools(&v) {
            Err(Error::Refused(m)) => m,
            other => panic!("{other:?}"),
        };
        assert!(
            err(json!([tool("snapshot_list"), tool("snapshot_read")])).contains("pinned digest")
        );
        assert!(err(json!([tool("snapshot_list")])).contains("tool set"));
        assert!(
            err(json!([
                tool("snapshot_list"),
                tool("snapshot_read"),
                tool("workspace_exec")
            ]))
            .contains("tool set")
        );
        let mut hosted = json!([tool("snapshot_list"), tool("snapshot_read")]);
        hosted[1]["type"] = json!("web_search");
        assert!(err(hosted).contains("tool set"));
        assert!(err(Value::Null).contains("absent"));
    }
    #[test]
    fn snapshot_admission_bounds() {
        let (_dir, tree) = snapshot(&[("a.md", b"x")]);
        let mut job: Job = serde_json::from_value(json!({"schema":"s","job_id":"j","state_dir":"/tmp",
            "authority":{"path":"/a","sha256":"0"},"packet":{"path":"/p","sha256":"0"},"docker":{"path":"/d","sha256":"0"},
            "image":"i","runtime":{"root":"/r","manifest":{"path":"/m","sha256":"0"}},
            "workspace":{"root":"/w","manifest":{"path":"/m","sha256":"0"}},
            "deadline_unix_seconds":1,"command_timeout_seconds":1,"idle_seconds":0})).unwrap();
        job.workspace = tree.clone();
        validate_snapshot(&job).unwrap();
        let big = vec![b'x'; SNAPSHOT_BYTES as usize + 1];
        let (_dir2, tree2) = snapshot(&[("big.md", &big)]);
        job.workspace = tree2;
        assert!(validate_snapshot(&job).is_err());
    }
    #[test]
    fn claude_sdk_completion_requires_no_forwarder_refusals() {
        let (_dir, mut h) = super::super::provider::ledger_tests::fixture();
        let counts = |h: &Host| h.analysis_counts();
        assert_eq!(counts(&h), json!({"providerRequests":0,"resets":0}));
        h.journal
            .job
            .coding
            .as_mut()
            .unwrap()
            .provider
            .as_mut()
            .unwrap()
            .provider = "claude-sdk".into();
        assert_eq!(
            counts(&h),
            json!({"providerRequests":0,"resets":0,"forwarderRefused":0})
        );
        assert_ne!(
            counts(&h),
            json!({"providerRequests":0,"resets":0,"forwarderRefused":1})
        );
        assert_ne!(counts(&h), json!({"providerRequests":0,"resets":0}));
    }
}
