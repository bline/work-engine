//! Test-only staging exercise. The trusted input values and export join are
//! supplied by the fixture, not measured from a builder execution. This code
//! has no runtime reader, authority, effect gate, or dispatch permit.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::Path,
};

use review_episode_core::implementation_review_v1::SubjectParts;
use serde::Serialize;
use serde_json::json;

use crate::{
    CampaignIdentity, CandidateRef,
    codec::{campaign_digest, canonical_campaign, raw_sha256},
};

#[derive(PartialEq, Eq, Serialize)]
struct BuilderInput {
    baseline: String,
    objective_sha256: String,
    task_sha256: String,
    gate_sha256: String,
}

#[derive(Serialize)]
struct FixtureAnchor {
    // Independent test owner supplies this input before there is a checkpoint.
    run_id: String,
    slice_number: u64,
    plan_version: String,
    obligation: String,
    builder: BuilderInput,
    evaluator_sha256: String,
}

struct FixtureBuilderBinding {
    anchor_sha256: String,
    dispatch: String,
}

/// A test supplier's assertion standing in for the missing Host-checked export.
struct FixtureExport {
    builder_dispatch: String,
    parent: String,
    candidate: CandidateRef,
    host_gate_receipt_sha256: String,
}

impl FixtureExport {
    fn scope(&self) -> String {
        SubjectParts {
            commit: &self.candidate.commit,
            tree: &self.candidate.tree,
            patch_identity: &self.candidate.patch_identity,
        }
        .digest()
    }
}

// These are inputs from distinct test suppliers standing in for checked owner
// readbacks. They are not reader implementations or future record schemas.
#[derive(Clone)]
struct FixtureHostGate {
    anchor_sha256: String,
    dispatch: String,
    command_sha256: String,
    tested_tree: String,
    receipt_sha256: String,
    passed: bool,
}

#[derive(Clone)]
struct FixtureEngineeringVerdict {
    anchor_sha256: String,
    dispatch: String,
    scope_digest: String,
    export_receipt_sha256: String,
    host_gate_receipt_sha256: String,
    evaluator_sha256: String,
    passed: bool,
}

impl FixtureAnchor {
    fn sha256(&self) -> String {
        campaign_digest(&json!({"kind":"fixture-anchor-v1", "terms":self})).unwrap()
    }

    fn bind_input(
        &self,
        proposed: &BuilderInput,
        applied_dispatch: &str,
    ) -> Option<FixtureBuilderBinding> {
        if proposed != &self.builder || applied_dispatch.is_empty() {
            return None;
        }
        Some(FixtureBuilderBinding {
            anchor_sha256: self.sha256(),
            dispatch: applied_dispatch.into(),
        })
    }

    fn host_gate_matches(
        &self,
        input: &FixtureBuilderBinding,
        gate: &FixtureHostGate,
        export: &FixtureExport,
    ) -> bool {
        input.anchor_sha256 == self.sha256()
            && input.anchor_sha256 == gate.anchor_sha256
            && input.dispatch == gate.dispatch
            && input.dispatch == export.builder_dispatch
            && self.builder.baseline == export.parent
            && self.builder.gate_sha256 == gate.command_sha256
            && export.candidate.tree == gate.tested_tree
            && export.host_gate_receipt_sha256 == gate.receipt_sha256
            && gate.passed
    }

    // The engineering receipts join on the export, not on each other's
    // assertions. This is not final acceptance; no review is read here.
    fn engineering_receipts_match(
        &self,
        input: &FixtureBuilderBinding,
        gate: &FixtureHostGate,
        export: &FixtureExport,
        verdict: Option<&FixtureEngineeringVerdict>,
    ) -> bool {
        self.host_gate_matches(input, gate, export)
            && verdict.is_some_and(|v| {
                v.anchor_sha256 == input.anchor_sha256
                    && v.dispatch == input.dispatch
                    && v.scope_digest == export.scope()
                    && v.export_receipt_sha256 == export.candidate.receipt_sha256
                    && v.host_gate_receipt_sha256 == gate.receipt_sha256
                    && v.evaluator_sha256 == self.evaluator_sha256
                    && v.passed
            })
    }

