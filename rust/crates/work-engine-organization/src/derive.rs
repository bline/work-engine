//! `derive` and `OrganizationCandidate` (contract 3.2 with amendment 5.1-A1).
//!
//! Derivation is pure and deterministic. Under `fixed-slice-review-v1` it emits the
//! relationships of amendment section 3, checks the acceptor dependency closure (section 3A.2),
//! records component binding requirements (section 3A.3) and separation requirements
//! (section 4), and traces every candidate member to its input source. The candidate carries
//! requirements only: it names no grant, claims no standing, and no public item can mark a
//! separation or a component binding as established.

use std::collections::{BTreeMap, BTreeSet};

use crate::digest::{Digest, DigestKind};
use crate::ids::Id;
use crate::json::{InvariantViolation, JsonValue, object, string, strings};
use crate::schema::{
    Condition, Dimension, Normative, Obligation, OrganizationalRequirements, ProblemScope, Profile,
    RealizationClass, RealizationConstraint, RequiredAuthority, constraints_json, id_json,
    id_set_json, normative_json, obligation_json, problem_scope_json,
};

// Profile constants of `fixed-slice-review-v1` (amendment 3 and 3A.1).
const CAMPAIGN: &str = "component:campaign";
const EPISODE_HOST: &str = "component:episode_host";
pub(crate) const BUILDER_VANTAGE: &str = "vantage-builder";
const PERMISSION_ACCEPT_CLAIM: &str = "accept_claim";
const PERMISSION_WRITE_EPISODE: &str = "write_episode";
const PERMISSION_READ_EPISODE: &str = "read_episode";
const PERMISSION_TERMINALIZE: &str = "terminalize_campaign";
const CLASS_BUILDER: &str = "builder";
const CLASS_CAMPAIGN: &str = "campaign";

/// The kind of a candidate participant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ParticipantKind {
    Vantage,
    Component,
    Principal,
}

impl ParticipantKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ParticipantKind::Vantage => "vantage",
            ParticipantKind::Component => "component",
            ParticipantKind::Principal => "principal",
        }
    }
}

/// `participants[]`. IDs follow contract 17.3: `vantage:<obligationId>`, `vantage-builder`,
/// `component:campaign`, `component:episode_host`, `principal:<principalId>`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Participant {
    pub id: String,
    pub kind: ParticipantKind,
    /// The obligations a reviewer vantage serves; empty for every other participant.
    pub serves: Vec<Id>,
}

/// `conditions[]`: one entry per accepted condition, meaning unchanged. No grant digest.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CandidateCondition {
    pub id: Id,
    pub obligation: Id,
    /// The participant ID of the obligation's reviewer vantage.
    pub vantage: String,
    /// The participant ID the condition's `consumerClass` maps to.
    pub consumer: String,
    pub boundary: Id,
    pub proposition: String,
    pub covered_state: Id,
    pub normative: Normative,
    pub realization_constraints: Vec<RealizationConstraint>,
    /// The acceptor's `principalId`.
    pub acceptor: Id,
    pub unestablished_route: Id,
}

/// `vantageRequirements[]`: the F4 combination of one vantage's conditions.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct VantageRequirement {
    pub vantage: String,
    pub effective: Normative,
    pub effective_constraints: Vec<RealizationConstraint>,
    pub derived_from: Vec<Id>,
}

/// The seven relationship kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RelationshipKind {
    Serves,
    HostsExecution,
    ConsumesAt,
    AcceptsClaim,
    WritesEpisode,
    ReadsEpisode,
    Terminalizes,
}

impl RelationshipKind {
    pub fn as_str(self) -> &'static str {
        match self {
            RelationshipKind::Serves => "serves",
            RelationshipKind::HostsExecution => "hosts_execution",
            RelationshipKind::ConsumesAt => "consumes_at",
            RelationshipKind::AcceptsClaim => "accepts_claim",
            RelationshipKind::WritesEpisode => "writes_episode",
            RelationshipKind::ReadsEpisode => "reads_episode",
            RelationshipKind::Terminalizes => "terminalizes",
        }
    }
}

/// `relationships[]`. Profile-fixed assignments (`hosts_execution`, `writes_episode`,
/// `reads_episode`, `terminalizes`) bind only if an organizational authority admits the candidate;
/// the candidate claims no standing.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Relationship {
    pub kind: RelationshipKind,
    pub from: String,
    pub to: Option<String>,
    pub condition: Option<Id>,
}

impl Relationship {
    fn sort_key(&self) -> (&'static str, &str, Option<&str>, Option<&str>) {
        (
            self.kind.as_str(),
            &self.from,
            self.to.as_deref(),
            self.condition.as_ref().map(Id::as_str),
        )
    }

    /// The rendered key used as the trace target id: `kind|from|to|condition`, absent parts empty.
    fn key(&self) -> String {
        format!(
            "{}|{}|{}|{}",
            self.kind.as_str(),
            self.from,
            self.to.as_deref().unwrap_or(""),
            self.condition.as_ref().map_or("", Id::as_str)
        )
    }
}

/// `authorityRequirements[]`: a required authority attached unchanged. It is a dependency, not a
/// grant, and covers nothing by itself.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct AuthorityRequirement {
    pub id: Id,
    pub principal: Id,
    pub permission: Id,
    pub boundaries: Option<BTreeSet<Id>>,
}

/// `separationRequirements[]`: "separation between the acceptor and these vantages is not
/// established by derivation". There is no status field; nothing in this crate can mark it
/// satisfied.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SeparationRequirement {
    pub condition: Id,
    /// `principal:<id>`.
    pub acceptor: String,
    /// The dependent vantage participant IDs.
    pub vantages: Vec<String>,
}

/// `componentBindingRequirements[]`: a component that must be bound to a principal by a verified
/// mapping which holds each permission over the problem scope. The entry has no principal member,
/// and no public function attaches one: this crate does not establish the binding.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ComponentBindingRequirement {
    pub component: String,
    pub permissions: Vec<String>,
}

/// A trace endpoint `{ kind, id, field }` (contract 17.3). Never a JSON path with an index.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub struct TraceRef {
    pub kind: &'static str,
    pub id: String,
    pub field: String,
}

/// `requirementTrace[]`: the candidate member `target` originates in the input item `source`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub struct TraceRow {
    pub target: TraceRef,
    pub source: TraceRef,
}

/// `OrganizationCandidate v0`. It can be produced only by [`derive`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrganizationCandidate {
    requirements_digest: Digest,
    profile: Profile,
    problem_scope: ProblemScope,
    obligations: Vec<Obligation>,
    participants: Vec<Participant>,
    conditions: Vec<CandidateCondition>,
    vantage_requirements: Vec<VantageRequirement>,
    relationships: Vec<Relationship>,
    authority_requirements: Vec<AuthorityRequirement>,
    separation_requirements: Vec<SeparationRequirement>,
    component_binding_requirements: Vec<ComponentBindingRequirement>,
    requirement_trace: Vec<TraceRow>,
    canonical: Vec<u8>,
    content_digest: Digest,
}

