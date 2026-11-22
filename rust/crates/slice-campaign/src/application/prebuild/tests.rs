use serde_json::{Value, json};

use super::*;
use crate::test_support::setup;

struct Scope {
    _temp: tempfile::TempDir,
    root: PathBuf,
    document: Value,
}

impl Scope {
    fn new() -> Self {
        let (temp, repository, workspace, mut identity, baseline, _) = setup();
        identity.run_id = "isolated-controlled".into();
        identity.attempt_id = "first-build".into();
        let root = temp.path().join("prebuild-campaign");
        let document = json!({
            "schemaVersion": 1,
            "root": root,
            "repository": repository,
            "workspace": workspace,
            "identity": identity,
            "acceptedBoundary": {
                "reference": "fixture approved objective",
                "sha256": raw_sha256(b"fixture approved objective"),
            },
            "baseline": baseline,
            "writerIdentity": "isolated-writer",
        });
        Self {
            _temp: temp,
            root,
            document,
        }
    }

    fn pinned(&self) -> (String, Vec<u8>) {
        let bytes = serde_json::to_vec(&self.document).unwrap();
        (raw_sha256(&bytes), bytes)
    }
}

#[test]
fn exact_owner_selection_pins_fresh_implementing_commit() {
    let scope = Scope::new();
    let (pin, bytes) = scope.pinned();
    let fresh = start(Some(&pin), &bytes).unwrap();
    let snapshot = fresh.snapshot();
    let reference = fresh.reference();
    let receipt = fresh.receipt();
    assert_eq!(fresh.anchored_root(), scope.root.canonicalize().unwrap());
    assert_eq!(snapshot.phase, "implementing");
    assert_eq!(snapshot.revision, reference.revision);
    assert_eq!(snapshot.identity, reference.identity);
    assert_eq!(
        serde_json::to_value(&snapshot.identity).unwrap(),
        scope.document["identity"]
    );
    assert_eq!(snapshot.repository, scope.document["repository"]);
    assert_eq!(snapshot.workspace, scope.document["workspace"]);
    assert_eq!(
        serde_json::to_value(&snapshot.baseline).unwrap(),
        scope.document["baseline"]
    );
    assert_eq!(
        serde_json::to_value(&snapshot.accepted_boundary).unwrap(),
        scope.document["acceptedBoundary"]
    );
    assert_eq!(receipt.root_id, reference.root_id);
    assert_eq!(receipt.identity, reference.identity);
    assert_eq!(receipt.result_revision, reference.revision);
    assert_eq!(receipt.profile_digest, pin);
    assert_eq!(receipt.operation_id, IMPLEMENTING);
    assert_eq!(receipt.kind, "advance");
    assert_eq!(
        receipt.result_reference,
        format!(
            "campaign:{}@{}",
            reference.identity.key(),
            reference.revision.as_str()
        )
    );
    assert!(snapshot.gate_capture_digest.is_none());
    assert!(snapshot.candidate.is_none());
    assert!(snapshot.selection_ref.is_none());
    assert!(snapshot.obligations.is_empty());
    assert!(matches!(
        start(Some(&pin), &bytes),
        Err(StartupError::Campaign(CampaignError::Root(_)))
    ));
    let reopened: Implementing = reopen(Some(&pin), &bytes, reference).unwrap();
    assert_eq!(&reopened.reference, reference);
    assert_eq!(
        serde_json::to_value(&reopened.snapshot).unwrap(),
        serde_json::to_value(snapshot).unwrap()
    );
}

#[test]
fn actual_owner_replay_and_no_effect_do_not_supply_fresh_commit_facts() {
    let scope = Scope::new();
    let (pin, bytes) = scope.pinned();
    let fresh = start(Some(&pin), &bytes).unwrap();
    let reference = fresh.reference();
    let receipt = fresh.receipt();
    let consequence = fresh.snapshot().latest_consequence.clone().unwrap();
    let mut owner = Campaign::open(config(Some(&pin), &bytes).unwrap()).unwrap();
    let replay = owner
        .advance(
            &reference.identity,
            receipt.prior_revision.as_ref().unwrap(),
            IMPLEMENTING,
            AdvancePhase::Implementing,
            consequence.clone(),
        )
        .unwrap();
    let Effect::Replayed {
        value,
        receipt: stored,
    } = &replay
    else {
        panic!("expected the owner's persisted Implementing replay");
    };
    assert_eq!(
        serde_json::to_value(stored).unwrap(),
        serde_json::to_value(receipt).unwrap()
    );
    assert_eq!(
        serde_json::to_value(value).unwrap(),
        serde_json::to_value(fresh.snapshot()).unwrap()
    );
    assert!(matches!(
        newly_applied(IMPLEMENTING, replay),
        Err(StartupError::Refused {
            step: IMPLEMENTING,
            ..
        })
    ));
    let refused = owner
        .advance(
            &reference.identity,
            &reference.revision,
            "another-implementing",
            AdvancePhase::Implementing,
            consequence,
        )
        .unwrap();
    assert!(matches!(refused, Effect::NoEffect(_)));
    assert!(matches!(
        newly_applied(IMPLEMENTING, refused),
        Err(StartupError::Refused {
            step: IMPLEMENTING,
            ..
        })
    ));
}