    fn review_bytes(
        &self,
        input: &FixtureBuilderBinding,
        gate: &FixtureHostGate,
        export: &FixtureExport,
        review_root: &str,
        identity: &CampaignIdentity,
    ) -> Option<Vec<u8>> {
        if !self.host_gate_matches(input, gate, export)
            || self.run_id != identity.run_id
            || self.slice_number != identity.slice_number
            || self.plan_version != identity.plan_version
            || review_root.is_empty()
        {
            return None;
        }
        let candidate = &export.candidate;
        Some(
            canonical_campaign(&json!({
                "standing": "fixture-only-no-entry",
                "anchorSha256": input.anchor_sha256,
                "builderDispatch": input.dispatch,
                "candidateReceiptSha256": candidate.receipt_sha256,
                "physicalProfileSha256": candidate.physical_profile_sha256,
                "obligation": self.obligation,
                "hostGateReceiptSha256": gate.receipt_sha256,
                "scopeDigest": export.scope(),
                "reviewRoot": review_root,
                "identity": identity,
            }))
            .unwrap(),
        )
    }

    fn slot(&self, owner_dir: &Path) -> std::path::PathBuf {
        // Paths are derived from the pinned run/obligation, not from an offered
        // review root, dispatch identity, candidate or new attempt.
        let key = campaign_digest(&json!([
            "fixture-review-issuance-v1",
            self.run_id,
            self.obligation
        ]))
        .unwrap();
        owner_dir.join(key)
    }
}

#[derive(Debug, PartialEq, Eq)]
enum FixtureReadback {
    Published { sha256: String },
    ExactExisting { sha256: String },
    Conflict,
    Unknown,
}

fn compare_final(path: &Path, expected: &[u8]) -> io::Result<Option<FixtureReadback>> {
    match fs::read(path) {
        Ok(raw) if raw == expected => Ok(Some(FixtureReadback::ExactExisting {
            sha256: raw_sha256(expected),
        })),
        Ok(_) => Ok(Some(FixtureReadback::Conflict)),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err),
    }
}

/// Simulate the owner's create-once write, using an isolated test-owned directory.
/// Neither success nor exact reopen provides a capability. An interrupted write
/// leaves a staging inode and cannot start another publication.
fn publish_fixture(
    path: &Path,
    bytes: &[u8],
    interrupt_before_commit: bool,
) -> io::Result<FixtureReadback> {
    if let Some(result) = compare_final(path, bytes)? {
        return Ok(result);
    }
    let pending = path.with_extension("pending");
    let mut file = match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&pending)
    {
        Ok(file) => file,
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
            return Ok(FixtureReadback::Unknown);
        }
        Err(err) => return Err(err),
    };
    let committed = file.write_all(bytes).and_then(|()| file.sync_all()).is_ok()
        && !interrupt_before_commit
        && fs::hard_link(&pending, path).is_ok()
        && File::open(path.parent().unwrap())
            .and_then(|dir| dir.sync_all())
            .is_ok();
    if !committed {
        return Ok(FixtureReadback::Unknown);
    }
    match compare_final(path, bytes)? {
        Some(FixtureReadback::ExactExisting { sha256 }) => {
            Ok(FixtureReadback::Published { sha256 })
        }
        _ => Ok(FixtureReadback::Unknown),
    }
}

fn anchor() -> FixtureAnchor {
    FixtureAnchor {
        run_id: "test-run".into(),
        slice_number: 1,
        plan_version: "plan".into(),
        obligation: "review".into(),
        builder: BuilderInput {
            baseline: "a".repeat(40),
            objective_sha256: "b".repeat(64),
            task_sha256: "c".repeat(64),
            gate_sha256: "d".repeat(64),
        },
        evaluator_sha256: "5".repeat(64),
    }
}