impl OrganizationCandidate {
    pub fn requirements_digest(&self) -> Digest {
        self.requirements_digest
    }
    pub fn profile(&self) -> Profile {
        self.profile
    }
    /// `problemScope`, copied unchanged from the validated requirements.
    pub fn problem_scope(&self) -> &ProblemScope {
        &self.problem_scope
    }
    /// `obligations[]`, a set keyed by `id`, copied unchanged from the validated requirements.
    pub fn obligations(&self) -> &[Obligation] {
        &self.obligations
    }
    pub fn participants(&self) -> &[Participant] {
        &self.participants
    }
    pub fn conditions(&self) -> &[CandidateCondition] {
        &self.conditions
    }
    pub fn vantage_requirements(&self) -> &[VantageRequirement] {
        &self.vantage_requirements
    }
    pub fn relationships(&self) -> &[Relationship] {
        &self.relationships
    }
    pub fn authority_requirements(&self) -> &[AuthorityRequirement] {
        &self.authority_requirements
    }
    pub fn separation_requirements(&self) -> &[SeparationRequirement] {
        &self.separation_requirements
    }
    pub fn component_binding_requirements(&self) -> &[ComponentBindingRequirement] {
        &self.component_binding_requirements
    }
    /// Test-only mutation of the candidate's relationships, so a malformed candidate that
    /// `derive` cannot produce can be built from a derived one. There is no production path.
    #[cfg(test)]
    pub(crate) fn relationships_mut(&mut self) -> &mut Vec<Relationship> {
        &mut self.relationships
    }
    /// Test-only, as [`Self::relationships_mut`].
    #[cfg(test)]
    pub(crate) fn vantage_requirements_mut(&mut self) -> &mut Vec<VantageRequirement> {
        &mut self.vantage_requirements
    }
    pub fn requirement_trace(&self) -> &[TraceRow] {
        &self.requirement_trace
    }
    /// The canonical bytes (contract 17.5) over which `contentDigest` is computed.
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical
    }
    /// `contentDigest`.
    pub fn content_digest(&self) -> Digest {
        self.content_digest
    }
}

/// The class of an unresolved item (contract 3.2, G25, amendment 3A.2 and 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UnresolvedKind {
    ConditionConflict,
    SelfAcceptance,
    UnknownConsumerClass,
    UnstatedAuthorityDependency,
    /// A derivation invariant failed (a duplicate in a set collection, a consumer without a
    /// participant, an integer outside the canonical range). Not reachable for validated input.
    InternalError,
}

impl UnresolvedKind {
    pub fn as_str(self) -> &'static str {
        match self {
            UnresolvedKind::ConditionConflict => "condition_conflict",
            UnresolvedKind::SelfAcceptance => "self_acceptance",
            UnresolvedKind::UnknownConsumerClass => "unknown_consumer_class",
            UnresolvedKind::UnstatedAuthorityDependency => "unstated_authority_dependency",
            UnresolvedKind::InternalError => "internal_error",
        }
    }
}

/// The closed reason tokens; the reason is a function of `(kind, subject, conditions)`.
/// `UnrecognizedClass` reports a class value in the input that is not recognized;
/// `BrokenInternalReference` reports a reference the derivation itself stated that no longer
/// resolves, and appears only with the kind `InternalError`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnresolvedReason {
    DifferingValues,
    DisjointAllowedClasses,
    UnrecognizedClass,
    BrokenInternalReference,
    AcceptorIsDependentVantage,
    NoMatchingAuthorityEntry,
    DuplicateCollectionKey,
    IntegerOutOfRange,
}

impl UnresolvedReason {
    pub fn as_str(self) -> &'static str {
        match self {
            UnresolvedReason::DifferingValues => "differing_values",
            UnresolvedReason::DisjointAllowedClasses => "disjoint_allowed_classes",
            UnresolvedReason::UnrecognizedClass => "unrecognized_class",
            UnresolvedReason::BrokenInternalReference => "broken_internal_reference",
            UnresolvedReason::AcceptorIsDependentVantage => "acceptor_is_dependent_vantage",
            UnresolvedReason::NoMatchingAuthorityEntry => "no_matching_authority_entry",
            UnresolvedReason::DuplicateCollectionKey => "duplicate_collection_key",
            UnresolvedReason::IntegerOutOfRange => "integer_out_of_range",
        }
    }
}

/// One typed unresolved result. A set keyed by `(kind, subject, conditions)`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Unresolved {
    pub kind: UnresolvedKind,
    pub subject: String,
    pub reason: UnresolvedReason,
    /// Sorted condition IDs; empty means the item names no condition.
    pub conditions: Vec<Id>,
}

/// Provenance of a derivation. Excluded from every identity.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Receipt {
    pub compiler_version: &'static str,
    pub profile_version: &'static str,
    pub requirements_digest: Digest,
    /// A sequence (order kept): the derivation steps that ran.
    pub rule_trace: Vec<&'static str>,
}

/// The result of [`derive`]: exactly one candidate, or none with at least one unresolved item.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct DerivationResult {
    pub candidates: Vec<OrganizationCandidate>,
    pub unresolved: Vec<Unresolved>,
    pub receipt: Receipt,
}

fn reviewer_vantage(obligation: &Id) -> String {
    reviewer_vantage_of(obligation.as_str())
}

/// The reviewer vantage that serves the obligation with this ID: the one definition of the
/// `vantage:<obligationId>` participant ID, used by derivation and by `view()` validation.
pub(crate) fn reviewer_vantage_of(obligation: &str) -> String {
    format!("vantage:{obligation}")
}

/// The participant a profile-fixed relationship kind must come from, where the profile fixes one
/// (amendment 5.1-A1 section 3): `hosts_execution` and `terminalizes` from the campaign,
/// `writes_episode` and `reads_episode` from the episode host. `None` for the kinds whose source
/// varies with the candidate (`serves`, `consumes_at`, `accepts_claim`). Derivation emits these
/// rows from the same two constants; `view()` validation reads them from here.
pub(crate) fn fixed_source(kind: RelationshipKind) -> Option<&'static str> {
    match kind {
        RelationshipKind::HostsExecution | RelationshipKind::Terminalizes => Some(CAMPAIGN),
        RelationshipKind::WritesEpisode | RelationshipKind::ReadsEpisode => Some(EPISODE_HOST),
        RelationshipKind::Serves
        | RelationshipKind::ConsumesAt
        | RelationshipKind::AcceptsClaim => None,
    }
}

fn principal_participant(principal: &Id) -> String {
    format!("principal:{principal}")
}

/// An acceptor "names" a vantage when its declared identifier equals the participant ID, or that
/// ID without its `vantage:` prefix (amendment 4). Only declared identifiers are compared.
fn names_vantage(acceptor: &Id, vantage: &str) -> bool {
    acceptor.as_str() == vantage || vantage.strip_prefix("vantage:") == Some(acceptor.as_str())
}

fn covers_acceptor_dependency(authority: &RequiredAuthority, condition: &Condition) -> bool {
    authority.principal == condition.acceptor
        && authority.permission.as_str() == PERMISSION_ACCEPT_CLAIM
        && authority
            .boundaries
            .as_ref()
            .is_none_or(|boundaries| boundaries.contains(&condition.consumption_boundary))
}

fn src(kind: &'static str, id: &str, field: &str) -> TraceRef {
    TraceRef {
        kind,
        id: id.to_owned(),
        field: field.to_owned(),
    }
}
fn src_obligation(id: &Id, field: &str) -> TraceRef {
    src("obligation", id.as_str(), field)
}
fn src_condition(id: &Id, field: &str) -> TraceRef {
    src("acceptanceCondition", id.as_str(), field)
}
fn src_authority(id: &Id, field: &str) -> TraceRef {
    src("requiredAuthority", id.as_str(), field)
}
fn src_scope() -> TraceRef {
    src("problemScope", "problemScope", "")
}
fn src_policy() -> TraceRef {
    src("organizationPolicy", "organizationPolicy", "profile")
}

