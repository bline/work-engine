use super::*;
use crate::application::offline_qualification::JoinedMarkReadback;
use crate::store::{
    offline_qualification::FIXTURE_NOW_MS,
    task_free_b::{Evidence, Kind, Slot},
};
use crate::{AdmitRequest, codec::campaign_digest, test_support as support};
use serde_json::{Value, json};
use std::{cell::Cell, io::Write, os::unix::fs::PermissionsExt};

const EXPIRY: i64 = 4_102_444_800_000;

struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    approval: Value,
    config: Box<dyn Fn() -> TrustedConfig>,
}

fn private_write(path: &Path, raw: &[u8]) {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .unwrap()
        .write_all(raw)
        .unwrap();
}

impl Fixture {
    fn new() -> Self {
        let (temp, repo, workspace, identity, baseline, boundary) = support::setup();
        let root = temp.path().join("natural-exit");
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let campaign = root.join("campaign");
        let (r, w, i, a, b) = (
            repo.clone(),
            workspace.clone(),
            identity.clone(),
            baseline.clone(),
            boundary.clone(),
        );
        let config = Box::new(move || {
            support::config(
                campaign.clone(),
                r.clone(),
                w.clone(),
                i.clone(),
                a.clone(),
                b.clone(),
            )
        });
        let mut app = Campaign::initialize(config()).unwrap();
        let head = support::applied(
            app.admit(
                "admit-foreground-only",
                AdmitRequest {
                    identity: identity.clone(),
                    repository: repo.clone(),
                    workspace: workspace.clone(),
                    accepted_boundary: boundary.clone(),
                    baseline: baseline.clone(),
                    expected_impact: None,
                },
            )
            .unwrap(),
        )
        .revision;
        app.attach_joined_offline_qualification_fixture().unwrap();
        let grant = "1".repeat(64);
        let attempt = campaign_digest(&json!([
            QualificationPurpose::NaturalExit.id(),
            "attempt",
            app.root_id(),
            grant
        ]))
        .unwrap();
        let approval = json!({
            "action": "approve-one-b-init-natural-exit-v1", "root_id": app.root_id(),
            "identity": identity, "head": head, "grant_id": grant, "attempt_id": attempt,
            "intent_sha256": "2".repeat(64), "recipe_sha256": "3".repeat(64),
            "serving_sha256": "4".repeat(64), "expires_at_unix_ms": EXPIRY,
            "repository": repo, "workspace": workspace, "accepted_boundary": boundary,
            "baseline": baseline, "writer_identity": "hp3-host-writer"
        });
        // Use the actual frozen SC0 bytes and production open path, with an
        // isolated directory instead of the administrator's installed root.
        let gate = app.owner().unwrap().config.gate.take().unwrap();
        for (name, raw) in [
            ("gate-capture.json", gate.capture),
            ("gate-manifest.json", gate.manifest),
            ("gate-receipt.json", gate.receipt),
        ] {
            private_write(&root.join(name), &raw);
        }
        private_write(&root.join("approval.json"), approval.to_string().as_bytes());
        drop(app);
        Self {
            _temp: temp,
            root,
            approval,
            config,
        }
    }

    fn raw_approval(&self) -> Result<ProtectedApproval> {
        ProtectedApproval::read(Inputs::new(&self.root, geteuid().as_raw())?)
    }

    fn invoke(&self) -> Result<ForegroundSelection> {
        self.raw_approval()?
            .open(&"2".repeat(64), &"3".repeat(64), &"4".repeat(64))
    }

    fn reopen(&self) -> Campaign {
        Campaign::open((self.config)()).unwrap()
    }

    fn replace(&self, value: &Value) {
        let path = self.root.join("approval.json");
        fs::remove_file(&path).unwrap();
        private_write(&path, value.to_string().as_bytes());
    }

