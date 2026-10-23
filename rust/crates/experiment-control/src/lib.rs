//! Experiment policy and transitions. Storage, caller authentication and native
//! host startup are deliberately outside this domain. Output is private until
//! the adapter commits the returned state and serializes delivery with changes.
pub mod model;
pub mod strict;
use base64::{Engine, engine::general_purpose::STANDARD};
pub use model::*;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_BYTES: usize = 8 * 1024 * 1024;
pub const HARD_MAX_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_ARTIFACT_BYTES: usize = 512 * 1024;
type Result<T> = std::result::Result<T, String>;
pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn require(ok: bool, reason: &str) -> Result<()> {
    if ok { Ok(()) } else { Err(reason.into()) }
}
fn id(s: &str) -> Result<()> {
    require(
        !s.is_empty()
            && s.len() <= 128
            && s.bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"-_.".contains(&c)),
        "invalid identifier",
    )
}
fn text(s: &str) -> Result<()> {
    require(
        !s.trim().is_empty() && s.len() <= 512,
        "invalid bounded text",
    )
}
fn identity(i: &Identity) -> Result<()> {
    id(&i.host)?;
    id(&i.session)?;
    if let Some(a) = &i.agent {
        id(a)?;
    }
    Ok(())
}
fn evidence(e: &Evidence) -> Result<()> {
    text(&e.reference)?;
    require(
        e.sha256.len() == 64
            && e.sha256
                .bytes()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
        "invalid evidence digest",
    )
}
fn decoded(s: &str) -> Result<Vec<u8>> {
    require(s.len() <= MAX_ARTIFACT_BYTES * 2, "artifact too large")?;
    let bytes = STANDARD
        .decode(s)
        .map_err(|_| "invalid artifact encoding")?;
    require(
        bytes.len() <= MAX_ARTIFACT_BYTES && STANDARD.encode(&bytes) == s,
        "invalid artifact encoding or size",
    )?;
    Ok(bytes)
}
fn permitted(p: &Profile, action: Action, resource: &str) -> bool {
    p.grants
        .iter()
        .any(|g| g.action == action && g.resource == resource)
}
fn delegate_key(rev: &str, a: &Identity) -> String {
    format!(
        "{rev}/{}",
        digest(&serde_json::to_vec(a).expect("identity serializes"))
    )
}
fn authorized(s: &State, rev: &str, a: &Identity, power: Authority) -> bool {
    spec(s, rev).is_ok_and(|sp| {
        &sp.owner == a
            || (!s.revoked_delegates.contains(&delegate_key(rev, a))
                && sp.delegates.get(&power).is_some_and(|xs| xs.contains(a)))
    })
}
pub fn validate_spec(s: &Specification) -> Result<()> {
    id(&s.experiment)?;
    identity(&s.owner)?;
    text(&s.authority_reference)?;
    require(
        !s.phases.is_empty()
            && s.phases.len() <= 32
            && !s.profiles.is_empty()
            && s.artifacts.len() <= 128
            && s.profiles.len() <= 128,
        "invalid specification size",
    )?;
    require(
        s.delegates
            .values()
            .flatten()
            .collect::<BTreeSet<_>>()
            .len()
            <= 64,
        "too many delegates",
    )?;
    require(
        s.inspect_after.iter().all(|p| s.phases.contains_key(p)),
        "unknown inspection phase",
    )?;
    for actors in s.delegates.values() {
        for a in actors {
            identity(a)?;
        }
    }
    for arm in &s.arms {
        id(arm)?;
    }
    for (phase, condition) in &s.phases {
        id(phase)?;
        text(condition)?;
    }
    let mut total = 0;
    for (name, a) in &s.artifacts {
        id(name)?;
        id(&a.resource)?;
        text(&a.provenance)?;
        let bytes = decoded(&a.data_base64)?;
        require(digest(&bytes) == a.sha256, "artifact digest mismatch")?;
        total += bytes.len();
    }
    require(total <= 2 * 1024 * 1024, "snapshot exceeds 2 MiB")?;
    for (name, env) in &s.environments {
        id(name)?;
        require(
            !env.artifacts.is_empty() && env.artifacts.iter().all(|a| s.artifacts.contains_key(a)),
            "invalid environment artifacts",
        )?;
        for limit in &env.limitations {
            text(limit)?;
        }
    }
    for (name, p) in &s.profiles {
        id(name)?;
        require(
            p.arm.as_ref().is_none_or(|a| s.arms.contains(a)),
            "unknown arm",
        )?;
        let env = s
            .environments
            .get(&p.environment)
            .ok_or("unknown environment")?;
        require(
            env.artifacts.contains(&p.projection),
            "role projection must be in startup environment",
        )?;
        for g in &p.grants {
            id(&g.resource)?;
            if g.action == Action::Dispatch {
                require(
                    s.profiles.contains_key(&g.resource),
                    "unknown dispatch profile",
                )?;
            }
        }
    }
    for required in [
        "host_startup",
        "direct_messages",
        "filesystem_access",
        "shared_resources",
    ] {
        require(
            s.coverage.contains_key(required),
            "missing host coverage declaration",
        )?;
    }
    for (surface, c) in &s.coverage {
        id(surface)?;
        text(&c.limitation)?;
        if c.mode == CoverageMode::Managed {
            require(
                matches!(
                    surface.as_str(),
                    "artifact_delivery" | "mailbox_delivery" | "membership_transitions"
                ),
                "unsupported enforcement claim",
            )?;
        }
    }
    Ok(())
}
fn spec<'a>(s: &'a State, rev: &str) -> Result<&'a Specification> {
    s.specs.get(rev).ok_or("operation refused".into())
}
fn binding<'a>(s: &'a State, h: &str) -> Result<&'a Binding> {
    s.bindings.get(h).ok_or("operation refused".into())
}
fn phase_key(rev: &str, phase: &str) -> String {
    format!("{rev}/{phase}")
}
fn phase_open(s: &State, rev: &str, phase: &str) -> Result<()> {
    require(
        spec(s, rev)?.phases.contains_key(phase)
            && !s.closed_phases.contains(&phase_key(rev, phase)),
        "phase unavailable",
    )
}
fn admitter(s: &State, b: &Binding, a: &Identity) -> bool {
    authorized(s, &b.spec, a, Authority::Admit)
        || b.parent.as_ref().is_some_and(|parent| {
            binding(s, parent).is_ok_and(|p| {
                &b.prepared_by == a
                    && active(s, parent, p.generation, a).is_ok()
                    && profile(s, p).is_ok_and(|pp| permitted(pp, Action::Dispatch, &b.profile))
            })
        })
}
fn pending(s: &State, h: &str) -> bool {
    s.receipts
        .values()
        .any(|r| r.handle == h && r.status == ReceiptStatus::Issued)
}
fn active<'a>(s: &'a State, h: &str, generation: u64, actor: &Identity) -> Result<&'a Binding> {
    let b = binding(s, h)?;
    require(
        b.status == BindingStatus::Active
            && !b.paused
            && b.identity.as_ref() == Some(actor)
            && b.generation == generation,
        "operation refused",
    )?;
    let branch = s.branches.get(&b.branch).ok_or("invalid branch state")?;
    require(
        branch.current.as_deref() == Some(h) && branch.generation == generation,
        "operation refused",
    )?;
    phase_open(s, &b.spec, &b.phase)?;
    Ok(b)
}
fn profile<'a>(s: &'a State, b: &Binding) -> Result<&'a Profile> {
    spec(s, &b.spec)?
        .profiles
        .get(&b.profile)
        .ok_or("invalid profile state".into())
}
fn artifact<'a>(s: &'a State, rev: &str, name: &str) -> Result<&'a Artifact> {
    spec(s, rev)?
        .artifacts
        .get(name)
        .or_else(|| s.imports.get(rev).and_then(|xs| xs.get(name)))
        .ok_or("operation refused".into())
}
fn payloads(s: &State, b: &Binding, ids: &BTreeSet<String>) -> Result<Value> {
    let mut result = BTreeMap::new();
    for name in ids {
        let a = artifact(s, &b.spec, name)?;
        // No arm, profile, source path or provenance in participant output.
        result.insert(name, json!({"sha256":a.sha256,"data_base64":a.data_base64}));
    }
    Ok(json!(result))
}
fn issue(
    s: &mut State,
    id_: &str,
    h: &str,
    kind: ReceiptKind,
    actor: &Identity,
    ids: BTreeSet<String>,
    next: Option<usize>,
) -> Result<String> {
    id(id_)?;
    let token = digest(&serde_json::to_vec(&(h, id_)).map_err(|_| "receipt encoding")?);
    require(
        !s.receipts.contains_key(&token) && s.receipts.len() < 2048,
        "receipt unavailable or capacity reached",
    )?;
    s.receipts.insert(
        token.clone(),
        Receipt {
            id: token.clone(),
            local_id: id_.into(),
            handle: h.into(),
            kind,
            status: ReceiptStatus::Issued,
            issued_by: actor.clone(),
            artifact_ids: ids,
            next_message: next,
            evidence: None,
            disposition: None,
        },
    );
    Ok(token)
}
fn validate_state(s: &State) -> Result<()> {
    require(
        s.schema == 1 && s.audit.len() <= 12288 && s.bindings.len() <= 512 && s.specs.len() <= 16,
        "unsupported or oversized state",
    )?;
    for (rev, sp) in &s.specs {
        validate_spec(sp)?;
        require(
            sp.experiment == s.experiment
                && *rev == digest(&serde_json::to_vec(sp).map_err(|_| "invalid spec")?),
            "invalid spec revision",
        )?;
    }
    for (h, b) in &s.bindings {
        require(
            h == &b.handle && s.branches.contains_key(&b.branch),
            "invalid binding state",
        )?;
        let sp = spec(s, &b.spec)?;
        require(
            sp.phases.contains_key(&b.phase) && sp.profiles.contains_key(&b.profile),
            "invalid binding state",
        )?;
    }
    for (name, branch) in &s.branches {
        if let Some(h) = &branch.current {
            let b = binding(s, h)?;
            require(
                b.status == BindingStatus::Active
                    && b.branch == *name
                    && b.generation == branch.generation,
                "invalid ownership state",
            )?;
        }
        require(
            branch.next_message <= s.inboxes.get(name).map_or(0, Vec::len),
            "invalid mailbox cursor",
        )?;
    }
    for r in s.receipts.values() {
        binding(s, &r.handle)?;
    }
    for (rev, items) in &s.imports {
        let sp = spec(s, rev)?;
        for (name, a) in items {
            require(
                !sp.artifacts.contains_key(name) && digest(&decoded(&a.data_base64)?) == a.sha256,
                "invalid imported artifact",
            )?;
        }
    }
    Ok(())
}