/// Derives the organization candidate under a fixed profile. Pure and deterministic; the same
/// requirements give the same candidate bytes. Returns no candidate when a condition conflicts,
/// a consumer class is unknown, an acceptor's `accept_claim` dependency is not stated, or an
/// acceptor is a dependent vantage.
pub fn derive(requirements: &OrganizationalRequirements, profile: Profile) -> DerivationResult {
    let Profile::FixedSliceReviewV1 = profile;
    let mut rule_trace: Vec<&'static str> = Vec::new();
    let mut unresolved: Vec<Unresolved> = Vec::new();
    let obligations = requirements.obligations();
    let conditions = requirements.conditions();

    let conditions_of = |obligation: &Obligation| -> Vec<&Condition> {
        conditions
            .iter()
            .filter(|condition| condition.obligation == obligation.id)
            .collect()
    };
    let is_builder = |condition: &Condition| condition.consumer_class.as_str() == CLASS_BUILDER;
    let has_builder_vantage = conditions.iter().any(is_builder);
    let consumer_of = |condition: &Condition| -> Option<&'static str> {
        match condition.consumer_class.as_str() {
            CLASS_BUILDER => Some(BUILDER_VANTAGE),
            CLASS_CAMPAIGN => Some(CAMPAIGN),
            _ => None,
        }
    };

    // Rule 2: an unknown consumer class is unresolved (one item per distinct class value).
    rule_trace.push("consumer_classes");
    let mut unknown_classes: BTreeMap<&str, Vec<Id>> = BTreeMap::new();
    for condition in conditions {
        if consumer_of(condition).is_none() {
            unknown_classes
                .entry(condition.consumer_class.as_str())
                .or_default()
                .push(condition.id.clone());
        }
    }
    for (_, ids) in unknown_classes {
        unresolved.push(Unresolved {
            kind: UnresolvedKind::UnknownConsumerClass,
            subject: "consumerClass".to_owned(),
            reason: UnresolvedReason::UnrecognizedClass,
            conditions: ids,
        });
    }

    // Rule 4: effective requirements by the F4 table, per reviewer vantage.
    rule_trace.push("vantage_requirements");
    let mut vantage_requirements: Vec<VantageRequirement> = Vec::new();
    for obligation in obligations {
        let attached = conditions_of(obligation);
        match combine(&attached) {
            Ok((effective, effective_constraints)) => {
                vantage_requirements.push(VantageRequirement {
                    vantage: reviewer_vantage(&obligation.id),
                    effective,
                    effective_constraints,
                    derived_from: attached.iter().map(|c| c.id.clone()).collect(),
                });
            }
            Err(conflicts) => unresolved.extend(conflicts),
        }
    }

    // Amendment 3A.2: the acceptor dependency closure (checks it is stated; establishes nothing).
    rule_trace.push("authority_closure");
    let mut uncovered: BTreeMap<&str, Vec<Id>> = BTreeMap::new();
    for condition in conditions {
        let covered = requirements
            .required_authorities()
            .iter()
            .any(|authority| covers_acceptor_dependency(authority, condition));
        if !covered {
            uncovered
                .entry(condition.acceptor.as_str())
                .or_default()
                .push(condition.id.clone());
        }
    }
    for (acceptor, ids) in uncovered {
        unresolved.push(Unresolved {
            kind: UnresolvedKind::UnstatedAuthorityDependency,
            subject: format!("principal:{acceptor}"),
            reason: UnresolvedReason::NoMatchingAuthorityEntry,
            conditions: ids,
        });
    }

    // Amendment 4: dependent vantages and self-acceptance (declared identifiers only).
    rule_trace.push("self_acceptance");
    let dependents_of = |condition: &Condition| -> Vec<String> {
        let mut vantages = vec![reviewer_vantage(&condition.obligation)];
        if has_builder_vantage {
            vantages.push(BUILDER_VANTAGE.to_owned());
        }
        vantages.sort();
        vantages
    };
    let mut self_accepting: BTreeMap<&str, Vec<Id>> = BTreeMap::new();
    for condition in conditions {
        if dependents_of(condition)
            .iter()
            .any(|vantage| names_vantage(&condition.acceptor, vantage))
        {
            self_accepting
                .entry(condition.acceptor.as_str())
                .or_default()
                .push(condition.id.clone());
        }
    }
    for (acceptor, ids) in self_accepting {
        unresolved.push(Unresolved {
            kind: UnresolvedKind::SelfAcceptance,
            subject: format!("principal:{acceptor}"),
            reason: UnresolvedReason::AcceptorIsDependentVantage,
            conditions: ids,
        });
    }

    let requirements_digest = requirements.requirements_digest();
    let make_receipt = |rule_trace: Vec<&'static str>| Receipt {
        compiler_version: env!("CARGO_PKG_VERSION"),
        profile_version: profile.as_str(),
        requirements_digest,
        rule_trace,
    };

    if !unresolved.is_empty() {
        sort_unresolved(&mut unresolved);
        return DerivationResult {
            candidates: Vec::new(),
            unresolved,
            receipt: make_receipt(rule_trace),
        };
    }

    // From here the input has no conflict: build the candidate members.
    let mut internal: Vec<InternalFault> = Vec::new();

    // Participants.
    rule_trace.push("participants");
    let mut participants: Vec<Participant> = vec![
        Participant {
            id: CAMPAIGN.to_owned(),
            kind: ParticipantKind::Component,
            serves: Vec::new(),
        },
        Participant {
            id: EPISODE_HOST.to_owned(),
            kind: ParticipantKind::Component,
            serves: Vec::new(),
        },
    ];
    for obligation in obligations {
        participants.push(Participant {
            id: reviewer_vantage(&obligation.id),
            kind: ParticipantKind::Vantage,
            serves: vec![obligation.id.clone()],
        });
    }
    if has_builder_vantage {
        participants.push(Participant {
            id: BUILDER_VANTAGE.to_owned(),
            kind: ParticipantKind::Vantage,
            serves: Vec::new(),
        });
    }
    let principals: BTreeSet<&Id> = conditions
        .iter()
        .map(|condition| &condition.acceptor)
        .chain(
            requirements
                .required_authorities()
                .iter()
                .map(|authority| &authority.principal),
        )
        .collect();
    for principal in &principals {
        participants.push(Participant {
            id: principal_participant(principal),
            kind: ParticipantKind::Principal,
            serves: Vec::new(),
        });
    }
    participants.sort_by(|a, b| a.id.cmp(&b.id));
    check_unique(
        participants.iter().map(|p| p.id.as_str()),
        "participants",
        &mut internal,
    );

    // Conditions.
    rule_trace.push("conditions");
    let candidate_conditions: Vec<CandidateCondition> = conditions
        .iter()
        .filter_map(|condition| {
            let consumer = require_consumer(consumer_of(condition), &mut internal)?;
            Some(CandidateCondition {
                id: condition.id.clone(),
                obligation: condition.obligation.clone(),
                vantage: reviewer_vantage(&condition.obligation),
                consumer: consumer.to_owned(),
                boundary: condition.consumption_boundary.clone(),
                proposition: condition.proposition.clone(),
                covered_state: condition.covered_state.clone(),
                normative: condition.normative.clone(),
                realization_constraints: condition.realization_constraints.clone(),
                acceptor: condition.acceptor.clone(),
                unestablished_route: condition.unestablished_route.clone(),
            })
        })
        .collect();

    // Relationships (amendment 3).
    rule_trace.push("relationships");
    let mut relationships: Vec<Relationship> = Vec::new();
    let mut relate = |kind, from: &str, to: Option<String>, condition: Option<&Id>| {
        relationships.push(Relationship {
            kind,
            from: from.to_owned(),
            to,
            condition: condition.cloned(),
        });
    };
    for obligation in obligations {
        let vantage = reviewer_vantage(&obligation.id);
        relate(
            RelationshipKind::Serves,
            &vantage,
            Some(obligation.id.as_str().to_owned()),
            None,
        );
        relate(
            RelationshipKind::HostsExecution,
            CAMPAIGN,
            Some(vantage.clone()),
            None,
        );
        relate(
            RelationshipKind::WritesEpisode,
            EPISODE_HOST,
            Some(vantage.clone()),
            None,
        );
        relate(
            RelationshipKind::ReadsEpisode,
            EPISODE_HOST,
            Some(vantage),
            None,
        );
    }
    if has_builder_vantage {
        relate(
            RelationshipKind::HostsExecution,
            CAMPAIGN,
            Some(BUILDER_VANTAGE.to_owned()),
            None,
        );
    }
    for condition in conditions {
        let vantage = reviewer_vantage(&condition.obligation);
        let Some(consumer) = require_consumer(consumer_of(condition), &mut internal) else {
            continue;
        };
        relate(
            RelationshipKind::ConsumesAt,
            consumer,
            Some(vantage.clone()),
            Some(&condition.id),
        );
        relate(
            RelationshipKind::AcceptsClaim,
            &principal_participant(&condition.acceptor),
            Some(vantage),
            Some(&condition.id),
        );
    }
    relate(RelationshipKind::Terminalizes, CAMPAIGN, None, None);
    relationships.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
    check_unique(
        relationships.iter().map(Relationship::key),
        "relationships",
        &mut internal,
    );

    // Authority requirements: each required authority attached unchanged.
    rule_trace.push("authority_requirements");
    let authority_requirements: Vec<AuthorityRequirement> = requirements
        .required_authorities()
        .iter()
        .map(|authority| AuthorityRequirement {
            id: authority.id.clone(),
            principal: authority.principal.clone(),
            permission: authority.permission.clone(),
            boundaries: authority.boundaries.clone(),
        })
        .collect();

    // Separation requirements (one per condition) and component binding requirements.
    rule_trace.push("separation_requirements");
    let separation_requirements: Vec<SeparationRequirement> = conditions
        .iter()
        .map(|condition| SeparationRequirement {
            condition: condition.id.clone(),
            acceptor: principal_participant(&condition.acceptor),
            vantages: dependents_of(condition),
        })
        .collect();
    rule_trace.push("component_binding_requirements");
    let component_binding_requirements = vec![
        ComponentBindingRequirement {
            component: CAMPAIGN.to_owned(),
            permissions: vec![PERMISSION_TERMINALIZE.to_owned()],
        },
        ComponentBindingRequirement {
            component: EPISODE_HOST.to_owned(),
            permissions: vec![
                PERMISSION_READ_EPISODE.to_owned(),
                PERMISSION_WRITE_EPISODE.to_owned(),
            ],
        },
    ];

    rule_trace.push("trace");
    let requirement_trace = build_trace(
        requirements,
        &participants,
        &candidate_conditions,
        &vantage_requirements,
        &relationships,
        &authority_requirements,
        &separation_requirements,
        &component_binding_requirements,
        has_builder_vantage,
        &mut internal,
    );

    if !internal.is_empty() {
        return DerivationResult {
            candidates: Vec::new(),
            unresolved: internal_error_items(internal),
            receipt: make_receipt(rule_trace),
        };
    }

    let mut candidate = OrganizationCandidate {
        requirements_digest,
        profile,
        problem_scope: requirements.problem_scope().clone(),
        obligations: obligations.to_vec(),
        participants,
        conditions: candidate_conditions,
        vantage_requirements,
        relationships,
        authority_requirements,
        separation_requirements,
        component_binding_requirements,
        requirement_trace,
        canonical: Vec::new(),
        content_digest: Digest::frame(DigestKind::Candidate, b""),
    };
    match candidate_json(&candidate) {
        Ok(json) => candidate.canonical = json.canonical_bytes(),
        Err(_violation) => {
            return DerivationResult {
                candidates: Vec::new(),
                unresolved: internal_error_items(vec![InternalFault::new(
                    "integer",
                    UnresolvedReason::IntegerOutOfRange,
                )]),
                receipt: make_receipt(rule_trace),
            };
        }
    }
    candidate.content_digest = Digest::frame(DigestKind::Candidate, &candidate.canonical);
    DerivationResult {
        candidates: vec![candidate],
        unresolved: Vec::new(),
        receipt: make_receipt(rule_trace),
    }
}

