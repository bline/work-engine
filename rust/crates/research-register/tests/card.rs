#![cfg(feature = "synthetic-card-host")]
#[path = "card_fixture.rs.inc"]
mod fixture;
use fixture::Fixture;
use research_register::{
    card_admission::{ApprovalEvent, DisclosureScope, Installation},
    card_host::{self, CardRequest},
    card_wire::{self, CurrentCardRequest},
    read_host::{HostProfile, ReadRequest},
    use_bundle::{Record, revision},
};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
    time::Instant,
};
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
fn cgroup() -> PathBuf {
    std::env::var_os("RESEARCH_READ_CGROUP_ROOT")
        .map(PathBuf::from)
        .expect("delegated Linux cgroup required")
}
fn run(q: CardRequest, p: HostProfile, i: &Installation) -> Result<card_host::Product, String> {
    card_host::run(
        Path::new(env!("CARGO_BIN_EXE_work-engine-research-card")),
        &cgroup(),
        q,
        p,
        i,
        &AtomicBool::new(false),
        Instant::now(),
    )
}
#[test]
#[ignore = "requires delegated Linux cgroup"]
fn actual_current_card_and_pinned_reconstruction() {
    let (f, q, p, i, _args) = setup();
    let out = run(q.clone(), p.clone(), &i).unwrap();
    let env = card_wire::decode(out.envelope_utf8.as_bytes()).unwrap();
    let c = env.card.unwrap();
    assert_eq!(c.delivered.len(), 1);
    assert_eq!(c.claims.status, "empty");
    assert_eq!(c.assessments.len(), 1);
    assert_eq!(c.qualifications.len(), 1);
    assert!(c.records.iter().any(|r| r.utf8.contains("not_assessed")));
    assert!(c.records.iter().any(|r| r.utf8.contains("unknown")));
    let basis = serde_json::to_vec(&out.reconstruction).unwrap();
    let digest = revision(&basis);
    std::fs::remove_dir_all(&f.repo).unwrap();
    assert_eq!(
        card_host::reconstruct(&out.reconstruction, &digest).unwrap(),
        out.envelope_utf8.as_bytes()
    );
    let mut missing = out.reconstruction.clone();
    missing.objects.pop();
    assert!(card_host::reconstruct(&missing, &digest).is_err());
    let new_digest = revision(&serde_json::to_vec(&missing).unwrap());
    assert!(card_host::reconstruct(&missing, &new_digest).is_err());
    let mut changed = out.reconstruction;
    changed.objects[0].bytes.push(0);
    let h = revision(&serde_json::to_vec(&changed).unwrap());
    assert!(card_host::reconstruct(&changed, &h).is_err());
}
#[test]
#[ignore = "requires delegated Linux cgroup"]
fn authority_and_request_refusals_are_discriminating() {
    let (_f, q, p, i, args) = setup();
    assert!(run(q.clone(), p.clone(), &i).is_ok());
    assert_eq!(
        run(q.clone(), p.clone(), &Installation::unavailable())
            .err()
            .unwrap(),
        "authority_unavailable"
    );
    for variant in ["issuer", "delegation", "purpose", "scope", "request"] {
        // Recreate explicit synthetic installation with independently changed trust configuration.
        let (issuers, delegation, purpose, scope, receipts) = args.clone();
        let mut scope = scope;
        let mut issuers = issuers;
        let mut delegation = delegation;
        let mut purpose = purpose;
        let mut request = q.clone();
        match variant {
            "issuer" => issuers = BTreeSet::from(["other".into()]),
            "delegation" => delegation = "other".into(),
            "purpose" => purpose = "other".into(),
            "scope" => {
                scope.finding_ids.clear();
            }
            _ => request.current.request_sha256 = "0".repeat(64),
        }
        let other = Installation::synthetic(issuers, delegation, purpose, scope, receipts);
        assert_eq!(
            run(request, p.clone(), &other).err().unwrap(),
            "authority_mismatch",
            "{variant}"
        );
        assert!(run(q.clone(), p.clone(), &i).is_ok());
    }
    let mut unknown = q.clone();
    unknown.current.decision_revision = "git:0000000".into();
    assert_eq!(
        run(unknown, p.clone(), &i).err().unwrap(),
        "revision_unknown"
    );
    let mut missing = q.clone();
    missing.current.decision_id = "no-decision".into();
    assert_eq!(run(missing, p.clone(), &i).err().unwrap(), "not_registered");
    let mut version = q.clone();
    version.codec = "other".into();
    assert_eq!(
        run(version, p.clone(), &i).err().unwrap(),
        "unsupported_version"
    );
    let mut output = p;
    output.output_bytes = 64;
    assert_eq!(run(q, output, &i).err().unwrap(), "resource_exceeded");
}
fn edit(f: &mut Fixture, path: &str, mutate: impl FnOnce(&mut serde_json::Value)) {
    let path = f.repo.join(path);
    let mut v = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    mutate(&mut v);
    std::fs::write(path, serde_json::to_vec(&v).unwrap()).unwrap();
}
fn commit_source(f: &mut Fixture) {
    f.fx.git(&f.repo, &["add", "."]);
    f.fx.git(&f.repo, &["commit", "-qm", "synthetic source changes"]);
    f.source = f.fx.git(&f.repo, &["rev-parse", "HEAD"]);
    let study = serde_json::to_value(f.pin(&f.source, "reg/studies/alpha.json")).unwrap();
    let target = serde_json::to_value(f.pin(&f.source, "policy.md")).unwrap();
    let dec: serde_json::Value =
        serde_json::from_slice(&std::fs::read(f.repo.join("reg/decisions.json")).unwrap()).unwrap();
    let rev = dec[0]["revision"].clone();
    for (_, doc) in &mut f.docs {
        match doc["record"]["kind"].as_str().unwrap() {
            "subject" => doc["record"]["study"] = study.clone(),
            "request" => {
                doc["record"]["target"] = target.clone();
                doc["record"]["decision_revision"] = rev.clone();
            }
            "applicability" => doc["record"]["decision_revision"] = rev.clone(),
            _ => {}
        }
    }
}
#[test]
#[ignore = "requires delegated Linux cgroup"]
fn correction_objection_competing_assessments_and_notices() {
    use serde_json::json;
    let (_f, q, p, i, _) = setup_with(|f| {
        let prior: serde_json::Value =
            serde_json::from_slice(&std::fs::read(f.repo.join("reg/decisions.json")).unwrap())
                .unwrap();
        let old = prior[0]["revision"].as_str().unwrap().to_string();
        let now = format!("git:{}", f.source);
        edit(f, "reg/studies/alpha.json", |v| {
            let mut two = v["findings"][0].clone();
            two["id"] = "alpha#two".into();
            two["claim"] = "corrective evidence".into();
            let mut objection = two.clone();
            objection["id"] = "alpha#objection".into();
            objection["claim"] = "MUST NOT DELIVER AS FINDING".into();
            v["findings"]
                .as_array_mut()
                .unwrap()
                .extend([two, objection]);
        });
        edit(f, "reg/decisions.json", |v| {
            v[0]["revision"] = now.clone().into();
            v[0]["revisions"] = json!([{"revision":old,"anchor":"policy","superseded_at":"2026-01-02","reason":"synthetic prior"}]);
            v[0]["historical"] = json!({"reason":"no inferred successor at text.md","marked_by":"owner","marked_at":"2026-01-02"});
            let mut other = v[0].clone();
            other["decision_id"] = "d-other".into();
            other["findings"] = json!(["alpha#objection"]);
            v.as_array_mut().unwrap().push(other);
        });
        let source =
            json!({"path":"policy.md","commit":f.source,"blob":f.pin(&f.source,"policy.md").blob});
        let events = [
            json!({"event_id":"ev-correction","kind":"correction","date":"2026-01-02","subject_finding":"alpha#one","by_finding":"alpha#two","basis":"correction retains both","source":source}),
            json!({"event_id":"ev-objection","kind":"challenge","date":"2026-01-02","subject_finding":"alpha#two","by_finding":"alpha#objection","basis":"material objection stays visible","source":source}),
        ];
        std::fs::write(
            f.repo.join("reg/events.jsonl"),
            events.iter().map(|v| format!("{v}\n")).collect::<String>(),
        )
        .unwrap();
        let app = |id: &str, decision: &str, rev: &str, status: &str, finding: &str| json!({"applicability_id":id,"finding":finding,"decision_id":decision,"decision_revision":rev,"relation":"not_established","rationale":"synthetic no support","evidence":[source],"proposed_by":"author","proposed_at":"2026-01-02","authorization":{"status":status,"owner":"synthetic owner","at":"2026-01-02"}});
        let rows = [
            app("ap-pending", "d-one", &now, "pending", "alpha#objection"),
            app("ap-declined", "d-one", &now, "declined", "alpha#objection"),
            app("ap-old", "d-one", &old, "declined", "alpha#one"),
            app("ap-other", "d-other", &now, "declined", "alpha#one"),
        ];
        std::fs::write(
            f.repo.join("reg/applicability.jsonl"),
            rows.iter().map(|v| format!("{v}\n")).collect::<String>(),
        )
        .unwrap();
        std::fs::write(f.repo.join("reg/materialized.jsonl"),"{\"finding\":\"alpha#two\",\"claim_id\":\"c\",\"revision_id\":\"r\",\"revision_sha256\":\"s\"}\n").unwrap();
        let original = f
            .docs
            .iter()
            .find(|(_, v)| v["record"]["kind"] == "assessment")
            .unwrap()
            .1["record"]
            .clone();
        let original_id = f.original_ids["assessment.json"].clone();
        let mut competing = original;
        competing["assessor"] = "competing assessor".into();
        competing["competes_with"] = json!([original_id]);
        competing["facets"]["consistency"]["state"] = "contested".into();
        f.add("competing", competing);
        commit_source(f);
    });
    let product = run(q.clone(), p.clone(), &i).unwrap();
    let c = card_wire::decode(product.envelope_utf8.as_bytes())
        .unwrap()
        .card
        .unwrap();
    assert_eq!(
        c.delivered
            .iter()
            .map(|d| d.finding.as_str())
            .collect::<Vec<_>>(),
        ["alpha#one", "alpha#two"]
    );
    assert_eq!(c.assessments.len(), 2);
    assert!(c.records.iter().any(|r| r.utf8.contains("contested")));
    assert_eq!(c.delivered[1].events.len(), 2);
    assert!(
        !product
            .envelope_utf8
            .contains("MUST NOT DELIVER AS FINDING")
    );
    assert_eq!(
        c.not_delivered.pending[0].notice.as_deref(),
        Some(research_register::card::PENDING)
    );
    assert_eq!(
        c.not_delivered.declined[0].notice.as_deref(),
        Some(research_register::card::DECLINED)
    );
    assert!(
        c.not_delivered.other_revision[0]
            .notice
            .as_ref()
            .unwrap()
            .contains("status: declined")
    );
    assert_eq!(c.not_delivered.cross_decision.len(), 1);
    assert_eq!(c.not_delivered.cross_decision[0].notice, None);
    assert_eq!(c.claims.status, "unavailable");
    assert_eq!(c.claims.row_count, 1);
    assert_eq!(
        c.historical_utf8.as_deref(),
        Some(
            "{\"marked_at\":\"2026-01-02\",\"marked_by\":\"owner\",\"reason\":\"no inferred successor at text.md\"}"
        )
    );
    let basis = serde_json::to_vec(&product.reconstruction).unwrap();
    assert_eq!(
        card_host::reconstruct(&product.reconstruction, &revision(&basis)).unwrap(),
        product.envelope_utf8.as_bytes()
    );
    let mut historical = q;
    historical.current.decision_revision =
        c.not_delivered.other_revision[0].decision_revision.clone();
    assert_eq!(
        run(historical, p, &i).err().unwrap(),
        "historical_unsupported"
    );
}
#[test]
#[ignore = "requires delegated Linux cgroup and frozen repository objects"]
fn frozen_eleven_current_cases_and_six_request_controls() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap();
    let at = "8dbe427541dc3c29fb30dfb73cc0f7e81ffc7b3c";
    let objects = card_host::synthetic_source_copies(&repository, at, "metrics/register").unwrap();
    let snapshot = research_register::PinnedSnapshot::build(
        research_register::GitObjects::open(&repository).unwrap(),
        at,
        "metrics/register",
    )
    .unwrap();
    let research_register::Outcome::Valid(valid) = research_register::validate(&snapshot).unwrap()
    else {
        panic!()
    };
    let expected = std::collections::BTreeMap::from([
        ("context-boundary.h2-levels", (4, [0, 3, 0, 3])),
        ("dogfood-plan.round2-status-line", (1, [1, 0, 0, 0])),
        ("model-selection.builder-default", (1, [1, 0, 0, 1])),
        (
            "model-selection.claude-host-builder-rows",
            (2, [0, 0, 1, 0]),
        ),
        (
            "model-selection.claude-host-builder-rows-at-df288c3d",
            (2, [0, 0, 0, 1]),
        ),
        ("model-selection.cost-table-effort-row", (1, [0, 1, 0, 0])),
        ("model-selection.cost-table-quota-row", (1, [0, 0, 1, 0])),
        (
            "model-selection.terra-mechanical-builder",
            (1, [1, 0, 0, 1]),
        ),
        ("pi-coding-profile.acceptance-contract", (1, [1, 0, 0, 0])),
        (
            "pi-docker-convergence.context-replacement",
            (3, [0, 1, 0, 4]),
        ),
        ("roadmap.current-strategic-route", (1, [1, 0, 0, 0])),
    ]);
    for d in &valid.records().decisions {
        let (f, q, p, i, _) = setup_with(|f| {
            for o in &objects {
                let actual = f.fx.git_with(
                    &f.repo,
                    &["hash-object", "-w", "--stdin", "-t", &o.kind],
                    Some(&o.bytes),
                    &[],
                );
                assert_eq!(actual, o.oid);
            }
            f.source = at.into();
            f.register_path = "metrics/register".into();
            let finding = d.findings.first().expect("frozen decision has finding");
            let study = valid
                .records()
                .studies
                .iter()
                .find(|s| s.findings.iter().any(|x| &x.id == finding))
                .unwrap();
            let sp =
                serde_json::to_value(f.pin(at, &format!("metrics/register/{}", study.at.file)))
                    .unwrap();
            let target = serde_json::to_value(f.pin(at, &d.document)).unwrap();
            for (_, doc) in &mut f.docs {
                match doc["record"]["kind"].as_str().unwrap() {
                    "subject" => {
                        doc["record"]["finding"] = finding.clone().into();
                        doc["record"]["study"] = sp.clone();
                    }
                    "request" => {
                        doc["record"]["decision"] = d.decision_id.clone().into();
                        doc["record"]["decision_revision"] = d.revision.text.clone().into();
                        doc["record"]["target"] = target.clone();
                    }
                    "applicability" => {
                        doc["record"]["decision"] = d.decision_id.clone().into();
                        doc["record"]["decision_revision"] = d.revision.text.clone().into();
                    }
                    _ => {}
                }
            }
        });
        let product = run(q.clone(), p.clone(), &i).unwrap();
        let c = card_wire::decode(product.envelope_utf8.as_bytes())
            .unwrap()
            .card
            .unwrap();
        let (delivered, groups) = expected
            .get(d.decision_id.as_str())
            .unwrap_or_else(|| panic!("unexpected frozen {}", d.decision_id));
        for e in &c.delivered {
            let memberships = e
                .membership
                .iter()
                .filter(|m| m.kind == "recorded_authorized_applicability")
                .map(|m| m.via.clone())
                .collect::<BTreeSet<_>>();
            let mut ids = vec![];
            for row in &e.legacy_applicability {
                let row: serde_json::Value = serde_json::from_str(row).unwrap();
                assert_eq!(row["finding"], e.finding);
                assert_eq!(row["decision_id"], q.current.decision_id);
                assert_eq!(row["decision_revision"], q.current.decision_revision);
                assert_eq!(row["authorization"]["status"], "authorized");
                assert!(row.get("relation").is_some());
                assert!(row.get("rationale").is_some());
                ids.push(row["applicability_id"].as_str().unwrap().to_string());
            }
            assert!(ids.windows(2).all(|w| w[0] < w[1]));
            assert_eq!(ids.into_iter().collect::<BTreeSet<_>>(), memberships);
        }
        assert_eq!(c.delivered.len(), *delivered, "{}", d.decision_id);
        assert_eq!(
            [
                c.not_delivered.pending.len(),
                c.not_delivered.declined.len(),
                c.not_delivered.other_revision.len(),
                c.not_delivered.cross_decision.len()
            ],
            *groups,
            "{}",
            d.decision_id
        );
        assert_eq!(
            card_host::reconstruct(
                &product.reconstruction,
                &revision(&serde_json::to_vec(&product.reconstruction).unwrap())
            )
            .unwrap(),
            product.envelope_utf8.as_bytes()
        );
        // Pin semantic goldens and complete reconstruction fixtures when explicitly requested
        // by the qualification runner. Normal tests only compare the owned frozen golden.
        if let Some(dir) = std::env::var_os("RESEARCH_CARD_GOLDEN_OUT") {
            let dir = PathBuf::from(dir);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join(format!("{}.json", d.decision_id)),
                &product.envelope_utf8,
            )
            .unwrap();
            std::fs::write(
                dir.join(format!("{}.basis.json", d.decision_id)),
                serde_json::to_vec(&product.reconstruction).unwrap(),
            )
            .unwrap();
        }
        if d.decision_id == "model-selection.cost-table-quota-row" {
            // X01 is the explicit same-current revision and therefore the same request bytes.
            assert_eq!(
                run(q.clone(), p.clone(), &i).unwrap().envelope_utf8,
                product.envelope_utf8
            );
            for (id, rev, outcome) in [
                (
                    d.decision_id.as_str(),
                    "git:1a1cb443",
                    "historical_unsupported",
                ),
                (d.decision_id.as_str(), "git:1a1cb44", "revision_unknown"),
                (
                    "model-selection.no-such-decision",
                    d.revision.text.as_str(),
                    "not_registered",
                ),
            ] {
                let mut request = q.clone();
                request.current.decision_id = id.into();
                request.current.decision_revision = rev.into();
                assert_eq!(run(request, p.clone(), &i).err().unwrap(), outcome);
            }
        }
        if d.decision_id == "model-selection.claude-host-builder-rows" {
            for (rev, outcome) in [
                ("git:4ff69ffd", "historical_unsupported"),
                ("git:df288c3d", "revision_unknown"),
            ] {
                let mut request = q.clone();
                request.current.decision_revision = rev.into();
                assert_eq!(run(request, p.clone(), &i).err().unwrap(), outcome);
            }
        }
        drop(f);
    }
}
#[test]
#[ignore = "requires delegated Linux cgroup"]
fn requested_gap_does_not_follow_other_decisions_null() {
    let (f, q, p, i, _) = setup_with(|f| {
        std::fs::write(f.repo.join("gone.md"), "# gone\n").unwrap();
        f.fx.git(&f.repo, &["add", "."]);
        f.fx.git(&f.repo, &["commit", "-qm", "synthetic second target"]);
        let rev = format!("git:{}", f.fx.git(&f.repo, &["rev-parse", "HEAD"]));
        edit(f, "reg/decisions.json", |v| {
            let mut other = v[0].clone();
            other["decision_id"] = "d-null".into();
            other["document"] = "gone.md".into();
            other["anchor"] = "gone".into();
            other["revision"] = rev.into();
            v.as_array_mut().unwrap().push(other);
        });
        std::fs::remove_file(f.repo.join("gone.md")).unwrap();
        commit_source(f);
    });
    assert!(run(q.clone(), p.clone(), &i).is_ok());
    let dec: serde_json::Value =
        serde_json::from_slice(&std::fs::read(f.repo.join("reg/decisions.json")).unwrap()).unwrap();
    let mut gap = q.clone();
    gap.current.decision_id = "d-null".into();
    gap.current.decision_revision = dec[1]["revision"].as_str().unwrap().into();
    assert_eq!(run(gap, p.clone(), &i).err().unwrap(), "anchor_gap");
    let mut bad = q.clone();
    bad.source.bundle.sha256 = "0".repeat(64);
    assert_eq!(run(bad, p.clone(), &i).err().unwrap(), "invalid_bundle");
    assert!(run(q.clone(), p.clone(), &i).is_ok());
    // Invalid register dominates even though a requested anchor gap is also present.
    std::fs::remove_file(f.repo.join("reg/materialized.jsonl")).unwrap();
    f.fx.git(&f.repo, &["add", "."]);
    f.fx.git(&f.repo, &["commit", "-qm", "synthetic invalid register"]);
    let mut invalid = q.clone();
    invalid.source.commit = f.fx.git(&f.repo, &["rev-parse", "HEAD"]);
    assert_eq!(
        run(invalid, p.clone(), &i).err().unwrap(),
        "invalid_register"
    );
    let id = &q.source.commit;
    std::fs::remove_file(f.repo.join(".git/objects").join(&id[..2]).join(&id[2..])).unwrap();
    assert_eq!(run(q, p, &i).err().unwrap(), "unreadable");
}
#[test]
#[ignore = "requires delegated Linux cgroup"]
fn honest_disclosure_refusal_and_unforgeable_request_shape() {
    let (f, q, p, i, args) = setup();
    assert!(run(q.clone(), p.clone(), &i).is_ok());
    let (issuers, delegation, purpose, mut scope, receipts) = args;
    scope.finding_ids.clear();
    let mut narrow = vec![];
    for (pin, bytes) in &receipts {
        let mut e: ApprovalEvent = serde_json::from_slice(bytes).unwrap();
        e.scope_sha256 = revision(&serde_json::to_vec(&scope).unwrap());
        let b = serde_json::to_vec(&e).unwrap();
        let pin = f.publish_raw(&pin.path, &b);
        narrow.push((pin, b));
    }
    let restricted = Installation::synthetic(
        issuers.clone(),
        delegation.clone(),
        purpose.clone(),
        scope,
        narrow,
    );
    assert_eq!(
        run(q.clone(), p.clone(), &restricted).err().unwrap(),
        "coverage_unavailable"
    );
    let mut forged = receipts;
    forged[0].1[0] = b'!';
    let forged = Installation::synthetic(
        issuers,
        delegation,
        purpose,
        DisclosureScope {
            decision_ids: BTreeSet::from(["d-one".into()]),
            finding_ids: BTreeSet::from(["alpha#one".into()]),
            cross_decision_metadata: true,
        },
        forged,
    );
    assert_eq!(
        run(q.clone(), p.clone(), &forged).err().unwrap(),
        "authority_mismatch"
    );
    let mut raw = serde_json::to_value(&q).unwrap();
    for k in ["installation", "verifier", "key", "owner", "authorization"] {
        raw[k] = true.into();
        assert!(serde_json::from_value::<CardRequest>(raw.clone()).is_err());
        raw.as_object_mut().unwrap().remove(k);
    }
    assert!(run(q, p, &i).is_ok());
}
#[test]
fn public_cli_bounds_input_deadline_and_cancellation() {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let dir = std::env::temp_dir().join(format!("card-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let profile = HostProfile {
        wall_millis: 150,
        ..HostProfile::default()
    };
    let path = dir.join("profile.json");
    std::fs::write(&path, serde_json::to_vec(&profile).unwrap()).unwrap();
    for mode in ["oversize", "deadline", "cancel"] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_work-engine-research-card"))
            .arg("/unavailable/cgroup")
            .arg(&path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut held_stdin = child.stdin.take().unwrap();
        if mode == "oversize" {
            held_stdin.write_all(&vec![b'x'; 17000]).unwrap();
        } else if mode == "cancel" {
            std::thread::sleep(std::time::Duration::from_millis(30));
            unsafe {
                libc::kill(child.id() as i32, libc::SIGTERM);
            }
        }
        let output = child.wait_with_output().unwrap();
        drop(held_stdin);
        let env = card_wire::decode(&output.stdout).unwrap();
        assert_eq!(
            env.outcome,
            if mode == "cancel" {
                "cancelled"
            } else {
                "resource_exceeded"
            }
        );
        assert!(env.card.is_none());
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
#[ignore = "requires delegated Linux cgroup"]
fn typed_closure_respects_disclosure_scope() {
    use serde_json::json;
    for typed_dependency in [false, true] {
        let (f, q, p, full_scope, args) = setup_with(|f| {
            edit(f, "reg/studies/alpha.json", |v| {
                let mut other = v["findings"][0].clone();
                other["id"] = "alpha#excluded".into();
                other["claim"] = "EXCLUDED CONTENT".into();
                v["findings"].as_array_mut().unwrap().push(other);
            });
            if typed_dependency {
                let source = json!({"path":"policy.md","commit":f.source,"blob":f.pin(&f.source,"policy.md").blob});
                let event = json!({"event_id":"excluded-objection","kind":"challenge","date":"2026-01-02","subject_finding":"alpha#one","by_finding":"alpha#excluded","basis":"material objection","source":source});
                std::fs::write(f.repo.join("reg/events.jsonl"), format!("{event}\n")).unwrap();
            }
            commit_source(f);
            let mut subject = f
                .docs
                .iter()
                .find(|(_, d)| d["record"]["kind"] == "subject")
                .unwrap()
                .1["record"]
                .clone();
            subject["finding"] = "alpha#excluded".into();
            let subject = f.add("excluded-subject", subject);
            let mut assessment = f
                .docs
                .iter()
                .find(|(_, d)| d["record"]["kind"] == "assessment")
                .unwrap()
                .1["record"]
                .clone();
            assessment["subject"] = subject.into();
            let excluded = f.add("excluded-assessment", assessment);
            let a = &mut f
                .docs
                .iter_mut()
                .find(|(n, _)| n == "assessment.json")
                .unwrap()
                .1["record"];
            a["scope"] = excluded.clone().into();
            a["facets"]["consistency"]["reason"] = excluded.clone().into();
        });
        assert!(run(q.clone(), p.clone(), &full_scope).is_ok());
        let (issuers, delegation, purpose, mut scope, receipts) = args;
        scope.finding_ids.remove("alpha#excluded");
        let mut narrow = vec![];
        for (pin, bytes) in receipts {
            let mut event: ApprovalEvent = serde_json::from_slice(&bytes).unwrap();
            event.scope_sha256 = revision(&serde_json::to_vec(&scope).unwrap());
            let bytes = serde_json::to_vec(&event).unwrap();
            narrow.push((f.publish_raw(&pin.path, &bytes), bytes));
        }
        let i = Installation::synthetic(issuers, delegation, purpose, scope, narrow);
        let result = run(q.clone(), p.clone(), &i);
        if typed_dependency {
            assert_eq!(result.err().unwrap(), "coverage_unavailable");
        } else {
            let env = card_wire::decode(result.unwrap().envelope_utf8.as_bytes()).unwrap();
            let card = env.card.unwrap();
            assert_eq!(card.assessments.len(), 1);
            assert!(
                !card
                    .records
                    .iter()
                    .any(|r| r.utf8.contains("alpha#excluded"))
            );
            assert_eq!(card.delivered.len(), 1);
        }
    }
}
#[test]
fn malformed_hash_public_cli_is_deterministic_refusal() {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };
    let (_f, q, p, _i, _) = setup();
    let dir = std::env::temp_dir().join(format!("malformed-card-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("profile.json");
    std::fs::write(&path, serde_json::to_vec(&p).unwrap()).unwrap();
    for hash in [
        "x".to_string(),
        "".into(),
        "G".repeat(64),
        "0".repeat(63),
        "0".repeat(65),
        "0".repeat(64),
    ] {
        let mut q = q.clone();
        q.current.request_sha256 = hash.clone();
        let mut prior = None;
        for _ in 0..2 {
            let mut child = Command::new(env!("CARGO_BIN_EXE_work-engine-research-card"))
                .arg("/unavailable/cgroup")
                .arg(&path)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            child
                .stdin
                .take()
                .unwrap()
                .write_all(&serde_json::to_vec(&q).unwrap())
                .unwrap();
            let out = child.wait_with_output().unwrap();
            assert_eq!(out.status.code(), Some(4));
            assert!(!String::from_utf8_lossy(&out.stderr).contains("panicked"));
            let env = card_wire::decode(&out.stdout).unwrap();
            assert!(env.card.is_none());
            if hash.len() == 64 && hash.chars().all(|c| c == '0') {
                assert_eq!(env.outcome, "controls_unavailable");
                assert!(env.request.is_some());
            } else {
                assert_eq!(env.outcome, "bad_request");
                assert!(env.request.is_none());
            }
            if let Some(prior) = prior {
                assert_eq!(out.stdout, prior);
            }
            prior = Some(out.stdout);
        }
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn unsupported_codec_public_cli_retains_valid_request_identity() {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };
    let (f, mut q, p, _i, _) = setup();
    q.codec = "unsupported.codec".into();
    let profile = f.fx.root.join("unsupported-codec-profile.json");
    std::fs::write(&profile, serde_json::to_vec(&p).unwrap()).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_work-engine-research-card"))
        .arg("/unavailable/cgroup")
        .arg(&profile)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&q).unwrap())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(4));
    let env = card_wire::decode(&out.stdout).unwrap();
    assert_eq!(env.outcome, "unsupported_version");
    assert_eq!(env.request.as_ref(), Some(&q.current));
    assert!(env.card.is_none());
    assert!(!String::from_utf8_lossy(&out.stderr).contains("panicked"));
}

#[test]
fn admitted_startup_constructor_rejects_role_or_pinned_byte_changes() {
    // Controlled Git/source fixture: this does not authenticate actual owners.
    let (_fixture, _request, _profile, _, (_, delegation, purpose, scope, receipts)) = setup();
    let typed = receipts
        .iter()
        .map(|(pin, bytes)| {
            let event: ApprovalEvent = serde_json::from_slice(bytes).unwrap();
            (event.role, event.issuer, pin.clone(), bytes.clone())
        })
        .collect::<Vec<_>>();
    let installation = research_register::card_admission::validate_installation_records(
        delegation.clone(),
        purpose.clone(),
        scope.clone(),
        typed.clone(),
    )
    .unwrap();
    let _installation = Installation::install_from_admitted_startup(installation); // consumer qualification remains subject to its existing cgroup controls.
    let mut swapped = typed.clone();
    swapped[0].0 = swapped[1].0.clone();
    assert!(
        research_register::card_admission::validate_installation_records(
            delegation.clone(),
            purpose.clone(),
            scope.clone(),
            swapped
        )
        .is_err()
    );
    let mut changed = typed;
    changed[0].3.push(b' ');
    assert!(
        research_register::card_admission::validate_installation_records(
            delegation, purpose, scope, changed
        )
        .is_err()
    );
}