    fn history_after_close(&self, original: &AttemptBinding, spent: bool, created_mark: bool) {
        let reopened = self.reopen();
        let port = reopened.offline_qualification_port();
        assert_eq!(
            port.read_qualification_spend(&original.history_key())
                .unwrap(),
            spent
        );
        let mut owner = reopened.owner().unwrap();
        // Test the actual Store opening check, not merely the port's refusal
        // to accept protected bindings at its fixture entry point.
        let err = owner.store.enter_qualification(original).unwrap_err();
        assert!(err.to_string().contains("selection Store opening differs"));
        if spent {
            let history = owner.store.read_b_history(original.lifecycle()).unwrap();
            assert_eq!(history.mark(Kind::Create).is_some(), created_mark);
            assert!(history.mark(Kind::Start).is_none());
            assert_eq!(history.evidence(Slot::Terminal), Evidence::Absent);
        }
        drop(owner);
        drop((port, reopened));
        assert!(
            self.invoke().is_err(),
            "same approved grant was selectable again"
        );
    }
}

fn no_delivery(_: SpentQualification) {
    panic!("must not deliver a Host continuation")
}

#[test]
fn foreground_same_opening_handoff_is_committed_before_host_and_owner_is_unlocked() {
    let f = Fixture::new();
    let s = f.invoke().unwrap();
    let count = Cell::new(0);
    assert!(s.port().enter(s.binding()).is_err(), "fixture port bypass");
    s.handoff(|spent| {
        assert_eq!(spent.binding(), s.binding());
        assert!(
            s.port()
                .inner
                .try_lock()
                .unwrap()
                .store
                .conn
                .is_autocommit()
        );
        assert!(
            s.port()
                .read_qualification_spend(&s.binding().history_key())
                .unwrap()
        );
        let req = s.binding().create_request_digest().unwrap();
        let _mark = spent
            .mark_joined_create(&req, JoinedMarkReadback::Check)
            .unwrap();
        count.set(count.get() + 1); // one synthetic effect, no Docker
    })
    .unwrap();
    assert_eq!(count.get(), 1);
    let b = s.binding().clone();
    drop(s);
    f.history_after_close(&b, true, true);
}

#[test]
fn foreground_exact_unspent_selection_is_not_entry_after_reopening() {
    let f = Fixture::new();
    let s = f.invoke().unwrap();
    let b = s.binding().clone();
    drop(s);
    f.history_after_close(&b, false, false);
}

#[test]
fn foreground_revocation_ordered_before_consumption_suppresses_delivery() {
    let f = Fixture::new();
    let s = f.invoke().unwrap();
    let entered = s.enter().unwrap();
    let b = s.binding().clone();
    s.port()
        .owner()
        .unwrap()
        .store
        .revoke_qualification(&b)
        .unwrap();
    assert!(
        s.deliver(entered, true, no_delivery)
            .unwrap_err()
            .to_string()
            .contains("one-shot spend unavailable")
    );
    drop(s);
    f.history_after_close(&b, false, false);
}

#[test]
fn foreground_withdrawal_or_expiry_after_entry_suppresses_delivery() {
    for withdraw in [false, true] {
        let f = Fixture::new();
        let s = f.invoke().unwrap();
        let entered = s.enter().unwrap();
        if withdraw {
            fs::remove_file(f.root.join("approval.json")).unwrap();
        } else {
            FIXTURE_NOW_MS.set(Some(EXPIRY));
        }
        let result = s.deliver(entered, true, no_delivery);
        FIXTURE_NOW_MS.set(None);
        assert!(result.is_err());
        assert!(
            !s.port()
                .read_qualification_spend(&s.binding().history_key())
                .unwrap()
        );
        let b = s.binding().clone();
        drop(s);
        if withdraw {
            private_write(
                &f.root.join("approval.json"),
                f.approval.to_string().as_bytes(),
            );
        }
        f.history_after_close(&b, false, false);
    }
}

#[test]
fn foreground_duplicate_invocation_has_no_second_handoff() {
    let f = Fixture::new();
    let s = f.invoke().unwrap();
    assert!(f.invoke().is_err()); // concurrent opening/second invocation
    s.handoff(drop).unwrap();
    assert!(s.handoff(no_delivery).is_err());
    let b = s.binding().clone();
    drop(s);
    f.history_after_close(&b, true, false);
}

