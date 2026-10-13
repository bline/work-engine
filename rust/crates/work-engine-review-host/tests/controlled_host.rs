#![cfg(all(unix, feature = "test-peer"))]
mod support;
use serde_json::json;
use support::*;

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
