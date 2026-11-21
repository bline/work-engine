//! Owner-attested records (Stage 0 rev6 §18 with Amendment 6.1 F4/F5).
//!
//! Every record has private fields and no production constructor. Constructors exist only with
//! the `fixture` feature, so outside it no record can exist. A record confers nothing by its
//! content: it counts only when an observation reports its issuance committed.

use std::collections::BTreeSet;

use work_engine_organization::{Digest, ProblemScope};

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

/// `OrganizationAdmission v1 { organizationId, revision, predecessor?: IssuanceId, problemScope,
/// requirementsDigest, candidateContentDigest, issuer: deployment_owner }`.
///
/// Owner: organizational authority (S-A). Operator decisions O1 and D4/D5 (2026-11-17); definition
/// recorded here under O-1 (2026-11-18). It adds one kind to the S-A record set (Stage 0 rev6 §18
/// with Amendment 6.1).
///
/// - `organizationId` names the admitted organization: one problem's organization (the execution
///   envelope identity), stable across revisions. It is not a tenant or an installation.
/// - A committed issuance admits exactly one candidate: the one whose content digest is
///   `candidateContentDigest`, derived from the requirements whose digest is `requirementsDigest`,
///   for `problemScope`. Like every record it confers nothing by its content; it counts only while
///   an observation reports its issuance committed, and it is revocable (6.1 F5).
/// - Revision 1 has no predecessor (create-only). At fixture standing `OA` verifies the selected
///   issuance's bounded lineage to that root in one observation. Each successor names an observed
///   `OrganizationAdmission` with the same `organizationId` and exactly the preceding revision;
///   that predecessor must be observed as superseded by this successor's issuance.
///   This checks the selected lineage, not the issuer's global head uniqueness: `OA` reports
///   ambiguity only over the exact committed candidate terms (even across different organizations).
///   The key for issuer-wide exclusivity (organization, problem scope, or candidate terms) remains
///   an open owner question.
/// - Vantage-actor bindings name the admission they belong to by its issuance
///   ([`VantageActorBinding`] `organization`). Revoking or superseding an admission therefore
///   leaves every binding that pins it without an admitted organization.
/// - The gate takes the organization of a request from the unique committed admission whose
///   candidate digest, requirements digest and problem scope equal the request's; no caller
///   supplies it. None refuses `OrganizationUnadmitted`, more than one refuses
///   `OrganizationHeadsAmbiguous` (check `OA`).
/// - Issuer: the deployment owner. Fixture standing only: the constructor exists only with the
///   `fixture` feature. Production issuance needs observation-source storage (O6), an independent
///   issuer (K-c) and production authorization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrganizationAdmission {
    pub(crate) organization_id: String,
    pub(crate) revision: u64,
    pub(crate) predecessor: Option<IssuanceId>,
    pub(crate) problem_scope: ProblemScopeRef,
    pub(crate) requirements_digest: Digest,
    pub(crate) candidate_content_digest: Digest,
}

impl OrganizationAdmission {
    pub fn organization_id(&self) -> &str {
        &self.organization_id
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn predecessor(&self) -> Option<IssuanceId> {
        self.predecessor
    }
}

/// `VantageActorBinding v1 { organization: IssuanceId, vantage, principal: IssuanceId, actor }`.
/// `organization` is the issuance of the [`OrganizationAdmission`] the binding belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VantageActorBinding {
    pub(crate) organization: IssuanceId,
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
    OrganizationAdmission(OrganizationAdmission),
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
            Record::OrganizationAdmission(_) => "organization_admission",
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
            Record::OrganizationAdmission(a) => {
                enc.str(&a.organization_id).u64(a.revision);
                match &a.predecessor {
                    None => enc.u64(0),
                    Some(id) => enc.u64(1).bytes(id.as_bytes()),
                };
                a.problem_scope.encode(&mut enc);
                enc.bytes(a.requirements_digest.as_bytes())
                    .bytes(a.candidate_content_digest.as_bytes());
            }
            Record::Principal(p) => {
                enc.str(&p.principal_id)
                    .u64(p.kind as u64)
                    .u64(p.anchor.kind as u64)
                    .str(&p.anchor.id);
            }
            Record::VantageActorBinding(b) => {
                enc.bytes(b.organization.as_bytes())
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
        pub fn fixture_organization_admission(
            organization_id: &str,
            revision: u64,
            predecessor: Option<IssuanceId>,
            problem_scope: ProblemScopeRef,
            requirements_digest: Digest,
            candidate_content_digest: Digest,
        ) -> Self {
            Record::OrganizationAdmission(OrganizationAdmission {
                organization_id: organization_id.to_owned(),
                revision,
                predecessor,
                problem_scope,
                requirements_digest,
                candidate_content_digest,
            })
        }

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
            organization: IssuanceId,
            vantage: &str,
            principal: IssuanceId,
            realization_digest: RealizationDigest,
            actor_identity: &str,
        ) -> Self {
            Record::VantageActorBinding(VantageActorBinding {
                organization,
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
