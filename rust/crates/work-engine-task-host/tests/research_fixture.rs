#![cfg(feature = "controlled-proof")]
#[path = "../../research-register/tests/card_fixture.rs.inc"]
mod fixture;
use fixture::Fixture;
use research_register::{
    card_admission::{ApprovalEvent, DisclosureScope, Installation},
    card_host::{self, CardRequest},
    card_wire::{self, CurrentCardRequest},
    read_host::{HostProfile, ReadRequest},
    use_bundle::{Record, revision},
};
use std::{collections::BTreeSet, path::PathBuf};
type Args = (
    BTreeSet<String>,
    String,
    String,
    DisclosureScope,
    Vec<(research_register::use_bundle::BlobPin, Vec<u8>)>,
);
fn setup() -> (Fixture, CardRequest, HostProfile, Installation, Args) {
    setup_with(|_| {})
}
fn setup_with(
    mutate: impl FnOnce(&mut Fixture),
) -> (Fixture, CardRequest, HostProfile, Installation, Args) {
    let mut f = Fixture::new();
    mutate(&mut f);
    for (_, doc) in &mut f.docs {
        if doc["record"]["kind"] == "qualification" {
            doc["record"]["purpose"] = "synthetic audit".into();
            doc["record"]["disposition"] = "accepted".into();
        }
    }
    let pin = f.publish(f.manifest());
    let b = f.run(pin.clone()).unwrap();
    let Record::Request {
        decision_revision,
        decision,
        ..
    } = b.records()[&b.manifest().request].record()
    else {
        panic!()
    };
    let current = CurrentCardRequest {
        decision_id: decision.clone(),
        decision_revision: decision_revision.clone(),
        request_sha256: b.manifest().request.clone(),
    };
    let snapshot = research_register::PinnedSnapshot::build(
        research_register::GitObjects::open(&f.repo).unwrap(),
        &f.source,
        &f.register_path,
    )
    .unwrap();
    let research_register::Outcome::Valid(valid) = research_register::validate(&snapshot).unwrap()
    else {
        panic!("fixture invalid")
    };
    let scope = DisclosureScope {
        decision_ids: valid
            .records()
            .decisions
            .iter()
            .map(|d| d.decision_id.clone())
            .collect(),
        finding_ids: valid
            .records()
            .studies
            .iter()
            .flat_map(|s| s.findings.iter().map(|f| f.id.clone()))
            .collect(),
        cross_decision_metadata: true,
    };
    let mut receipts = vec![];
    for role in ["qualification", "applicability"] {
        let event = ApprovalEvent {
            issuer: "synthetic-owner".into(),
            role: role.into(),
            delegation: "synthetic-use-only".into(),
            purpose: "synthetic audit".into(),
            decision_id: current.decision_id.clone(),
            decision_revision: current.decision_revision.clone(),
            request_sha256: current.request_sha256.clone(),
            bundle_sha256: pin.sha256.clone(),
            records: b.records().keys().cloned().collect(),
            scope_sha256: revision(&serde_json::to_vec(&scope).unwrap()),
        };
        let bytes = serde_json::to_vec(&event).unwrap();
        let p = f.publish_raw(&format!("approvals/{role}.json"), &bytes);
        receipts.push((p, bytes));
    }
    let args = (
        BTreeSet::from(["synthetic-owner".into()]),
        "synthetic-use-only".into(),
        "synthetic audit".into(),
        scope.clone(),
        receipts.clone(),
    );
    let installed = Installation::synthetic(
        BTreeSet::from(["synthetic-owner".into()]),
        "synthetic-use-only".into(),
        "synthetic audit".into(),
        scope,
        receipts,
    );
    let profile = HostProfile {
        repository_scope: Some(f.repo.clone()),
        ..HostProfile::default()
    };
    let request = CardRequest {
        source: ReadRequest {
            repository: f.repo.clone(),
            commit: f.source.clone(),
            register_path: f.register_path.clone(),
            bundle: pin,
        },
        current,
        codec: card_wire::CODEC.into(),
    };
    (f, request, profile, installed, args)
}

