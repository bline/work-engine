use base64::{Engine, engine::general_purpose::STANDARD};
use experiment_control::*;
use serde_json::{Value, json};
use std::collections::BTreeSet;

fn who(name: &str) -> Identity {
    Identity {
        host: "synthetic".into(),
        session: name.into(),
        agent: None,
    }
}
fn ev() -> Evidence {
    Evidence {
        reference: "synthetic-observation".into(),
        sha256: "0".repeat(64),
    }
}
fn set(xs: &[&str]) -> BTreeSet<String> {
    xs.iter().map(|s| s.to_string()).collect()
}
fn fixture() -> Specification {
    strict::parse(include_bytes!("../examples/specification.json")).unwrap()
}
struct Run {
    state: Option<State>,
    n: usize,
    rev: String,
}
impl Run {
    fn new() -> Self {
        let mut r = Self {
            state: None,
            n: 0,
            rev: String::new(),
        };
        let out = r.ok(
            "owner",
            Command::Register {
                specification: fixture(),
            },
        );
        r.rev = out["spec"].as_str().unwrap().into();
        r
    }
    fn call(&mut self, actor: &str, mut command: Command) -> Result<Value, String> {
        if let Command::Acknowledge { receipt, .. } | Command::Dispose { receipt, .. } =
            &mut command
            && let Some(r) = self
                .state
                .as_ref()
                .and_then(|s| s.receipts.values().find(|r| r.local_id == *receipt))
        {
            *receipt = r.id.clone();
        }
        self.n += 1;
        let out = apply(Input {
            state: self.state.clone(),
            request: Request {
                id: format!("request-{}", self.n),
                actor: who(actor),
                observed_at: "synthetic-time".into(),
                command,
            },
        })?;
        self.state = Some(out.state);
        Ok(out.result)
    }
    fn ok(&mut self, actor: &str, command: Command) -> Value {
        self.call(actor, command).unwrap()
    }
    fn no(&mut self, actor: &str, command: Command) {
        let old = serde_json::to_value(&self.state).unwrap();
        assert!(self.call(actor, command).is_err());
        assert_eq!(serde_json::to_value(&self.state).unwrap(), old);
    }
    fn prepare(
        &mut self,
        h: &str,
        branch: &str,
        profile: &str,
        phase: &str,
        replaces: Option<&str>,
    ) {
        self.ok(
            "owner",
            Command::Prepare {
                handle: h.into(),
                spec: self.rev.clone(),
                phase: phase.into(),
                profile: profile.into(),
                unit: "unit".into(),
                branch: branch.into(),
                parent: None,
                replaces: replaces.map(str::to_owned),
                continuation: if replaces.is_some() {
                    set(&["handoff"])
                } else {
                    set(&[])
                },
            },
        );
    }
    fn verify(&mut self, h: &str, session: &str) {
        let receipt = format!("startup-{h}");
        let output = self.ok(
            "owner",
            Command::Startup {
                handle: h.into(),
                receipt: receipt.clone(),
            },
        );
        let ids = output["artifacts"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        self.ok(
            "owner",
            Command::Acknowledge {
                receipt,
                evidence: ev(),
            },
        );
        self.ok(
            "owner",
            Command::Bind {
                handle: h.into(),
                identity: who(session),
                exposure: Exposure {
                    identity_evidence: ev(),
                    observation_evidence: Some(ev()),
                    supplied_artifacts: ids,
                    observed_artifacts: output["artifacts"]
                        .as_object()
                        .unwrap()
                        .keys()
                        .cloned()
                        .collect(),
                    prior_bindings: set(&[]),
                    inherited_context: vec![],
                },
            },
        );
        self.ok(
            "owner",
            Command::Verify {
                handle: h.into(),
                evidence: ev(),
            },
        );
    }
    fn enroll(&mut self, h: &str, profile: &str, phase: &str) {
        self.prepare(h, h, profile, phase, None);
        self.verify(h, h);
        self.ok(
            "owner",
            Command::Activate {
                handle: h.into(),
                expected_generation: 0,
                settlement: None,
            },
        );
    }
    fn deliver(&mut self, h: &str, g: u64, ids: &[&str], receipt: &str) -> Result<Value, String> {
        self.call(
            h,
            Command::Deliver {
                handle: h.into(),
                generation: g,
                artifacts: set(ids),
                receipt: receipt.into(),
            },
        )
    }
    fn handoff(&mut self) {
        let a = Artifact {
            sha256: digest(b"continue unit"),
            data_base64: STANDARD.encode(b"continue unit"),
            provenance: "produced under active unit".into(),
            resource: "continuation-a".into(),
        };
        self.ok(
            "c",
            Command::Import {
                spec: self.rev.clone(),
                artifact_id: "handoff".into(),
                artifact: a,
                evidence: ev(),
                producer: Some("c".into()),
                generation: Some(1),
            },
        );
    }
}

#[test]
fn pinned_delivery_excludes_other_arm_and_metadata() {
    let mut r = Run::new();
    r.enroll("c", "control", "execution");
    r.enroll("t", "treatment", "execution");
    let out = r.deliver("c", 1, &["baseline"], "read1").unwrap();
    assert_eq!(
        STANDARD
            .decode(
                out["artifacts"]["baseline"]["data_base64"]
                    .as_str()
                    .unwrap()
            )
            .unwrap(),
        b"Frozen baseline instructions."
    );
    assert!(out.get("arm").is_none());
    assert!(out.get("spec").is_none());
    assert!(r.deliver("c", 1, &["treatment"], "read2").is_err());
    r.no(
        "foreign",
        Command::Deliver {
            handle: "t".into(),
            generation: 1,
            artifacts: set(&["treatment"]),
            receipt: "steal".into(),
        },
    );
    r.no(
        "c",
        Command::Inspect {
            spec: r.rev.clone(),
        },
    );
}
#[test]
fn revisions_do_not_retarget_existing_bindings() {
    let mut r = Run::new();
    r.enroll("c", "control", "execution");
    let old = r.rev.clone();
    let mut spec = fixture();
    let a = spec.artifacts.get_mut("baseline").unwrap();
    a.data_base64 = STANDARD.encode(b"updated global instructions");
    a.sha256 = digest(b"updated global instructions");
    let result = r.ok(
        "owner",
        Command::Amend {
            from_spec: old.clone(),
            specification: spec,
            evidence: ev(),
        },
    );
    assert_ne!(result["spec"], old);
    assert_eq!(r.state.as_ref().unwrap().bindings["c"].spec, old);
    let out = r.deliver("c", 1, &["baseline"], "read1").unwrap();
    assert_eq!(
        out["artifacts"]["baseline"]["sha256"],
        digest(b"Frozen baseline instructions.")
    );
}
#[test]
fn handoff_is_fenced_pending_output_blocks_it_and_pause_is_immediate() {
    let mut r = Run::new();
    r.enroll("c", "control", "execution");
    r.handoff();
    for h in ["next1", "next2"] {
        r.prepare(h, "c", "control", "execution", Some("c"));
        r.verify(h, h);
    }
    r.deliver("c", 1, &["baseline"], "pending").unwrap();
    r.no(
        "owner",
        Command::Activate {
            handle: "next1".into(),
            expected_generation: 1,
            settlement: Some(ev()),
        },
    );
    r.ok(
        "owner",
        Command::Pause {
            handle: "c".into(),
            evidence: ev(),
        },
    );
    assert!(r.deliver("c", 1, &["baseline"], "again").is_err());
    r.no(
        "next1",
        Command::Dispose {
            receipt: "pending".into(),
            evidence: ev(),
            disposition: "collected producer; emission unknown".into(),
        },
    );
    r.ok(
        "owner",
        Command::Dispose {
            receipt: "pending".into(),
            evidence: ev(),
            disposition: "producer collected; cohort marked uncertain".into(),
        },
    );
    r.ok(
        "owner",
        Command::Activate {
            handle: "next1".into(),
            expected_generation: 1,
            settlement: Some(ev()),
        },
    );
    r.no(
        "owner",
        Command::Activate {
            handle: "next2".into(),
            expected_generation: 1,
            settlement: Some(ev()),
        },
    );
    assert!(r.deliver("c", 1, &["baseline"], "stale").is_err());
    assert!(r.deliver("next1", 2, &["handoff"], "successor").is_ok());
}
#[test]
fn phase_closure_requires_preparations_and_outputs_settled_and_keeps_scoring() {
    let mut r = Run::new();
    r.prepare("pending", "pending", "control", "execution", None);
    r.enroll("judge", "reviewer", "scoring");
    r.no(
        "owner",
        Command::ClosePhase {
            spec: r.rev.clone(),
            phase: "execution".into(),
            evidence: ev(),
        },
    );
    r.ok(
        "owner",
        Command::Retire {
            handle: "pending".into(),
            evidence: ev(),
        },
    );
    r.ok(
        "owner",
        Command::ClosePhase {
            spec: r.rev.clone(),
            phase: "execution".into(),
            evidence: ev(),
        },
    );
    r.no(
        "owner",
        Command::Startup {
            handle: "pending".into(),
            receipt: "late".into(),
        },
    );
    assert!(r.deliver("judge", 1, &["result"], "score").is_ok());
}
#[test]
fn managed_mailbox_requires_both_permissions_and_preserves_cursor_on_handoff() {
    let mut r = Run::new();
    r.enroll("c", "control", "execution");
    r.enroll("other", "control", "execution");
    r.enroll("t", "treatment", "execution");
    let send = |target: &str, id: &str| Command::Send {
        handle: "other".into(),
        generation: 1,
        id: id.into(),
        targets: set(&[target]),
        resource: "channel-a".into(),
        data_base64: STANDARD.encode(b"allowed request"),
        provenance: "same-arm sender".into(),
    };
    r.no("other", send("t", "no"));
    r.ok("other", send("c", "yes"));
    let out = r.ok(
        "c",
        Command::Receive {
            handle: "c".into(),
            generation: 1,
            receipt: "messages".into(),
            limit: 10,
        },
    );
    assert_eq!(out["messages"].as_array().unwrap().len(), 1);
    r.no(
        "c",
        Command::Receive {
            handle: "c".into(),
            generation: 1,
            receipt: "duplicate".into(),
            limit: 10,
        },
    );
    r.ok(
        "c",
        Command::Acknowledge {
            receipt: "messages".into(),
            evidence: ev(),
        },
    );
    r.handoff();
    r.prepare("next", "c", "control", "execution", Some("c"));
    r.verify("next", "next");
    r.ok(
        "owner",
        Command::Activate {
            handle: "next".into(),
            expected_generation: 1,
            settlement: Some(ev()),
        },
    );
    let out = r.ok(
        "next",
        Command::Receive {
            handle: "next".into(),
            generation: 2,
            receipt: "empty".into(),
            limit: 10,
        },
    );
    assert_eq!(out["messages"], json!([]));
}
#[test]
fn observer_access_does_not_imply_feedback_permission() {
    let mut r = Run::new();
    r.enroll("obs", "observer", "scoring");
    assert!(
        r.deliver("obs", 1, &["baseline", "treatment"], "observe")
            .is_ok()
    );
    let result = r.ok(
        "obs",
        Command::Evaluate {
            handle: "obs".into(),
            generation: 1,
            action: Action::Send,
            resource: "channel-a".into(),
        },
    );
    assert_eq!(result["allowed"], false);
    assert_eq!(result["not_a_permit"], true);
}
#[test]
fn unknown_exposure_and_unverified_identity_cannot_activate() {
    let mut r = Run::new();
    r.prepare("c", "c", "control", "execution", None);
    r.no(
        "owner",
        Command::Activate {
            handle: "c".into(),
            expected_generation: 0,
            settlement: None,
        },
    );
    r.ok(
        "owner",
        Command::Startup {
            handle: "c".into(),
            receipt: "s".into(),
        },
    );
    r.ok(
        "owner",
        Command::Acknowledge {
            receipt: "s".into(),
            evidence: ev(),
        },
    );
    r.no(
        "owner",
        Command::Bind {
            handle: "c".into(),
            identity: who("c"),
            exposure: Exposure {
                identity_evidence: ev(),
                observation_evidence: None,
                supplied_artifacts: set(&["baseline", "rules"]),
                observed_artifacts: set(&[]),
                prior_bindings: set(&[]),
                inherited_context: vec![],
            },
        },
    );
}
#[test]
fn malformed_or_unsupported_spec_and_duplicate_keys_fail() {
    let mut s = fixture();
    s.coverage.get_mut("host_startup").unwrap().mode = CoverageMode::Managed;
    assert!(validate_spec(&s).is_err());
    s = fixture();
    s.artifacts.get_mut("treatment").unwrap().sha256 = "0".repeat(64);
    assert!(validate_spec(&s).is_err());
    assert!(strict::parse::<Value>(br#"{"outer":{"x":1,"x":2}}"#).is_err());
    assert!(strict::parse::<Input>(br#"{"state":null,"request":{},"unknown":true}"#).is_err());
}
#[test]
fn imports_never_replace_frozen_instructions() {
    let mut r = Run::new();
    r.enroll("c", "control", "execution");
    let a = fixture().artifacts["treatment"].clone();
    r.no(
        "c",
        Command::Import {
            spec: r.rev.clone(),
            artifact_id: "new".into(),
            artifact: a.clone(),
            evidence: ev(),
            producer: Some("c".into()),
            generation: Some(1),
        },
    );
    r.no(
        "owner",
        Command::Import {
            spec: r.rev.clone(),
            artifact_id: "baseline".into(),
            artifact: a,
            evidence: ev(),
            producer: None,
            generation: None,
        },
    );
}

#[test]
fn retired_parent_cannot_use_a_prepared_child_as_a_dispatch_bypass() {
    let mut r = Run::new();
    r.enroll("c", "control", "execution");
    r.ok(
        "c",
        Command::Prepare {
            handle: "child".into(),
            spec: r.rev.clone(),
            phase: "execution".into(),
            profile: "control".into(),
            unit: "child-unit".into(),
            branch: "child".into(),
            parent: Some("c".into()),
            replaces: None,
            continuation: set(&[]),
        },
    );
    r.ok(
        "owner",
        Command::Retire {
            handle: "c".into(),
            evidence: ev(),
        },
    );
    r.no(
        "c",
        Command::Startup {
            handle: "child".into(),
            receipt: "escape".into(),
        },
    );
}

#[test]
fn dispatch_permission_cannot_replace_an_unrelated_branch() {
    let mut r = Run::new();
    r.enroll("c", "control", "execution");
    r.enroll("other", "control", "execution");
    r.handoff();
    r.no(
        "c",
        Command::Prepare {
            handle: "takeover".into(),
            spec: r.rev.clone(),
            phase: "execution".into(),
            profile: "control".into(),
            unit: "unit".into(),
            branch: "other".into(),
            parent: Some("c".into()),
            replaces: Some("other".into()),
            continuation: set(&["handoff"]),
        },
    );
}

#[test]
fn reusing_known_identity_cannot_hide_prior_exposure() {
    let mut r = Run::new();
    r.enroll("c", "control", "execution");
    r.ok(
        "owner",
        Command::Retire {
            handle: "c".into(),
            evidence: ev(),
        },
    );
    r.prepare("new", "new", "control", "execution", None);
    r.ok(
        "owner",
        Command::Startup {
            handle: "new".into(),
            receipt: "s-new".into(),
        },
    );
    r.ok(
        "owner",
        Command::Acknowledge {
            receipt: "s-new".into(),
            evidence: ev(),
        },
    );
    let mut exposure = Exposure {
        identity_evidence: ev(),
        observation_evidence: Some(ev()),
        supplied_artifacts: set(&["baseline", "rules"]),
        observed_artifacts: set(&["baseline", "rules"]),
        prior_bindings: set(&[]),
        inherited_context: vec![],
    };
    r.no(
        "owner",
        Command::Bind {
            handle: "new".into(),
            identity: who("c"),
            exposure: exposure.clone(),
        },
    );
    exposure.prior_bindings = set(&["c"]);
    r.no(
        "owner",
        Command::Bind {
            handle: "new".into(),
            identity: who("c"),
            exposure,
        },
    );
}

#[test]
fn retirement_can_resume_only_its_last_owner_with_settlement() {
    let mut r = Run::new();
    r.enroll("c", "control", "execution");
    r.handoff();
    r.ok(
        "owner",
        Command::Retire {
            handle: "c".into(),
            evidence: ev(),
        },
    );
    r.prepare("successor", "c", "control", "execution", Some("c"));
    r.verify("successor", "successor");
    r.no(
        "owner",
        Command::Activate {
            handle: "successor".into(),
            expected_generation: 2,
            settlement: None,
        },
    );
    r.ok(
        "owner",
        Command::Activate {
            handle: "successor".into(),
            expected_generation: 2,
            settlement: Some(ev()),
        },
    );
    assert_eq!(r.state.as_ref().unwrap().branches["c"].generation, 3);
    assert!(r.deliver("c", 1, &["baseline"], "old").is_err());
}

#[test]
fn amendment_coexists_until_explicit_admission_seal_and_delegate_revocation() {
    let mut r = Run::new();
    let old = r.rev.clone();
    let mut sp = fixture();
    sp.delegates.insert(Authority::Admit, vec![who("launcher")]);
    r.rev = r.ok(
        "owner",
        Command::Amend {
            from_spec: old.clone(),
            specification: sp,
            evidence: ev(),
        },
    )["spec"]
        .as_str()
        .unwrap()
        .into();
    let new = r.rev.clone();
    r.rev = old.clone();
    r.prepare("old-admitted", "old-admitted", "control", "execution", None);
    r.ok(
        "owner",
        Command::SealAdmissions {
            spec: old.clone(),
            phase: "execution".into(),
            evidence: ev(),
        },
    );
    r.no(
        "owner",
        Command::Prepare {
            handle: "denied".into(),
            spec: old,
            phase: "execution".into(),
            profile: "control".into(),
            unit: "unit".into(),
            branch: "denied".into(),
            parent: None,
            replaces: None,
            continuation: set(&[]),
        },
    );
    // Already prepared membership is deliberately unchanged by the seal.
    r.verify("old-admitted", "old-admitted");
    let command = Command::Prepare {
        handle: "new".into(),
        spec: new.clone(),
        phase: "execution".into(),
        profile: "control".into(),
        unit: "unit".into(),
        branch: "new".into(),
        parent: None,
        replaces: None,
        continuation: set(&[]),
    };
    r.ok("launcher", command.clone());
    r.ok(
        "owner",
        Command::RevokeDelegate {
            spec: new,
            identity: who("launcher"),
            evidence: ev(),
        },
    );
    r.no(
        "launcher",
        Command::Startup {
            handle: "new".into(),
            receipt: "blocked".into(),
        },
    );
    r.ok(
        "owner",
        Command::Startup {
            handle: "new".into(),
            receipt: "allowed".into(),
        },
    );
}

#[test]
fn recovery_has_headroom_after_normal_capacity_is_exhausted() {
    let mut r = Run::new();
    r.enroll("c", "control", "execution");
    r.deliver("c", 1, &["baseline"], "uncollected").unwrap();
    // Seed the capacity boundary directly; this tests reducer limits, not 4,096 launches.
    let s = r.state.as_mut().unwrap();
    while s.audit.len() < 4096 {
        s.audit.push(Audit {
            id: format!("capacity-{}", s.audit.len()),
            actor: who("owner"),
            observed_at: "fixture".into(),
            command: json!({"fixture":"x".repeat(2200)}),
        });
    }
    assert!(serde_json::to_vec(s).unwrap().len() > MAX_BYTES);
    assert!(r.deliver("c", 1, &["baseline"], "full").is_err());
    r.ok(
        "owner",
        Command::Pause {
            handle: "c".into(),
            evidence: ev(),
        },
    );
    r.no(
        "owner",
        Command::Pause {
            handle: "c".into(),
            evidence: ev(),
        },
    );
    r.ok(
        "owner",
        Command::Dispose {
            receipt: "uncollected".into(),
            evidence: ev(),
            disposition: "unobserved; excluded".into(),
        },
    );
    r.ok(
        "owner",
        Command::Retire {
            handle: "c".into(),
            evidence: ev(),
        },
    );
    r.ok(
        "owner",
        Command::ClosePhase {
            spec: r.rev.clone(),
            phase: "execution".into(),
            evidence: ev(),
        },
    );
}

#[test]
fn receipt_and_message_ids_are_local_and_unauthorized_receipt_status_is_hidden() {
    let mut r = Run::new();
    r.enroll("c", "control", "execution");
    r.enroll("d", "control", "execution");
    let c = r.deliver("c", 1, &["baseline"], "same-local-id").unwrap();
    let d = r.deliver("d", 1, &["baseline"], "same-local-id").unwrap();
    assert_ne!(c["receipt"], d["receipt"]);
    let token = c["receipt"].as_str().unwrap().to_string();
    let command = Command::Acknowledge {
        receipt: token,
        evidence: ev(),
    };
    assert_eq!(
        r.call("d", command.clone()).unwrap_err(),
        "operation refused"
    );
    r.ok("c", command.clone());
    assert_eq!(r.call("d", command).unwrap_err(), "operation refused");
    for h in ["c", "d"] {
        r.ok(
            h,
            Command::Send {
                handle: h.into(),
                generation: 1,
                id: "same-message-id".into(),
                targets: set(&["c"]),
                resource: "channel-a".into(),
                data_base64: STANDARD.encode(b"message"),
                provenance: "fixture".into(),
            },
        );
    }
    assert_eq!(r.state.as_ref().unwrap().inboxes["c"].len(), 2);
}

#[test]
fn dispatch_does_not_authorize_parent_to_read_child_startup() {
    let mut r = Run::new();
    let mut sp = fixture();
    sp.profiles.get_mut("control").unwrap().grants.push(Grant {
        action: Action::Dispatch,
        resource: "control".into(),
    });
    // rules is in startup but there is no ongoing read grant for it.
    r.rev = r.ok(
        "owner",
        Command::Amend {
            from_spec: r.rev.clone(),
            specification: sp,
            evidence: ev(),
        },
    )["spec"]
        .as_str()
        .unwrap()
        .into();
    r.enroll("parent", "control", "execution");
    r.ok(
        "parent",
        Command::Prepare {
            handle: "child".into(),
            spec: r.rev.clone(),
            phase: "execution".into(),
            profile: "control".into(),
            unit: "child".into(),
            branch: "child".into(),
            parent: Some("parent".into()),
            replaces: None,
            continuation: set(&[]),
        },
    );
    r.no(
        "parent",
        Command::Startup {
            handle: "child".into(),
            receipt: "forbidden".into(),
        },
    );
    r.ok(
        "owner",
        Command::Startup {
            handle: "child".into(),
            receipt: "launcher".into(),
        },
    );
}

#[test]
fn delegated_inspection_is_phase_gated_scoped_and_audited() {
    let mut r = Run::new();
    let old = r.rev.clone();
    let mut sp = fixture();
    sp.delegates
        .insert(Authority::Inspect, vec![who("reviewer")]);
    r.rev = r.ok(
        "owner",
        Command::Amend {
            from_spec: old.clone(),
            specification: sp,
            evidence: ev(),
        },
    )["spec"]
        .as_str()
        .unwrap()
        .into();
    r.no(
        "reviewer",
        Command::Inspect {
            spec: r.rev.clone(),
        },
    );
    r.ok(
        "owner",
        Command::ClosePhase {
            spec: r.rev.clone(),
            phase: "execution".into(),
            evidence: ev(),
        },
    );
    let before = r.state.as_ref().unwrap().audit.len();
    let out = r.ok(
        "reviewer",
        Command::Inspect {
            spec: r.rev.clone(),
        },
    );
    assert!(out.get("specs").is_none());
    assert_eq!(r.state.as_ref().unwrap().audit.len(), before + 1);
    r.no("reviewer", Command::Inspect { spec: old });
}

#[test]
fn delegated_amendment_cannot_mint_authority_or_restore_revoked_delegates() {
    let mut r = Run::new();
    let mut sp = fixture();
    sp.delegates.insert(Authority::Amend, vec![who("editor")]);
    sp.delegates.insert(Authority::Admit, vec![who("launcher")]);
    r.rev = r.ok(
        "owner",
        Command::Amend {
            from_spec: r.rev.clone(),
            specification: sp.clone(),
            evidence: ev(),
        },
    )["spec"]
        .as_str()
        .unwrap()
        .into();
    r.ok(
        "owner",
        Command::RevokeDelegate {
            spec: r.rev.clone(),
            identity: who("launcher"),
            evidence: ev(),
        },
    );
    for field in ["delegates", "inspect_after", "coverage"] {
        let mut bad = sp.clone();
        match field {
            "delegates" => {
                bad.delegates
                    .insert(Authority::Inspect, vec![who("editor")]);
            }
            "inspect_after" => bad.inspect_after.clear(),
            _ => bad.coverage.get_mut("host_startup").unwrap().limitation = "changed claim".into(),
        }
        r.no(
            "editor",
            Command::Amend {
                from_spec: r.rev.clone(),
                specification: bad,
                evidence: ev(),
            },
        );
    }
    sp.authority_reference = "prospective experimental input amendment".into();
    r.rev = r.ok(
        "editor",
        Command::Amend {
            from_spec: r.rev.clone(),
            specification: sp,
            evidence: ev(),
        },
    )["spec"]
        .as_str()
        .unwrap()
        .into();
    r.no(
        "launcher",
        Command::Prepare {
            handle: "forbidden".into(),
            spec: r.rev.clone(),
            phase: "execution".into(),
            profile: "control".into(),
            unit: "unit".into(),
            branch: "forbidden".into(),
            parent: None,
            replaces: None,
            continuation: set(&[]),
        },
    );
    r.prepare("owner-can", "owner-can", "control", "execution", None);
}

#[test]
fn sealing_admissions_also_blocks_replacement_preparation() {
    let mut r = Run::new();
    r.enroll("c", "control", "execution");
    r.handoff();
    r.ok(
        "owner",
        Command::SealAdmissions {
            spec: r.rev.clone(),
            phase: "execution".into(),
            evidence: ev(),
        },
    );
    r.no(
        "owner",
        Command::Prepare {
            handle: "new".into(),
            spec: r.rev.clone(),
            phase: "execution".into(),
            profile: "control".into(),
            unit: "unit".into(),
            branch: "c".into(),
            parent: None,
            replaces: Some("c".into()),
            continuation: set(&["handoff"]),
        },
    );
    r.ok(
        "owner",
        Command::Retire {
            handle: "c".into(),
            evidence: ev(),
        },
    );
}
