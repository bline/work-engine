//! Synthetic protocol-unit fixtures for the hosted analysis grammar; never
//! original SDK evidence or authority.
use lifecycle_core::*;
use lifecycle_store::*;
use serde_json::{Value, json};
use tempfile::tempdir;
use work_engine_types::CodecContract;
fn clock() -> ClockSample {
    ClockSample {
        wall: WallTimeMs::new(100),
        wait_budget: WaitBudgetMs::new(100),
    }
}
fn hash(b: &[u8]) -> String {
    CodecContract::BinaryArtifactV1
        .digest_binary(b)
        .unwrap()
        .hex()
}
fn setup(
    root: &std::path::Path,
    profile: &str,
    protocol: &str,
) -> (SqliteLifecycleStore, NativeSessionEntry) {
    let mut s = SqliteLifecycleStore::open(root, "issuer".into()).unwrap();
    s.register_subject(
        SubjectId::parse("s").unwrap(),
        ContextGeneration::parse("c").unwrap(),
        BuildId::parse("b").unwrap(),
        ProofRunId::parse("p").unwrap(),
        clock(),
    )
    .unwrap();
    s.install_trusted_grant(TrustedGrant {
        id: GrantId::parse("g").unwrap(),
        issuer: "issuer".into(),
        principal: "operator".into(),
        subject: SubjectId::parse("s").unwrap(),
        context: ContextGeneration::parse("c").unwrap(),
        build: BuildId::parse("b").unwrap(),
        proof_run: ProofRunId::parse("p").unwrap(),
        scope: GrantScope::NativeTurn,
        expires: AuthorityExpiresAt::new(WallTimeMs::new(1000)),
        revision: Revision::new(1),
        revoked: false,
    })
    .unwrap();
    let mut session = s
        .reserve_native_session(
            NativeSessionId::parse("n").unwrap(),
            SubjectId::parse("s").unwrap(),
            ContextGeneration::parse("c").unwrap(),
            RuntimeIncarnation::parse("i").unwrap(),
            profile,
            PI_SDK_INDEX_SHA256,
            protocol,
            SnapshotId::parse("snap").unwrap(),
            &"a".repeat(64),
            false,
            clock(),
        )
        .unwrap();
    s.bind_native_thread(
        &mut session,
        ProviderThreadId::parse("physical:pi").unwrap(),
    )
    .unwrap();
    (s, session)
}
fn record(
    s: &mut SqliteLifecycleStore,
    session: &NativeSessionEntry,
    inv: &str,
    values: &[Value],
) -> (NativeTurnEntry, PiSettlementProof) {
    let mut turn = s
        .claim_native_turn(
            session,
            NativeTurnRequest {
                invocation_id: inv.into(),
                attempt: AttemptId::parse(format!("attempt:{inv}")).unwrap(),
                grant: GrantId::parse("g").unwrap(),
                grant_revision: 1,
                purpose: NativeInvocationPurpose::DomainWork,
                prompt: inv.as_bytes().to_vec(),
            },
            "operator",
            clock(),
        )
        .unwrap();
    s.mark_native_send_intent(&turn).unwrap();
    s.bind_native_turn(
        &mut turn,
        ProviderTurnId::parse(format!("turn:{inv}")).unwrap(),
    )
    .unwrap();
    for (i, v) in values.iter().enumerate() {
        s.record_native_ingress(
            session.id(),
            "wire",
            1 + i as u64,
            &serde_json::to_vec(v).unwrap(),
            "qualified",
            v["type"].as_str(),
            Some("physical:pi"),
            Some(&format!("turn:{inv}")),
            None,
            clock(),
        )
        .unwrap();
    }
    (
        turn,
        PiSettlementProof {
            transport: "wire".into(),
            first_sequence: 1,
            history_sequence: values.len() as u64,
        },
    )
}
fn delivery(inv: &str, id: &str, tool: &str, arguments: Value, result: Value) -> (Value, Value) {
    let delivered = json!({"type":"snapshot_result","id":id,"ok":true,"result":result});
    let bytes = serde_json::to_string(&delivered).unwrap();
    let read = json!({"type":"pi.analysis_read","invocationId":inv,"call_id":id,"tool":tool,"arguments":arguments,
        "result":delivered,"artifact":{"path":format!("/state/{id}.json"),"sha256":hash(bytes.as_bytes())},"artifact_bytes":bytes});
    let content = json!([{"type":"text","text":serde_json::to_string(&result).unwrap()}]);
    let tool_message = json!({"role":"toolResult","toolCallId":id,"toolName":tool,"content":content,"details":{"hostReportedSuccess":true},"isError":false});
    (read, tool_message)
}
fn request(inv: &str, input: &[Value]) -> Value {
    json!({"type":"pi.provider_request","producer":"pi-1.5.0","sessionId":"physical:pi","invocationId":inv,
        "turnCorrelation":format!("turn:{inv}"),"prompt_sha256":hash(inv.as_bytes()),"messages":input,"agentMessages":input})
}
/// Three provider requests: a list call, then a read and an SDK-refused call
/// in one message, then the final answer.
fn events(inv: &str) -> Vec<Value> {
    let user = json!({"role":"user","content":[{"type":"text","text":inv}]});
    let call1 = json!({"type":"toolCall","id":"c1","name":"snapshot_list","arguments":{}});
    let a1 = json!({"role":"assistant","content":[call1],"stopReason":"toolUse"});
    let (r1, t1) = delivery(
        inv,
        "c1",
        "snapshot_list",
        json!({}),
        json!({"members":[{"path":"a.md","bytes":5}]}),
    );
    let call2 =
        json!({"type":"toolCall","id":"c2","name":"snapshot_read","arguments":{"path":"a.md"}});
    let call3 = json!({"type":"toolCall","id":"c3","name":"bogus","arguments":{}});
    let a2 = json!({"role":"assistant","content":[{"type":"text","text":"reading"},call2,call3],"stopReason":"toolUse"});
    let (r2, t2) = delivery(
        inv,
        "c2",
        "snapshot_read",
        json!({"path":"a.md"}),
        json!({"path":"a.md","text":"hello"}),
    );
    let t3 = json!({"role":"toolResult","toolCallId":"c3","toolName":"bogus","content":[{"type":"text","text":"Tool bogus not found"}],"details":{},"isError":true});
    let a3 = json!({"role":"assistant","content":[{"type":"text","text":"supported: hello"}],"stopReason":"stop"});
    let full = vec![
        user.clone(),
        a1.clone(),
        t1.clone(),
        a2.clone(),
        t2.clone(),
        t3.clone(),
        a3.clone(),
    ];
    let mut branch = vec![];
    let mut parent = Value::Null;
    for (i, m) in full.iter().enumerate() {
        branch.push(json!({"type":"message","id":format!("m{i}"),"parentId":parent,"message":m}));
        parent = json!(format!("m{i}"));
    }
    let end = |id: &str, name: &str, t: &Value| json!({"type":"tool_execution_end","toolCallId":id,"toolName":name,"result":{"content":t["content"],"details":t["details"]},"isError":t["isError"]});
    vec![
        json!({"type":"agent_start"}),
        json!({"type":"turn_start"}),
        json!({"type":"message_start","message":user}),
        json!({"type":"message_end","message":user}),
        request(inv, &full[..1]),
        json!({"type":"message_start","message":{"role":"assistant"}}),
        json!({"type":"message_end","message":a1}),
        json!({"type":"tool_execution_start","toolCallId":"c1","toolName":"snapshot_list","args":{}}),
        r1,
        end("c1", "snapshot_list", &t1),
        json!({"type":"message_start","message":t1}),
        json!({"type":"message_end","message":t1}),
        json!({"type":"turn_end","message":a1,"toolResults":[t1]}),
        json!({"type":"turn_start"}),
        request(inv, &full[..3]),
        json!({"type":"message_start","message":{"role":"assistant"}}),
        json!({"type":"message_end","message":a2}),
        json!({"type":"tool_execution_start","toolCallId":"c2","toolName":"snapshot_read","args":{"path":"a.md"}}),
        r2,
        end("c2", "snapshot_read", &t2),
        json!({"type":"message_start","message":t2}),
        json!({"type":"message_end","message":t2}),
        json!({"type":"tool_execution_start","toolCallId":"c3","toolName":"bogus","args":{}}),
        end("c3", "bogus", &t3),
        json!({"type":"message_start","message":t3}),
        json!({"type":"message_end","message":t3}),
        json!({"type":"turn_end","message":a2,"toolResults":[t2,t3]}),
        json!({"type":"turn_start"}),
        request(inv, &full[..6]),
        json!({"type":"message_start","message":{"role":"assistant"}}),
        json!({"type":"message_end","message":a3}),
        json!({"type":"turn_end","message":a3,"toolResults":[]}),
        json!({"type":"agent_end","messages":full,"willRetry":false}),
        json!({"type":"agent_settled","aborted":false}),
        json!({"type":"pi.session_history","producer":"pi-1.5.0","sessionId":"physical:pi","header":{"type":"session","id":"physical:pi"},"leafId":"m6","messages":full,"branch":branch}),
    ]
}
fn settle(
    values: &[Value],
) -> (
    Result<(), StoreError>,
    SqliteLifecycleStore,
    tempfile::TempDir,
) {
    let root = tempdir().unwrap();
    let (mut s, session) = setup(
        root.path(),
        &pi_analysis_profile_digest(),
        &pi_analysis_protocol_digest(),
    );
    let (turn, proof) = record(&mut s, &session, "audit", values);
    let r = s.settle_pi_coding(&turn, &proof, clock());
    (r, s, root)
}
#[test]
fn analysis_turn_settles_with_host_deliveries_and_sdk_refusals() {
    let (r, s, _root) = settle(&events("audit"));
    r.unwrap();
    let recovery = s.native_recovery("audit").unwrap().unwrap();
    assert_eq!(recovery.class, "CompletedSettled");
}
#[test]
fn analysis_grammar_refuses_each_unbound_or_altered_tool_result() {
    let base = events("audit");
    let pos = |kind: &str, id: &str| {
        base.iter()
            .position(|v| v["type"] == kind && (v["call_id"] == id || v["toolCallId"] == id))
            .unwrap()
    };
    let mut cases: Vec<(&str, Vec<Value>)> = vec![];
    // The SDK result differs from the Host delivery.
    let mut v = base.clone();
    let i = pos("tool_execution_end", "c2");
    v[i]["result"]["content"][0]["text"] = json!("{\"path\":\"a.md\",\"text\":\"forged\"}");
    cases.push(("result differs from delivery", v));
    // Executed call without a Host delivery.
    let mut v = base.clone();
    v.remove(pos("pi.analysis_read", "c2"));
    cases.push(("missing delivery", v));
    // The same delivery twice.
    let mut v = base.clone();
    let i = pos("pi.analysis_read", "c2");
    v.insert(i, base[i].clone());
    cases.push(("duplicate delivery", v));
    // Delivery recorded after the call ended.
    let mut v = base.clone();
    let i = pos("pi.analysis_read", "c2");
    let r = v.remove(i);
    v.insert(i + 1, r);
    cases.push(("delivery after end", v));
    // An SDK refusal that reached the Host.
    let mut v = base.clone();
    let i = pos("tool_execution_end", "c3");
    let mut forged = base[pos("pi.analysis_read", "c2")].clone();
    forged["call_id"] = json!("c3");
    v.insert(i, forged);
    cases.push(("refusal with delivery", v));
    // Delivery bytes that do not match their digest.
    let mut v = base.clone();
    let i = pos("pi.analysis_read", "c1");
    v[i]["artifact_bytes"] = json!(
        v[i]["artifact_bytes"]
            .as_str()
            .unwrap()
            .replace("a.md", "b.md")
    );
    cases.push(("tampered delivery bytes", v));
    // A delivery under another tool name.
    let mut v = base.clone();
    let i = pos("pi.analysis_read", "c1");
    v[i]["tool"] = json!("snapshot_read");
    cases.push(("delivery tool mismatch", v));
    // A coding effect inside an analysis turn.
    let mut v = base.clone();
    v.insert(
        8,
        json!({"type":"pi.coding_tool_intent","invocationId":"audit","intent":{}}),
    );
    cases.push(("coding intent", v));
    // The final answer is not a clean stop.
    let mut v = base.clone();
    let n = v.len();
    for j in [n - 5, n - 4, n - 3] {
        if v[j]["message"]["role"] == "assistant" {
            v[j]["message"]["stopReason"] = json!("length");
        }
    }
    cases.push(("final not stop", v));
    // The request input omits a tool result.
    let mut v = base.clone();
    let i = base
        .iter()
        .rposition(|v| v["type"] == "pi.provider_request")
        .unwrap();
    v[i]["agentMessages"].as_array_mut().unwrap().pop();
    cases.push(("request input altered", v));
    for (name, values) in cases {
        let (r, s, _root) = settle(&values);
        assert!(r.is_err(), "{name}");
        assert_eq!(
            s.native_recovery("audit").unwrap().unwrap().class,
            "EnteredUncertain",
            "{name}"
        );
    }
}
#[test]
fn analysis_turns_need_the_analysis_binding_and_have_no_effect_entry() {
    // The coding live grammar refuses an analysis turn.
    let root = tempdir().unwrap();
    let (mut s, session) = setup(
        root.path(),
        &pi_coding_live_profile_digest(),
        &pi_coding_live_protocol_digest(),
    );
    let (turn, proof) = record(&mut s, &session, "audit", &events("audit"));
    assert!(s.settle_pi_coding(&turn, &proof, clock()).is_err());
    // Mixed analysis bindings are not a Pi coding kind.
    let root = tempdir().unwrap();
    let (mut s, session) = setup(
        root.path(),
        &pi_analysis_profile_digest(),
        &pi_coding_live_protocol_digest(),
    );
    let (turn, proof) = record(&mut s, &session, "audit", &events("audit"));
    assert!(s.settle_pi_coding(&turn, &proof, clock()).is_err());
    // No workspace effect can be entered under the analysis binding.
    let root = tempdir().unwrap();
    let (mut s, session) = setup(
        root.path(),
        &pi_analysis_profile_digest(),
        &pi_analysis_protocol_digest(),
    );
    let (turn, _) = record(&mut s, &session, "audit", &events("audit")[..4]);
    let intent = json!({"job_id":"j","input_digest":"a".repeat(64),"session_id":"physical:pi","call_id":"c1","command_sha256":"b".repeat(64)});
    let entered = s.with_pi_coding_effect(&turn, "wire", 1, &intent, clock(), |_| Ok::<(), ()>(()));
    assert!(entered.is_err());
}
