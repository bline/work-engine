#![cfg(all(unix, feature = "test-peer"))]
mod support;
use serde_json::json;
use support::*;

#[cfg(feature = "test-support")]
fn synthetic_join_fixture(findings: bool) -> Fixture {
    use review_execution_evidence::test_support::{FIXTURE_PROFILE, FIXTURE_VERIFIER};
    let mut fixture = Fixture::new(0, "result");
    let root = fixture.temp.path();
    let custody = json!({"owner":"review-execution-evidence","verifier":FIXTURE_VERIFIER,
        "profile":FIXTURE_PROFILE,"artifact_schemes":["execution-artifact:"],
        "reference_schemes":["execution-evidence:"]});
    let mut source: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&fixture.document.claims.source.path).unwrap())
            .unwrap();
    source["custody_config_sha256"] = json!(claim_digest(&custody));
    source["grant_ids"] = json!(["h1-production", "h1-completion"]);
    let source_pin = data(root, "synthetic-claims-source.json", &source);
    let authority = json!({"owner":source["owner"],"reference":source["reference"],
        "revision":source["revision"],"integrity_sha256":source_pin.sha256,
        "freshness":source["freshness"],"status":"verified"});
    let mut config: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&fixture.document.claims.configuration.path).unwrap(),
    )
    .unwrap();
    config["trusted_custody"] = custody;
    config["grants"][0]["authority_reference"] = authority;
    let completion_authority = config["grants"][0]["authority_reference"].clone();
    config["grants"]
        .as_array_mut()
        .unwrap()
        .push(json!({"schema_version":1,
        "grant_id":"h1-completion","actor":"campaign",
        "profile":"revision-bound-review-finding-v1",
        "permissions":["create_claim","publish_revision","record_reliance"],
        "decision_scope":"h1-initial","authority_reference":completion_authority}));
    fixture.document.claims.production_grant_sha256 = claim_digest(&config["grants"][0]);
    fixture.document.claims.source = source_pin;
    fixture.document.claims.configuration = data(root, "synthetic-claims-config.json", &config);
    if findings {
        let mut input: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&fixture.document.request_input.path).unwrap())
                .unwrap();
        input["resultTemplate"] = serde_json::from_str(include_str!(
            "../../../../app-server/tests/fixtures/implementation-review/remediation-required.json"
        ))
        .unwrap();
        fixture.document.request_input = data(root, "synthetic-findings-input.json", &input);
    }
    fixture.save();
    fixture
}