#[test]
fn uncertain_effect_preserves_recovery_locator_instead_of_fresh_result() {
    let scope = Scope::new();
    let (pin, bytes) = scope.pinned();
    let fresh = start(Some(&pin), &bytes).unwrap();
    let receipt = fresh.receipt();
    // Exercise the real result classifier with this variant; no storage fault
    // is injected and no claim is made that the fixture commit was uncertain.
    let locator = RecoveryLocator {
        root_id: receipt.root_id.clone(),
        anchored_root: fresh.anchored_root().to_owned(),
        identity: receipt.identity.clone(),
        operation_id: receipt.operation_id.clone(),
        kind: receipt.kind.clone(),
        request_digest: receipt.request_digest.clone(),
        expected_revision: receipt.prior_revision.clone(),
        profile_digest: receipt.profile_digest.clone(),
    };
    let expected = serde_json::to_value(&locator).unwrap();
    let Err(StartupError::OutcomeUnknown {
        step: IMPLEMENTING,
        locator,
    }) = newly_applied(IMPLEMENTING, Effect::OutcomeUnknown(locator))
    else {
        panic!("uncertain effect did not retain its recovery locator");
    };
    assert_eq!(serde_json::to_value(locator).unwrap(), expected);
}

#[test]
fn no_selection_or_changed_pinned_scope_refuses_before_creating_root() {
    let scope = Scope::new();
    let (pin, bytes) = scope.pinned();
    assert!(matches!(
        start(None, &bytes),
        Err(StartupError::Campaign(CampaignError::Unsupported(_)))
    ));
    assert!(!scope.root.exists());
    let other = scope.root.with_file_name("other-root");
    for (path, replacement) in [
        ("/root", json!(other)),
        ("/workspace", json!(scope.root)),
        ("/identity/attemptId", json!("a-different-attempt")),
        ("/acceptedBoundary/sha256", json!("0".repeat(64))),
        ("/baseline/acceptedTree", json!("0".repeat(40))),
    ] {
        let mut changed = scope.document.clone();
        *changed.pointer_mut(path).unwrap() = replacement;
        let changed = serde_json::to_vec(&changed).unwrap();
        assert!(matches!(
            start(Some(&pin), &changed),
            Err(StartupError::Campaign(CampaignError::Contract(_)))
        ));
        assert!(!scope.root.exists());
        assert!(!other.exists());
    }
}

#[test]
fn pin_does_not_substitute_for_a_valid_local_baseline() {
    let mut scope = Scope::new();
    scope.document["baseline"]["acceptedTree"] = json!("0".repeat(40));
    let (pin, bytes) = scope.pinned();
    assert!(matches!(
        start(Some(&pin), &bytes),
        Err(StartupError::Campaign(CampaignError::Subject(_)))
    ));
    assert!(!scope.root.exists());
}

#[test]
fn reopen_rejects_crossed_root_revision_or_authority_and_gate_cannot_advance() {
    let mut scope = Scope::new();
    let (pin, bytes) = scope.pinned();
    let entered = start(Some(&pin), &bytes).unwrap();
    let other_scope = Scope::new();
    let (other_pin, other_bytes) = other_scope.pinned();
    let other = start(Some(&other_pin), &other_bytes).unwrap();
    assert!(matches!(
        reopen(Some(&pin), &bytes, other.reference()),
        Err(CampaignError::Conflict(_))
    ));
    let mut stale = entered.reference().clone();
    stale.revision = crate::CampaignRevision::new("0".repeat(64)).unwrap();
    assert!(matches!(
        reopen(Some(&pin), &bytes, &stale),
        Err(CampaignError::Conflict(_))
    ));
    assert!(matches!(
        reopen(None, &bytes, entered.reference()),
        Err(CampaignError::Unsupported(_))
    ));
    scope.document["writerIdentity"] = json!("different-writer");
    let (cross_pin, cross_bytes) = scope.pinned();
    assert!(matches!(
        reopen(Some(&cross_pin), &cross_bytes, entered.reference()),
        Err(CampaignError::Root(_))
    ));

    let mut campaign = Campaign::open(config(Some(&pin), &bytes).unwrap()).unwrap();
    assert!(matches!(
        campaign.advance(
            &entered.reference().identity,
            &entered.reference().revision,
            "not-a-gate",
            AdvancePhase::GateReady,
            entered.snapshot().latest_consequence.clone().unwrap(),
        ),
        Err(CampaignError::Unsupported(_))
    ));
    drop(campaign);
    assert_eq!(
        &reopen(Some(&pin), &bytes, entered.reference())
            .unwrap()
            .reference,
        entered.reference()
    );
}
