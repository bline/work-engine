//! `OrganizationalRequirements v0` (contract 3.1, 17; decisions in `gap-decisions-r1.md`).
//!
//! Requirements are meaning only and create no authority. They are built from authored JSON bytes
//! by one strict parser: unknown fields, duplicate keys and IDs, explicit null, floats, empty sets
//! where empty would mean absent, and realization identities are refused with a typed
//! [`ValidationError`]. The validated value is held in canonical (sorted) form, so its canonical
//! bytes and `requirementsDigest` do not depend on authored order.

use std::collections::BTreeSet;

use crate::digest::{Digest, DigestKind};
use crate::error::{ValidationCode, ValidationError};
use crate::ids::Id;
use crate::json::{
    InvariantViolation, JsonValue, MAX_SAFE_INTEGER, object, safe_integer, string, strings,
};

/// The derivation profile. The only profile is `fixed-slice-review-v1`; `derive` takes it as an
/// argument so a second profile is a compile-visible change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Profile {
    FixedSliceReviewV1,
}

impl Profile {
    pub fn as_str(self) -> &'static str {
        match self {
            Profile::FixedSliceReviewV1 => "fixed-slice-review-v1",
        }
    }

    /// The version of this profile's class vocabulary. Adding or removing a recognized class is a
    /// revision of this value and of the profile (decision N2); it does not enter any digest.
    pub fn vocabulary_version(self) -> &'static str {
        match self {
            Profile::FixedSliceReviewV1 => "fixed-slice-review-v1/classes-1",
        }
    }

    /// The recognized classes of the profile, as spelled in authored requirements. Everything not
    /// listed is refused by validation. `capabilities` has an empty vocabulary in v1.
    pub fn vocabulary(self) -> Vocabulary {
        match self {
            Profile::FixedSliceReviewV1 => Vocabulary {
                provider_family: ProviderFamily::ALL.iter().map(|c| c.as_str()).collect(),
                execution_confinement: ExecutionConfinement::ALL
                    .iter()
                    .map(|c| c.as_str())
                    .collect(),
                observer_class: ObserverClass::ALL.iter().map(|c| c.as_str()).collect(),
                capabilities: Vec::new(),
            },
        }
    }

    fn recognize_provider_family(self, text: &str) -> Option<ProviderFamily> {
        match self {
            Profile::FixedSliceReviewV1 => ProviderFamily::parse(text),
        }
    }

    fn recognize_execution_confinement(self, text: &str) -> Option<ExecutionConfinement> {
        match self {
            Profile::FixedSliceReviewV1 => ExecutionConfinement::parse(text),
        }
    }

    fn recognize_observer_class(self, text: &str) -> Option<ObserverClass> {
        match self {
            Profile::FixedSliceReviewV1 => ObserverClass::parse(text),
        }
    }
}

/// A profile's closed class vocabulary (see [`Profile::vocabulary`]).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Vocabulary {
    pub provider_family: Vec<&'static str>,
    pub execution_confinement: Vec<&'static str>,
    pub observer_class: Vec<&'static str>,
    pub capabilities: Vec<&'static str>,
}

macro_rules! closed_enum {
    ($(#[$meta:meta])* $name:ident { $($variant:ident = $text:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum $name { $($variant),+ }
        impl $name {
            /// Every recognized value, in declaration order.
            pub const ALL: &'static [$name] = &[$($name::$variant),+];
            pub fn as_str(self) -> &'static str {
                match self { $($name::$variant => $text),+ }
            }
            fn parse(text: &str) -> Option<$name> {
                match text { $($text => Some($name::$variant),)+ _ => None }
            }
        }
    };
}