#[test]
fn foreground_crash_after_spend_leaves_only_unresolved_history() {
    let f = Fixture::new();
    let s = f.invoke().unwrap();
    let b = s.binding().clone();
    // Unwind at the first Host instruction, before any mark or effect. Test
    // the same durable prefix a process death would leave, then reopen it.
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            s.handoff(|_| panic!("simulated crash after durable spend"))
                .unwrap();
        }))
        .is_err()
    );
    drop(s);
    f.history_after_close(&b, true, false);
}

#[test]
fn foreground_lost_spend_reply_has_no_delivery_or_replay() {
    let f = Fixture::new();
    let s = f.invoke().unwrap();
    let b = s.binding().clone();
    assert!(
        s.deliver(s.enter().unwrap(), false, no_delivery)
            .unwrap_err()
            .to_string()
            .contains("spend confirmation lost")
    );
    drop(s);
    f.history_after_close(&b, true, false);
}

#[test]
fn foreground_lost_create_reply_does_not_send_again_or_start() {
    let f = Fixture::new();
    let s = f.invoke().unwrap();
    let b = s.binding().clone();
    let sends = Cell::new(0);
    s.handoff(|spent| {
        let _marked = spent
            .mark_joined_create(
                &b.create_request_digest().unwrap(),
                JoinedMarkReadback::Check,
            )
            .unwrap();
        sends.set(sends.get() + 1);
        // Possible Create; no exact serving readback arrived. No continuation.
    })
    .unwrap();
    assert!(s.handoff(no_delivery).is_err());
    assert_eq!(sends.get(), 1);
    drop(s);
    f.history_after_close(&b, true, true);
}

#[test]
fn foreground_failed_acceptance_does_not_refund_spend() {
    let f = Fixture::new();
    let s = f.invoke().unwrap();
    let b = s.binding().clone();
    assert_eq!(
        s.handoff(|_| Err::<(), _>("protected capture sync failed"))
            .unwrap(),
        Err("protected capture sync failed")
    );
    assert!(s.handoff(no_delivery).is_err());
    drop(s);
    f.history_after_close(&b, true, false);
}

#[test]
fn foreground_requires_exact_attempt_root_head_and_host_pins() {
    let f = Fixture::new();
    for field in [
        "attempt_id",
        "root_id",
        "head",
        "intent_sha256",
        "recipe_sha256",
        "serving_sha256",
    ] {
        let mut changed = f.approval.clone();
        changed[field] = json!("f".repeat(64));
        f.replace(&changed);
        assert!(f.invoke().is_err(), "accepted crossed {field}");
    }
    f.replace(&f.approval);
    f.invoke().unwrap().handoff(drop).unwrap();
}

#[test]
fn foreground_refuses_campaign_database_with_another_link() {
    let f = Fixture::new();
    let link = f.root.join("alternate-db");
    fs::hard_link(f.root.join("campaign/slice-campaign.sqlite"), &link).unwrap();
    assert!(f.invoke().is_err());
    fs::remove_file(link).unwrap();
    f.invoke().unwrap().handoff(drop).unwrap();
}

#[test]
fn foreground_input_rejects_symlink_hardlink_readable_and_oversize() {
    let f = Fixture::new();
    let path = f.root.join("approval.json");
    let alternate = f.root.join("alternate.json");
    fs::rename(&path, &alternate).unwrap();
    std::os::unix::fs::symlink(&alternate, &path).unwrap();
    assert!(f.raw_approval().is_err());
    fs::remove_file(&path).unwrap();
    fs::hard_link(&alternate, &path).unwrap();
    assert!(f.raw_approval().is_err());
    fs::remove_file(&path).unwrap();
    fs::rename(&alternate, &path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(f.raw_approval().is_err());
    fs::remove_file(&path).unwrap();
    private_write(&path, &vec![b' '; MAX_APPROVAL as usize + 1]);
    assert!(f.raw_approval().is_err());
}