/// A failed derivation invariant: the subject it concerns and the reason token.
#[derive(Debug, Clone, PartialEq, Eq)]
struct InternalFault {
    subject: String,
    reason: UnresolvedReason,
    /// The condition IDs the fault names; empty when it names none.
    conditions: Vec<Id>,
}

impl InternalFault {
    fn new(subject: &str, reason: UnresolvedReason) -> Self {
        InternalFault {
            subject: subject.to_owned(),
            reason,
            conditions: Vec::new(),
        }
    }

    /// A reference that the derivation itself stated and that no longer resolves: the trace
    /// subject names what was being traced, the condition ID names the missing link.
    fn broken_reference(subject: String, conditions: Vec<Id>) -> Self {
        InternalFault {
            subject,
            reason: UnresolvedReason::BrokenInternalReference,
            conditions,
        }
    }
}

/// The unresolved items for failed derivation invariants, sorted.
fn internal_error_items(faults: Vec<InternalFault>) -> Vec<Unresolved> {
    let mut items: Vec<Unresolved> = faults
        .into_iter()
        .map(|fault| Unresolved {
            kind: UnresolvedKind::InternalError,
            subject: fault.subject,
            reason: fault.reason,
            conditions: {
                let mut conditions = fault.conditions;
                conditions.sort();
                conditions
            },
        })
        .collect();
    sort_unresolved(&mut items);
    items.dedup();
    items
}

fn check_unique<I, S>(keys: I, collection: &'static str, internal: &mut Vec<InternalFault>)
where
    I: IntoIterator<Item = S>,
    S: Ord,
{
    let mut seen = BTreeSet::new();
    for key in keys {
        if !seen.insert(key) {
            internal.push(InternalFault::new(
                collection,
                UnresolvedReason::DuplicateCollectionKey,
            ));
            return;
        }
    }
}

/// The consumer participant of a condition that passed the early consumer-class check. A condition
/// without one cannot reach this point; if it does, record an internal fault and yield no
/// consumer, never a default one (the derivation then returns no candidate).
fn require_consumer(
    consumer: Option<&'static str>,
    internal: &mut Vec<InternalFault>,
) -> Option<&'static str> {
    if consumer.is_none() {
        internal.push(InternalFault::new(
            "consumerClass",
            UnresolvedReason::BrokenInternalReference,
        ));
    }
    consumer
}

fn sort_unresolved(items: &mut [Unresolved]) {
    items.sort_by(|a, b| {
        (a.kind.as_str(), &a.subject, &a.conditions).cmp(&(
            b.kind.as_str(),
            &b.subject,
            &b.conditions,
        ))
    });
}

/// The F4 combination of the conditions attached to one vantage (contract r4 3.2).
fn combine(
    attached: &[&Condition],
) -> Result<(Normative, Vec<RealizationConstraint>), Vec<Unresolved>> {
    let ids = |select: &dyn Fn(&Condition) -> bool| -> Vec<Id> {
        attached
            .iter()
            .filter(|c| select(c))
            .map(|c| c.id.clone())
            .collect()
    };
    let conflict = |subject: String, reason, conditions| Unresolved {
        kind: UnresolvedKind::ConditionConflict,
        subject,
        reason,
        conditions,
    };
    let mut conflicts = Vec::new();
    let first = &attached[0].normative;

    if attached
        .iter()
        .any(|c| c.normative.continuity != first.continuity)
    {
        conflicts.push(conflict(
            "continuity".to_owned(),
            UnresolvedReason::DifferingValues,
            ids(&|_| true),
        ));
    }
    if attached
        .iter()
        .any(|c| c.normative.observer_class != first.observer_class)
    {
        conflicts.push(conflict(
            "observerClass".to_owned(),
            UnresolvedReason::DifferingValues,
            ids(&|_| true),
        ));
    }

    let mut constraints: Vec<RealizationConstraint> = Vec::new();
    for dimension in [Dimension::ExecutionConfinement, Dimension::ProviderFamily] {
        let stating: Vec<&RealizationConstraint> = attached
            .iter()
            .flat_map(|c| c.realization_constraints.iter())
            .filter(|constraint| constraint.dimension == dimension)
            .collect();
        let Some(head) = stating.first() else {
            continue;
        };
        let intersection: BTreeSet<RealizationClass> = stating
            .iter()
            .skip(1)
            .fold(head.allowed.clone(), |acc, constraint| {
                acc.intersection(&constraint.allowed).cloned().collect()
            });
        if intersection.is_empty() {
            conflicts.push(conflict(
                format!("realization:{}", dimension.as_str()),
                UnresolvedReason::DisjointAllowedClasses,
                ids(&|c| {
                    c.realization_constraints
                        .iter()
                        .any(|constraint| constraint.dimension == dimension)
                }),
            ));
        } else {
            constraints.push(RealizationConstraint {
                dimension,
                allowed: intersection,
            });
        }
    }
    if !conflicts.is_empty() {
        return Err(conflicts);
    }

    let mut independence: Option<BTreeSet<_>> = None;
    for condition in attached {
        if let Some(from) = &condition.normative.independence {
            independence
                .get_or_insert_with(BTreeSet::new)
                .extend(from.iter().copied());
        }
    }
    let capabilities = attached
        .iter()
        .flat_map(|c| c.normative.capabilities.iter().cloned())
        .collect();
    let effective = Normative {
        continuity: first.continuity,
        mutation: first.mutation,
        independence,
        capabilities,
        integrity: first.integrity,
        evidence_custody: first.evidence_custody,
        observer_class: first.observer_class,
    };
    Ok((effective, constraints))
}