closed_enum!(
    /// `normative.continuity`.
    Continuity { FreshInitial = "fresh_initial", Retained = "retained" }
);
closed_enum!(
    /// `normative.mutation`: `none` is the only v0 value.
    Mutation { None = "none" }
);
closed_enum!(
    /// A member of `normative.independence.from`.
    IndependenceSource { BuilderContext = "state.builder_context" }
);
closed_enum!(
    /// `normative.integrity`.
    Integrity { VerifiedArtifacts = "verified_artifacts" }
);
closed_enum!(
    /// `normative.evidenceCustody`.
    EvidenceCustody { ExecutionEvidenceCustody = "execution_evidence_custody" }
);
closed_enum!(
    /// The closed dimensions of a realization constraint.
    Dimension { ProviderFamily = "provider_family", ExecutionConfinement = "execution_confinement" }
);
closed_enum!(
    /// A recognized `provider_family` class of `fixed-slice-review-v1`. A class, never a model.
    ProviderFamily { Anthropic = "anthropic" }
);
closed_enum!(
    /// A recognized `execution_confinement` class of `fixed-slice-review-v1`.
    ExecutionConfinement { GuardedSandbox = "guarded_sandbox" }
);
/// The recognized `normative.observerClass` of `fixed-slice-review-v1`: a class, not an
/// observer identity.
///
/// Written out instead of through `closed_enum!` so that a second value exists in `cfg(test)`
/// builds only: the internal F4 observerClass conflict test needs two different classes at one
/// vantage, which the one-class v1 vocabulary cannot provide. The test-only variant is absent
/// from `ALL`, from `parse` and so from the vocabulary and from validation; it cannot be spelled
/// in authored requirements and does not exist outside `cfg(test)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ObserverClass {
    IndependentExecutionEvidence,
    #[cfg(test)]
    TestOnlyAlternate,
}

impl ObserverClass {
    /// Every recognized value, in declaration order.
    pub const ALL: &'static [ObserverClass] = &[ObserverClass::IndependentExecutionEvidence];
    pub fn as_str(self) -> &'static str {
        match self {
            ObserverClass::IndependentExecutionEvidence => "independent_execution_evidence",
            #[cfg(test)]
            ObserverClass::TestOnlyAlternate => "test_only_alternate_observer",
        }
    }
    fn parse(text: &str) -> Option<ObserverClass> {
        match text {
            "independent_execution_evidence" => Some(ObserverClass::IndependentExecutionEvidence),
            _ => None,
        }
    }
}

/// A recognized member of `realizationConstraints[].allowed`; the variant fixes the dimension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RealizationClass {
    ProviderFamily(ProviderFamily),
    ExecutionConfinement(ExecutionConfinement),
}

impl RealizationClass {
    pub fn dimension(self) -> Dimension {
        match self {
            RealizationClass::ProviderFamily(_) => Dimension::ProviderFamily,
            RealizationClass::ExecutionConfinement(_) => Dimension::ExecutionConfinement,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            RealizationClass::ProviderFamily(class) => class.as_str(),
            RealizationClass::ExecutionConfinement(class) => class.as_str(),
        }
    }
}

/// `problemScope`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ProblemScope {
    pub run_id: Id,
    pub plan_lineage: Id,
    pub slice_number: u64,
}

/// `obligations[].resultContract`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ResultContract {
    pub id: Id,
    pub version: u64,
}

/// `obligations[]`; `kind` and `subject` are single-value literals and are not stored.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Obligation {
    pub id: Id,
    pub result_contract: ResultContract,
    pub scope_digest: Digest,
}

/// The single value of `obligations[].kind` (contract 3.1).
pub(crate) const OBLIGATION_KIND: &str = "candidate_review";
/// The single value of `obligations[].subject` (contract 3.1).
pub(crate) const OBLIGATION_SUBJECT: &str = "campaign_candidate";

impl Obligation {
    /// `kind`: the literal `candidate_review`.
    pub fn kind(&self) -> &'static str {
        OBLIGATION_KIND
    }

    /// `subject`: the literal `campaign_candidate`.
    pub fn subject(&self) -> &'static str {
        OBLIGATION_SUBJECT
    }
}

/// `NormativeRequirements v0`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Normative {
    pub continuity: Continuity,
    pub mutation: Mutation,
    /// Present iff the condition counts the review as independent; never present-and-empty.
    pub independence: Option<BTreeSet<IndependenceSource>>,
    pub capabilities: BTreeSet<Id>,
    pub integrity: Integrity,
    pub evidence_custody: EvidenceCustody,
    pub observer_class: ObserverClass,
}

/// `RealizationConstraint v0`: a restriction expressed only as classes.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RealizationConstraint {
    pub dimension: Dimension,
    /// Recognized classes of the dimension; never empty.
    pub allowed: BTreeSet<RealizationClass>,
}

