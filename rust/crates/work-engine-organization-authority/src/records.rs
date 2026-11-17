//! Owner-attested records (Stage 0 rev6 §18 with Amendment 6.1 F4/F5).
//!
//! Every record has private fields and no production constructor. Constructors exist only with
//! the `fixture` feature, so outside it no record can exist. A record confers nothing by its
//! content: it counts only when an observation reports its issuance committed.

use std::collections::BTreeSet;

use work_engine_organization::ProblemScope;

use crate::ids::{BodyDigest, Enc, ExecutableDigest, IssuanceId, RealizationDigest};
use work_engine_organization::{
    Continuity, EvidenceCustody, Integrity, Mutation, ObserverClass, RealizationClass,
};

/// `Principal v1 kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrincipalKind {
    Human,
    Service,
}

/// The kind of real actor an anchor names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AnchorKind {
    OwnerAccount,
    ServiceAccount,
}

/// The real actor a principal stands for. Two principals with equal anchors are the same actor.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActorAnchor {
    kind: AnchorKind,
    id: String,
}

impl ActorAnchor {
    pub fn kind(&self) -> AnchorKind {
        self.kind
    }
    pub fn id(&self) -> &str {
        &self.id
    }
}

/// A problem scope by value (run, plan lineage, slice). Compared by equality with the admitted
/// candidate's problem scope; no digest codec is defined for it here.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProblemScopeRef {
    run_id: String,
    plan_lineage: String,
    slice_number: u64,
}

impl ProblemScopeRef {
    pub fn of(scope: &ProblemScope) -> Self {
        ProblemScopeRef {
            run_id: scope.run_id.as_str().to_owned(),
            plan_lineage: scope.plan_lineage.as_str().to_owned(),
            slice_number: scope.slice_number,
        }
    }
    fn encode(&self, enc: &mut Enc) {
        enc.str(&self.run_id)
            .str(&self.plan_lineage)
            .u64(self.slice_number);
    }
}

/// `Principal v1 { principalId, kind, actorAnchor, issuer: deployment_owner }`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Principal {
    pub(crate) principal_id: String,
    pub(crate) kind: PrincipalKind,
    pub(crate) anchor: ActorAnchor,
}

impl Principal {
    pub fn principal_id(&self) -> &str {
        &self.principal_id
    }
    pub fn kind(&self) -> PrincipalKind {
        self.kind
    }
    pub fn anchor(&self) -> &ActorAnchor {
        &self.anchor
    }
}

/// The actor a vantage is bound to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundActor {
    pub(crate) realization_digest: RealizationDigest,
    pub(crate) actor_identity: String,
}

/// `VantageActorBinding v1 { organizationId, vantage, principal: IssuanceId, actor }`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VantageActorBinding {
    pub(crate) organization_id: String,
    pub(crate) vantage: String,
    pub(crate) principal: IssuanceId,
    pub(crate) actor: BoundActor,
}

/// `ComponentBinding v1 { component, principal: IssuanceId, executable { installedDigest },
/// problemScope }`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentBinding {
    pub(crate) component: String,
    pub(crate) principal: IssuanceId,
    pub(crate) installed_digest: ExecutableDigest,
    pub(crate) problem_scope: ProblemScopeRef,
}

/// `DecisionScope v1 { id, revision, problemScope, actions, boundaries? }` (6.1 F4: no
/// `decisions` field).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionScope {
    pub(crate) id: String,
    pub(crate) revision: u64,
    pub(crate) problem_scope: ProblemScopeRef,
    pub(crate) actions: BTreeSet<String>,
    pub(crate) boundaries: Option<BTreeSet<String>>,
}

/// `AuthorityGrant v1 { grantId, principal: IssuanceId, permissions, problemScope,
/// decisionScope { id, revision }, expiresAtMillis? }`. A grant without a resolving decision
/// scope covers nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityGrant {
    pub(crate) grant_id: String,
    pub(crate) principal: IssuanceId,
    pub(crate) permissions: BTreeSet<String>,
    pub(crate) problem_scope: ProblemScopeRef,
    pub(crate) decision_scope: (String, u64),
    pub(crate) expires_at_millis: Option<u64>,
}

