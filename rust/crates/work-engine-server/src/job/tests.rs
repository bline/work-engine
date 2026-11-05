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
journal() {{ printf '{{"schema":"work-engine.pi-docker-custody.v1","job":{{"job_id":"job-1","state_dir":"%s"}},"input_digest":"%s","phase":"%s","removed":%s,"receipt_id":"rcpt-1"}}' "$S" "d1d1d1d1" "$1" "$2" > "$S/pi-state.json"; }}
receipt() {{ printf '{{"receipt":{{"status":"component_passed","phase":"collected","rpc_verified":true,"failures":[],"coding":{{"completed":true,"tool_removed":true}},"removed":true,"daemon_exit":0,"acceptance":"owner_pending","domain_effect_settlement":"not_claimed","receipt_id":"rcpt-1","input_digest":"d1d1d1d1"}},"receipt_id":"rcpt-1"}}\n'; }}
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
    let coding = match provider {
        None => json!({"schema":"work-engine.pi-coding-fixture.v1","max_tool_calls":1}),
        Some(p) => json!({"schema":"x","provider":p}),
    };
    let job = json!({"schema":"work-engine.pi-docker-custody.v1","job_id":"job-1",
        "state_dir":state,"deadline_unix_seconds":now()+120,
        "packet":{"path":packet,"sha256":hex(b"{\"task\":\"fixture\"}")},"coding":coding});
    std::fs::write(&job_file, job.to_string()).unwrap();
    let auth = Authorization {
        authorization_id: "auth-1".into(),
        caller: "caller-1".into(),
        credential_sha256: hex(CRED.as_bytes()),
        job_file: job_file.to_string_lossy().into(),
        job_sha256: hex(job.to_string().as_bytes()),
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
    assert_eq!(done["host_input_digest"], "d1d1d1d1");
    assert_eq!(
        done["process_cleanup"]["run"],
        "owned_process_group_collected"
    );
    let result = fx
        .jobs
        .call("read_coding_job_result", read_args(REQ))
        .unwrap();
    assert_eq!(result["result"], "available");
    assert_eq!(result["receipt_id"], "rcpt-1");
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