// ---------------------------------------------------------------------------------------------
// Trace

#[allow(clippy::too_many_arguments)]
fn build_trace(
    requirements: &OrganizationalRequirements,
    participants: &[Participant],
    candidate_conditions: &[CandidateCondition],
    vantage_requirements: &[VantageRequirement],
    relationships: &[Relationship],
    authority_requirements: &[AuthorityRequirement],
    separation_requirements: &[SeparationRequirement],
    component_binding_requirements: &[ComponentBindingRequirement],
    has_builder_vantage: bool,
    internal: &mut Vec<InternalFault>,
) -> Vec<TraceRow> {
    let conditions = requirements.conditions();
    let condition = |id: &Id| conditions.iter().find(|c| &c.id == id);
    let mut rows: BTreeSet<TraceRow> = BTreeSet::new();
    let mut add = |kind: &'static str, id: &str, field: &str, source: TraceRef| {
        rows.insert(TraceRow {
            target: src(kind, id, field),
            source,
        });
    };
    let builder_conditions: Vec<&Condition> = conditions
        .iter()
        .filter(|c| c.consumer_class.as_str() == CLASS_BUILDER)
        .collect();

    // Problem scope and obligations: copied unchanged, one row per member.
    for field in ["runId", "planLineage", "sliceNumber"] {
        add(
            "problemScope",
            "problemScope",
            field,
            src("problemScope", "problemScope", field),
        );
    }
    for obligation in requirements.obligations() {
        let id = obligation.id.as_str();
        for field in [
            "id",
            "kind",
            "subject",
            "resultContract.id",
            "resultContract.version",
            "scope.digest",
        ] {
            add(
                "obligation",
                id,
                field,
                src_obligation(&obligation.id, field),
            );
        }
    }

    // Participants.
    for participant in participants {
        let id = participant.id.as_str();
        match participant.kind {
            ParticipantKind::Component => add("participant", id, "", src_policy()),
            ParticipantKind::Vantage if id == BUILDER_VANTAGE => {
                for c in &builder_conditions {
                    add("participant", id, "", src_condition(&c.id, "consumerClass"));
                }
            }
            ParticipantKind::Vantage => {
                for obligation in &participant.serves {
                    add("participant", id, "", src_obligation(obligation, "id"));
                }
            }
            ParticipantKind::Principal => {
                for c in conditions {
                    if principal_participant(&c.acceptor) == id {
                        add("participant", id, "", src_condition(&c.id, "acceptor"));
                    }
                }
                for a in requirements.required_authorities() {
                    if principal_participant(&a.principal) == id {
                        add("participant", id, "", src_authority(&a.id, "principal"));
                    }
                }
            }
        }
    }

    // Conditions: one row per candidate member that originates in the input.
    for c in candidate_conditions {
        let id = c.id.as_str();
        let mut fields: Vec<(String, &str, String)> = vec![
            ("id".into(), id, "id".into()),
            ("obligation".into(), id, "obligation".into()),
            ("vantage".into(), id, "obligation".into()),
            ("consumer".into(), id, "consumerClass".into()),
            ("boundary".into(), id, "consumptionBoundary".into()),
            ("proposition".into(), id, "proposition".into()),
            ("coveredState".into(), id, "coveredState".into()),
            ("acceptor".into(), id, "acceptor".into()),
            ("unestablishedRoute".into(), id, "unestablishedRoute".into()),
        ];
        for property in [
            "continuity",
            "mutation",
            "capabilities",
            "integrity",
            "evidenceCustody",
            "observerClass",
        ] {
            fields.push((
                format!("normative.{property}"),
                id,
                format!("normative.{property}"),
            ));
        }
        if c.normative.independence.is_some() {
            fields.push((
                "normative.independence".into(),
                id,
                "normative.independence".into(),
            ));
        }
        for constraint in &c.realization_constraints {
            let name = format!("realizationConstraints.{}", constraint.dimension.as_str());
            fields.push((name.clone(), id, name));
        }
        for (target_field, id, source_field) in fields {
            add(
                "condition",
                id,
                &target_field,
                src_condition(&c.id, &source_field),
            );
        }
    }

    // Vantage requirements: one row per contributing condition for each effective property.
    for requirement in vantage_requirements {
        let target = requirement.vantage.as_str();
        for condition_id in &requirement.derived_from {
            // A condition the derivation itself listed in `derived_from` must exist in the
            // requirements. Skipping it would drop its trace rows and still yield a candidate.
            let Some(c) = condition(condition_id) else {
                internal.push(InternalFault::broken_reference(
                    format!("trace:vantageRequirement:{target}:derivedFrom"),
                    vec![condition_id.clone()],
                ));
                continue;
            };
            add(
                "vantageRequirement",
                target,
                "derivedFrom",
                src_condition(&c.id, "id"),
            );
            for property in [
                "continuity",
                "mutation",
                "capabilities",
                "integrity",
                "evidenceCustody",
                "observerClass",
            ] {
                add(
                    "vantageRequirement",
                    target,
                    &format!("effective.{property}"),
                    src_condition(&c.id, &format!("normative.{property}")),
                );
            }
            if c.normative.independence.is_some() {
                add(
                    "vantageRequirement",
                    target,
                    "effective.independence",
                    src_condition(&c.id, "normative.independence"),
                );
            }
            for constraint in &c.realization_constraints {
                let name = constraint.dimension.as_str();
                add(
                    "vantageRequirement",
                    target,
                    &format!("effectiveConstraints.{name}"),
                    src_condition(&c.id, &format!("realizationConstraints.{name}")),
                );
            }
        }
    }

    // Relationships: the sources of amendment 3. Every link a relationship states must resolve:
    // a missing condition, vantage target or serving obligation is a broken internal reference,
    // never a legitimate absence (the builder vantage serves no obligation and is handled apart).
    for relationship in relationships {
        let key = relationship.key();
        let mut add_source = |source: TraceRef| add("relationship", &key, "", source);
        let obligation_of = |vantage: &str| -> Option<&Id> {
            participants
                .iter()
                .find(|p| p.id == vantage)
                .and_then(|p| p.serves.first())
        };
        let mut broken = |what: &str, conditions: Vec<Id>| {
            internal.push(InternalFault::broken_reference(
                format!("trace:relationship:{key}:{what}"),
                conditions,
            ));
        };
        match relationship.kind {
            RelationshipKind::Serves => match obligation_of(&relationship.from) {
                Some(obligation) => add_source(src_obligation(obligation, "id")),
                None => broken("serves", Vec::new()),
            },
            RelationshipKind::ConsumesAt | RelationshipKind::AcceptsClaim => {
                let field_sources: &[&str] = if relationship.kind == RelationshipKind::ConsumesAt {
                    &["consumerClass", "consumptionBoundary"]
                } else {
                    &["acceptor"]
                };
                match &relationship.condition {
                    Some(id) if condition(id).is_some() => {
                        for field in field_sources {
                            add_source(src_condition(id, field));
                        }
                    }
                    Some(id) => broken("condition", vec![id.clone()]),
                    None => broken("condition", Vec::new()),
                }
            }
            RelationshipKind::HostsExecution
            | RelationshipKind::WritesEpisode
            | RelationshipKind::ReadsEpisode => {
                add_source(src_policy());
                match relationship.to.as_deref() {
                    Some(BUILDER_VANTAGE) => {
                        for c in &builder_conditions {
                            add_source(src_condition(&c.id, "consumerClass"));
                        }
                    }
                    Some(vantage) => match obligation_of(vantage) {
                        Some(obligation) => add_source(src_obligation(obligation, "id")),
                        None => broken("target", Vec::new()),
                    },
                    None => broken("target", Vec::new()),
                }
            }
            RelationshipKind::Terminalizes => {
                add_source(src_policy());
                add_source(src_scope());
            }
        }
    }

    // Authority requirements, per member.
    for authority in authority_requirements {
        let id = authority.id.as_str();
        let scope_field = if authority.boundaries.is_some() {
            "scope.boundaries"
        } else {
            "scope"
        };
        for field in ["id", "principal", "permission", scope_field] {
            add(
                "authorityRequirement",
                id,
                field,
                src_authority(&authority.id, field),
            );
        }
    }

    // Separation requirements.
    for separation in separation_requirements {
        let id = separation.condition.as_str();
        add(
            "separationRequirement",
            id,
            "",
            src_condition(&separation.condition, "acceptor"),
        );
        add(
            "separationRequirement",
            id,
            "",
            src_condition(&separation.condition, "obligation"),
        );
        if has_builder_vantage {
            for c in &builder_conditions {
                add(
                    "separationRequirement",
                    id,
                    "",
                    src_condition(&c.id, "consumerClass"),
                );
            }
        }
    }

    // Component binding requirements: the profile plus the sources of the implying relationships.
    for binding in component_binding_requirements {
        let id = binding.component.as_str();
        add("componentBindingRequirement", id, "", src_policy());
        if id == EPISODE_HOST {
            for obligation in requirements.obligations() {
                add(
                    "componentBindingRequirement",
                    id,
                    "",
                    src_obligation(&obligation.id, "id"),
                );
            }
        } else {
            add("componentBindingRequirement", id, "", src_scope());
        }
    }

    rows.into_iter().collect()
}