/// What an accepted realization provides, as the accepting owner states it. R1, R2 and R4 read
/// these values from the committed record; a realization claim never supplies them. `None` means
/// the record does not state the value (unestablished).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealizationProperties {
    pub(crate) classes: BTreeSet<RealizationClass>,
    pub(crate) continuity: Option<Continuity>,
    pub(crate) mutation: Option<Mutation>,
    pub(crate) integrity: Option<Integrity>,
    pub(crate) evidence_custody: Option<EvidenceCustody>,
    pub(crate) observer_class: Option<ObserverClass>,
}

/// `RealizationAcceptance v1 { realizationDigest, profile, provides, acceptedBy: deployment_owner }`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealizationAcceptance {
    pub(crate) realization_digest: RealizationDigest,
    pub(crate) profile: String,
    pub(crate) provides: RealizationProperties,
}

/// One owner-attested record. Every kind is revocable (6.1 F5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Record {
    Principal(Principal),
    VantageActorBinding(VantageActorBinding),
    ComponentBinding(ComponentBinding),
    DecisionScope(DecisionScope),
    Grant(AuthorityGrant),
    RealizationAcceptance(RealizationAcceptance),
}

fn set(items: &BTreeSet<String>) -> impl ExactSizeIterator<Item = &str> {
    items.iter().map(String::as_str)
}

impl Record {
    pub fn kind(&self) -> &'static str {
        match self {
            Record::Principal(_) => "principal",
            Record::VantageActorBinding(_) => "vantage_actor_binding",
            Record::ComponentBinding(_) => "component_binding",
            Record::DecisionScope(_) => "decision_scope",
            Record::Grant(_) => "authority_grant",
            Record::RealizationAcceptance(_) => "realization_acceptance",
        }
    }

    /// The content identity of this record's terms.
    pub fn body_digest(&self) -> BodyDigest {
        let mut enc = Enc::new(self.kind());
        match self {
            Record::Principal(p) => {
                enc.str(&p.principal_id)
                    .u64(p.kind as u64)
                    .u64(p.anchor.kind as u64)
                    .str(&p.anchor.id);
            }
            Record::VantageActorBinding(b) => {
                enc.str(&b.organization_id)
                    .str(&b.vantage)
                    .bytes(b.principal.as_bytes())
                    .str(b.actor.realization_digest.as_str())
                    .str(&b.actor.actor_identity);
            }
            Record::ComponentBinding(b) => {
                enc.str(&b.component)
                    .bytes(b.principal.as_bytes())
                    .str(b.installed_digest.as_str());
                b.problem_scope.encode(&mut enc);
            }
            Record::DecisionScope(d) => {
                enc.str(&d.id).u64(d.revision);
                d.problem_scope.encode(&mut enc);
                enc.strs(set(&d.actions));
                match &d.boundaries {
                    None => enc.u64(0),
                    Some(b) => enc.u64(1).strs(set(b)),
                };
            }
            Record::Grant(g) => {
                enc.str(&g.grant_id)
                    .bytes(g.principal.as_bytes())
                    .strs(set(&g.permissions));
                g.problem_scope.encode(&mut enc);
                enc.str(&g.decision_scope.0)
                    .u64(g.decision_scope.1)
                    .opt_u64(g.expires_at_millis);
            }
            Record::RealizationAcceptance(r) => {
                let p = &r.provides;
                enc.str(r.realization_digest.as_str())
                    .str(&r.profile)
                    .strs(p.classes.iter().map(|c| c.as_str()));
                for value in [
                    p.continuity.map(|v| v.as_str()),
                    p.mutation.map(|v| v.as_str()),
                    p.integrity.map(|v| v.as_str()),
                    p.evidence_custody.map(|v| v.as_str()),
                    p.observer_class.map(|v| v.as_str()),
                ] {
                    match value {
                        None => enc.u64(0),
                        Some(text) => enc.u64(1).str(text),
                    };
                }
            }
        }
        BodyDigest(enc.finish())
    }
}