/// `acceptanceConditions[]`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Condition {
    pub id: Id,
    pub obligation: Id,
    pub consumption_boundary: Id,
    pub consumer_class: Id,
    pub proposition: String,
    pub covered_state: Id,
    pub normative: Normative,
    /// Sorted by dimension spelling; empty means no constraint (absent in the canonical form).
    pub realization_constraints: Vec<RealizationConstraint>,
    pub acceptor: Id,
    pub unestablished_route: Id,
}

/// `requiredAuthorities[]`: a dependency, never a grant. `boundaries` absent means the whole
/// problem scope; a present set is never empty.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RequiredAuthority {
    pub id: Id,
    pub principal: Id,
    pub permission: Id,
    pub boundaries: Option<BTreeSet<Id>>,
}

/// Validated, canonical organizational requirements. The only constructor is
/// [`OrganizationalRequirements::from_json_bytes`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrganizationalRequirements {
    problem_scope: ProblemScope,
    obligations: Vec<Obligation>,
    conditions: Vec<Condition>,
    required_authorities: Vec<RequiredAuthority>,
    profile: Profile,
    canonical: Vec<u8>,
    digest: Digest,
}

impl OrganizationalRequirements {
    /// Validates authored JSON bytes. Refuses ambiguity; normalizes only set order and key order.
    pub fn from_json_bytes(bytes: &[u8]) -> Result<OrganizationalRequirements, ValidationError> {
        let value =
            JsonValue::parse_v0(bytes).map_err(|error| fail("", ValidationCode::Codec(error)))?;
        parse_requirements(&value)
    }

    pub fn problem_scope(&self) -> &ProblemScope {
        &self.problem_scope
    }

    /// Sorted by `id`.
    pub fn obligations(&self) -> &[Obligation] {
        &self.obligations
    }

    /// `acceptanceConditions`, sorted by `id`.
    pub fn conditions(&self) -> &[Condition] {
        &self.conditions
    }

    /// Sorted by `id`.
    pub fn required_authorities(&self) -> &[RequiredAuthority] {
        &self.required_authorities
    }

    pub fn profile(&self) -> Profile {
        self.profile
    }

    /// The canonical bytes (contract 17.5) over which `requirementsDigest` is computed.
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical
    }

    /// `requirementsDigest`.
    pub fn requirements_digest(&self) -> Digest {
        self.digest
    }
}

// ---------------------------------------------------------------------------------------------
// Canonical form

pub(crate) fn id_json(id: &Id) -> JsonValue {
    string(id.as_str())
}

/// `problemScope`, shared by the requirements and the candidate so a copy cannot drift.
pub(crate) fn problem_scope_json(scope: &ProblemScope) -> Result<JsonValue, InvariantViolation> {
    Ok(object(vec![
        ("runId", id_json(&scope.run_id)),
        ("planLineage", id_json(&scope.plan_lineage)),
        ("sliceNumber", safe_integer(scope.slice_number)?),
    ]))
}

/// One obligation, shared by the requirements and the candidate so a copy cannot drift.
pub(crate) fn obligation_json(obligation: &Obligation) -> Result<JsonValue, InvariantViolation> {
    Ok(object(vec![
        ("id", id_json(&obligation.id)),
        ("kind", string(obligation.kind())),
        ("subject", string(obligation.subject())),
        (
            "resultContract",
            object(vec![
                ("id", id_json(&obligation.result_contract.id)),
                ("version", safe_integer(obligation.result_contract.version)?),
            ]),
        ),
        (
            "scope",
            object(vec![("digest", string(&obligation.scope_digest.to_hex()))]),
        ),
    ]))
}

pub(crate) fn id_set_json(set: &BTreeSet<Id>) -> JsonValue {
    strings(set.iter().map(Id::as_str))
}

pub(crate) fn normative_json(normative: &Normative) -> JsonValue {
    let Normative {
        continuity,
        mutation,
        independence,
        capabilities,
        integrity,
        evidence_custody,
        observer_class,
    } = normative;
    let mut members = vec![
        ("continuity", string(continuity.as_str())),
        ("mutation", string(mutation.as_str())),
        ("capabilities", id_set_json(capabilities)),
        ("integrity", string(integrity.as_str())),
        ("evidenceCustody", string(evidence_custody.as_str())),
        ("observerClass", string(observer_class.as_str())),
    ];
    if let Some(from) = independence {
        members.push((
            "independence",
            object(vec![("from", strings(from.iter().map(|s| s.as_str())))]),
        ));
    }
    object(members)
}