fn host_gate(anchor: &FixtureAnchor, dispatch: &str, receipt_sha256: &str) -> FixtureHostGate {
    FixtureHostGate {
        anchor_sha256: anchor.sha256(),
        dispatch: dispatch.into(),
        command_sha256: anchor.builder.gate_sha256.clone(),
        tested_tree: "f".repeat(40),
        receipt_sha256: receipt_sha256.into(),
        passed: true,
    }
}

fn export() -> FixtureExport {
    FixtureExport {
        builder_dispatch: "builder-dispatch".into(),
        parent: "a".repeat(40),
        host_gate_receipt_sha256: "4".repeat(64),
        candidate: CandidateRef {
            commit: "e".repeat(40),
            tree: "f".repeat(40),
            patch_identity: "0".repeat(64),
            receipt_sha256: "1".repeat(64),
            physical_profile_sha256: "2".repeat(64),
        },
    }
}

fn engineering_verdict(
    anchor: &FixtureAnchor,
    export: &FixtureExport,
) -> FixtureEngineeringVerdict {
    FixtureEngineeringVerdict {
        anchor_sha256: anchor.sha256(),
        dispatch: export.builder_dispatch.clone(),
        scope_digest: export.scope(),
        export_receipt_sha256: export.candidate.receipt_sha256.clone(),
        host_gate_receipt_sha256: export.host_gate_receipt_sha256.clone(),
        evaluator_sha256: anchor.evaluator_sha256.clone(),
        passed: true,
    }
}

fn identity(attempt: &str) -> CampaignIdentity {
    CampaignIdentity {
        run_id: "test-run".into(),
        slice_number: 1,
        plan_version: "plan".into(),
        attempt_id: attempt.into(),
    }
}

#[test]
fn fixture_staging_binds_builder_terms_then_exact_dispatch_export_scope() {
    let anchor = anchor();
    let mut different = self::anchor().builder;
    different.gate_sha256 = "3".repeat(64);
    assert!(anchor.bind_input(&different, "builder-dispatch").is_none());
    let binding = anchor
        .bind_input(&anchor.builder, "builder-dispatch")
        .unwrap();
    let mut export = export();
    let gate = host_gate(&anchor, &binding.dispatch, &export.host_gate_receipt_sha256);
    let valid = anchor
        .review_bytes(&binding, &gate, &export, "root-a", &identity("initial"))
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&valid).unwrap();
    assert_ne!(value["scopeDigest"], export.candidate.patch_identity);
    export.candidate.commit = "8".repeat(40); // Identical patch/tree does not identify this checkpoint.
    let twin = anchor
        .review_bytes(&binding, &gate, &export, "root-a", &identity("initial"))
        .unwrap();
    let twin: serde_json::Value = serde_json::from_slice(&twin).unwrap();
    assert_ne!(value["scopeDigest"], twin["scopeDigest"]);
    export.parent = "3".repeat(40);
    assert!(
        anchor
            .review_bytes(&binding, &gate, &export, "root-a", &identity("initial"))
            .is_none()
    );
    export.parent = "a".repeat(40);
    export.builder_dispatch = "different-dispatch".into();
    assert!(
        anchor
            .review_bytes(&binding, &gate, &export, "root-a", &identity("initial"))
            .is_none()
    );
}

#[test]
fn fixture_staging_has_one_slot_across_roots_attempts_and_reopen() {
    let anchor = anchor();
    let binding = anchor
        .bind_input(&anchor.builder, "builder-dispatch")
        .unwrap();
    let repo = tempfile::tempdir().unwrap();
    let slot = anchor.slot(repo.path());
    let export = export();
    let gate = host_gate(&anchor, &binding.dispatch, &export.host_gate_receipt_sha256);
    let bytes = |root, attempt| {
        anchor
            .review_bytes(&binding, &gate, &export, root, &identity(attempt))
            .unwrap()
    };
    let original = bytes("root-a", "initial");
    let sha256 = raw_sha256(&original);
    assert_eq!(
        publish_fixture(&slot, &original, false).unwrap(),
        FixtureReadback::Published {
            sha256: sha256.clone()
        }
    );
    for (root, attempt) in [("root-b", "initial"), ("root-a", "different")] {
        assert_eq!(
            publish_fixture(&slot, &bytes(root, attempt), false).unwrap(),
            FixtureReadback::Conflict
        );
    }
    assert_eq!(
        publish_fixture(&slot, &original, false).unwrap(),
        FixtureReadback::ExactExisting { sha256 }
    );
    assert_eq!(fs::read(&slot).unwrap(), original);
}