#[test]
#[ignore = "writes explicitly nominated synthetic full A4 fixture"]
fn derive_full_a4_fixture() {
    let out = PathBuf::from(std::env::var("WE_A4_FIXTURE_OUT").unwrap());
    std::fs::create_dir_all(&out).unwrap();
    let (_fixture, request, profile, _installation, args) = setup();
    let input = serde_json::json!({"request":request,"profile":profile,"producer":revision(b"synthetic-offline-qualification"),"installation":{"issuers":args.0,"delegation":args.1,"purpose":args.2,"scope":args.3,"receipts":args.4,"role_issuers":null}});
    let reconstruction =
        card_host::synthetic_reconstruction(serde_json::from_value(input).unwrap()).unwrap();
    let bytes = serde_json::to_vec(&reconstruction).unwrap();
    assert_eq!(
        card_host::reconstruct(&reconstruction, &revision(&bytes)).unwrap(),
        reconstruction.card_bytes
    );
    let envelope = card_wire::decode(&reconstruction.card_bytes).unwrap();
    let request = envelope
        .card
        .as_ref()
        .unwrap()
        .records
        .iter()
        .find(|r| r.revision == reconstruction.request.current.request_sha256)
        .unwrap()
        .utf8
        .as_bytes();
    std::fs::write(out.join("request.json"), request).unwrap();
    std::fs::write(out.join("card.json"), &reconstruction.card_bytes).unwrap();
    std::fs::write(out.join("reconstruction.json"), &bytes).unwrap();
    std::fs::write(out.join("target.txt"), b"# policy\nsynthetic\n").unwrap();
    // Corrupted frozen-object bytes refuse; register semantics are tested separately.
    let mut broken = reconstruction.clone();
    for o in &mut broken.objects {
        if o.kind == "blob" && String::from_utf8_lossy(&o.bytes).contains("decision_id") {
            o.bytes = b"invalid register".to_vec();
            break;
        }
    }
    assert!(
        card_host::reconstruct(&broken, &revision(&serde_json::to_vec(&broken).unwrap())).is_err()
    );
    println!(
        "request={} card={} reconstruction={} corrupted-object=refused",
        request.len(),
        reconstruction.card_bytes.len(),
        bytes.len()
    );
}

#[test]
fn invalid_d6_register_refuses_before_card_composition() {
    let (f, mut request, profile, _installation, args) = setup();
    // Publish a real, hash-consistent Git commit with duplicate decision IDs.
    // This exercises register semantics before any use/card reconstruction.
    let decisions = f.repo.join("reg/decisions.json");
    let mut records: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&decisions).unwrap()).unwrap();
    let duplicate = records[0].clone();
    records.as_array_mut().unwrap().push(duplicate);
    std::fs::write(&decisions, serde_json::to_vec(&records).unwrap()).unwrap();
    f.fx.git(&f.repo, &["add", "reg/decisions.json"]);
    f.fx.git(
        &f.repo,
        &["commit", "-qm", "synthetic invalid D6 duplicate decision"],
    );
    request.source.commit = f.fx.git(&f.repo, &["rev-parse", "HEAD"]);
    let snapshot = research_register::PinnedSnapshot::build(
        research_register::GitObjects::open(&f.repo).unwrap(),
        &request.source.commit,
        &request.source.register_path,
    )
    .unwrap();
    assert!(matches!(
        research_register::validate(&snapshot).unwrap(),
        research_register::Outcome::Invalid(_)
    ));
    let input = serde_json::json!({"request":request,"profile":profile,"producer":revision(b"synthetic-invalid-register-control"),"installation":{"issuers":args.0,"delegation":args.1,"purpose":args.2,"scope":args.3,"receipts":args.4,"role_issuers":null}});
    let result = card_host::synthetic_reconstruction(serde_json::from_value(input).unwrap());
    assert_eq!(result.err().as_deref(), Some("invalid_register"));
    println!(
        "D6 hash-consistent invalid register -> invalid_register; no reconstruction/card returned"
    );
}