/// Class spellings in set order of contract 17.2 (UTF-16 code unit, equal to byte order for ASCII).
fn classes_sorted(set: &BTreeSet<RealizationClass>) -> Vec<&'static str> {
    let mut names: Vec<&'static str> = set.iter().map(|class| class.as_str()).collect();
    names.sort_unstable();
    names
}

pub(crate) fn constraints_json(constraints: &[RealizationConstraint]) -> JsonValue {
    JsonValue::Array(
        constraints
            .iter()
            .map(|constraint| {
                object(vec![
                    ("dimension", string(constraint.dimension.as_str())),
                    (
                        "allowed",
                        strings(classes_sorted(&constraint.allowed).into_iter()),
                    ),
                ])
            })
            .collect(),
    )
}

impl OrganizationalRequirements {
    fn to_json(&self) -> Result<JsonValue, InvariantViolation> {
        let obligations = self
            .obligations
            .iter()
            .map(obligation_json)
            .collect::<Result<Vec<_>, _>>()?;
        let conditions = self.conditions.iter().map(|condition| {
            let mut members = vec![
                ("id", id_json(&condition.id)),
                ("obligation", id_json(&condition.obligation)),
                (
                    "consumptionBoundary",
                    id_json(&condition.consumption_boundary),
                ),
                ("consumerClass", id_json(&condition.consumer_class)),
                ("proposition", string(&condition.proposition)),
                ("coveredState", id_json(&condition.covered_state)),
                ("normative", normative_json(&condition.normative)),
                ("acceptor", id_json(&condition.acceptor)),
                (
                    "unestablishedRoute",
                    id_json(&condition.unestablished_route),
                ),
            ];
            if !condition.realization_constraints.is_empty() {
                members.push((
                    "realizationConstraints",
                    constraints_json(&condition.realization_constraints),
                ));
            }
            object(members)
        });
        let authorities = self.required_authorities.iter().map(|authority| {
            let mut scope = Vec::new();
            if let Some(boundaries) = &authority.boundaries {
                scope.push(("boundaries", id_set_json(boundaries)));
            }
            object(vec![
                ("id", id_json(&authority.id)),
                ("principal", id_json(&authority.principal)),
                ("permission", id_json(&authority.permission)),
                ("scope", object(scope)),
            ])
        });
        Ok(object(vec![
            ("problemScope", problem_scope_json(&self.problem_scope)?),
            ("obligations", JsonValue::Array(obligations)),
            (
                "acceptanceConditions",
                JsonValue::Array(conditions.collect()),
            ),
            (
                "requiredAuthorities",
                JsonValue::Array(authorities.collect()),
            ),
            (
                "organizationPolicy",
                object(vec![
                    ("mode", string("fixed")),
                    ("profile", string(self.profile.as_str())),
                ]),
            ),
        ]))
    }
}

// ---------------------------------------------------------------------------------------------
// Strict parsing

fn fail(path: &str, code: ValidationCode) -> ValidationError {
    ValidationError {
        code,
        path: path.to_owned(),
    }
}

struct Reader<'a> {
    members: &'a [(String, JsonValue)],
    path: String,
    seen: Vec<&'static str>,
}

impl<'a> Reader<'a> {
    fn new(value: &'a JsonValue, path: &str) -> Result<Reader<'a>, ValidationError> {
        match value {
            JsonValue::Object(members) => Ok(Reader {
                members,
                path: path.to_owned(),
                seen: Vec::new(),
            }),
            _ => Err(fail(path, ValidationCode::WrongType { expected: "object" })),
        }
    }

    fn child(&self, key: &str) -> String {
        format!("{}/{}", self.path, key)
    }

    fn req(&mut self, key: &'static str) -> Result<(&'a JsonValue, String), ValidationError> {
        self.opt(key).ok_or_else(|| {
            fail(
                &self.path,
                ValidationCode::MissingField {
                    name: key.to_owned(),
                },
            )
        })
    }

    fn opt(&mut self, key: &'static str) -> Option<(&'a JsonValue, String)> {
        self.seen.push(key);
        self.members
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| (value, self.child(key)))
    }