#[test]
fn fixture_staging_interruption_does_not_mint_a_second_publication() {
    let anchor = anchor();
    let binding = anchor
        .bind_input(&anchor.builder, "builder-dispatch")
        .unwrap();
    let temp = tempfile::tempdir().unwrap();
    let slot = anchor.slot(temp.path());
    let export = export();
    let gate = host_gate(&anchor, &binding.dispatch, &export.host_gate_receipt_sha256);
    let original = anchor
        .review_bytes(&binding, &gate, &export, "root-a", &identity("initial"))
        .unwrap();
    let alternate = anchor
        .review_bytes(&binding, &gate, &export, "root-b", &identity("different"))
        .unwrap();
    assert_eq!(
        publish_fixture(&slot, &original, true).unwrap(),
        FixtureReadback::Unknown
    );
    assert_eq!(
        publish_fixture(&slot, &original, false).unwrap(),
        FixtureReadback::Unknown
    );
    assert_eq!(
        publish_fixture(&slot, &alternate, false).unwrap(),
        FixtureReadback::Unknown
    );
    assert!(compare_final(&slot, &original).unwrap().is_none());
    assert_eq!(fs::read(slot.with_extension("pending")).unwrap(), original);
}

#[test]
fn engineering_fixture_joins_gate_a_before_review_and_gate_b_only_on_exact_export() {
    let anchor = anchor();
    let mut a = export();
    let mut b = export();
    b.builder_dispatch = "other-dispatch".into();
    b.host_gate_receipt_sha256 = "6".repeat(64);
    b.candidate.receipt_sha256 = "7".repeat(64);
    // Independent dispatch/export fixtures intentionally share the same Git
    // tuple. Neither a tree match nor even matching S authorizes a crossed join.
    assert_eq!(a.scope(), b.scope());
    let bind = |e: &FixtureExport| {
        anchor
            .bind_input(&anchor.builder, &e.builder_dispatch)
            .unwrap()
    };
    let a_binding = bind(&a);
    let b_binding = bind(&b);
    let mut a_gate = host_gate(&anchor, &a_binding.dispatch, &a.host_gate_receipt_sha256);
    let b_gate = host_gate(&anchor, &b_binding.dispatch, &b.host_gate_receipt_sha256);
    let mut a_verdict = engineering_verdict(&anchor, &a);
    let b_verdict = engineering_verdict(&anchor, &b);
    let review = |gate: &FixtureHostGate, e: &FixtureExport| {
        anchor.review_bytes(&a_binding, gate, e, "root-a", &identity("initial"))
    };
    assert!(review(&a_gate, &a).is_some());
    assert!(review(&b_gate, &a).is_none());
    assert!(!anchor.engineering_receipts_match(&a_binding, &a_gate, &a, None));
    assert!(anchor.engineering_receipts_match(&a_binding, &a_gate, &a, Some(&a_verdict)));
    assert!(anchor.engineering_receipts_match(&b_binding, &b_gate, &b, Some(&b_verdict)));
    assert!(!anchor.engineering_receipts_match(&a_binding, &a_gate, &a, Some(&b_verdict)));

    a_gate.passed = false;
    assert!(review(&a_gate, &a).is_none());
    assert!(!anchor.engineering_receipts_match(&a_binding, &a_gate, &a, Some(&a_verdict)));
    a_gate.passed = true;
    a_gate.tested_tree = "3".repeat(40);
    assert!(review(&a_gate, &a).is_none());
    a_gate.tested_tree = a.candidate.tree.clone();
    a_verdict.passed = false;
    assert!(review(&a_gate, &a).is_some()); // Gate B is not consumed by review.
    assert!(!anchor.engineering_receipts_match(&a_binding, &a_gate, &a, Some(&a_verdict)));
    a_verdict.passed = true;
    a_verdict.evaluator_sha256 = "8".repeat(64);
    assert!(!anchor.engineering_receipts_match(&a_binding, &a_gate, &a, Some(&a_verdict)));
    a_verdict.evaluator_sha256 = anchor.evaluator_sha256.clone();
    a.candidate.commit = "9".repeat(40);
    assert!(review(&a_gate, &a).is_some()); // Gate A tested a tree, not this commit.
    assert!(!anchor.engineering_receipts_match(&a_binding, &a_gate, &a, Some(&a_verdict)));
}

