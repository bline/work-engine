use super::*;
use std::os::unix::fs::PermissionsExt;

const CRED: &str = "job-credential-0123456789abcdefghijklmnop";
const SESSION_GRANT: &str = "session-grant-0123456789abcdefghijklmnopq";
const REQ: &str = "11111111-1111-4111-8111-111111111111";
const REQ2: &str = "22222222-2222-4222-8222-222222222222";

struct Fx {
    _t: tempfile::TempDir,
    root: PathBuf,
    state: PathBuf,
    jobs: Arc<Jobs>,
    auth: Authorization,
}

/// Fake Host: same CLI shape as `work-engine-manual-host pi-docker`, behaviour selected by flag files.
fn fake_host(state: &Path, runs: &Path) -> String {
    format!(
        r#"#!/bin/sh
S='{s}'; R='{r}'
[ "$1" = pi-docker ] || exit 64
act="$2"
T="$(dirname "$S")"
journal() {{ sed -e "s/@P@/$1/" -e "s/@R@/$2/" "$T/journal.tpl" > "$S/pi-state.json"; }}
receipt() {{ cat "$T/receipt.json"; echo; }}
case "$act" in
 run)
  echo run >> "$R"
  [ -e "$S/pi-state.json" ] && {{ echo "existing job: reconcile instead of launch replay" >&2; exit 1; }}
  [ -e "$S/../nostate" ] && exit 1
  journal running false
  [ -e "$S/../crash" ] && exit 0
  if [ -e "$S/../hang" ]; then sleep 60; fi
  [ -e "$S/../slow" ] && sleep 2
  journal collected true; receipt;;
 inspect) [ -e "$S/pi-state.json" ] || exit 1; [ -e "$S/../busy" ] && exit 1; cat "$S/pi-state.json";;
 cancel) echo c >> "$S/../cancel-marker"; printf '{{"status":"cancellation_requested","settlement":"not_claimed"}}';;
 reconcile) echo rec >> "$S/../reconciles"; journal collected true; receipt;;
 *) exit 64;;
esac
"#,
        s = state.display(),
        r = runs.display()
    )
}

const RECEIPT_ID: &str = "5adaf203ef0b4049b9965502bbbaa0a0739733fc62985b836ca2441a67bb9828";