// ---------------------------------------------------------------------------------------------
// Canonical form of the candidate

fn trace_ref_json(reference: &TraceRef) -> JsonValue {
    object(vec![
        ("kind", string(reference.kind)),
        ("id", string(&reference.id)),
        ("field", string(&reference.field)),
    ])
}

fn candidate_json(candidate: &OrganizationCandidate) -> Result<JsonValue, InvariantViolation> {
    let obligations = candidate
        .obligations
        .iter()
        .map(obligation_json)
        .collect::<Result<Vec<_>, _>>()?;
    let participants = candidate.participants.iter().map(|p| {
        let mut members = vec![("id", string(&p.id)), ("kind", string(p.kind.as_str()))];
        if !p.serves.is_empty() {
            members.push(("serves", strings(p.serves.iter().map(Id::as_str))));
        }
        object(members)
    });
    let conditions = candidate.conditions.iter().map(|c| {
        let mut members = vec![
            ("id", id_json(&c.id)),
            ("obligation", id_json(&c.obligation)),
            ("vantage", string(&c.vantage)),
            ("consumer", string(&c.consumer)),
            ("boundary", id_json(&c.boundary)),
            ("proposition", string(&c.proposition)),
            ("coveredState", id_json(&c.covered_state)),
            ("normative", normative_json(&c.normative)),
            ("acceptor", id_json(&c.acceptor)),
            ("unestablishedRoute", id_json(&c.unestablished_route)),
        ];
        if !c.realization_constraints.is_empty() {
            members.push((
                "realizationConstraints",
                constraints_json(&c.realization_constraints),
            ));
        }
        object(members)
    });
    let vantage_requirements = candidate.vantage_requirements.iter().map(|v| {
        let mut members = vec![
            ("vantage", string(&v.vantage)),
            ("effective", normative_json(&v.effective)),
            (
                "derivedFrom",
                strings(v.derived_from.iter().map(Id::as_str)),
            ),
        ];
        if !v.effective_constraints.is_empty() {
            members.push((
                "effectiveConstraints",
                constraints_json(&v.effective_constraints),
            ));
        }
        object(members)
    });
    let relationships = candidate.relationships.iter().map(|r| {
        let mut members = vec![("kind", string(r.kind.as_str())), ("from", string(&r.from))];
        if let Some(to) = &r.to {
            members.push(("to", string(to)));
        }
        if let Some(condition) = &r.condition {
            members.push(("condition", id_json(condition)));
        }
        object(members)
    });
    let authorities = candidate.authority_requirements.iter().map(|a| {
        let mut scope = Vec::new();
        if let Some(boundaries) = &a.boundaries {
            scope.push(("boundaries", id_set_json(boundaries)));
        }
        object(vec![
            ("id", id_json(&a.id)),
            ("principal", id_json(&a.principal)),
            ("permission", id_json(&a.permission)),
            ("scope", object(scope)),
        ])
    });
    let separations = candidate.separation_requirements.iter().map(|s| {
        object(vec![
            ("condition", id_json(&s.condition)),
            ("acceptor", string(&s.acceptor)),
            ("vantages", strings(s.vantages.iter().map(String::as_str))),
        ])
    });
    let bindings = candidate.component_binding_requirements.iter().map(|b| {
        object(vec![
            ("component", string(&b.component)),
            (
                "permissions",
                strings(b.permissions.iter().map(String::as_str)),
            ),
        ])
    });
    let trace = candidate.requirement_trace.iter().map(|row| {
        object(vec![
            ("target", trace_ref_json(&row.target)),
            ("source", trace_ref_json(&row.source)),
        ])
    });
    Ok(object(vec![
        (
            "requirementsDigest",
            string(&candidate.requirements_digest.to_hex()),
        ),
        ("profile", string(candidate.profile.as_str())),
        (
            "problemScope",
            problem_scope_json(&candidate.problem_scope)?,
        ),
        ("obligations", JsonValue::Array(obligations)),
        ("participants", JsonValue::Array(participants.collect())),
        ("conditions", JsonValue::Array(conditions.collect())),
        (
            "vantageRequirements",
            JsonValue::Array(vantage_requirements.collect()),
        ),
        ("relationships", JsonValue::Array(relationships.collect())),
        (
            "authorityRequirements",
            JsonValue::Array(authorities.collect()),
        ),
        (
            "separationRequirements",
            JsonValue::Array(separations.collect()),
        ),
        (
            "componentBindingRequirements",
            JsonValue::Array(bindings.collect()),
        ),
        ("requirementTrace", JsonValue::Array(trace.collect())),
    ]))
}

/// Fault injection for derivation invariants that validated input cannot reach (crate-private,
/// test-only: it does not widen the public surface).
#[cfg(test)]
mod injection_tests {
    use super::*;
    use crate::view::view;
    use serde_json::json;

    fn requirements() -> OrganizationalRequirements {
        let doc = json!({
            "problemScope": {"runId": "run-1", "planLineage": "plan-a", "sliceNumber": 1},
            "obligations": [{
                "id": "ob-a", "kind": "candidate_review", "subject": "campaign_candidate",
                "resultContract": {"id": "implementation-review", "version": 1},
                "scope": {"digest": "a".repeat(64)}
            }],
            "acceptanceConditions": [{
                "id": "cond-1", "obligation": "ob-a",
                "consumptionBoundary": "campaign_terminalization", "consumerClass": "campaign",
                "proposition": "p", "coveredState": "review-result",
                "normative": {
                    "continuity": "fresh_initial", "mutation": "none", "capabilities": [],
                    "integrity": "verified_artifacts",
                    "evidenceCustody": "execution_evidence_custody",
                    "observerClass": "independent_execution_evidence"
                },
                "acceptor": "owner-a",
                "unestablishedRoute": "hold-consumption-for-owner-disposition"
            }],
            "requiredAuthorities": [
                {"id": "auth-1", "principal": "owner-a", "permission": "accept_claim", "scope": {}}
            ],
            "organizationPolicy": {"mode": "fixed", "profile": "fixed-slice-review-v1"}
        });
        OrganizationalRequirements::from_json_bytes(&serde_json::to_vec(&doc).unwrap()).unwrap()
    }