    /// Refuses any member that was not read.
    fn done(self) -> Result<(), ValidationError> {
        for (name, _) in self.members {
            if !self.seen.contains(&name.as_str()) {
                return Err(fail(
                    &self.child(name),
                    ValidationCode::UnknownField { name: name.clone() },
                ));
            }
        }
        Ok(())
    }
}

fn array<'a>(value: &'a JsonValue, path: &str) -> Result<&'a [JsonValue], ValidationError> {
    match value {
        JsonValue::Array(items) => Ok(items),
        _ => Err(fail(path, ValidationCode::WrongType { expected: "array" })),
    }
}

fn text<'a>(value: &'a JsonValue, path: &str) -> Result<&'a str, ValidationError> {
    match value {
        JsonValue::String(text) => Ok(text),
        _ => Err(fail(path, ValidationCode::WrongType { expected: "string" })),
    }
}

fn id_at(value: &JsonValue, path: &str) -> Result<Id, ValidationError> {
    Id::parse(text(value, path)?).map_err(|error| fail(path, ValidationCode::InvalidId(error)))
}

/// A string whose spelling the profile must recognize as a class of `field`. Anything else is
/// refused (decision N2): a concrete identity is never a class, whether or not it has ID syntax.
fn recognized<T>(
    value: &JsonValue,
    path: &str,
    field: &'static str,
    recognize: impl FnOnce(&str) -> Option<T>,
) -> Result<T, ValidationError> {
    let spelled = text(value, path)?;
    recognize(spelled).ok_or_else(|| unrecognized(path, field, spelled))
}

fn unrecognized(path: &str, field: &'static str, value: &str) -> ValidationError {
    fail(
        path,
        ValidationCode::UnrecognizedClass {
            field,
            value: value.to_owned(),
        },
    )
}

fn literal(value: &JsonValue, path: &str, expected: &'static str) -> Result<(), ValidationError> {
    if text(value, path)? == expected {
        Ok(())
    } else {
        Err(fail(
            path,
            ValidationCode::InvalidValue {
                detail: match expected {
                    "candidate_review" => "kind must be candidate_review",
                    "campaign_candidate" => "subject must be campaign_candidate",
                    "fixed" => "mode must be fixed",
                    "fixed-slice-review-v1" => "profile must be fixed-slice-review-v1",
                    _ => "unexpected literal",
                },
            },
        ))
    }
}

fn enum_at<T>(
    value: &JsonValue,
    path: &str,
    parse: fn(&str) -> Option<T>,
    detail: &'static str,
) -> Result<T, ValidationError> {
    parse(text(value, path)?).ok_or_else(|| fail(path, ValidationCode::InvalidValue { detail }))
}

fn unsigned_at(value: &JsonValue, path: &str, minimum: u64) -> Result<u64, ValidationError> {
    match value {
        JsonValue::Integer(number) if *number >= 0 && *number <= MAX_SAFE_INTEGER => {
            let number = *number as u64;
            if number < minimum {
                Err(fail(
                    path,
                    ValidationCode::InvalidValue {
                        detail: "integer below the minimum for this field",
                    },
                ))
            } else {
                Ok(number)
            }
        }
        JsonValue::Integer(_) => Err(fail(
            path,
            ValidationCode::InvalidValue {
                detail: "negative integer",
            },
        )),
        _ => Err(fail(
            path,
            ValidationCode::WrongType {
                expected: "integer",
            },
        )),
    }
}

/// A non-empty set of ID-syntax strings. Duplicates and an empty array are refused.
fn id_set(value: &JsonValue, path: &str) -> Result<BTreeSet<Id>, ValidationError> {
    let items = array(value, path)?;
    if items.is_empty() {
        return Err(fail(path, ValidationCode::EmptySet));
    }
    let mut set = BTreeSet::new();
    for (index, item) in items.iter().enumerate() {
        let item_path = format!("{path}/{index}");
        let id = id_at(item, &item_path)?;
        if !set.insert(id.clone()) {
            return Err(fail(
                &item_path,
                ValidationCode::DuplicateMember {
                    member: id.as_str().to_owned(),
                },
            ));
        }
    }
    Ok(set)
}