/// Fixture constructors. Synthetic owner attestations for tests; they issue nothing anywhere.
#[cfg(feature = "fixture")]
mod fixture_constructors {
    use super::*;

    impl ActorAnchor {
        pub fn fixture(kind: AnchorKind, id: &str) -> Self {
            ActorAnchor {
                kind,
                id: id.to_owned(),
            }
        }
    }

    impl ProblemScopeRef {
        pub fn fixture(run_id: &str, plan_lineage: &str, slice_number: u64) -> Self {
            ProblemScopeRef {
                run_id: run_id.to_owned(),
                plan_lineage: plan_lineage.to_owned(),
                slice_number,
            }
        }
    }

    fn strings(items: &[&str]) -> BTreeSet<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }

    impl Record {
        pub fn fixture_principal(
            principal_id: &str,
            kind: PrincipalKind,
            anchor: ActorAnchor,
        ) -> Self {
            Record::Principal(Principal {
                principal_id: principal_id.to_owned(),
                kind,
                anchor,
            })
        }

        pub fn fixture_vantage_binding(
            organization_id: &str,
            vantage: &str,
            principal: IssuanceId,
            realization_digest: RealizationDigest,
            actor_identity: &str,
        ) -> Self {
            Record::VantageActorBinding(VantageActorBinding {
                organization_id: organization_id.to_owned(),
                vantage: vantage.to_owned(),
                principal,
                actor: BoundActor {
                    realization_digest,
                    actor_identity: actor_identity.to_owned(),
                },
            })
        }

        pub fn fixture_component_binding(
            component: &str,
            principal: IssuanceId,
            installed_digest: ExecutableDigest,
            problem_scope: ProblemScopeRef,
        ) -> Self {
            Record::ComponentBinding(ComponentBinding {
                component: component.to_owned(),
                principal,
                installed_digest,
                problem_scope,
            })
        }

        pub fn fixture_decision_scope(
            id: &str,
            revision: u64,
            problem_scope: ProblemScopeRef,
            actions: &[&str],
            boundaries: Option<&[&str]>,
        ) -> Self {
            Record::DecisionScope(DecisionScope {
                id: id.to_owned(),
                revision,
                problem_scope,
                actions: strings(actions),
                boundaries: boundaries.map(strings),
            })
        }

        pub fn fixture_grant(
            grant_id: &str,
            principal: IssuanceId,
            permissions: &[&str],
            problem_scope: ProblemScopeRef,
            decision_scope: (&str, u64),
            expires_at_millis: Option<u64>,
        ) -> Self {
            Record::Grant(AuthorityGrant {
                grant_id: grant_id.to_owned(),
                principal,
                permissions: strings(permissions),
                problem_scope,
                decision_scope: (decision_scope.0.to_owned(), decision_scope.1),
                expires_at_millis,
            })
        }

        pub fn fixture_realization_acceptance(
            realization_digest: RealizationDigest,
            profile: &str,
            provides: RealizationProperties,
        ) -> Self {
            Record::RealizationAcceptance(RealizationAcceptance {
                realization_digest,
                profile: profile.to_owned(),
                provides,
            })
        }
    }

    impl RealizationProperties {
        pub fn fixture(
            classes: &[RealizationClass],
            continuity: Option<Continuity>,
            mutation: Option<Mutation>,
            integrity: Option<Integrity>,
            evidence_custody: Option<EvidenceCustody>,
            observer_class: Option<ObserverClass>,
        ) -> Self {
            RealizationProperties {
                classes: classes.iter().copied().collect(),
                continuity,
                mutation,
                integrity,
                evidence_custody,
                observer_class,
            }
        }
    }
}