pub fn apply(input: Input) -> Result<Output> {
    let Input { state, request } = input;
    id(&request.id)?;
    identity(&request.actor)?;
    text(&request.observed_at)?;
    let mut s = if let Some(s) = state {
        validate_state(&s)?;
        s
    } else {
        let Command::Register { specification } = &request.command else {
            return Err("experiment unavailable".into());
        };
        require(specification.owner == request.actor, "operation refused")?;
        State {
            schema: 1,
            experiment: specification.experiment.clone(),
            specs: BTreeMap::new(),
            bindings: BTreeMap::new(),
            branches: BTreeMap::new(),
            closed_phases: BTreeSet::new(),
            sealed_admissions: BTreeSet::new(),
            revoked_delegates: BTreeSet::new(),
            receipts: BTreeMap::new(),
            inboxes: BTreeMap::new(),
            audit: vec![],
            imports: BTreeMap::new(),
        }
    };
    require(
        !s.audit
            .iter()
            .any(|a| a.id == request.id && a.actor == request.actor),
        "request already recorded; inspect existing evidence",
    )?;
    let actor = &request.actor;
    let mut changed = true;
    let result = match &request.command {
        Command::Register { specification } | Command::Amend { specification, .. } => {
            if let Command::Amend {
                from_spec,
                evidence: ev,
                ..
            } = &request.command
            {
                let old = spec(&s, from_spec)?;
                require(
                    authorized(&s, from_spec, actor, Authority::Amend)
                        && old.owner == specification.owner,
                    "operation refused",
                )?;
                if &old.owner != actor {
                    require(
                        old.delegates == specification.delegates
                            && old.inspect_after == specification.inspect_after
                            && serde_json::to_value(&old.coverage)
                                .map_err(|_| "invalid coverage")?
                                == serde_json::to_value(&specification.coverage)
                                    .map_err(|_| "invalid coverage")?,
                        "delegate cannot amend administrative authority or coverage",
                    )?;
                }
                evidence(ev)?;
            } else {
                require(s.specs.is_empty(), "experiment already registered")?;
            }
            validate_spec(specification)?;
            require(
                s.experiment == specification.experiment && s.specs.len() < 16,
                "experiment mismatch or revision capacity reached",
            )?;
            let rev = digest(&serde_json::to_vec(specification).map_err(|_| "invalid spec")?);
            require(
                !s.specs.contains_key(&rev),
                "specification already registered",
            )?;
            // A delegated amendment cannot re-enable previously revoked actors.
            // The owner may deliberately restore them in a new revision.
            if let Command::Amend { from_spec, .. } = &request.command
                && &specification.owner != actor
            {
                let inherited: Vec<_> = specification
                    .delegates
                    .values()
                    .flatten()
                    .filter(|a| s.revoked_delegates.contains(&delegate_key(from_spec, a)))
                    .map(|a| delegate_key(&rev, a))
                    .collect();
                s.revoked_delegates.extend(inherited);
            }
            s.specs.insert(rev.clone(), specification.clone());
            json!({"spec":rev,"authority":"caller_asserted","applicability":"opt-in coexistence; existing revisions remain admissible until sealed"})
        }
        Command::Prepare {
            handle,
            spec: rev,
            phase,
            profile: pid,
            unit,
            branch,
            parent,
            replaces,
            continuation,
        } => {
            for x in [handle, phase, pid, unit, branch] {
                id(x)?;
            }
            require(
                !s.bindings.contains_key(handle) && s.bindings.len() < 512,
                "binding unavailable or capacity reached",
            )?;
            phase_open(&s, rev, phase)?;
            require(
                !s.sealed_admissions.contains(&phase_key(rev, phase)),
                "admissions sealed",
            )?;
            let sp = spec(&s, rev)?;
            let p = sp.profiles.get(pid).ok_or("operation refused")?;
            let mut allowed = authorized(&s, rev, actor, Authority::Admit);
            if let Some(parent) = parent {
                let pb = binding(&s, parent)?;
                active(&s, parent, pb.generation, actor)?;
                let pp = profile(&s, pb)?;
                require(
                    pb.spec == *rev && pb.phase == *phase && pp.arm == p.arm,
                    "parent constraint conflict",
                )?;
                allowed |= permitted(pp, Action::Dispatch, pid);
            }
            require(allowed, "operation refused")?;
            for a in continuation {
                let item = artifact(&s, rev, a)?;
                require(
                    permitted(p, Action::Read, &item.resource),
                    "operation refused",
                )?;
            }
            if let Some(old) = replaces {
                require(
                    !continuation.is_empty(),
                    "replacement needs a pinned continuation",
                )?;
                let b = binding(&s, old)?;
                if !authorized(&s, rev, actor, Authority::Admit) {
                    require(
                        b.parent.is_some() && b.parent == *parent,
                        "replacement outside parent responsibility",
                    )?;
                }
                require(
                    matches!(b.status, BindingStatus::Active | BindingStatus::Retired)
                        && s.branches.get(branch).is_some_and(|br| {
                            br.current.as_deref() == Some(old)
                                || (br.current.is_none() && br.last_owner.as_deref() == Some(old))
                        })
                        && b.branch == *branch
                        && b.unit == *unit
                        && b.spec == *rev
                        && b.phase == *phase
                        && b.profile == *pid,
                    "replacement binding conflict",
                )?;
            } else {
                require(
                    !s.branches.contains_key(branch),
                    "branch already exists; use explicit replacement",
                )?;
            }
            s.branches.entry(branch.clone()).or_insert(Branch {
                current: None,
                last_owner: None,
                generation: 0,
                next_message: 0,
            });
            s.bindings.insert(
                handle.clone(),
                Binding {
                    handle: handle.clone(),
                    spec: rev.clone(),
                    phase: phase.clone(),
                    profile: pid.clone(),
                    unit: unit.clone(),
                    branch: branch.clone(),
                    parent: parent.clone(),
                    replaces: replaces.clone(),
                    continuation: continuation.clone(),
                    status: BindingStatus::Prepared,
                    paused: false,
                    identity: None,
                    exposure: None,
                    generation: 0,
                    prepared_by: actor.clone(),
                    verified_by: None,
                    verification: None,
                },
            );
            json!({"handle":handle,"status":"prepared"})
        }
        Command::Startup { handle, receipt } => {
            let b = binding(&s, handle)?.clone();
            require(
                admitter(&s, &b, actor) && b.status == BindingStatus::Prepared && !b.paused,
                "operation refused",
            )?;
            phase_open(&s, &b.spec, &b.phase)?;
            let sp = spec(&s, &b.spec)?;
            let mut ids = sp.environments[&profile(&s, &b)?.environment]
                .artifacts
                .clone();
            ids.extend(b.continuation.iter().cloned());
            if !authorized(&s, &b.spec, actor, Authority::Admit) {
                let parent = binding(&s, b.parent.as_ref().ok_or("operation refused")?)?;
                for name in &ids {
                    require(
                        permitted(
                            profile(&s, parent)?,
                            Action::Read,
                            &artifact(&s, &b.spec, name)?.resource,
                        ),
                        "operation refused",
                    )?;
                }
            }
            let bytes = payloads(&s, &b, &ids)?;
            let receipt = issue(
                &mut s,
                receipt,
                handle,
                ReceiptKind::Startup,
                actor,
                ids,
                None,
            )?;
            json!({"handle":handle,"receipt":receipt,"artifacts":bytes,"evidence_class":"issued_emission_unobserved"})
        }
        Command::Bind {
            handle,
            identity: i,
            exposure: x,
        } => {
            let b = binding(&s, handle)?.clone();
            require(
                admitter(&s, &b, actor) && b.status == BindingStatus::Prepared && !b.paused,
                "operation refused",
            )?;
            phase_open(&s, &b.spec, &b.phase)?;
            identity(i)?;
            evidence(&x.identity_evidence)?;
            if let Some(e) = &x.observation_evidence {
                evidence(e)?;
            }
            for e in &x.inherited_context {
                evidence(e)?;
            }
            let p = profile(&s, &b)?;
            let sp = spec(&s, &b.spec)?;
            let expected: BTreeSet<_> = sp.environments[&p.environment]
                .artifacts
                .union(&b.continuation)
                .cloned()
                .collect();
            require(
                x.supplied_artifacts == expected && x.observed_artifacts.is_subset(&expected),
                "exposure declaration mismatch",
            )?;
            if p.require_observed_exposure {
                require(
                    x.observed_artifacts == expected && x.observation_evidence.is_some(),
                    "observed exposure evidence required",
                )?;
            }
            let known: BTreeSet<_> = s
                .bindings
                .values()
                .filter(|old| old.identity.as_ref() == Some(i))
                .map(|old| old.handle.clone())
                .collect();
            require(
                known.is_subset(&x.prior_bindings)
                    && x.prior_bindings.iter().all(|h| s.bindings.contains_key(h)),
                "prior exposure declaration incomplete",
            )?;
            if !p.allow_carryover {
                require(
                    x.prior_bindings.is_empty() && x.inherited_context.is_empty(),
                    "carryover not permitted",
                )?;
            }
            require(
                s.receipts.values().any(|r| {
                    r.handle == *handle
                        && r.kind == ReceiptKind::Startup
                        && r.status == ReceiptStatus::Collected
                        && r.artifact_ids == expected
                }),
                "startup collection evidence required",
            )?;
            let target = s.bindings.get_mut(handle).unwrap();
            target.identity = Some(i.clone());
            target.exposure = Some(x.clone());
            target.status = BindingStatus::Bound;
            json!({"handle":handle,"status":"bound","evidence_class":"caller_attested"})
        }
        Command::Verify {
            handle,
            evidence: ev,
        } => {
            let b = binding(&s, handle)?.clone();
            require(
                authorized(&s, &b.spec, actor, Authority::Verify)
                    && b.status == BindingStatus::Bound
                    && !b.paused,
                "operation refused",
            )?;
            phase_open(&s, &b.spec, &b.phase)?;
            evidence(ev)?;
            let b = s.bindings.get_mut(handle).unwrap();
            b.status = BindingStatus::Verified;
            b.verified_by = Some(actor.clone());
            b.verification = Some(ev.clone());
            json!({"handle":handle,"status":"verified_by_attestation","independent_host_verification":false})
        }
        Command::Activate {
            handle,
            expected_generation,
            settlement,
        } => {
            let b = binding(&s, handle)?.clone();
            require(
                admitter(&s, &b, actor)
                    && b.status == BindingStatus::Verified
                    && !b.paused
                    && !pending(&s, handle),
                "operation refused",
            )?;
            phase_open(&s, &b.spec, &b.phase)?;
            let br = s.branches.get(&b.branch).unwrap();
            require(
                br.generation == *expected_generation
                    && (br.current == b.replaces
                        || (br.current.is_none()
                            && b.replaces.is_some()
                            && br.last_owner == b.replaces)),
                "ownership conflict",
            )?;
            require(
                !s.bindings.values().any(|old| {
                    old.status == BindingStatus::Active
                        && old.identity == b.identity
                        && b.replaces.as_deref() != Some(&old.handle)
                }),
                "identity already active",
            )?;
            if let Some(old) = &b.replaces {
                require(!pending(&s, old), "predecessor has unresolved output")?;
                evidence(
                    settlement
                        .as_ref()
                        .ok_or("process/child settlement attestation required")?,
                )?;
                s.bindings.get_mut(old).unwrap().status = BindingStatus::Retired;
            }
            let generation = expected_generation
                .checked_add(1)
                .ok_or("generation overflow")?;
            let br = s.branches.get_mut(&b.branch).unwrap();
            br.current = Some(handle.clone());
            br.last_owner = Some(handle.clone());
            br.generation = generation;
            let b = s.bindings.get_mut(handle).unwrap();
            b.generation = generation;
            b.status = BindingStatus::Active;
            json!({"handle":handle,"generation":generation,"status":"active","telemetry_binding":"requires new host enrollment"})
        }
        Command::Evaluate {
            handle,
            generation,
            action,
            resource,
        } => {
            let b = active(&s, handle, *generation, actor)?;
            changed = false;
            json!({"allowed":permitted(profile(&s,b)?,*action,resource),"authority":"observation_only","not_a_permit":true})
        }
        Command::Deliver {
            handle,
            generation,
            artifacts,
            receipt,
        } => {
            let b = active(&s, handle, *generation, actor)?.clone();
            require(!artifacts.is_empty(), "empty delivery")?;
            for a in artifacts {
                let item = artifact(&s, &b.spec, a)?;
                require(
                    permitted(profile(&s, &b)?, Action::Read, &item.resource),
                    "operation refused",
                )?;
            }
            let bytes = payloads(&s, &b, artifacts)?;
            let receipt = issue(
                &mut s,
                receipt,
                handle,
                ReceiptKind::Artifact,
                actor,
                artifacts.clone(),
                None,
            )?;
            json!({"receipt":receipt,"artifacts":bytes,"evidence_class":"issued_emission_unobserved"})
        }
        Command::Import {
            spec: rev,
            artifact_id,
            artifact: a,
            evidence: ev,
            producer,
            generation,
        } => {
            id(artifact_id)?;
            id(&a.resource)?;
            text(&a.provenance)?;
            evidence(ev)?;
            let sp = spec(&s, rev)?;
            let allowed = if let Some(h) = producer {
                let b = active(&s, h, generation.ok_or("generation required")?, actor)?;
                b.spec == *rev && permitted(profile(&s, b)?, Action::Publish, &a.resource)
            } else {
                generation.is_none() && authorized(&s, rev, actor, Authority::Intervene)
            };
            require(allowed, "operation refused")?;
            require(
                !sp.phases
                    .keys()
                    .all(|p| s.closed_phases.contains(&phase_key(rev, p))),
                "specification phases closed",
            )?;
            require(
                artifact(&s, rev, artifact_id).is_err(),
                "artifact identity already used",
            )?;
            require(
                digest(&decoded(&a.data_base64)?) == a.sha256,
                "artifact digest mismatch",
            )?;
            s.imports
                .entry(rev.clone())
                .or_default()
                .insert(artifact_id.clone(), a.clone());
            json!({"artifact_id":artifact_id,"sha256":a.sha256,"source":"attributed; semantic contamination not verified"})
        }
        Command::Send {
            handle,
            generation,
            id: mid,
            targets,
            resource,
            data_base64,
            provenance,
        } => {
            let b = active(&s, handle, *generation, actor)?.clone();
            id(mid)?;
            text(provenance)?;
            require(
                permitted(profile(&s, &b)?, Action::Send, resource) && !targets.is_empty(),
                "operation refused",
            )?;
            let bytes = decoded(data_base64)?;
            require(
                !s.inboxes
                    .values()
                    .flatten()
                    .any(|m| m.id == *mid && m.sender.as_deref() == Some(handle)),
                "message already recorded",
            )?;
            for branch in targets {
                let target = s
                    .branches
                    .get(branch)
                    .and_then(|br| br.current.as_ref())
                    .and_then(|h| s.bindings.get(h))
                    .ok_or("operation refused")?;
                require(
                    target.spec == b.spec
                        && target.phase == b.phase
                        && permitted(profile(&s, target)?, Action::Receive, resource),
                    "operation refused",
                )?;
            }
            for branch in targets {
                s.inboxes.entry(branch.clone()).or_default().push(Message {
                    id: mid.clone(),
                    source: actor.clone(),
                    sender: Some(handle.clone()),
                    resource: resource.clone(),
                    data_base64: data_base64.clone(),
                    sha256: digest(&bytes),
                    provenance: provenance.clone(),
                });
            }
            json!({"id":mid,"status":"published","delivery_established":false})
        }
        Command::Receive {
            handle,
            generation,
            receipt,
            limit,
        } => {
            let b = active(&s, handle, *generation, actor)?.clone();
            require(*limit > 0 && *limit <= 100, "invalid limit")?;
            require(
                !s.receipts.values().any(|r| {
                    r.handle == *handle
                        && r.kind == ReceiptKind::Messages
                        && r.status == ReceiptStatus::Issued
                }),
                "previous mailbox output unresolved",
            )?;
            let br = &s.branches[&b.branch];
            let all = s.inboxes.get(&b.branch).cloned().unwrap_or_default();
            let mut rows = vec![];
            let mut next = br.next_message;
            for (i, m) in all.iter().enumerate().skip(next) {
                // A queued item is checked at delivery, not only publication.
                require(
                    permitted(profile(&s, &b)?, Action::Receive, &m.resource),
                    "queued message not permitted",
                )?;
                rows.push(json!({"id":m.id,"resource":m.resource,"sha256":m.sha256,"data_base64":m.data_base64}));
                next = i + 1;
                if rows.len() == *limit {
                    break;
                }
            }
            if rows.is_empty() {
                changed = false;
                json!({"messages":[],"receipt":null})
            } else {
                let receipt = issue(
                    &mut s,
                    receipt,
                    handle,
                    ReceiptKind::Messages,
                    actor,
                    BTreeSet::new(),
                    Some(next),
                )?;
                json!({"messages":rows,"receipt":receipt,"evidence_class":"issued_emission_unobserved"})
            }
        }
        Command::Acknowledge {
            receipt,
            evidence: ev,
        }
        | Command::Dispose {
            receipt,
            evidence: ev,
            ..
        } => {
            let r = s.receipts.get(receipt).ok_or("operation refused")?.clone();
            let b = binding(&s, &r.handle)?.clone();
            let dispose = matches!(request.command, Command::Dispose { .. });

            if dispose {
                require(
                    authorized(&s, &b.spec, actor, Authority::Intervene),
                    "operation refused",
                )?;
            } else {
                require(
                    b.identity.as_ref() == Some(actor)
                        || (r.kind == ReceiptKind::Startup && r.issued_by == *actor),
                    "operation refused",
                )?;
            }
            require(r.status == ReceiptStatus::Issued, "receipt already settled")?;
            evidence(ev)?;
            if !dispose && let Some(next) = r.next_message {
                s.branches.get_mut(&b.branch).unwrap().next_message = next;
            }
            let r = s.receipts.get_mut(receipt).unwrap();
            r.status = if dispose {
                ReceiptStatus::Disposed
            } else {
                ReceiptStatus::Collected
            };
            r.evidence = Some(ev.clone());
            if let Command::Dispose { disposition, .. } = &request.command {
                text(disposition)?;
                r.disposition = Some(disposition.clone());
            }
            json!({"receipt":receipt,"status":r.status,"consequence_established":false})
        }
        Command::Pause {
            handle,
            evidence: ev,
        }
        | Command::Resume {
            handle,
            evidence: ev,
            ..
        }
        | Command::Retire {
            handle,
            evidence: ev,
        } => {
            let b = binding(&s, handle)?.clone();
            require(
                authorized(&s, &b.spec, actor, Authority::Intervene),
                "operation refused",
            )?;
            evidence(ev)?;
            require(
                !matches!(b.status, BindingStatus::Retired | BindingStatus::Cancelled),
                "binding terminal",
            )?;
            match &request.command {
                Command::Pause { .. } => {
                    require(!b.paused, "already paused")?;
                    s.bindings.get_mut(handle).unwrap().paused = true;
                }
                Command::Resume { disposition, .. } => {
                    require(b.paused, "not paused")?;
                    phase_open(&s, &b.spec, &b.phase)?;
                    text(disposition)?;
                    s.bindings.get_mut(handle).unwrap().paused = false;
                }
                _ => {
                    require(
                        !pending(&s, handle),
                        "output unresolved; collect or dispose first",
                    )?;
                    let target = s.bindings.get_mut(handle).unwrap();
                    target.status = if b.status == BindingStatus::Active {
                        BindingStatus::Retired
                    } else {
                        BindingStatus::Cancelled
                    };
                    let br = s.branches.get_mut(&b.branch).unwrap();
                    if br.current.as_deref() == Some(handle) {
                        br.current = None;
                        br.generation =
                            br.generation.checked_add(1).ok_or("generation overflow")?;
                    }
                }
            }
            json!({"handle":handle,"status":s.bindings[handle].status,"paused":s.bindings[handle].paused,"native_process_stop_established":false})
        }
        Command::ClosePhase {
            spec: rev,
            phase,
            evidence: ev,
        } => {
            require(
                authorized(&s, rev, actor, Authority::Close),
                "operation refused",
            )?;
            phase_open(&s, rev, phase)?;
            evidence(ev)?;
            require(
                !s.bindings.values().any(|b| {
                    b.spec == *rev
                        && b.phase == *phase
                        && (!matches!(b.status, BindingStatus::Retired | BindingStatus::Cancelled)
                            || pending(&s, &b.handle))
                }),
                "phase has live or pending bindings",
            )?;
            s.closed_phases.insert(phase_key(rev, phase));
            json!({"status":"closed","spec":rev,"phase":phase,"end_condition_evidence":"caller_attested; no native process observation"})
        }
        Command::SealAdmissions {
            spec: rev,
            phase,
            evidence: ev,
        } => {
            require(
                authorized(&s, rev, actor, Authority::Amend),
                "operation refused",
            )?;
            phase_open(&s, rev, phase)?;
            evidence(ev)?;
            require(
                s.sealed_admissions.insert(phase_key(rev, phase)),
                "already sealed",
            )?;
            json!({"status":"admissions_sealed","existing_bindings":"unchanged"})
        }
        Command::RevokeDelegate {
            spec: rev,
            identity: target,
            evidence: ev,
        } => {
            let sp = spec(&s, rev)?;
            require(
                &sp.owner == actor
                    && &sp.owner != target
                    && sp.delegates.values().any(|xs| xs.contains(target)),
                "operation refused",
            )?;
            evidence(ev)?;
            require(
                s.revoked_delegates.insert(delegate_key(rev, target)),
                "already revoked",
            )?;
            json!({"status":"delegate_revoked","scope":"this revision; profile permissions unchanged"})
        }
        Command::Status { handle } => {
            let b = binding(&s, handle)?;
            require(
                b.identity.as_ref() == Some(actor) || admitter(&s, b, actor),
                "operation refused",
            )?;
            changed = false;
            json!({"handle":handle,"status":b.status,"paused":b.paused,"generation":b.generation,"pending_receipts":s.receipts.values().filter(|r|r.handle==*handle && r.status==ReceiptStatus::Issued).map(|r|&r.id).collect::<Vec<_>>()})
        }
        Command::Inspect { spec: rev } => {
            require(
                authorized(&s, rev, actor, Authority::Inspect),
                "operation refused",
            )?;
            let sp = spec(&s, rev)?;
            require(
                &sp.owner == actor
                    || sp
                        .inspect_after
                        .iter()
                        .all(|p| s.closed_phases.contains(&phase_key(rev, p))),
                "inspection phase not closed",
            )?;
            // Inspection is deliberately unblinded, and its issuance is recorded.
            if &sp.owner == actor {
                json!(s)
            } else {
                json!({"specification":sp, "bindings":s.bindings.values().filter(|b| b.spec == *rev).collect::<Vec<_>>(),
                "receipts":s.receipts.values().filter(|r| s.bindings[&r.handle].spec == *rev).collect::<Vec<_>>(),
                "imports":s.imports.get(rev),
                "branches":s.branches.iter().filter(|(name,_)| s.bindings.values().any(|b| b.spec == *rev && b.branch == **name)).collect::<BTreeMap<_,_>>()})
            }
        }
    };
    let recovery = matches!(
        request.command,
        Command::Pause { .. }
            | Command::Retire { .. }
            | Command::Dispose { .. }
            | Command::Acknowledge { .. }
            | Command::ClosePhase { .. }
            | Command::SealAdmissions { .. }
            | Command::RevokeDelegate { .. }
    );
    if changed {
        require(
            s.audit.len() < if recovery { 12288 } else { 4096 },
            "audit capacity reached",
        )?;
        let mut command = serde_json::to_value(&request.command).map_err(|_| "invalid command")?;
        if command.get("specification").is_some() {
            command["specification"] = result["spec"].clone();
        }
        if let Some(bytes) = command.get("data_base64").and_then(Value::as_str) {
            command["data_base64"] = json!({"sha256":digest(&decoded(bytes)?)});
        }
        if let Some(a) = command.get_mut("artifact").and_then(Value::as_object_mut) {
            a.remove("data_base64");
        }
        s.audit.push(Audit {
            id: request.id,
            actor: request.actor,
            observed_at: request.observed_at,
            command,
        });
    }
    require(
        serde_json::to_vec(&s).map_err(|_| "invalid state")?.len()
            <= if recovery || !changed {
                HARD_MAX_BYTES
            } else {
                MAX_BYTES
            },
        "aggregate capacity reached",
    )?;
    Ok(Output {
        state: s,
        result,
        changed,
    })
}