    #[test]
    fn a_duplicate_key_is_detected_and_named_as_an_internal_error() {
        let mut internal = Vec::new();
        check_unique(["a", "b", "c"], "participants", &mut internal);
        assert!(internal.is_empty());
        check_unique(["a", "b", "a"], "participants", &mut internal);
        check_unique([1, 1], "relationships", &mut internal);
        let subjects: Vec<&str> = internal.iter().map(|f| f.subject.as_str()).collect();
        assert_eq!(subjects, ["participants", "relationships"]);
        let items = internal_error_items(internal.into_iter().rev().collect());
        let rendered: Vec<(&str, &str, &str)> = items
            .iter()
            .map(|u| (u.kind.as_str(), u.subject.as_str(), u.reason.as_str()))
            .collect();
        assert_eq!(
            rendered,
            [
                ("internal_error", "participants", "duplicate_collection_key"),
                (
                    "internal_error",
                    "relationships",
                    "duplicate_collection_key"
                ),
            ]
        );
        assert!(items.iter().all(|u| u.conditions.is_empty()));
    }

    #[test]
    fn a_consumer_that_slipped_past_the_early_check_is_an_internal_error_not_the_campaign() {
        let mut internal = Vec::new();
        assert_eq!(
            require_consumer(Some(CAMPAIGN), &mut internal),
            Some(CAMPAIGN)
        );
        assert!(internal.is_empty());
        assert_eq!(require_consumer(None, &mut internal), None);
        assert_eq!(internal.len(), 1);
        let items = internal_error_items(internal);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, UnresolvedKind::InternalError);
        assert_eq!(items[0].subject, "consumerClass");
        assert_eq!(items[0].reason, UnresolvedReason::BrokenInternalReference);
    }

    #[test]
    fn a_validated_integer_outside_the_canonical_range_fails_closed_in_candidate_and_json() {
        use crate::json::MAX_SAFE_INTEGER;
        let result = derive(&requirements(), Profile::FixedSliceReviewV1);
        let mut candidate = result.candidates.into_iter().next().unwrap();
        assert!(candidate_json(&candidate).is_ok());
        candidate.problem_scope.slice_number = MAX_SAFE_INTEGER as u64 + 1;
        let violation = candidate_json(&candidate).unwrap_err();
        assert!(violation.detail.contains("2^53 - 1"));
        candidate.problem_scope.slice_number = 1;
        candidate.obligations[0].result_contract.version = u64::MAX;
        assert!(candidate_json(&candidate).is_err());
    }

    /// Capabilities have an empty v1 vocabulary, so validated input never states one; the F4
    /// union is still the rule (contract r4 3.2) and is checked on a crate-private value.
    #[test]
    fn f4_unions_capabilities_when_conditions_state_them() {
        let validated = requirements();
        let mut one = validated.conditions()[0].clone();
        let mut two = one.clone();
        one.normative.capabilities = [Id::parse("x").unwrap()].into_iter().collect();
        two.normative.capabilities = [Id::parse("y").unwrap(), Id::parse("x").unwrap()]
            .into_iter()
            .collect();
        let (effective, _) = combine(&[&one, &two]).unwrap();
        let names: Vec<&str> = effective.capabilities.iter().map(Id::as_str).collect();
        assert_eq!(names, ["x", "y"]);
    }

    /// A disjoint `allowed` needs two disjoint recognized sets, which one class per dimension
    /// cannot form; an empty set (not constructible by validation) stands in for the second.
    #[test]
    fn f4_reports_a_disjoint_dimension_as_a_condition_conflict() {
        use crate::schema::{ProviderFamily, RealizationClass};
        let validated = requirements();
        let mut with_class = validated.conditions()[0].clone();
        let mut without = with_class.clone();
        with_class.id = Id::parse("cond-1").unwrap();
        without.id = Id::parse("cond-2").unwrap();
        with_class.realization_constraints = vec![RealizationConstraint {
            dimension: Dimension::ProviderFamily,
            allowed: [RealizationClass::ProviderFamily(ProviderFamily::Anthropic)]
                .into_iter()
                .collect(),
        }];
        without.realization_constraints = vec![RealizationConstraint {
            dimension: Dimension::ProviderFamily,
            allowed: BTreeSet::new(),
        }];
        let conflicts = combine(&[&with_class, &without]).unwrap_err();
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].kind, UnresolvedKind::ConditionConflict);
        assert_eq!(conflicts[0].subject, "realization:provider_family");
        assert_eq!(
            conflicts[0].reason,
            UnresolvedReason::DisjointAllowedClasses
        );
        let ids: Vec<&str> = conflicts[0].conditions.iter().map(Id::as_str).collect();
        assert_eq!(ids, ["cond-1", "cond-2"]);
    }

    /// Internal combination test, not reachable through the public v1 vocabulary (one
    /// `observerClass` value): the real `combine` is fed two conditions at one vantage whose
    /// observer classes differ, using the `cfg(test)`-only `ObserverClass::TestOnlyAlternate`.
    /// No production class is added and validation cannot produce the second value.
    #[test]
    fn f4_reports_differing_observer_classes_as_a_condition_conflict() {
        use crate::schema::ObserverClass;
        let validated = requirements();
        let mut first = validated.conditions()[0].clone();
        let mut second = first.clone();
        first.id = Id::parse("cond-1").unwrap();
        second.id = Id::parse("cond-2").unwrap();
        second.normative.observer_class = ObserverClass::TestOnlyAlternate;
        assert_ne!(
            first.normative.observer_class,
            second.normative.observer_class
        );
        let conflicts = combine(&[&first, &second]).unwrap_err();
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].kind, UnresolvedKind::ConditionConflict);
        assert_eq!(conflicts[0].subject, "observerClass");
        assert_eq!(conflicts[0].reason, UnresolvedReason::DifferingValues);
        let ids: Vec<&str> = conflicts[0].conditions.iter().map(Id::as_str).collect();
        assert_eq!(ids, ["cond-1", "cond-2"]);

        // The same two conditions agreeing yield an effective class; the conflicting pair yields
        // no effective normative requirements at all (the Err above carries no candidate value).
        let (effective, _) = combine(&[&first, &first.clone()]).unwrap();
        assert_eq!(
            effective.observer_class,
            ObserverClass::IndependentExecutionEvidence
        );
    }

    /// Runs the real `build_trace` over a derived candidate's parts, after `inject` has broken
    /// one internal link, and returns the rows and the recorded faults.
    fn trace_with_injected_break(
        inject: impl FnOnce(&mut Vec<VantageRequirement>, &mut Vec<Relationship>),
    ) -> (Vec<TraceRow>, Vec<InternalFault>) {
        let validated = requirements();
        let result = derive(&validated, Profile::FixedSliceReviewV1);
        let candidate = result.candidates.into_iter().next().unwrap();
        let mut vantage_requirements = candidate.vantage_requirements().to_vec();
        let mut relationships = candidate.relationships().to_vec();
        inject(&mut vantage_requirements, &mut relationships);
        let has_builder = candidate
            .participants()
            .iter()
            .any(|p| p.id == BUILDER_VANTAGE);
        let mut faults = Vec::new();
        let rows = build_trace(
            &validated,
            candidate.participants(),
            candidate.conditions(),
            &vantage_requirements,
            &relationships,
            candidate.authority_requirements(),
            candidate.separation_requirements(),
            candidate.component_binding_requirements(),
            has_builder,
            &mut faults,
        );
        (rows, faults)
    }

    #[test]
    fn an_unbroken_trace_records_no_fault() {
        let (rows, faults) = trace_with_injected_break(|_, _| {});
        assert!(faults.is_empty());
        let candidate = derive(&requirements(), Profile::FixedSliceReviewV1)
            .candidates
            .into_iter()
            .next()
            .unwrap();
        assert_eq!(rows, candidate.requirement_trace());
    }

    /// Before the repair a `derived_from` entry naming an absent condition was skipped with
    /// `continue`: its trace rows vanished and the candidate was still produced.
    #[test]
    fn a_vantage_requirement_naming_an_absent_condition_is_an_internal_error_naming_both() {
        let (_, faults) = trace_with_injected_break(|vantage_requirements, _| {
            vantage_requirements[0]
                .derived_from
                .push(Id::parse("no-such-condition").unwrap());
        });
        let vantage = vantage_requirements_vantage();
        let items = internal_error_items(faults);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, UnresolvedKind::InternalError);
        assert_eq!(
            items[0].subject,
            format!("trace:vantageRequirement:{vantage}:derivedFrom")
        );
        let ids: Vec<&str> = items[0].conditions.iter().map(Id::as_str).collect();
        assert_eq!(ids, ["no-such-condition"]);
    }

    /// The broken link is an internal-integrity fault, not an unrecognized class value: the item
    /// is an `internal_error` with `broken_internal_reference` and names the broken condition.
    #[test]
    fn a_broken_trace_link_is_reported_as_a_broken_internal_reference_naming_the_condition() {
        let (_, faults) = trace_with_injected_break(|vantage_requirements, relationships| {
            vantage_requirements[0]
                .derived_from
                .push(Id::parse("no-such-condition").unwrap());
            relationships
                .iter_mut()
                .find(|r| r.kind == RelationshipKind::ConsumesAt)
                .unwrap()
                .condition = Some(Id::parse("also-missing").unwrap());
        });
        let items = internal_error_items(faults);
        assert_eq!(items.len(), 2, "{items:?}");
        for item in &items {
            assert_eq!(item.kind, UnresolvedKind::InternalError);
            assert_eq!(item.reason, UnresolvedReason::BrokenInternalReference);
            assert_eq!(item.reason.as_str(), "broken_internal_reference");
            assert_ne!(item.reason, UnresolvedReason::UnrecognizedClass);
        }
        let named: Vec<Vec<&str>> = items
            .iter()
            .map(|i| i.conditions.iter().map(Id::as_str).collect())
            .collect();
        assert!(named.contains(&vec!["no-such-condition"]), "{named:?}");
        assert!(named.contains(&vec!["also-missing"]), "{named:?}");
    }

    fn vantage_requirements_vantage() -> String {
        let candidate = derive(&requirements(), Profile::FixedSliceReviewV1)
            .candidates
            .into_iter()
            .next()
            .unwrap();
        candidate.vantage_requirements()[0].vantage.clone()
    }

    #[test]
    fn a_relationship_with_an_unresolved_link_is_an_internal_error_not_a_skipped_row() {
        // Serves from a vantage that is not a participant.
        let (_, faults) = trace_with_injected_break(|_, relationships| {
            let serves = relationships
                .iter_mut()
                .find(|r| r.kind == RelationshipKind::Serves)
                .unwrap();
            serves.from = "vantage:ghost".to_owned();
        });
        assert_eq!(faults.len(), 1);
        assert!(faults[0].subject.ends_with(":serves"), "{faults:?}");

        // ConsumesAt naming an absent condition, and one naming none.
        let (_, faults) = trace_with_injected_break(|_, relationships| {
            let consumes = relationships
                .iter_mut()
                .find(|r| r.kind == RelationshipKind::ConsumesAt)
                .unwrap();
            consumes.condition = Some(Id::parse("no-such-condition").unwrap());
        });
        assert_eq!(faults.len(), 1);
        assert!(faults[0].subject.ends_with(":condition"), "{faults:?}");
        assert_eq!(faults[0].conditions[0].as_str(), "no-such-condition");
        let (_, faults) = trace_with_injected_break(|_, relationships| {
            relationships
                .iter_mut()
                .find(|r| r.kind == RelationshipKind::AcceptsClaim)
                .unwrap()
                .condition = None;
        });
        assert_eq!(faults.len(), 1);
        assert!(faults[0].subject.ends_with(":condition"), "{faults:?}");

        // Hosting a vantage that serves no obligation and is not the builder, and one with no
        // target at all.
        for target in [Some("vantage:ghost".to_owned()), None] {
            let (_, faults) = trace_with_injected_break(|_, relationships| {
                relationships
                    .iter_mut()
                    .find(|r| r.kind == RelationshipKind::HostsExecution)
                    .unwrap()
                    .to = target;
            });
            assert_eq!(faults.len(), 1);
            assert!(faults[0].subject.ends_with(":target"), "{faults:?}");
        }
    }

    #[test]
    fn a_view_obligation_without_a_serves_row_is_shown_unestablished_not_dropped_or_filled() {
        let result = derive(&requirements(), Profile::FixedSliceReviewV1);
        let mut candidate = result.candidates.into_iter().next().unwrap();
        assert_eq!(
            view(&candidate).unwrap().obligations[0]
                .served_by
                .as_deref(),
            Some("vantage:ob-a")
        );
        candidate
            .relationships
            .retain(|r| r.kind != RelationshipKind::Serves);
        let projected = view(&candidate).unwrap();
        assert_eq!(projected.obligations.len(), 1);
        assert_eq!(projected.obligations[0].id.as_str(), "ob-a");
        assert_eq!(projected.obligations[0].served_by, None);
        assert_eq!(
            projected.obligations[0].result_contract.id.as_str(),
            "implementation-review"
        );
    }

    /// Exhaustive destructuring (no `..`): a field added to or removed from the view or the
    /// report fails to compile here until builder/p3-completeness-matrix.md and its tests are
    /// updated. The counts are the matrix's field counts per type, nested fields included.
    #[test]
    fn the_view_and_report_field_lists_are_the_matrix_lists() {
        use crate::enforce::{EnforceabilityReport, EnforceabilityRow, enforceability};
        use crate::view::{
            EpisodeAccess, Hosting, OrganizationView, Terminalization, ViewObligation, ViewVantage,
        };
        let candidate = derive(&requirements(), Profile::FixedSliceReviewV1)
            .candidates
            .remove(0);
        let projected = view(&candidate).unwrap();
        let OrganizationView {
            problem_scope,
            obligations,
            vantages,
            conditions,
            hosting,
            episode_access,
            terminalization,
            separation_requirements,
            authority_requirements,
            component_binding_requirements,
        } = &projected;
        let ProblemScope {
            run_id: _,
            plan_lineage: _,
            slice_number: _,
        } = problem_scope;
        let ViewObligation {
            id: _,
            kind: _,
            subject: _,
            result_contract: crate::schema::ResultContract { id: _, version: _ },
            scope_digest: _,
            served_by: _,
        } = &obligations[0];
        let ViewVantage {
            id: _,
            effective: _,
            effective_constraints: _,
            conditions: _,
        } = &vantages[0];
        let CandidateCondition {
            id: _,
            obligation: _,
            vantage: _,
            consumer: _,
            boundary: _,
            proposition: _,
            covered_state: _,
            normative: _,
            realization_constraints: _,
            acceptor: _,
            unestablished_route: _,
        } = &conditions[0];
        let Hosting {
            component: _,
            vantage: _,
        } = &hosting[0];
        let EpisodeAccess {
            vantage: _,
            writer: _,
            readers: _,
        } = &episode_access[0];
        let Some(Terminalization { component: _ }) = terminalization else {
            panic!("terminalization is always present");
        };
        let SeparationRequirement {
            condition: _,
            acceptor: _,
            vantages: _,
        } = &separation_requirements[0];
        let AuthorityRequirement {
            id: _,
            principal: _,
            permission: _,
            boundaries: _,
        } = &authority_requirements[0];
        let ComponentBindingRequirement {
            component: _,
            permissions: _,
        } = &component_binding_requirements[0];

        let report = enforceability(&candidate, Profile::FixedSliceReviewV1);
        let EnforceabilityReport { profile: _, rows } = &report;
        let EnforceabilityRow {
            property: _,
            scope: _,
            requirement: _,
            status: _,
            evidence_class: _,
            note: _,
        } = &rows[0];
    }
}