/// A set keyed by `id`: sorted by id, duplicate ids refused.
fn keyed<T>(
    value: &JsonValue,
    path: &str,
    mut parse_item: impl FnMut(&JsonValue, &str) -> Result<T, ValidationError>,
    key: impl Fn(&T) -> &Id,
) -> Result<Vec<T>, ValidationError> {
    let mut out: Vec<T> = Vec::new();
    for (index, item) in array(value, path)?.iter().enumerate() {
        let item_path = format!("{path}/{index}");
        let parsed = parse_item(item, &item_path)?;
        if out.iter().any(|existing| key(existing) == key(&parsed)) {
            return Err(fail(
                &item_path,
                ValidationCode::DuplicateId {
                    id: key(&parsed).as_str().to_owned(),
                },
            ));
        }
        out.push(parsed);
    }
    out.sort_by(|a, b| key(a).cmp(key(b)));
    Ok(out)
}

fn parse_normative(
    value: &JsonValue,
    path: &str,
    profile: Profile,
) -> Result<Normative, ValidationError> {
    let mut reader = Reader::new(value, path)?;
    let (v, p) = reader.req("continuity")?;
    let continuity = enum_at(v, &p, Continuity::parse, "unknown continuity")?;
    let (v, p) = reader.req("mutation")?;
    let mutation = enum_at(v, &p, Mutation::parse, "mutation must be none")?;
    let independence = match reader.opt("independence") {
        None => None,
        Some((v, p)) => {
            let mut inner = Reader::new(v, &p)?;
            let (from, from_path) = inner.req("from")?;
            let items = array(from, &from_path)?;
            if items.is_empty() {
                return Err(fail(&from_path, ValidationCode::EmptySet));
            }
            let mut set = BTreeSet::new();
            for (index, item) in items.iter().enumerate() {
                let item_path = format!("{from_path}/{index}");
                let source = enum_at(
                    item,
                    &item_path,
                    IndependenceSource::parse,
                    "independence.from accepts only state.builder_context",
                )?;
                if !set.insert(source) {
                    return Err(fail(
                        &item_path,
                        ValidationCode::DuplicateMember {
                            member: source.as_str().to_owned(),
                        },
                    ));
                }
            }
            inner.done()?;
            Some(set)
        }
    };
    let (v, p) = reader.req("capabilities")?;
    // The v1 capability vocabulary is empty: any member is unrecognized (decision N2).
    let capabilities = {
        let items = array(v, &p)?;
        if let Some(first) = items.first() {
            let spelled = text(first, &format!("{p}/0"))?;
            return Err(unrecognized(&format!("{p}/0"), "capabilities", spelled));
        }
        BTreeSet::new()
    };
    let (v, p) = reader.req("integrity")?;
    let integrity = enum_at(
        v,
        &p,
        Integrity::parse,
        "integrity must be verified_artifacts",
    )?;
    let (v, p) = reader.req("evidenceCustody")?;
    let evidence_custody = enum_at(
        v,
        &p,
        EvidenceCustody::parse,
        "evidenceCustody must be execution_evidence_custody",
    )?;
    let (v, p) = reader.req("observerClass")?;
    let observer_class = recognized(v, &p, "observerClass", |spelled| {
        profile.recognize_observer_class(spelled)
    })?;
    reader.done()?;
    Ok(Normative {
        continuity,
        mutation,
        independence,
        capabilities,
        integrity,
        evidence_custody,
        observer_class,
    })
}

/// `allowed[]`: a non-empty set of classes recognized for `dimension`; duplicates are refused.
fn parse_allowed(
    value: &JsonValue,
    path: &str,
    dimension: Dimension,
    profile: Profile,
) -> Result<BTreeSet<RealizationClass>, ValidationError> {
    let items = array(value, path)?;
    if items.is_empty() {
        return Err(fail(path, ValidationCode::EmptySet));
    }
    let mut set = BTreeSet::new();
    for (index, item) in items.iter().enumerate() {
        let item_path = format!("{path}/{index}");
        let class = match dimension {
            Dimension::ProviderFamily => {
                recognized(item, &item_path, dimension.as_str(), |spelled| {
                    profile
                        .recognize_provider_family(spelled)
                        .map(RealizationClass::ProviderFamily)
                })?
            }
            Dimension::ExecutionConfinement => {
                recognized(item, &item_path, dimension.as_str(), |spelled| {
                    profile
                        .recognize_execution_confinement(spelled)
                        .map(RealizationClass::ExecutionConfinement)
                })?
            }
        };
        if !set.insert(class) {
            return Err(fail(
                &item_path,
                ValidationCode::DuplicateMember {
                    member: class.as_str().to_owned(),
                },
            ));
        }
    }
    Ok(set)
}