/// The Host's typed `Job` serialization written by hand in its struct field order and compact form:
/// its SHA-256 is the Host input digest, independent of the adapter's mirror under test.
fn host_job(
    state: &Path,
    packet: &Path,
    packet_sha: &str,
    deadline: u64,
    extra_coding: &str,
) -> String {
    let q = |p: &Path| serde_json::to_string(&p.to_string_lossy()).unwrap();
    let art = |path: &str, sha: &str| format!(r#"{{"path":"{path}","sha256":"{sha}"}}"#);
    let tree = |root: &str| {
        format!(
            r#"{{"root":"{root}","manifest":{}}}"#,
            art("/m/manifest.json", &"c".repeat(64))
        )
    };
    format!(
        r#"{{"schema":"work-engine.pi-docker-custody.v1","job_id":"job-1","state_dir":{state},"authority":{authority},"packet":{{"path":{packet},"sha256":"{packet_sha}"}},"docker":{docker},"image":"node@sha256:4b150b48b4116e2a22f2c39cc27025fbf4f153e2078fd3b14d159fdb5c7279da","runtime":{runtime},"workspace":{workspace},"deadline_unix_seconds":{deadline},"command_timeout_seconds":30,"idle_seconds":0,"coding":{{"schema":"work-engine.pi-coding-fixture.v1","shim":{shim},"toolchain":{toolchain},"acceptance":{acceptance},"fixture_command":"true","max_tool_calls":1{extra_coding}}}}}"#,
        state = q(state),
        packet = q(packet),
        authority = art("/a/authority.json", &"a".repeat(64)),
        docker = art("/usr/bin/docker", &"d".repeat(64)),
        runtime = tree("/r/runtime"),
        workspace = tree("/w/workspace"),
        shim = art("/s/shim.mjs", &"e".repeat(64)),
        toolchain = tree("/t/toolchain"),
        acceptance = art("/s/acceptance.rs", &"f".repeat(64)),
    )
}

fn fixture(deadline: u64, provider: Option<&str>) -> Fx {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().to_path_buf();
    let state = root.join("state");
    for d in [&state, &root.join("ops")] {
        std::fs::create_dir(d).unwrap();
        std::fs::set_permissions(d, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let runs = root.join("runs");
    let host = root.join("host.sh");
    std::fs::write(&host, fake_host(&state, &runs)).unwrap();
    std::fs::set_permissions(&host, std::fs::Permissions::from_mode(0o700)).unwrap();
    let packet = root.join("packet.json");
    std::fs::write(&packet, b"{\"task\":\"fixture\"}").unwrap();
    let job_file = root.join("job.json");
    let extra = provider.map_or(String::new(), |p| format!(r#","provider":"{p}""#));
    let canonical = host_job(
        &state,
        &packet,
        &hex(b"{\"task\":\"fixture\"}"),
        now() + 120,
        &extra,
    );
    let host_digest = hex(canonical.as_bytes());
    // The operator's job file is pretty (and key-sorted), so its digest is not the Host digest.
    let job: Value = serde_json::from_str(&canonical).unwrap();
    let job_text = serde_json::to_string_pretty(&job).unwrap();
    std::fs::write(&job_file, &job_text).unwrap();
    // Templates for the fake Host: journal and the actual `{receipt_id, receipt}` wrapper.
    let journal = format!(
        r#"{{"schema":"work-engine.pi-docker-custody.v1","binary_sha256":"{b}","job":{canonical},"input_digest":"{host_digest}","container_name":"we-pi-x","phase":"@P@","removed":@R@,"receipt_id":"{RECEIPT_ID}"}}"#,
        b = "b".repeat(64)
    );
    std::fs::write(root.join("journal.tpl"), journal).unwrap();
    let wrapper = format!(
        r#"{{"receipt_id":"{RECEIPT_ID}","receipt":{{"schema":"work-engine.pi-docker-custody.v1","job":{canonical},"job_id":"job-1","input_digest":"{host_digest}","status":"component_passed","acceptance":"owner_pending","domain_effect_settlement":"not_claimed","provider":"disabled","phase":"collected","rpc_verified":true,"daemon_exit":0,"removed":true,"failures":[],"coding":{{"completed":true,"tool_removed":true}}}}}}"#
    );
    std::fs::write(root.join("receipt.json"), wrapper).unwrap();
    let auth = Authorization {
        authorization_id: "auth-1".into(),
        caller: "caller-1".into(),
        credential_sha256: hex(CRED.as_bytes()),
        job_file: job_file.to_string_lossy().into(),
        job_sha256: hex(job_text.as_bytes()),
        host_input_digest: host_digest,
        packet_sha256: hex(b"{\"task\":\"fixture\"}"),
        job_id: "job-1".into(),
        state_dir: state.to_string_lossy().into(),
    };
    let jobs = jobs_for(&root, &host, &auth, deadline);
    Fx {
        _t: t,
        root,
        state,
        jobs,
        auth,
    }
}
fn jobs_for(root: &Path, host: &Path, auth: &Authorization, deadline: u64) -> Arc<Jobs> {
    let profile = JobProfile {
        version: 1,
        kind: PROFILE_KIND.into(),
        capability: CAPABILITY_OFFLINE.into(),
        class: CLASS_OFFLINE.into(),
        host_binary: host.to_string_lossy().into(),
        host_binary_sha256: hex(&std::fs::read(host).unwrap()),
        operation_root: root.join("ops").to_string_lossy().into(),
        env: BTreeMap::new(),
        collector_deadline_seconds: deadline,
        cancel_grace_seconds: 1,
        authorizations: vec![auth.clone()],
    };
    Arc::new(Jobs::new(profile).unwrap())
}
fn submit_args(fx: &Fx, request: &str) -> Value {
    json!({"request_id":request,"authorization_id":"auth-1","job_credential":CRED,
        "job_sha256":fx.auth.job_sha256,"packet_sha256":fx.auth.packet_sha256})
}
fn read_args(request: &str) -> Value {
    json!({"request_id":request,"authorization_id":"auth-1","job_credential":CRED})
}
fn runs(fx: &Fx) -> usize {
    std::fs::read_to_string(fx.root.join("runs")).map_or(0, |s| s.lines().count())
}
fn wait_state(fx: &Fx, want: &str) -> Value {
    for _ in 0..400 {
        let v = fx
            .jobs
            .call("read_coding_job_operation", read_args(REQ))
            .unwrap();
        if v["state"] == want {
            return v;
        }
        thread::sleep(Duration::from_millis(25));
    }
    panic!("never reached {want}");
}

#[test]
fn authority_is_separate_from_session_grants_and_bearer() {
    let fx = fixture(30, None);
    for credential in [SESSION_GRANT, "short", ""] {
        let mut a = submit_args(&fx, REQ);
        a["job_credential"] = json!(credential);
        assert_eq!(fx.jobs.call("submit_coding_job", a), Err("unauthorized"));
    }
    let mut a = submit_args(&fx, REQ);
    a["authorization_id"] = json!("other");
    assert_eq!(fx.jobs.call("submit_coding_job", a), Err("unauthorized"));
    for forbidden in [
        ("job_file", json!("/tmp/x")),
        ("host_binary", json!("/bin/sh")),
        ("state_dir", json!("/tmp")),
    ] {
        let mut a = submit_args(&fx, REQ);
        a[forbidden.0] = forbidden.1;
        assert_eq!(
            fx.jobs.call("submit_coding_job", a),
            Err("invalid_arguments")
        );
    }
    assert_eq!(runs(&fx), 0);
    assert!(
        std::fs::read_dir(fx.root.join("ops"))
            .unwrap()
            .next()
            .is_none()
    );
}

#[test]
fn altered_job_packet_hash_and_capability_are_rejected_before_entry() {
    let fx = fixture(30, None);
    let mut a = submit_args(&fx, REQ);
    a["job_sha256"] = json!(hex(b"other"));
    assert_eq!(fx.jobs.call("submit_coding_job", a), Err("job_mismatch"));
    let mut a = submit_args(&fx, REQ);
    a["packet_sha256"] = json!(hex(b"other"));
    assert_eq!(fx.jobs.call("submit_coding_job", a), Err("job_mismatch"));
    // Altered job bytes on disk no longer match the operator pin.
    std::fs::write(&fx.auth.job_file, b"{}").unwrap();
    assert_eq!(
        fx.jobs.call("submit_coding_job", submit_args(&fx, REQ)),
        Err("job_mismatch")
    );
    // Altered packet bytes.
    let fx = fixture(30, None);
    std::fs::write(fx.root.join("packet.json"), b"changed").unwrap();
    assert_eq!(
        fx.jobs.call("submit_coding_job", submit_args(&fx, REQ)),
        Err("job_mismatch")
    );
    // A stale prepared job is refused before binding, leaving the authorization usable.
    let fx = fixture(30, None);
    let mut stale: Value =
        serde_json::from_slice(&std::fs::read(&fx.auth.job_file).unwrap()).unwrap();
    stale["deadline_unix_seconds"] = json!(now() - 10);
    let stale = stale.to_string();
    std::fs::write(&fx.auth.job_file, &stale).unwrap();
    let mut auth = fx.auth.clone();
    auth.job_sha256 = hex(stale.as_bytes());
    let host = fx.root.join("host.sh");
    let jobs = jobs_for(&fx.root, &host, &auth, 30);
    let mut a = submit_args(&fx, REQ);
    a["job_sha256"] = json!(auth.job_sha256);
    assert_eq!(
        jobs.call("submit_coding_job", a),
        Err("job_deadline_window")
    );
    assert!(
        std::fs::read_dir(fx.root.join("ops"))
            .unwrap()
            .next()
            .is_none()
    );
    // Provider-bearing job is wrong for the offline capability.
    let fx = fixture(30, Some("native-auth"));
    assert_eq!(
        fx.jobs.call("submit_coding_job", submit_args(&fx, REQ)),
        Err("capability_mismatch")
    );
    // A stale state directory is not bound.
    let fx = fixture(30, None);
    std::fs::write(fx.state.join("pi-state.json"), b"{}").unwrap();
    assert_eq!(
        fx.jobs.call("submit_coding_job", submit_args(&fx, REQ)),
        Err("state_not_fresh")
    );
    assert_eq!(runs(&fx), 0);
    assert!(
        std::fs::read_dir(fx.root.join("ops"))
            .unwrap()
            .next()
            .is_none()
    );
    // The wrong capability string never loads.
    let mut p = fx.jobs.profile.clone();
    p.capability = "offline-synthetic-v1".into();
    assert!(Jobs::new(p).is_err());
}

#[test]
fn submit_returns_promptly_and_repeats_observe_the_same_operation() {
    let fx = fixture(30, None);
    std::fs::write(fx.root.join("slow"), b"").unwrap();
    let t = Instant::now();
    let first = fx
        .jobs
        .call("submit_coding_job", submit_args(&fx, REQ))
        .unwrap();
    assert!(
        t.elapsed() < Duration::from_millis(1500),
        "submit must not wait for the job"
    );
    assert_eq!(first["entry"], "collector_started");
    assert_eq!(first["state"], "collecting");
    // Exact repeat while running, different request id, and a read all leave one admission.
    let repeat = fx
        .jobs
        .call("submit_coding_job", submit_args(&fx, REQ))
        .unwrap();
    assert_eq!(repeat["operation_id"], first["operation_id"]);
    assert_eq!(repeat["entry"], "existing_operation");
    assert_eq!(
        fx.jobs.call("submit_coding_job", submit_args(&fx, REQ2)),
        Err("authorization_consumed")
    );
    assert_eq!(
        fx.jobs.call("read_coding_job_operation", read_args(REQ2)),
        Err("identity_conflict")
    );
    let done = wait_state(&fx, "collected");
    assert_eq!(runs(&fx), 1);
    // The Host ran the verified snapshot inside the operation directory, not the operator path.
    let snapshot = fx
        .root
        .join("ops")
        .join(fx.jobs.operation_id(&fx.auth))
        .join("job.json");
    assert_eq!(hex(&std::fs::read(snapshot).unwrap()), fx.auth.job_sha256);
    assert_eq!(done["host_input_digest"], fx.auth.host_input_digest);
    assert_ne!(fx.auth.host_input_digest, fx.auth.job_sha256);
    assert_eq!(
        done["process_cleanup"]["run"],
        "owned_process_group_collected"
    );
    let result = fx
        .jobs
        .call("read_coding_job_result", read_args(REQ))
        .unwrap();
    assert_eq!(result["result"], "available");
    assert_eq!(result["receipt_id"], RECEIPT_ID);
    assert_eq!(result["operation"]["host_receipt_id"], RECEIPT_ID);
    assert!(
        result["receipt"].get("receipt_id").is_none(),
        "the ID is the wrapper's, not part of the receipt body"
    );
    assert_eq!(result["acceptance"], "owner_pending");
    assert_eq!(result["domain_effect_settlement"], "not_claimed");
    assert_eq!(result["terminal"]["status"], "component_passed");
    assert_eq!(result["validation"]["rpc_verified"], true);
    assert_eq!(result["cleanup"]["removed"], true);
    assert_eq!(result["host_output"]["action"], "run");
    assert_eq!(result["host_output"]["exit_code"], 0);
    assert_eq!(runs(&fx), 1);
    // Reading a terminal result after completion is still the same operation.
    let again = fx
        .jobs
        .call("submit_coding_job", submit_args(&fx, REQ))
        .unwrap();
    assert_eq!(again["operation_id"], first["operation_id"]);
    assert_eq!(runs(&fx), 1);
}

#[test]
fn restart_gaps_reconcile_or_stay_uncertain_without_replacement() {
    // Collector lost after the Host journaled `running`: a new server instance reconciles.
    let fx = fixture(30, None);
    std::fs::write(fx.root.join("crash"), b"").unwrap();
    fx.jobs
        .call("submit_coding_job", submit_args(&fx, REQ))
        .unwrap();
    for _ in 0..200 {
        if fx.jobs.read_collector_idle() {
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }
    let host = fx.root.join("host.sh");
    let restarted = jobs_for(&fx.root, &host, &fx.auth, 30);
    let repeat = restarted
        .call("submit_coding_job", submit_args(&fx, REQ))
        .unwrap();
    assert_eq!(repeat["entry"], "existing_operation");
    assert_ne!(repeat["state"], "bound_not_entered");
    for _ in 0..200 {
        let v = restarted
            .call("read_coding_job_operation", read_args(REQ))
            .unwrap();
        if v["state"] == "collected" {
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }
    let v = restarted
        .call("read_coding_job_operation", read_args(REQ))
        .unwrap();
    assert_eq!(v["state"], "collected");
    assert_eq!(
        runs(&fx),
        1,
        "recovery reconciled and never re-ran the Host"
    );
    assert!(fx.root.join("reconciles").exists());

    // Entry possible but no Host state: truthfully uncertain, never relaunched.
    let fx = fixture(30, None);
    std::fs::write(fx.root.join("nostate"), b"").unwrap();
    fx.jobs
        .call("submit_coding_job", submit_args(&fx, REQ))
        .unwrap();
    for _ in 0..200 {
        if fx.jobs.read_collector_idle() {
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }
    let restarted = jobs_for(&fx.root, &fx.root.join("host.sh"), &fx.auth, 30);
    for _ in 0..3 {
        let v = restarted
            .call("submit_coding_job", submit_args(&fx, REQ))
            .unwrap();
        assert_eq!(v["state"], "entry_uncertain");
        assert!(
            v["limitations"]
                .to_string()
                .contains("no_replacement_admitted")
        );
    }
    assert_eq!(runs(&fx), 1);
    let r = restarted
        .call("read_coding_job_result", read_args(REQ))
        .unwrap();
    assert_eq!(r["result"], "not_available");

    // Host state present but locked/unreadable (unobserved Host after a crash): reported as such,
    // never reconciled blind, never relaunched.
    let fx = fixture(30, None);
    std::fs::write(fx.root.join("crash"), b"").unwrap();
    fx.jobs
        .call("submit_coding_job", submit_args(&fx, REQ))
        .unwrap();
    for _ in 0..200 {
        if fx.jobs.read_collector_idle() {
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }
    std::fs::write(fx.root.join("busy"), b"").unwrap();
    let restarted = jobs_for(&fx.root, &fx.root.join("host.sh"), &fx.auth, 30);
    for _ in 0..3 {
        let v = restarted
            .call("submit_coding_job", submit_args(&fx, REQ))
            .unwrap();
        assert_eq!(v["state"], "host_state_unreadable");
        assert!(
            v["limitations"]
                .to_string()
                .contains("host_inspection_unavailable")
        );
    }
    assert_eq!(runs(&fx), 1);
    assert!(!fx.root.join("reconciles").exists());
}

#[test]
fn deadline_cancellation_and_shutdown_collect_the_owned_process_group() {
    // Collector deadline: cancel marker requested, then group killed and status collected.
    let fx = fixture(1, None);
    std::fs::write(fx.root.join("hang"), b"").unwrap();
    fx.jobs
        .call("submit_coding_job", submit_args(&fx, REQ))
        .unwrap();
    let mut v = Value::Null;
    for _ in 0..400 {
        v = fx
            .jobs
            .call("read_coding_job_operation", read_args(REQ))
            .unwrap();
        if v["collector_run"].is_object() {
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }
    let run = &v["collector_run"];
    assert_eq!(run["collector_deadline_exceeded"], true);
    assert_eq!(run["cancel_marker_requested"], true);
    assert_eq!(run["killed_after_grace"], true);
    assert_eq!(run["status_collected"], true);
    assert_eq!(run["process_cleanup"], "owned_process_group_collected");
    assert!(fx.root.join("cancel-marker").exists());
    assert_ne!(
        v["state"], "collected",
        "a killed collector is not completion"
    );

    // Explicit cancellation never claims cancellation or closure.
    let fx = fixture(30, None);
    std::fs::write(fx.root.join("hang"), b"").unwrap();
    fx.jobs
        .call("submit_coding_job", submit_args(&fx, REQ))
        .unwrap();
    thread::sleep(Duration::from_millis(200));
    let c = fx.jobs.call("cancel_coding_job", read_args(REQ)).unwrap();
    assert_eq!(c["cancellation"], "requested");
    assert_eq!(c["cancellation_established"], false);
    assert_eq!(c["container_closure_established"], false);
    assert_eq!(c["settlement"], "not_claimed");
    // Server shutdown requests cancellation, kills after grace and collects actual status.
    assert!(fx.jobs.shutdown(Duration::from_secs(10)));
    let run = read_json(
        &fx.root
            .join("ops")
            .join(fx.jobs.operation_id(&fx.auth))
            .join("collector-run.json"),
    )
    .unwrap()
    .unwrap();
    assert_eq!(run["server_stop_requested"], true, "{run}");
    assert_eq!(run["status_collected"], true, "{run}");
    assert_eq!(runs(&fx), 1);
}

#[test]
fn schema_required_keys_match_what_the_server_accepts() {
    let schema: Value =
        serde_json::from_str(include_str!("../../schemas/coding-job-v1.json")).unwrap();
    let required = |name: &str| -> Vec<String> {
        schema["$defs"][name]["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|k| k.as_str().unwrap().to_string())
            .collect()
    };
    let fx = fixture(30, None);
    for (op, name, args) in [
        (
            "submit_coding_job",
            "submit_coding_job",
            submit_args(&fx, REQ),
        ),
        (
            "read_coding_job_operation",
            "read_or_cancel",
            read_args(REQ),
        ),
        ("read_coding_job_result", "read_or_cancel", read_args(REQ)),
        ("cancel_coding_job", "read_or_cancel", read_args(REQ)),
    ] {
        let keys = required(name);
        let mut have: Vec<String> = args.as_object().unwrap().keys().cloned().collect();
        have.sort();
        let mut want = keys.clone();
        want.sort();
        assert_eq!(have, want, "{op}: schema and DTO key sets differ");
        for key in keys {
            let mut missing = args.clone();
            missing.as_object_mut().unwrap().remove(&key);
            assert_eq!(
                fx.jobs.call(op, missing),
                Err("invalid_arguments"),
                "{op} without {key}"
            );
        }
    }
    let mut profile_keys: Vec<String> = required("profile");
    profile_keys.sort();
    assert_eq!(profile_keys.len(), 10);
    assert_eq!(
        schema["$defs"]["profile"]["properties"]["capability"]["const"],
        CAPABILITY_OFFLINE
    );
}

#[path = "../job_output.rs"]
mod job_output;

fn submit_and_collect(fx: &Fx) {
    fx.jobs
        .call("submit_coding_job", submit_args(fx, REQ))
        .unwrap();
    wait_state(fx, "collected");
}

#[test]
fn host_digest_follows_the_host_serialization_not_the_job_file_bytes() {
    // A job written by the real Phase A fixture and the input_digest its actual Host journaled for it.
    let text = include_str!("real-host-job.json");
    let job: Value = serde_json::from_str(text).unwrap();
    assert_eq!(
        host_input_digest(&job).as_deref(),
        Some("d2ad1ee8edb9824c0b3af18ea545636b6db85b027d24f56e4200e7e9f91ffd7f")
    );
    assert_ne!(hex(text.as_bytes()), host_input_digest(&job).unwrap());
    // Whitespace and key order in the file do not change the Host's typed serialization.
    let reordered: Value =
        serde_json::from_str(&serde_json::to_string_pretty(&job).unwrap()).unwrap();
    assert_eq!(host_input_digest(&reordered), host_input_digest(&job));
    // Any job semantic change, other Host version, or non-offline field has another digest or none.
    for mutate in [
        |j: &mut Value| j["deadline_unix_seconds"] = json!(1),
        |j: &mut Value| j["idle_seconds"] = json!(5),
        |j: &mut Value| j["coding"]["fixture_command"] = json!("true"),
    ] {
        let mut j = job.clone();
        mutate(&mut j);
        assert_ne!(host_input_digest(&j), host_input_digest(&job));
    }
    for mutate in [
        |j: &mut Value| j["schema"] = json!("work-engine.pi-docker-custody.v2"),
        |j: &mut Value| j["coding"]["schema"] = json!("work-engine.pi-coding-fixture.v2"),
        |j: &mut Value| j["coding"]["provider"] = json!({}),
        |j: &mut Value| j["receiver"] = json!({}),
        |j: &mut Value| j["unexpected"] = json!(1),
    ] {
        let mut j = job.clone();
        mutate(&mut j);
        assert_eq!(host_input_digest(&j), None);
    }
}

#[test]
fn wrong_expected_host_identity_is_refused_before_binding_or_entry() {
    let fx = fixture(30, None);
    let mut auth = fx.auth.clone();
    auth.host_input_digest = hex(b"another job");
    let jobs = jobs_for(&fx.root, &fx.root.join("host.sh"), &auth, 30);
    assert_eq!(
        jobs.call("submit_coding_job", submit_args(&fx, REQ)),
        Err("host_identity_mismatch")
    );
    assert_eq!(runs(&fx), 0);
    assert!(
        std::fs::read_dir(fx.root.join("ops"))
            .unwrap()
            .next()
            .is_none()
    );
    assert!(!fx.state.join("pi-state.json").exists());
}

#[test]
fn foreign_or_altered_host_state_blocks_read_reconcile_cancel_and_result() {
    for tamper in [
        // Another job's identity.
        |s: &str| s.replace("\"job_id\":\"job-1\"", "\"job_id\":\"job-2\""),
        // A digest that is not the bound one.
        |s: &str| s.replace("\"input_digest\":\"", "\"input_digest\":\"0"),
    ] {
        let fx = fixture(30, None);
        submit_and_collect(&fx);
        let journal = fx.state.join("pi-state.json");
        let altered = tamper(&std::fs::read_to_string(&journal).unwrap());
        std::fs::write(&journal, altered).unwrap();
        for op in [
            "read_coding_job_operation",
            "read_coding_job_result",
            "cancel_coding_job",
        ] {
            assert_eq!(
                fx.jobs.call(op, read_args(REQ)),
                Err("identity_conflict"),
                "{op}"
            );
        }
        assert_eq!(
            fx.jobs.call("submit_coding_job", submit_args(&fx, REQ)),
            Err("identity_conflict")
        );
        assert!(
            !fx.root.join("cancel-marker").exists(),
            "cancel never reached the Host"
        );
        assert!(!fx.root.join("reconciles").exists());
        assert_eq!(runs(&fx), 1);
    }
    // The journal's digest must match its own job, not just the bound value.
    let fx = fixture(30, None);
    submit_and_collect(&fx);
    let journal = fx.state.join("pi-state.json");
    let altered = std::fs::read_to_string(&journal).unwrap().replace(
        "\"command_timeout_seconds\":30",
        "\"command_timeout_seconds\":29",
    );
    std::fs::write(&journal, altered).unwrap();
    assert_eq!(
        fx.jobs.call("read_coding_job_operation", read_args(REQ)),
        Err("identity_conflict")
    );
}

#[test]
fn result_preserves_the_outer_host_receipt_id_and_rejects_conflicts() {
    let fx = fixture(30, None);
    submit_and_collect(&fx);
    let result = fx
        .jobs
        .call("read_coding_job_result", read_args(REQ))
        .unwrap();
    assert_eq!(result["receipt_id"], RECEIPT_ID);
    assert_eq!(result["input_digest"], fx.auth.host_input_digest);
    let path = fx
        .root
        .join("ops")
        .join(fx.jobs.operation_id(&fx.auth))
        .join("host-run.json");
    let original: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let check = |edit: &dyn Fn(&mut Value), want: Result<(), &'static str>| {
        let mut v = original.clone();
        edit(&mut v);
        std::fs::write(&path, v.to_string()).unwrap();
        let got = fx
            .jobs
            .call("read_coding_job_result", read_args(REQ))
            .map(|_| ());
        assert_eq!(got, want);
    };
    // The old fixture shape: an ID only inside the receipt, none in the wrapper.
    check(
        &|v| {
            let id = v.as_object_mut().unwrap().remove("receipt_id").unwrap();
            v["receipt"]["receipt_id"] = id;
        },
        Err("custody_lost"),
    );
    // Wrapper ID that is not the journal's recorded receipt.
    check(
        &|v| v["receipt_id"] = json!("9".repeat(64)),
        Err("identity_conflict"),
    );
    // Malformed ID, extra wrapper member, and receipts for another input digest or job.
    check(&|v| v["receipt_id"] = json!("rcpt-1"), Err("custody_lost"));
    check(&|v| v["extra"] = json!(1), Err("custody_lost"));
    check(
        &|v| v["receipt"]["input_digest"] = json!("1".repeat(64)),
        Err("identity_conflict"),
    );
    check(
        &|v| v["receipt"]["job_id"] = json!("job-2"),
        Err("identity_conflict"),
    );
    check(
        &|v| v["receipt"]["job"]["state_dir"] = json!("/elsewhere"),
        Err("identity_conflict"),
    );
    check(&|_| {}, Ok(()));
}

#[test]
fn overlapping_state_ownership_is_rejected_at_profile_load() {
    let fx = fixture(30, None);
    let profile = |mutate: &dyn Fn(&mut Vec<Authorization>)| {
        let mut p = fx.jobs.profile.clone();
        let mut second = fx.auth.clone();
        second.authorization_id = "auth-2".into();
        second.job_id = "job-2".into();
        second.host_input_digest = hex(b"second");
        second.state_dir = fx.root.join("state-2").to_string_lossy().into();
        p.authorizations.push(second);
        mutate(&mut p.authorizations);
        Jobs::new(p).err()
    };
    // Disjoint authorizations are fine.
    assert_eq!(profile(&|_| {}), None);
    let state = fx.state.to_string_lossy().into_owned();
    assert_eq!(
        profile(&|a| a[1].state_dir = state.clone()),
        Some("overlapping_state_ownership")
    );
    assert_eq!(
        profile(&|a| a[1].state_dir = format!("{state}/nested")),
        Some("overlapping_state_ownership")
    );
    assert_eq!(
        profile(&|a| a[1].state_dir = fx.root.to_string_lossy().into()),
        Some("overlapping_state_ownership")
    );
    assert_eq!(
        profile(&|a| a[1].job_id = "job-1".into()),
        Some("overlapping_state_ownership")
    );
    assert_eq!(
        profile(&|a| a[1].host_input_digest = a[0].host_input_digest.clone()),
        Some("overlapping_state_ownership")
    );
    // A state inside the operation root (or the root inside a state) would share custody.
    let ops = fx.root.join("ops");
    assert_eq!(
        profile(&|a| a[1].state_dir = ops.join("x").to_string_lossy().into()),
        Some("overlapping_state_ownership")
    );
    assert_eq!(
        profile(&|a| a[1].state_dir = format!("{state}/../state-3")),
        Some("invalid_job_profile")
    );
    assert_eq!(
        profile(&|a| a[1].host_input_digest = "xyz".into()),
        Some("invalid_job_profile")
    );
}

#[test]
fn submit_after_shutdown_cannot_enter_the_host() {
    // A request that outlives its transport timeout can reach `start` after shutdown has drained.
    let fx = fixture(30, None);
    assert!(fx.jobs.shutdown(Duration::from_secs(5)));
    assert_eq!(
        fx.jobs.call("submit_coding_job", submit_args(&fx, REQ)),
        Err("shutting_down")
    );
    let dir = fx.root.join("ops").join(fx.jobs.operation_id(&fx.auth));
    assert!(!dir.join("entry-attempt.json").exists());
    assert_eq!(runs(&fx), 0);
    assert!(
        fx.jobs.shutdown(Duration::from_secs(1)),
        "shutdown stays idempotent and empty"
    );
    assert!(fx.jobs.read_collector_idle());
}

#[test]
fn concurrent_submit_and_shutdown_never_leave_an_unowned_collector() {
    for round in 0..12 {
        let fx = fixture(30, None);
        std::fs::write(fx.root.join("slow"), b"").unwrap();
        let jobs = fx.jobs.clone();
        let args = submit_args(&fx, REQ);
        let late = thread::spawn(move || jobs.call("submit_coding_job", args));
        thread::sleep(Duration::from_millis(round * 3));
        let drained = fx.jobs.shutdown(Duration::from_secs(15));
        let outcome = late.join().unwrap();
        let dir = fx.root.join("ops").join(fx.jobs.operation_id(&fx.auth));
        match outcome {
            Ok(_) => {
                // Admitted before the fence: owned, joined and its process status was collected.
                assert!(drained, "round {round}: admitted collector must be drained");
                assert!(dir.join("collector-run.json").exists(), "round {round}");
            }
            Err(code) => {
                assert_eq!(code, "shutting_down", "round {round}");
                assert!(!dir.join("entry-attempt.json").exists(), "round {round}");
                assert_eq!(runs(&fx), 0, "round {round}");
            }
        }
        // After drain no collector exists, running or unjoined.
        assert!(fx.jobs.read_collector_idle());
        assert!(fx.jobs.shutdown(Duration::from_secs(1)), "round {round}");
    }
}

#[test]
fn real_outputs_satisfy_the_strict_contract_and_malformed_ones_do_not() {
    use job_output::validate;
    let fx = fixture(30, None);
    let submit = fx
        .jobs
        .call("submit_coding_job", submit_args(&fx, REQ))
        .unwrap();
    assert_eq!(
        validate("submit_coding_job", &submit_args(&fx, REQ), &submit),
        Ok(())
    );
    wait_state(&fx, "collected");
    let read = read_args(REQ);
    let op = fx
        .jobs
        .call("read_coding_job_operation", read.clone())
        .unwrap();
    assert_eq!(validate("read_coding_job_operation", &read, &op), Ok(()));
    // Submission-only `entry` is not valid on a read, and a read shape is not a submission.
    assert_eq!(
        validate("submit_coding_job", &read, &op),
        Err("invalid_response")
    );
    assert_eq!(
        validate("read_coding_job_operation", &read, &submit),
        Err("invalid_response")
    );
    let result = fx
        .jobs
        .call("read_coding_job_result", read.clone())
        .unwrap();
    assert_eq!(validate("read_coding_job_result", &read, &result), Ok(()));
    let cancel = fx.jobs.call("cancel_coding_job", read.clone()).unwrap();
    assert_eq!(validate("cancel_coding_job", &read, &cancel), Ok(()));

    let bad = |name: &str, value: &Value, op: &str| {
        assert_eq!(
            validate(op, &read, value),
            Err("invalid_response"),
            "{name}"
        );
    };
    // The published-definition counterexample and neighbours.
    bad(
        "false schema",
        &json!({"schema":false,"result":"available","operation":null}),
        "read_coding_job_result",
    );
    bad(
        "null operation",
        &json!({"schema":"work-engine.coding-job.v1","result":"not_available","operation":null}),
        "read_coding_job_result",
    );
    bad(
        "unknown result",
        &json!({"schema":"work-engine.coding-job.v1","result":"maybe","operation":op}),
        "read_coding_job_result",
    );
    let edit = |base: &Value, f: &dyn Fn(&mut Value)| {
        let mut v = base.clone();
        f(&mut v);
        v
    };
    bad(
        "future schema",
        &edit(&op, &|v| v["schema"] = json!("work-engine.coding-job.v2")),
        "read_coding_job_operation",
    );
    bad(
        "unknown field",
        &edit(&op, &|v| v["extra"] = json!(1)),
        "read_coding_job_operation",
    );
    bad(
        "missing field",
        &edit(&op, &|v| {
            v.as_object_mut().unwrap().remove("job_id");
        }),
        "read_coding_job_operation",
    );
    bad(
        "state variant",
        &edit(&op, &|v| v["state"] = json!("done")),
        "read_coding_job_operation",
    );
    bad(
        "bad operation id",
        &edit(&op, &|v| v["operation_id"] = json!("abc")),
        "read_coding_job_operation",
    );
    bad(
        "other request",
        &edit(&op, &|v| v["request_id"] = json!(REQ2)),
        "read_coding_job_operation",
    );
    bad(
        "observed != expected digest",
        &edit(&op, &|v| v["host_input_digest"] = json!("1".repeat(64))),
        "read_coding_job_operation",
    );
    bad(
        "settlement upgraded",
        &edit(&op, &|v| v["settlement"] = json!("settled")),
        "read_coding_job_operation",
    );
    bad(
        "collected without removal",
        &edit(&op, &|v| v["host_removed"] = json!(false)),
        "read_coding_job_operation",
    );
    bad(
        "cleanup variant",
        &edit(&op, &|v| v["process_cleanup"]["run"] = json!("clean")),
        "read_coding_job_operation",
    );
    bad(
        "receipt id mismatch",
        &edit(&result, &|v| v["receipt_id"] = json!("2".repeat(64))),
        "read_coding_job_result",
    );
    bad(
        "receipt id not hex",
        &edit(&result, &|v| v["receipt_id"] = json!("rcpt-1")),
        "read_coding_job_result",
    );
    bad(
        "receipt for other job",
        &edit(&result, &|v| v["receipt"]["job_id"] = json!("job-9")),
        "read_coding_job_result",
    );
    bad(
        "receipt digest",
        &edit(&result, &|v| {
            v["receipt"]["input_digest"] = json!("3".repeat(64))
        }),
        "read_coding_job_result",
    );
    bad(
        "receipt not object",
        &edit(&result, &|v| v["receipt"] = json!(null)),
        "read_coding_job_result",
    );
    bad(
        "acceptance missing",
        &edit(&result, &|v| {
            v.as_object_mut().unwrap().remove("acceptance");
        }),
        "read_coding_job_result",
    );
    bad(
        "nested identity",
        &edit(&result, &|v| v["operation"]["job_id"] = json!("bad id")),
        "read_coding_job_result",
    );
    bad(
        "closure claimed",
        &edit(&cancel, &|v| {
            v["container_closure_established"] = json!(true)
        }),
        "cancel_coding_job",
    );
    bad(
        "cancel established",
        &edit(&cancel, &|v| v["cancellation_established"] = json!(true)),
        "cancel_coding_job",
    );
    bad(
        "cancel variant",
        &edit(&cancel, &|v| v["cancellation"] = json!("done")),
        "cancel_coding_job",
    );
    bad(
        "cancel settlement",
        &edit(&cancel, &|v| v["settlement"] = json!("settled")),
        "cancel_coding_job",
    );
    assert_eq!(
        validate("list_sessions", &read, &op),
        Err("invalid_request")
    );
}

#[test]
fn published_output_schema_names_exactly_the_fields_the_server_emits() {
    let schema: Value =
        serde_json::from_str(include_str!("../../schemas/coding-job-v1.json")).unwrap();
    let keys = |v: &Value| -> Vec<String> {
        let mut k: Vec<String> = v.as_object().unwrap().keys().cloned().collect();
        k.sort();
        k
    };
    let defs = &schema["$defs"];
    let fx = fixture(30, None);
    let submit = fx
        .jobs
        .call("submit_coding_job", submit_args(&fx, REQ))
        .unwrap();
    wait_state(&fx, "collected");
    let read = fx
        .jobs
        .call("read_coding_job_operation", read_args(REQ))
        .unwrap();
    let result = fx
        .jobs
        .call("read_coding_job_result", read_args(REQ))
        .unwrap();
    let cancel = fx.jobs.call("cancel_coding_job", read_args(REQ)).unwrap();
    for (name, value) in [
        ("submit_output", &submit),
        ("operation", &read),
        ("result_available", &result),
        ("cancel", &cancel),
    ] {
        assert_eq!(defs[name]["additionalProperties"], false, "{name}");
        assert_eq!(keys(&defs[name]["properties"]), keys(value), "{name}");
        let mut required: Vec<String> = defs[name]["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|k| k.as_str().unwrap().to_string())
            .collect();
        required.sort();
        assert_eq!(required, keys(value), "{name} required");
    }
    assert_eq!(
        defs["operation"]["properties"]["schema"]["$ref"],
        "#/$defs/schema_version"
    );
    assert_eq!(defs["schema_version"]["const"], SCHEMA);
}
