#![cfg(all(unix, feature = "test-peer", feature = "test-faults"))]
mod support;
use std::{
    fs,
    time::{Duration, Instant},
};
use support::*;

#[test]
fn actual_host_cuts_recover_without_any_additional_peer_entry() {
    let cases = [
        ("REVIEW_HOST_FAULT_CUT", "before_dispatch", false),
        ("REVIEW_HOST_FAULT_CUT", "after_preparation", false),
        ("REVIEW_HOST_FAULT_CUT", "after_dispatch", false),
        ("REVIEW_HOST_FAULT_CUT", "after_episode_begin", false),
        ("WORK_ENGINE_EVIDENCE_FAULT_CUT", "launch_intent", false),
        ("REVIEW_HOST_FAULT_CUT", "after_e1_terminal", true),
        (
            "SLICE_CAMPAIGN_FAULT_CUT",
            "j1_after_establishment_builder_commit",
            true,
        ),
        (
            "SLICE_CAMPAIGN_FAULT_CUT",
            "j1_after_establishment_campaign_commit",
            true,
        ),
        ("REVIEW_HOST_FAULT_CUT", "after_episode_result", true),
        ("REVIEW_HOST_FAULT_CUT", "after_j1_joined", true),
        ("REVIEW_HOST_FAULT_CUT", "after_j2_outcome", true),
    ];
    for (environment, cut, terminal) in cases {
        eprintln!("H1 actual host cut {cut}");
        let f = Fixture::new(0, "result");
        let initial = f.initialize();
        let dir = f.temp.path().to_str().unwrap();
        let mut c = f.start(&[
            (environment, cut),
            ("REVIEW_HOST_FAULT_DIR", dir),
            ("SLICE_CAMPAIGN_FAULT_DIR", dir),
            (
                "WORK_ENGINE_EVIDENCE_FAULT_READY",
                f.temp.path().join(format!("{cut}.ready")).to_str().unwrap(),
            ),
        ]);
        c.ready();
        c.domain(
            "execute_initial",
            initial["campaign"]["revision"].as_str().unwrap(),
        );
        let signal = f.temp.path().join(format!("{cut}.ready"));
        let deadline = Instant::now() + Duration::from_secs(45);
        while !signal.exists() {
            assert!(
                c.child.try_wait().unwrap().is_none(),
                "host exited before {cut}"
            );
            if let Ok(reply) = c.rx.try_recv() {
                panic!("host replied before cut {cut}: {reply}");
            }
            assert!(Instant::now() < deadline, "cut not reached: {cut}");
            std::thread::sleep(Duration::from_millis(10));
        }
        c.child.kill().unwrap();
        c.child.wait().unwrap();
        let entries = f.entries();
        assert_eq!(entries, usize::from(terminal), "{cut}");
        let mut recovered = f.start(&[]);
        let ready = recovered.ready();
        let revision = ready["result"]["lastObservation"]["revision"]
            .as_str()
            .unwrap();
        recovered.domain("recover_initial", revision);
        let reply = recovered.receive();
        if terminal {
            assert_eq!(
                reply["result"]["outcome"]["view"]["kind"], "evidence_unestablished",
                "{cut}: {reply}"
            );
        } else {
            assert!(
                matches!(
                    reply["result"]["status"].as_str(),
                    Some("unresolved" | "not_dispatched")
                ),
                "{cut}: {reply}"
            );
        }
        assert_eq!(f.entries(), entries, "recovery relaunched at {cut}");
        recovered.finish();
        let trace = serde_json::json!({"cut":cut,"entriesBeforeRecovery":entries,"entriesAfterRecovery":f.entries(),"reply":reply});
        if let Ok(root) = std::env::var("H1_EVIDENCE_ROOT") {
            fs::create_dir_all(&root).unwrap();
            fs::write(
                std::path::Path::new(&root).join(format!("crash-{cut}.json")),
                serde_json::to_vec_pretty(&trace).unwrap(),
            )
            .unwrap();
        }
    }
}