fn parse_constraints(
    value: &JsonValue,
    path: &str,
    profile: Profile,
) -> Result<Vec<RealizationConstraint>, ValidationError> {
    let items = array(value, path)?;
    if items.is_empty() {
        return Err(fail(path, ValidationCode::EmptySet));
    }
    let mut out: Vec<RealizationConstraint> = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let item_path = format!("{path}/{index}");
        let mut reader = Reader::new(item, &item_path)?;
        let (v, p) = reader.req("dimension")?;
        let dimension = enum_at(
            v,
            &p,
            Dimension::parse,
            "dimension must be provider_family or execution_confinement",
        )?;
        let (v, p) = reader.req("allowed")?;
        let allowed = parse_allowed(v, &p, dimension, profile)?;
        reader.done()?;
        if out.iter().any(|existing| existing.dimension == dimension) {
            return Err(fail(
                &item_path,
                ValidationCode::DuplicateDimension {
                    dimension: dimension.as_str().to_owned(),
                },
            ));
        }
        out.push(RealizationConstraint { dimension, allowed });
    }
    out.sort_by_key(|constraint| constraint.dimension.as_str());
    Ok(out)
}

fn parse_requirements(root: &JsonValue) -> Result<OrganizationalRequirements, ValidationError> {
    let mut top = Reader::new(root, "")?;

    // The profile owns the class vocabulary, so it is read before any class is recognized.
    let (v, p) = top.req("organizationPolicy")?;
    let mut policy = Reader::new(v, &p)?;
    let (pv, pp) = policy.req("mode")?;
    literal(pv, &pp, "fixed")?;
    let (pv, pp) = policy.req("profile")?;
    literal(pv, &pp, "fixed-slice-review-v1")?;
    policy.done()?;
    let profile = Profile::FixedSliceReviewV1;

    let (v, p) = top.req("problemScope")?;
    let mut reader = Reader::new(v, &p)?;
    let (rv, rp) = reader.req("runId")?;
    let run_id = id_at(rv, &rp)?;
    let (rv, rp) = reader.req("planLineage")?;
    let plan_lineage = id_at(rv, &rp)?;
    let (rv, rp) = reader.req("sliceNumber")?;
    let slice_number = unsigned_at(rv, &rp, 0)?;
    reader.done()?;
    let problem_scope = ProblemScope {
        run_id,
        plan_lineage,
        slice_number,
    };

    let (v, p) = top.req("obligations")?;
    let obligations = keyed(
        v,
        &p,
        |item, path| {
            let mut reader = Reader::new(item, path)?;
            let (v, p) = reader.req("id")?;
            let id = id_at(v, &p)?;
            let (v, p) = reader.req("kind")?;
            literal(v, &p, "candidate_review")?;
            let (v, p) = reader.req("subject")?;
            literal(v, &p, "campaign_candidate")?;
            let (v, p) = reader.req("resultContract")?;
            let mut contract = Reader::new(v, &p)?;
            let (cv, cp) = contract.req("id")?;
            let contract_id = id_at(cv, &cp)?;
            let (cv, cp) = contract.req("version")?;
            let version = unsigned_at(cv, &cp, 1)?;
            contract.done()?;
            let (v, p) = reader.req("scope")?;
            let mut scope = Reader::new(v, &p)?;
            let (sv, sp) = scope.req("digest")?;
            let scope_digest = Digest::from_hex(text(sv, &sp)?)
                .ok_or_else(|| fail(&sp, ValidationCode::InvalidDigest))?;
            scope.done()?;
            reader.done()?;
            Ok(Obligation {
                id,
                result_contract: ResultContract {
                    id: contract_id,
                    version,
                },
                scope_digest,
            })
        },
        |obligation| &obligation.id,
    )?;
    if obligations.is_empty() {
        return Err(fail(
            &p,
            ValidationCode::EmptyCollection {
                name: "obligations",
            },
        ));
    }

    let (v, p) = top.req("acceptanceConditions")?;
    let conditions = keyed(
        v,
        &p,
        |item, path| {
            let mut reader = Reader::new(item, path)?;
            let (v, p) = reader.req("id")?;
            let id = id_at(v, &p)?;
            let (v, p) = reader.req("obligation")?;
            let obligation = id_at(v, &p)?;
            let (v, p) = reader.req("consumptionBoundary")?;
            let consumption_boundary = id_at(v, &p)?;
            let (v, p) = reader.req("consumerClass")?;
            let consumer_class = id_at(v, &p)?;
            let (v, p) = reader.req("proposition")?;
            let proposition = text(v, &p)?;
            if proposition.is_empty() {
                return Err(fail(&p, ValidationCode::EmptyString));
            }
            let proposition = proposition.to_owned();
            let (v, p) = reader.req("coveredState")?;
            let covered_state = id_at(v, &p)?;
            let (v, p) = reader.req("normative")?;
            let normative = parse_normative(v, &p, profile)?;
            let realization_constraints = match reader.opt("realizationConstraints") {
                None => Vec::new(),
                Some((v, p)) => parse_constraints(v, &p, profile)?,
            };
            let (v, p) = reader.req("acceptor")?;
            let acceptor = id_at(v, &p)?;
            let (v, p) = reader.req("unestablishedRoute")?;
            let unestablished_route = id_at(v, &p)?;
            reader.done()?;
            Ok(Condition {
                id,
                obligation,
                consumption_boundary,
                consumer_class,
                proposition,
                covered_state,
                normative,
                realization_constraints,
                acceptor,
                unestablished_route,
            })
        },
        |condition| &condition.id,
    )?;
    if conditions.is_empty() {
        return Err(fail(
            &p,
            ValidationCode::EmptyCollection {
                name: "acceptanceConditions",
            },
        ));
    }

    let (v, p) = top.req("requiredAuthorities")?;
    let required_authorities = keyed(
        v,
        &p,
        |item, path| {
            let mut reader = Reader::new(item, path)?;
            let (v, p) = reader.req("id")?;
            let id = id_at(v, &p)?;
            let (v, p) = reader.req("principal")?;
            let principal = id_at(v, &p)?;
            let (v, p) = reader.req("permission")?;
            let permission = id_at(v, &p)?;
            let (v, p) = reader.req("scope")?;
            let mut scope = Reader::new(v, &p)?;
            let boundaries = match scope.opt("boundaries") {
                None => None,
                Some((bv, bp)) => Some(id_set(bv, &bp)?),
            };
            scope.done()?;
            reader.done()?;
            Ok(RequiredAuthority {
                id,
                principal,
                permission,
                boundaries,
            })
        },
        |authority| &authority.id,
    )?;

    top.done()?;

    // Referential integrity (decision G12).
    for (index, condition) in conditions.iter().enumerate() {
        if !obligations
            .iter()
            .any(|obligation| obligation.id == condition.obligation)
        {
            return Err(fail(
                &format!("/acceptanceConditions/{index}/obligation"),
                ValidationCode::UnknownObligation {
                    obligation: condition.obligation.as_str().to_owned(),
                },
            ));
        }
    }
    for (index, obligation) in obligations.iter().enumerate() {
        if !conditions
            .iter()
            .any(|condition| condition.obligation == obligation.id)
        {
            return Err(fail(
                &format!("/obligations/{index}"),
                ValidationCode::ObligationWithoutCondition {
                    id: obligation.id.as_str().to_owned(),
                },
            ));
        }
    }

    let mut requirements = OrganizationalRequirements {
        problem_scope,
        obligations,
        conditions,
        required_authorities,
        profile,
        canonical: Vec::new(),
        digest: Digest::frame(DigestKind::Requirements, b""),
    };
    let json = requirements.to_json().map_err(|violation| {
        fail(
            "",
            ValidationCode::InternalInvariant {
                detail: violation.detail,
            },
        )
    })?;
    requirements.canonical = json.canonical_bytes();
    requirements.digest = Digest::frame(DigestKind::Requirements, &requirements.canonical);
    Ok(requirements)
}