#[cfg(feature = "test-support")]
fn initialize_synthetic(fixture: &Fixture) -> serde_json::Value {
    let output = std::process::Command::new(&fixture.host)
        .args(["initialize", "--startup"])
        .arg(&fixture.startup)
        .env("RP2_TEST_SYNTHETIC_FIXTURE_JOIN", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[cfg(feature = "test-support")]
#[test]
fn synthetic_fixture_runs_the_shared_workflow_join_with_distinct_profile_identities() {
    let fixture = synthetic_join_fixture(false);
    let initialized = initialize_synthetic(&fixture);
    let revision = initialized["campaign"]["revision"].as_str().unwrap();
    let mut host = fixture.start(&[("RP2_TEST_SYNTHETIC_FIXTURE_JOIN", "1")]);
    host.ready();
    host.domain("execute_initial", revision);
    let joined = host.receive();
    assert_eq!(
        joined["result"]["outcome"]["status"], "committed",
        "{joined}"
    );
    assert_eq!(
        joined["result"]["outcome"]["view"]["kind"],
        "evidence_unestablished"
    );
    assert_eq!(
        joined["result"]["executionEvidence"]["profile"],
        "controlled-fixture-v1"
    );
    assert_eq!(fixture.entries(), 1);
    let joined_receipt = joined["result"]["outcome"]["receipt"].clone();
    let last = joined["result"]["lastCheckedRevision"]
        .as_str()
        .unwrap()
        .to_owned();
    host.finish();
    let mut cold = fixture.start(&[("RP2_TEST_SYNTHETIC_FIXTURE_JOIN", "1")]);
    cold.ready();
    cold.domain("recover_initial", &last);
    let replay = cold.receive();
    assert_eq!(
        replay["result"]["outcome"]["receipt"], joined_receipt,
        "{replay}"
    );
    assert_eq!(fixture.entries(), 1, "cold recovery entered the peer again");
    cold.finish();
}

#[cfg(feature = "test-support")]
#[test]
fn synthetic_fixture_shared_join_commits_finding_without_production_custody() {
    let fixture = synthetic_join_fixture(true);
    let initialized = initialize_synthetic(&fixture);
    let revision = initialized["campaign"]["revision"].as_str().unwrap();
    let mut host = fixture.start(&[
        ("RP2_TEST_SYNTHETIC_FIXTURE_JOIN", "1"),
        ("RP2_TEST_SYNTHETIC_ESTABLISHED", "1"),
    ]);
    host.ready();
    host.domain("execute_initial", revision);
    let joined = host.receive();
    assert_eq!(
        joined["result"]["outcome"]["status"], "committed",
        "{joined}"
    );
    let view = &joined["result"]["outcome"]["view"];
    assert_eq!(view["kind"], "awaiting_builder", "{view}");
    assert_eq!(view["findings"].as_array().unwrap().len(), 1, "{view}");
    assert_eq!(fixture.entries(), 1);
    host.finish();
}

#[cfg(all(feature = "test-support", feature = "test-faults"))]
#[test]
fn synthetic_fixture_join_recovers_owner_stage_cuts_without_reentry() {
    use std::time::{Duration, Instant};
    for (environment, cut) in [
        ("REVIEW_HOST_FAULT_CUT", "after_e1_terminal"),
        (
            "SLICE_CAMPAIGN_FAULT_CUT",
            "j1_after_establishment_builder_commit",
        ),
        ("REVIEW_HOST_FAULT_CUT", "after_episode_result"),
        ("REVIEW_HOST_FAULT_CUT", "after_j2_outcome"),
    ] {
        let fixture = synthetic_join_fixture(false);
        let initialized = initialize_synthetic(&fixture);
        let revision = initialized["campaign"]["revision"].as_str().unwrap();
        let directory = fixture.temp.path().to_str().unwrap();
        let mut host = fixture.start(&[
            ("RP2_TEST_SYNTHETIC_FIXTURE_JOIN", "1"),
            (environment, cut),
            ("REVIEW_HOST_FAULT_DIR", directory),
            ("SLICE_CAMPAIGN_FAULT_DIR", directory),
        ]);
        host.ready();
        host.domain("execute_initial", revision);
        let signal = fixture.temp.path().join(format!("{cut}.ready"));
        let deadline = Instant::now() + Duration::from_secs(180);
        while !signal.exists() {
            assert!(
                host.child.try_wait().unwrap().is_none(),
                "host exited before {cut}"
            );
            assert!(
                Instant::now() < deadline,
                "synthetic cut not reached: {cut}"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        host.child.kill().unwrap();
        host.child.wait().unwrap();
        drop(host);
        assert_eq!(fixture.entries(), 1, "{cut}");
        let mut recovered = fixture.start(&[("RP2_TEST_SYNTHETIC_FIXTURE_JOIN", "1")]);
        let ready = recovered.ready();
        let latest = ready["result"]["lastObservation"]["revision"]
            .as_str()
            .unwrap();
        recovered.domain("recover_initial", latest);
        let reply = recovered.receive();
        assert_eq!(
            reply["result"]["outcome"]["status"], "committed",
            "{cut}: {reply}"
        );
        assert_eq!(
            reply["result"]["outcome"]["view"]["kind"],
            "evidence_unestablished"
        );
        assert_eq!(
            fixture.entries(),
            1,
            "recovery launched a second peer at {cut}"
        );
        recovered.finish();
    }
}

#[test]
fn permanent_cli_runs_real_owners_once_and_recovers_original_outcome() {
    let f = Fixture::new(50, "result");
    let initialized = f.initialize();
    let revision = initialized["campaign"]["revision"].as_str().unwrap();
    let mut client = f.start(&[]);
    client.ready();
    client.domain("execute_initial", revision);
    let result = client.receive();
    assert_eq!(
        result["result"]["outcome"]["status"], "committed",
        "{result}"
    );
    assert_eq!(
        result["result"]["outcome"]["view"]["kind"], "evidence_unestablished",
        "{result}"
    );
    assert_eq!(f.entries(), 1);
    let view = &result["result"]["outcome"]["view"];
    for boundary in ["builder", "campaign"] {
        assert_eq!(view["pair"][boundary]["status"], "false");
        assert!(
            view["pair"][boundary]["reasons"]
                .as_array()
                .unwrap()
                .contains(&json!("mutation_authorized"))
        );
    }
    assert!(view["findings"].as_array().unwrap().is_empty());
    assert!(
        view["consumptions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|v| v["consumedAtOperationId"].is_null())
    );
    let final_revision = result["result"]["lastCheckedRevision"]
        .as_str()
        .unwrap()
        .to_owned();
    let original = result["result"]["outcome"]["receipt"].clone();
    client.finish();
    let mut reopened = f.start(&[]);
    reopened.ready();
    reopened.domain("recover_initial", &final_revision);
    let recovered = reopened.receive();
    assert_eq!(
        recovered["result"]["outcome"]["receipt"], original,
        "{recovered}"
    );
    assert_eq!(f.entries(), 1);
    reopened.finish();
    let same = f.initialize();
    assert_eq!(same["campaign"]["revision"], final_revision);
}

#[test]
fn unsupported_stale_and_wrong_scope_commands_have_no_domain_effects() {
    let f = Fixture::new(0, "result");
    let initialized = f.initialize();
    let initial = &initialized["campaign"]["revision"];
    let mut c = f.start(&[]);
    c.ready();
    for request in [
        json!({"version":1,"requestId":"bad","operation":"retry"}),
        json!({"version":1,"requestId":"bad","operation":"execute_initial","obligationId":"other","operationId":"h1-initial","expectedRevision":initial}),
        json!({"version":1,"requestId":"bad","operation":"execute_initial","obligationId":"review-0","operationId":"h1-initial","expectedRevision":"0".repeat(64)}),
    ] {
        c.send(request);
        assert!(c.receive().get("error").is_some());
        c.ready();
    }
    c.domain("inspect_initial", initial.as_str().unwrap());
    let observed = c.receive();
    assert_eq!(&observed["result"]["campaign"]["revision"], initial);
    assert_eq!(f.entries(), 0);
    assert!(!f.document.roots.evidence.exists());
    assert!(!f.document.roots.claims.exists());
    c.finish();
}

#[test]
fn invalid_profile_and_changed_startup_refuse_before_effects() {
    for altered in ["profile", "evidence", "session", "root"] {
        let mut f = Fixture::new(0, "result");
        match altered {
            "profile" => f.document.profile = "arbitrary-campaign".into(),
            "evidence" => f.document.evidence_profile = "qualified-provider-v1".into(),
            "session" => f.document.peer.session = "changed-session".into(),
            "root" => f.document.roots.evidence = f.document.roots.episode.clone(),
            _ => unreachable!(),
        }
        f.save();
        let output = std::process::Command::new(&f.host)
            .args(["initialize", "--startup"])
            .arg(&f.startup)
            .output()
            .unwrap();
        assert!(!output.status.success(), "{altered}");
        assert!(!f.document.roots.campaign.exists());
        assert!(!f.document.roots.episode.exists());
        assert_eq!(f.entries(), 0);
    }
    let mut f = Fixture::new(0, "result");
    f.initialize();
    f.document.operation_id = "changed-operation".into();
    f.save();
    let output = std::process::Command::new(&f.host)
        .args(["initialize", "--startup"])
        .arg(&f.startup)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(f.entries(), 0);
}

#[test]
fn incompatible_derived_native_ids_refuse_before_any_owner_or_peer_effect() {
    for (operation_id, expected) in [
        ("x".repeat(122), "derived native transition identifiers"),
        ("x".repeat(128), "derived native begin identifiers"),
        ("internal space".into(), "derived native begin identifiers"),
        (
            "embedded\tcontrol".into(),
            "invalid bounded startup identity",
        ),
        ("non-ascii-é".into(), "invalid bounded startup identity"),
    ] {
        let mut f = Fixture::new(0, "result");
        f.document.operation_id = operation_id.clone();
        f.save();
        let output = std::process::Command::new(&f.host)
            .args(["initialize", "--startup"])
            .arg(&f.startup)
            .output()
            .unwrap();
        let diagnostic = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "admitted {operation_id:?}");
        assert!(diagnostic.contains(expected), "{diagnostic}");
        for root in [
            &f.document.roots.campaign,
            &f.document.roots.episode,
            &f.document.roots.claims,
            &f.document.roots.evidence,
        ] {
            assert!(!root.exists(), "{operation_id:?}: created {root:?}");
        }
        assert_eq!(f.entries(), 0, "{operation_id:?}");
        assert!(!f.count.exists(), "peer entered for {operation_id:?}");
        eprintln!(
            "R1 rejected {operation_id:?}: all four owner roots absent, peer entries=0; {diagnostic}"
        );
    }
}

#[test]
fn longest_native_operation_id_executes_and_recovers_original_outcome() {
    let mut f = Fixture::new(0, "result");
    // The result suffix reaches the owner's 128-byte boundary exactly.
    f.document.operation_id = format!("A0-_:{}.", "x".repeat(115));
    assert_eq!(f.document.operation_id.len(), 121);
    f.save();
    let initialized = f.initialize();
    let mut client = f.start(&[]);
    client.ready();
    client.send(json!({"version":1,"requestId":"execute","operation":"execute_initial","obligationId":f.document.obligation_id,"operationId":f.document.operation_id,"expectedRevision":initialized["campaign"]["revision"]}));
    let completed = client.receive();
    assert_eq!(
        completed["result"]["outcome"]["status"], "committed",
        "{completed}"
    );
    assert_eq!(
        completed["result"]["outcome"]["view"]["kind"],
        "evidence_unestablished"
    );
    assert_eq!(f.entries(), 1);
    client.finish();
    let mut reopened = f.start(&[]);
    let ready = reopened.ready();
    assert_eq!(ready["result"]["admissionOpen"], true, "{ready}");
    assert_eq!(ready["result"]["busy"], false, "{ready}");
    assert_eq!(
        ready["result"]["lastObservation"]["selectedAttempt"]["preparationOperationId"],
        format!("{}:prepare", f.document.operation_id),
        "{ready}"
    );
    assert_eq!(
        ready["result"]["lastObservation"]["selectedAttempt"]["outcome"], "failed_with_result",
        "{ready}"
    );
    reopened.send(json!({"version":1,"requestId":"recover","operation":"recover_initial","obligationId":f.document.obligation_id,"operationId":f.document.operation_id,"expectedRevision":completed["result"]["lastCheckedRevision"]}));
    let recovered = reopened.receive();
    assert_eq!(
        recovered["result"]["outcome"]["receipt"], completed["result"]["outcome"]["receipt"],
        "{recovered}"
    );
    assert_eq!(f.entries(), 1);
    reopened.finish();
    assert_eq!(
        f.initialize()["campaign"]["revision"],
        completed["result"]["lastCheckedRevision"]
    );
}

#[test]
fn malformed_and_oversized_actual_peer_outputs_retain_uncertainty() {
    for mode in ["malformed", "oversized"] {
        let f = Fixture::new(0, mode);
        let initialized = f.initialize();
        let mut c = f.start(&[]);
        c.ready();
        c.domain(
            "execute_initial",
            initialized["campaign"]["revision"].as_str().unwrap(),
        );
        let result = c.receive();
        assert_eq!(result["error"]["kind"], "unresolved", "{result}");
        assert_eq!(f.entries(), 1);
        c.finish();
        let mut recovered = f.start(&[]);
        let ready = recovered.ready();
        recovered.domain(
            "recover_initial",
            ready["result"]["lastObservation"]["revision"]
                .as_str()
                .unwrap(),
        );
        let reply = recovered.receive();
        assert_eq!(reply["result"]["status"], "unresolved", "{reply}");
        assert!(!f.document.roots.claims.exists());
        assert_eq!(f.entries(), 1);
        recovered.finish();
    }
}

#[test]
fn competing_hosts_cannot_dispatch_twice() {
    let f = Fixture::new(200, "result");
    let initialized = f.initialize();
    let revision = initialized["campaign"]["revision"].as_str().unwrap();
    let mut a = f.start(&[]);
    a.ready();
    let mut b = f.start(&[]);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        b.send(json!({"version":1,"requestId":"status","operation":"status"}));
        let reply = b.receive();
        if reply["result"]["busy"] == false {
            assert_eq!(reply["result"]["admissionOpen"], false, "{reply}");
            assert!(
                reply["result"]["lastObservation"]["startupError"]["message"]
                    .as_str()
                    .unwrap()
                    .contains("another campaign writer"),
                "{reply}"
            );
            break;
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    a.domain("execute_initial", revision);
    b.domain("execute_initial", revision);
    let replies = [a.receive(), b.receive()];
    assert_eq!(
        replies
            .iter()
            .filter(|r| r["result"]["outcome"]["status"] == "committed")
            .count(),
        1,
        "{replies:?}"
    );
    assert_eq!(
        replies.iter().filter(|r| r.get("error").is_some()).count(),
        1,
        "{replies:?}"
    );
    assert_eq!(f.entries(), 1);
    a.finish();
    b.finish();
}

#[test]
fn unknown_partial_episode_root_is_preserved() {
    let f = Fixture::new(0, "result");
    std::fs::create_dir(&f.document.roots.episode).unwrap();
    let unknown = f.document.roots.episode.join("partial-owner-marker");
    std::fs::write(&unknown, b"preserve this unresolved owner").unwrap();
    let output = std::process::Command::new(&f.host)
        .args(["initialize", "--startup"])
        .arg(&f.startup)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(
        std::fs::read(&unknown).unwrap(),
        b"preserve this unresolved owner"
    );
    assert!(!f.document.roots.campaign.exists());
    assert_eq!(f.entries(), 0);
}

#[test]
fn changed_native_marker_and_pinned_bytes_refuse_before_dispatch() {
    use std::os::unix::fs::PermissionsExt;
    for changed in ["marker", "request", "peer", "selection", "subject"] {
        let f = Fixture::new(0, "result");
        let initial = f.initialize();
        let mut c = f.start(&[]);
        c.ready();
        let path = match changed {
            "marker" => f
                .document
                .roots
                .episode
                .join(".review-episode-native-host-v1"),
            "request" => f.document.request_input.path.clone(),
            "peer" => f.document.peer.executable.path.clone(),
            "selection" => f.document.selection.path.clone(),
            "subject" => f.document.candidate.path.clone(),
            _ => unreachable!(),
        };
        let old = std::fs::read(&path).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        std::fs::write(&path, b"changed replay bytes").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
        c.domain(
            "execute_initial",
            initial["campaign"]["revision"].as_str().unwrap(),
        );
        let reply = c.receive();
        assert!(reply.get("error").is_some(), "{changed}: {reply}");
        assert_eq!(f.entries(), 0);
        assert!(!f.document.roots.evidence.exists());
        // Restore the exact selected bytes solely to inspect the unchanged campaign.
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        std::fs::write(&path, old).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
        c.ready();
        c.domain(
            "inspect_initial",
            initial["campaign"]["revision"].as_str().unwrap(),
        );
        let read = c.receive();
        assert_eq!(
            read["result"]["campaign"]["revision"],
            initial["campaign"]["revision"]
        );
        c.finish();
    }
}