#[test]
fn review_and_engineering_joins_require_each_host_gate_binding() {
    let anchor = anchor();
    let export = export();
    let binding = anchor
        .bind_input(&anchor.builder, &export.builder_dispatch)
        .unwrap();
    let gate = host_gate(&anchor, &binding.dispatch, &export.host_gate_receipt_sha256);
    let verdict = engineering_verdict(&anchor, &export);
    let review = |gate: &FixtureHostGate| {
        anchor.review_bytes(&binding, gate, &export, "root-a", &identity("initial"))
    };
    assert!(review(&gate).is_some());
    assert!(anchor.engineering_receipts_match(&binding, &gate, &export, Some(&verdict)));

    // Change exactly one reported fact, leaving the independent pins unchanged.
    for (field, wrong) in [
        (
            "anchor",
            FixtureHostGate {
                anchor_sha256: "9".repeat(64),
                ..gate.clone()
            },
        ),
        (
            "dispatch",
            FixtureHostGate {
                dispatch: "other-dispatch".into(),
                ..gate.clone()
            },
        ),
        (
            "command",
            FixtureHostGate {
                command_sha256: "9".repeat(64),
                ..gate.clone()
            },
        ),
        (
            "receipt",
            FixtureHostGate {
                receipt_sha256: "9".repeat(64),
                ..gate.clone()
            },
        ),
    ] {
        assert!(
            review(&wrong).is_none(),
            "review accepted wrong Gate A {field}"
        );
        assert!(
            !anchor.engineering_receipts_match(&binding, &wrong, &export, Some(&verdict)),
            "engineering join accepted wrong Gate A {field}",
        );
    }
}

#[test]
fn engineering_join_requires_each_verdict_binding_on_the_same_export() {
    let anchor = anchor();
    let export = export();
    let binding = anchor
        .bind_input(&anchor.builder, &export.builder_dispatch)
        .unwrap();
    let gate = host_gate(&anchor, &binding.dispatch, &export.host_gate_receipt_sha256);
    let verdict = engineering_verdict(&anchor, &export);
    let join = |verdict: &FixtureEngineeringVerdict| {
        anchor.engineering_receipts_match(&binding, &gate, &export, Some(verdict))
    };
    assert!(join(&verdict));

    for (field, wrong) in [
        (
            "anchor",
            FixtureEngineeringVerdict {
                anchor_sha256: "9".repeat(64),
                ..verdict.clone()
            },
        ),
        (
            "dispatch",
            FixtureEngineeringVerdict {
                dispatch: "other-dispatch".into(),
                ..verdict.clone()
            },
        ),
        (
            "export receipt",
            FixtureEngineeringVerdict {
                export_receipt_sha256: "9".repeat(64),
                ..verdict.clone()
            },
        ),
        (
            "Host gate receipt",
            FixtureEngineeringVerdict {
                host_gate_receipt_sha256: "9".repeat(64),
                ..verdict.clone()
            },
        ),
    ] {
        assert!(
            !join(&wrong),
            "engineering join accepted wrong Gate B {field}"
        );
    }
}
