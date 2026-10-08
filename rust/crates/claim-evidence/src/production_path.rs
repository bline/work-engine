//! Immutable production-path evidence, admitted through the campaign owner and
//! read from the private claim-evidence root. The port supplies custody from a
//! trusted owner; request JSON cannot grant custody to itself.
use crate::application::{ClaimsApplication, FindingAdmissionPort};
use crate::authority::{PRODUCTION_PATH_PROFILE, grant_digest};
use crate::codec::{JsValue, canonical_json, digest, exact_fields, field, parse_json};
use crate::contract::{ClaimError, ClaimResult, RootIdentity, ensure};
use crate::findings::{array, text_field};
use rusqlite::{OptionalExtension, params};

const MAX_COMMAND: usize = 1024 * 1024;
const MAX_READBACK: usize = 4 * 1024 * 1024;
const MAX_TOTAL_ROWS: i64 = 16 * 1024 * 1024;
const PROFILE_REVISION: &str = "production-path-profile-v1";
const EVALUATOR: &str = "claim-evidence.production-path-v1";
const UNAVAILABLE: &str = "observation:unavailable";
const ZERO_SHA: &str = "0000000000000000000000000000000000000000000000000000000000000000";
type ObservationReplayRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
);
type ObservationSqlRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
);
type EstablishmentReplayRow = (
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
);
type EstablishmentReadRow = (
    String,
    String,
    String,
    Option<String>,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
);
type EstablishmentReconcileRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    String,
    String,
    String,
    String,
);
type ObservationReconcileRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProductionPathStage {
    RecordObservation,
    EstablishClaim,
    ReadAdmission,
}
impl ProductionPathStage {
    fn permission(self) -> &'static str {
        match self {
            Self::RecordObservation => "record_observation",
            Self::EstablishClaim => "establish_claim",
            Self::ReadAdmission => "read_admission",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProductionPathAccess {
    Original,
    RecoverExact,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProductionPathAdmissionBinding {
    pub campaign_root_id: String,
    pub campaign_revision: String,
    pub campaign_operation_id: String,
    pub obligation_id: String,
    pub candidate_digest: String,
    pub selection_digest: String,
    pub profile_digest: String,
    pub prepared_request_sha256: String,
    pub child_request_sha256: String,
    pub review_episode_id: String,
    pub attempt_id: String,
    pub native_result_sha256: String,
    pub episode_revision: Option<String>,
    pub session_id: Option<String>,
    pub stage: ProductionPathStage,
    pub access: ProductionPathAccess,
}
impl ProductionPathAdmissionBinding {
    pub fn validate(&self) -> ClaimResult<()> {
        for (name, value) in [
            ("campaign root", &self.campaign_root_id),
            ("campaign revision", &self.campaign_revision),
            ("campaign operation", &self.campaign_operation_id),
            ("obligation", &self.obligation_id),
            ("review episode", &self.review_episode_id),
            ("attempt", &self.attempt_id),
        ] {
            ensure(!value.trim().is_empty(), &format!("{name} absent"))?;
        }
        for (name, value) in [
            ("candidate", &self.candidate_digest),
            ("selection", &self.selection_digest),
            ("profile", &self.profile_digest),
            ("prepared request", &self.prepared_request_sha256),
            ("child request", &self.child_request_sha256),
            ("native result", &self.native_result_sha256),
        ] {
            sha(value, name)?;
        }
        if let Some(value) = &self.episode_revision {
            ensure(!value.is_empty(), "empty episode revision")?;
        }
        if let Some(value) = &self.session_id {
            ensure(!value.is_empty(), "empty session ID")?;
        }
        Ok(())
    }
    fn value(&self) -> JsValue {
        let optional = |v: &Option<String>| v.as_ref().map_or(JsValue::Null, |x| JsValue::text(x));
        JsValue::object([
            ("campaign_root_id", JsValue::text(&self.campaign_root_id)),
            ("campaign_revision", JsValue::text(&self.campaign_revision)),
            (
                "campaign_operation_id",
                JsValue::text(&self.campaign_operation_id),
            ),
            ("obligation_id", JsValue::text(&self.obligation_id)),
            ("candidate_digest", JsValue::text(&self.candidate_digest)),
            ("selection_digest", JsValue::text(&self.selection_digest)),
            ("profile_digest", JsValue::text(&self.profile_digest)),
            (
                "prepared_request_sha256",
                JsValue::text(&self.prepared_request_sha256),
            ),
            (
                "child_request_sha256",
                JsValue::text(&self.child_request_sha256),
            ),
            ("review_episode_id", JsValue::text(&self.review_episode_id)),
            ("attempt_id", JsValue::text(&self.attempt_id)),
            (
                "native_result_sha256",
                JsValue::text(&self.native_result_sha256),
            ),
            ("episode_revision", optional(&self.episode_revision)),
            ("session_id", optional(&self.session_id)),
            ("stage", JsValue::text(self.stage.permission())),
            (
                "access",
                JsValue::text(match self.access {
                    ProductionPathAccess::Original => "original",
                    ProductionPathAccess::RecoverExact => "recover_exact",
                }),
            ),
        ])
    }
    pub fn digest(&self) -> String {
        crate::codec::digest_unchecked(&self.value())
    }
    pub fn original_digest(&self) -> String {
        let mut original = self.clone();
        original.access = ProductionPathAccess::Original;
        original.digest()
    }
}
#[derive(Clone, Debug)]
pub struct ObservationCommand {
    pub grant_id: String,
    pub admission: ProductionPathAdmissionBinding,
    pub observation: JsValue,
}
#[derive(Clone, Debug)]
pub struct EstablishmentCommand {
    pub grant_id: String,
    pub admission: ProductionPathAdmissionBinding,
    pub operation_id: String,
    pub claim: JsValue,
    pub observation_id: Option<String>,
}
#[derive(Clone, Debug)]
pub struct AdmissionReadRequest {
    pub grant_id: String,
    pub admission: ProductionPathAdmissionBinding,
    pub claim: JsValue,
    pub establishment_id: String,
    pub establishment_sha256: String,
    pub establishment_request_sha256: String,
    pub establishment_admission_sha256: String,
    pub observation_id: Option<String>,
    pub observation_sha256: Option<String>,
}
#[derive(Clone, Debug)]
pub enum ProductionPathRequest {
    RecordObservation(ObservationCommand),
    EstablishClaim(EstablishmentCommand),
    ReadAdmission(AdmissionReadRequest),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustodyEvidence {
    pub owner: String,
    pub reference: String,
    pub revision: String,
    pub sha256: String,
    pub verifier: String,
    pub profile: String,
    pub attempt_id: String,
    pub native_result_sha256: String,
    pub session_id: Option<String>,
    pub observation_sha256: Option<String>,
    pub artifact_references: Vec<String>,
}
impl CustodyEvidence {
    fn value(&self) -> JsValue {
        JsValue::object([
            ("schema_version", JsValue::Number(1.0)),
            ("owner", JsValue::text(&self.owner)),
            ("reference", JsValue::text(&self.reference)),
            ("revision", JsValue::text(&self.revision)),
            ("sha256", JsValue::text(&self.sha256)),
            ("verifier", JsValue::text(&self.verifier)),
            ("profile", JsValue::text(&self.profile)),
            ("attempt_id", JsValue::text(&self.attempt_id)),
            (
                "native_result_sha256",
                JsValue::text(&self.native_result_sha256),
            ),
            (
                "session_id",
                self.session_id
                    .as_ref()
                    .map_or(JsValue::Null, |s| JsValue::text(s)),
            ),
            (
                "observation_sha256",
                self.observation_sha256
                    .as_ref()
                    .map_or(JsValue::Null, |s| JsValue::text(s)),
            ),
            (
                "artifact_references",
                JsValue::Array(
                    self.artifact_references
                        .iter()
                        .map(|s| JsValue::text(s))
                        .collect(),
                ),
            ),
        ])
    }
}
pub trait ProductionPathLease {
    /// Must read the actual immutable evidence owner while the campaign gate is held.
    fn read_custody(&self, observation: Option<&JsValue>) -> ClaimResult<CustodyEvidence>;
}
pub trait ProductionPathAdmissionPort {
    fn acquire_production_path<'a>(
        &'a self,
        binding: &ProductionPathAdmissionBinding,
        request: &ProductionPathRequest,
    ) -> ClaimResult<Box<dyn ProductionPathLease + 'a>>;
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProductionPathLocator {
    pub root: RootIdentity,
    pub stage: ProductionPathStage,
    pub key: String,
    pub payload_sha256: String,
    pub grant_sha256: String,
    pub admission_sha256: String,
    pub custody_sha256: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProductionPathReceipt {
    pub id: String,
    pub sha256: String,
    pub locator: ProductionPathLocator,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProductionPathPublication {
    Applied(ProductionPathReceipt),
    Replayed(ProductionPathReceipt),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProductionPathWriteError {
    NoEffect(ClaimError),
    OutcomeUnknown(Box<ProductionPathLocator>),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProductionPathReconciliation {
    Absent,
    Committed(Box<ProductionPathReceipt>),
    Conflicting,
    Unresolved(String),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProductionPathReference {
    pub owner: &'static str,
    pub reference: String,
    pub revision: String,
    pub sha256: String,
    pub freshness: &'static str,
}
impl ProductionPathReference {
    pub fn to_value(&self) -> JsValue {
        JsValue::object([
            ("owner", JsValue::text(self.owner)),
            ("reference", JsValue::text(&self.reference)),
            ("revision", JsValue::text(&self.revision)),
            ("sha256", JsValue::text(&self.sha256)),
            ("freshness", JsValue::text(self.freshness)),
        ])
    }
}
/// Constructed only by checked application readback.
///
/// ```compile_fail
/// use claim_evidence::CheckedClaimEvidence;
/// let _forged = CheckedClaimEvidence {};
/// ```
///
/// ```compile_fail
/// let _: claim_evidence::CheckedClaimEvidence = serde_json::from_str("{}").unwrap();
/// ```
///
/// ```compile_fail
/// let _: Box<dyn claim_evidence::ProductionPathLease> =
///     Box::new(claim_evidence::codec::JsValue::Null);
/// ```
#[derive(Clone, Debug)]
pub struct CheckedClaimEvidence {
    claim: JsValue,
    observation: Option<JsValue>,
    establishment: JsValue,
    custody: CustodyEvidence,
    binding: ProductionPathAdmissionBinding,
    root: RootIdentity,
}
impl CheckedClaimEvidence {
    pub fn claim(&self) -> &JsValue {
        &self.claim
    }
    pub fn observation(&self) -> Option<&JsValue> {
        self.observation.as_ref()
    }
    pub fn establishment(&self) -> &JsValue {
        &self.establishment
    }
    pub fn custody(&self) -> &CustodyEvidence {
        &self.custody
    }
    pub fn binding(&self) -> &ProductionPathAdmissionBinding {
        &self.binding
    }
    pub fn root(&self) -> &RootIdentity {
        &self.root
    }
    pub fn status(&self) -> ClaimResult<String> {
        text_field(&self.establishment, "status")
    }
    pub fn reasons(&self) -> ClaimResult<Vec<String>> {
        strings(&self.establishment, "reasons", false)
    }
    pub fn boundary(&self) -> ClaimResult<String> {
        text_field(&self.claim, "consumptionBoundary")
    }
    pub fn consumer(&self) -> ClaimResult<String> {
        text_field(&self.claim, "consumer")
    }
    pub fn claim_reference(&self) -> ClaimResult<ProductionPathReference> {
        production_reference(
            required_text(&self.claim, "claimId")?,
            required_text(&self.claim, "revision")?,
            &self.claim,
        )
    }
    pub fn observation_reference(&self) -> ClaimResult<Option<ProductionPathReference>> {
        self.observation
            .as_ref()
            .map(|o| {
                let id = required_text(o, "id")?;
                production_reference(id.clone(), id, o)
            })
            .transpose()
    }
    pub fn establishment_reference(&self) -> ClaimResult<ProductionPathReference> {
        let id = required_text(&self.establishment, "id")?;
        production_reference(id.clone(), id, &self.establishment)
    }
}
fn production_reference(
    reference: String,
    revision: String,
    value: &JsValue,
) -> ClaimResult<ProductionPathReference> {
    Ok(ProductionPathReference {
        owner: "claim-evidence",
        reference,
        revision,
        sha256: digest(value)?,
        freshness: "exact immutable revision",
    })
}

fn sha(value: &str, name: &str) -> ClaimResult<()> {
    ensure(
        value.len() == 64
            && value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        &format!("{name} must be lowercase SHA-256"),
    )
}
fn value_sha(value: &JsValue, name: &str) -> ClaimResult<String> {
    let value = value.nonempty_text(name)?.to_string_checked()?;
    sha(&value, name)?;
    Ok(value)
}
fn required_text(value: &JsValue, key: &str) -> ClaimResult<String> {
    field(value, key)?.nonempty_text(key)?.to_string_checked()
}
fn strings(value: &JsValue, key: &str, nonempty: bool) -> ClaimResult<Vec<String>> {
    let input = array(field(value, key)?, key)?;
    ensure(!nonempty || !input.is_empty(), &format!("{key} empty"))?;
    let mut result = Vec::new();
    for item in input {
        let s = item.nonempty_text(key)?.to_string_checked()?;
        ensure(!result.contains(&s), &format!("{key} duplicate"))?;
        result.push(s);
    }
    Ok(result)
}
fn same_artifact_references(left: &[String], right: &[String]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut left = left.to_vec();
    let mut right = right.to_vec();
    left.sort_unstable();
    right.sort_unstable();
    left == right
}
fn candidate(value: &JsValue) -> ClaimResult<()> {
    exact_fields(value, &["commit", "tree", "patchIdentity"], "candidate")?;
    for key in ["commit", "tree", "patchIdentity"] {
        required_text(value, key)?;
    }
    Ok(())
}
fn subject(value: &JsValue) -> ClaimResult<()> {
    exact_fields(value, &["candidate", "reviewEpisodeId"], "subject")?;
    candidate(field(value, "candidate")?)?;
    required_text(value, "reviewEpisodeId")?;
    Ok(())
}
fn without(value: &JsValue, key: &str) -> ClaimResult<JsValue> {
    let mut map = value.as_object()?.clone();
    map.remove(&key.into());
    Ok(JsValue::Object(map))
}
pub fn validate_production_path_claim(claim: &JsValue) -> ClaimResult<()> {
    exact_fields(
        claim,
        &[
            "schemaVersion",
            "claimId",
            "revision",
            "proposition",
            "subject",
            "coveredState",
            "consumptionBoundary",
            "consumer",
            "acceptance",
            "profile",
        ],
        "production claim",
    )?;
    ensure(
        field(claim, "schemaVersion")? == &JsValue::Number(1.0),
        "claim schema unsupported",
    )?;
    required_text(claim, "proposition")?;
    subject(field(claim, "subject")?)?;
    required_text(claim, "coveredState")?;
    let boundary = required_text(claim, "consumptionBoundary")?;
    ensure(
        boundary == "builder_projection" || boundary == "campaign_terminalization",
        "claim boundary invalid",
    )?;
    let consumer = required_text(claim, "consumer")?;
    ensure(
        consumer.starts_with(if boundary == "builder_projection" {
            "slice-builder:"
        } else {
            "slice-campaign:"
        }),
        "claim consumer invalid",
    )?;
    let acceptance = field(claim, "acceptance")?;
    exact_fields(
        acceptance,
        &["owner", "source", "unestablishedRoute"],
        "acceptance",
    )?;
    for key in ["owner", "source", "unestablishedRoute"] {
        required_text(acceptance, key)?;
    }
    ensure(
        !["reviewer", "builder", "adapter", "terminalizer"]
            .contains(&required_text(acceptance, "owner")?.as_str()),
        "claim self-authorized",
    )?;
    let profile = field(claim, "profile")?;
    exact_fields(
        profile,
        &[
            "id",
            "revision",
            "allowedMechanisms",
            "admissibleObservers",
            "integrityRequired",
            "requiredRealization",
            "requiredCapabilities",
            "continuity",
        ],
        "evidence profile",
    )?;
    ensure(
        required_text(profile, "id")? == PRODUCTION_PATH_PROFILE
            && required_text(profile, "revision")? == PROFILE_REVISION,
        "evidence profile unsupported",
    )?;
    strings(profile, "allowedMechanisms", true)?;
    strings(profile, "admissibleObservers", true)?;
    strings(profile, "requiredCapabilities", false)?;
    required_text(profile, "requiredRealization")?;
    ensure(
        matches!(field(profile, "integrityRequired")?, JsValue::Bool(_)),
        "integrity requirement invalid",
    )?;
    ensure(
        ["fresh_initial", "retained"].contains(&required_text(profile, "continuity")?.as_str()),
        "continuity invalid",
    )?;
    let claim_id = format!(
        "production-path-claim-v1@{}",
        digest(&JsValue::object([
            ("proposition", field(claim, "proposition")?.clone()),
            ("subject", field(claim, "subject")?.clone()),
            ("coveredState", field(claim, "coveredState")?.clone()),
            (
                "consumptionBoundary",
                field(claim, "consumptionBoundary")?.clone()
            ),
            ("consumer", field(claim, "consumer")?.clone())
        ]))?
    );
    ensure(
        required_text(claim, "claimId")? == claim_id,
        "claim ID invalid",
    )?;
    ensure(
        required_text(claim, "revision")?
            == format!(
                "production-path-claim-revision-v1@{}",
                digest(&without(claim, "revision")?)?
            ),
        "claim revision invalid",
    )
}
pub fn validate_production_path_observation(obs: &JsValue) -> ClaimResult<()> {
    exact_fields(
        obs,
        &[
            "schema_version",
            "id",
            "event_identity",
            "kind",
            "selection",
            "obligationId",
            "subject",
            "coveredState",
            "execution",
            "realization",
            "capabilityEnvelope",
            "continuity",
            "transport",
            "observer",
            "observedAt",
            "adapterVersion",
            "artifacts",
        ],
        "observation",
    )?;
    ensure(
        field(obs, "schema_version")? == &JsValue::Number(2.0)
            && required_text(obs, "kind")? == "production_path",
        "observation schema invalid",
    )?;
    required_text(obs, "event_identity")?;
    let selection = field(obs, "selection")?;
    exact_fields(selection, &["id", "revision"], "selection")?;
    required_text(selection, "id")?;
    value_sha(field(selection, "revision")?, "selection revision")?;
    required_text(obs, "obligationId")?;
    subject(field(obs, "subject")?)?;
    required_text(obs, "coveredState")?;
    let execution = field(obs, "execution")?;
    exact_fields(execution, &["attemptId", "resultDigest"], "execution")?;
    required_text(execution, "attemptId")?;
    value_sha(field(execution, "resultDigest")?, "result digest")?;
    let realization = field(obs, "realization")?;
    exact_fields(realization, &["requested", "observed"], "realization")?;
    required_text(realization, "requested")?;
    required_text(realization, "observed")?;
    let cap = field(obs, "capabilityEnvelope")?;
    exact_fields(
        cap,
        &["capabilities", "mutationAuthorized"],
        "capability envelope",
    )?;
    strings(cap, "capabilities", false)?;
    ensure(
        matches!(field(cap, "mutationAuthorized")?, JsValue::Bool(_)),
        "mutation flag invalid",
    )?;
    let continuity = field(obs, "continuity")?;
    exact_fields(continuity, &["mode", "sessionId"], "continuity")?;
    ensure(
        ["fresh_initial", "same_session_resume"]
            .contains(&required_text(continuity, "mode")?.as_str()),
        "continuity invalid",
    )?;
    required_text(continuity, "sessionId")?;
    let transport = field(obs, "transport")?;
    exact_fields(transport, &["mechanism", "digest"], "transport")?;
    required_text(transport, "mechanism")?;
    value_sha(field(transport, "digest")?, "transport digest")?;
    let observer = field(obs, "observer")?;
    exact_fields(observer, &["identity", "kind"], "observer")?;
    required_text(observer, "identity")?;
    required_text(observer, "kind")?;
    let timestamp = required_text(obs, "observedAt")?;
    ensure(canonical_millis_utc(&timestamp), "observation time invalid")?;
    required_text(obs, "adapterVersion")?;
    for artifact in array(field(obs, "artifacts")?, "artifacts")? {
        exact_fields(
            artifact,
            &["owner", "reference", "digest", "status"],
            "artifact",
        )?;
        required_text(artifact, "owner")?;
        required_text(artifact, "reference")?;
        match required_text(artifact, "status")?.as_str() {
            "verified" => {
                value_sha(field(artifact, "digest")?, "artifact digest")?;
            }
            "unavailable" => {
                ensure(
                    field(artifact, "digest")? == &JsValue::Null,
                    "unavailable artifact has digest",
                )?;
            }
            _ => return Err(ClaimError::new("artifact status invalid")),
        }
    }
    ensure(
        required_text(obs, "id")?
            == format!(
                "production-path-observation-v1@{}",
                digest(&without(obs, "id")?)?
            ),
        "observation ID invalid",
    )
}
pub fn validate_production_path_establishment(value: &JsValue) -> ClaimResult<()> {
    exact_fields(
        value,
        &[
            "schemaVersion",
            "id",
            "operationId",
            "claimId",
            "claimRevision",
            "observationId",
            "observationDigest",
            "evaluator",
            "profileRevision",
            "status",
            "reasons",
            "predecessor",
        ],
        "establishment",
    )?;
    ensure(
        field(value, "schemaVersion")? == &JsValue::Number(1.0),
        "establishment schema invalid",
    )?;
    for key in [
        "id",
        "operationId",
        "claimId",
        "claimRevision",
        "observationId",
    ] {
        required_text(value, key)?;
    }
    value_sha(field(value, "observationDigest")?, "observation digest")?;
    ensure(
        required_text(value, "evaluator")? == EVALUATOR
            && required_text(value, "profileRevision")? == PROFILE_REVISION,
        "establishment evaluator unsupported",
    )?;
    ensure(
        ["established", "false", "unestablished"]
            .contains(&required_text(value, "status")?.as_str()),
        "establishment status invalid",
    )?;
    let reasons = strings(value, "reasons", false)?;
    let mut sorted = reasons.clone();
    sorted.sort();
    ensure(reasons == sorted, "establishment reasons unsorted")?;
    ensure(
        field(value, "predecessor")? == &JsValue::Null,
        "establishment predecessor unsupported",
    )?;
    ensure(
        required_text(value, "id")?
            == format!(
                "production-path-establishment-v1@{}",
                digest(&without(value, "id")?)?
            ),
        "establishment ID invalid",
    )
}
fn canonical_millis_utc(value: &str) -> bool {
    // Date.toISOString uses four digits for years 0..9999, signed six digits
    // otherwise, and clips dates outside +/-8.64e15 milliseconds.
    let b = value.as_bytes();
    let year_end = match b.len() {
        24 if b[4] == b'-' && b[..4].iter().all(u8::is_ascii_digit) => 4,
        27 if matches!(b[0], b'+' | b'-')
            && b[1..7].iter().all(u8::is_ascii_digit)
            && b[7] == b'-' =>
        {
            7
        }
        _ => return false,
    };
    let year = if year_end == 4 {
        value[..4].parse::<i32>().unwrap_or(i32::MAX)
    } else {
        let magnitude = value[1..7].parse::<i32>().unwrap_or(i32::MAX);
        if b[0] == b'+' {
            if magnitude < 10_000 {
                return false;
            }
            magnitude
        } else {
            if magnitude == 0 {
                return false;
            }
            -magnitude
        }
    };
    for (offset, expected) in [
        (3, b'-'),
        (6, b'T'),
        (9, b':'),
        (12, b':'),
        (15, b'.'),
        (19, b'Z'),
    ] {
        if b[year_end + offset] != expected {
            return false;
        }
    }
    for (start, end) in [(1, 3), (4, 6), (7, 9), (10, 12), (13, 15), (16, 19)] {
        if !b[year_end + start..year_end + end]
            .iter()
            .all(u8::is_ascii_digit)
        {
            return false;
        }
    }
    let n = |start, end| {
        value[year_end + start..year_end + end]
            .parse::<u32>()
            .unwrap_or(u32::MAX)
    };
    let (m, d, h, mi, s, ms) = (n(1, 3), n(4, 6), n(7, 9), n(10, 12), n(13, 15), n(16, 19));
    if !(-271_821..=275_760).contains(&year)
        || !(1..=12).contains(&m)
        || h > 23
        || mi > 59
        || s > 59
        || ms > 999
    {
        return false;
    }
    if year == -271_821 && (m, d) < (4, 20) {
        return false;
    }
    if year == 275_760 && ((m, d) > (9, 13) || ((m, d) == (9, 13) && (h, mi, s, ms) > (0, 0, 0, 0)))
    {
        return false;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    d >= 1 && d <= days[(m - 1) as usize]
}
fn evaluate(claim: &JsValue, obs: Option<&JsValue>) -> ClaimResult<(String, Vec<String>)> {
    let mut reasons = Vec::new();
    if let Some(obs) = obs {
        let p = field(claim, "profile")?;
        if digest(field(claim, "subject")?)? != digest(field(obs, "subject")?)? {
            reasons.push("subject_mismatch".into());
        }
        if field(claim, "coveredState")? != field(obs, "coveredState")? {
            reasons.push("covered_state_mismatch".into());
        }
        if !strings(p, "allowedMechanisms", true)?
            .contains(&required_text(field(obs, "transport")?, "mechanism")?)
        {
            reasons.push("mechanism_mismatch".into());
        }
        if !strings(p, "admissibleObservers", true)?
            .contains(&required_text(field(obs, "observer")?, "identity")?)
        {
            reasons.push("observer_not_admissible".into());
        }
        if field(p, "requiredRealization")? != field(field(obs, "realization")?, "observed")? {
            reasons.push("realization_mismatch".into());
        }
        if field(field(obs, "capabilityEnvelope")?, "mutationAuthorized")? == &JsValue::Bool(true) {
            reasons.push("mutation_authorized".into());
        }
        let have = strings(field(obs, "capabilityEnvelope")?, "capabilities", false)?;
        if !strings(p, "requiredCapabilities", false)?
            .iter()
            .all(|c| have.contains(c))
        {
            reasons.push("capability_mismatch".into());
        }
        let continuity = required_text(field(obs, "continuity")?, "mode")?;
        let actual = if continuity == "fresh_initial" {
            "fresh_initial"
        } else {
            "retained"
        };
        if required_text(p, "continuity")? != actual {
            reasons.push("continuity_mismatch".into());
        }
        if field(p, "integrityRequired")? == &JsValue::Bool(true)
            && array(field(obs, "artifacts")?, "artifacts")?
                .iter()
                .any(|a| required_text(a, "status").ok().as_deref() != Some("verified"))
        {
            reasons.push("integrity_or_artifact_unavailable".into());
        }
    } else {
        reasons.push("observation_unavailable".into());
    }
    reasons.sort();
    let status = if reasons.iter().any(|r| r == "mutation_authorized") {
        "false"
    } else if reasons.is_empty() {
        "established"
    } else {
        "unestablished"
    };
    Ok((status.into(), reasons))
}
fn establishment(
    operation_id: &str,
    claim: &JsValue,
    obs: Option<&JsValue>,
) -> ClaimResult<JsValue> {
    ensure(!operation_id.trim().is_empty(), "operation absent")?;
    let (status, reasons) = evaluate(claim, obs)?;
    let mut value = JsValue::object([
        ("schemaVersion", JsValue::Number(1.0)),
        ("operationId", JsValue::text(operation_id)),
        ("claimId", field(claim, "claimId")?.clone()),
        ("claimRevision", field(claim, "revision")?.clone()),
        (
            "observationId",
            obs.map_or_else(
                || JsValue::text(UNAVAILABLE),
                |v| field(v, "id").expect("validated observation").clone(),
            ),
        ),
        (
            "observationDigest",
            JsValue::text(
                &obs.map(digest)
                    .transpose()?
                    .unwrap_or_else(|| ZERO_SHA.into()),
            ),
        ),
        ("evaluator", JsValue::text(EVALUATOR)),
        ("profileRevision", JsValue::text(PROFILE_REVISION)),
        ("status", JsValue::text(&status)),
        (
            "reasons",
            JsValue::Array(reasons.iter().map(|r| JsValue::text(r)).collect()),
        ),
        ("predecessor", JsValue::Null),
    ]);
    let id = format!("production-path-establishment-v1@{}", digest(&value)?);
    if let JsValue::Object(map) = &mut value {
        map.insert("id".into(), JsValue::text(&id));
    }
    validate_production_path_establishment(&value)?;
    Ok(value)
}

fn row_error(e: impl std::fmt::Display) -> ClaimError {
    ClaimError::new(format!("production-path storage: {e}"))
}
fn request_value(request: &ProductionPathRequest) -> ClaimResult<JsValue> {
    Ok(match request {
        ProductionPathRequest::RecordObservation(c) => JsValue::object([
            ("stage", JsValue::text("record_observation")),
            ("observation", c.observation.clone()),
        ]),
        ProductionPathRequest::EstablishClaim(c) => JsValue::object([
            ("stage", JsValue::text("establish_claim")),
            ("operation_id", JsValue::text(&c.operation_id)),
            ("claim", c.claim.clone()),
            (
                "observation_id",
                c.observation_id
                    .as_ref()
                    .map_or(JsValue::Null, |s| JsValue::text(s)),
            ),
        ]),
        ProductionPathRequest::ReadAdmission(c) => JsValue::object([
            ("stage", JsValue::text("read_admission")),
            ("establishment_id", JsValue::text(&c.establishment_id)),
            (
                "establishment_sha256",
                JsValue::text(&c.establishment_sha256),
            ),
            (
                "establishment_request_sha256",
                JsValue::text(&c.establishment_request_sha256),
            ),
            (
                "establishment_admission_sha256",
                JsValue::text(&c.establishment_admission_sha256),
            ),
            ("claim", c.claim.clone()),
            (
                "observation_id",
                c.observation_id
                    .as_ref()
                    .map_or(JsValue::Null, |s| JsValue::text(s)),
            ),
            (
                "observation_sha256",
                c.observation_sha256
                    .as_ref()
                    .map_or(JsValue::Null, |s| JsValue::text(s)),
            ),
        ]),
    })
}
pub fn production_path_request_sha256(request: &ProductionPathRequest) -> ClaimResult<String> {
    digest(&request_value(request)?)
}
fn request_parts(
    request: &ProductionPathRequest,
) -> (
    &str,
    &ProductionPathAdmissionBinding,
    ProductionPathStage,
    String,
) {
    match request {
        ProductionPathRequest::RecordObservation(c) => (
            &c.grant_id,
            &c.admission,
            ProductionPathStage::RecordObservation,
            required_text(&c.observation, "event_identity").unwrap_or_default(),
        ),
        ProductionPathRequest::EstablishClaim(c) => (
            &c.grant_id,
            &c.admission,
            ProductionPathStage::EstablishClaim,
            c.operation_id.clone(),
        ),
        ProductionPathRequest::ReadAdmission(c) => (
            &c.grant_id,
            &c.admission,
            ProductionPathStage::ReadAdmission,
            c.establishment_id.clone(),
        ),
    }
}
impl<P: FindingAdmissionPort + ProductionPathAdmissionPort> ClaimsApplication<P> {
    fn preflight_observation_row(&self, key: &str, by_id: bool) -> ClaimResult<()> {
        let sql = if by_id {
            "SELECT length(CAST(event_identity||observation_id||observation_sha256||observation_json||request_sha256||admission_sha256||grant_id||grant_sha256||custody_json||custody_sha256 AS BLOB)) FROM production_path_observations WHERE observation_id=?"
        } else {
            "SELECT length(CAST(event_identity||observation_id||observation_sha256||observation_json||request_sha256||admission_sha256||grant_id||grant_sha256||custody_json||custody_sha256 AS BLOB)) FROM production_path_observations WHERE event_identity=?"
        };
        let bytes: Option<i64> = self
            .store
            .conn
            .query_row(sql, [key], |r| r.get(0))
            .optional()
            .map_err(row_error)?;
        if let Some(bytes) = bytes {
            ensure(
                bytes >= 0 && bytes as usize <= MAX_COMMAND,
                "stored observation row exceeds capacity",
            )?;
        }
        Ok(())
    }
    fn preflight_establishment_row(&self, key: &str, by_id: bool) -> ClaimResult<()> {
        let sql = if by_id {
            "SELECT length(CAST(operation_id||establishment_id||claim_revision||claim_sha256||coalesce(observation_id,'')||establishment_sha256||establishment_json||request_sha256||admission_sha256||grant_id||grant_sha256||custody_sha256||custody_json AS BLOB)) FROM production_path_establishments WHERE establishment_id=?"
        } else {
            "SELECT length(CAST(operation_id||establishment_id||claim_revision||claim_sha256||coalesce(observation_id,'')||establishment_sha256||establishment_json||request_sha256||admission_sha256||grant_id||grant_sha256||custody_sha256||custody_json AS BLOB)) FROM production_path_establishments WHERE operation_id=?"
        };
        let bytes: Option<i64> = self
            .store
            .conn
            .query_row(sql, [key], |r| r.get(0))
            .optional()
            .map_err(row_error)?;
        if let Some(bytes) = bytes {
            ensure(
                bytes >= 0 && bytes as usize <= MAX_COMMAND,
                "stored establishment row exceeds capacity",
            )?;
        }
        Ok(())
    }
    fn validate_saved_custody(
        &self,
        json: &str,
        sha256: &str,
        observation: Option<&JsValue>,
    ) -> ClaimResult<()> {
        ensure(json.len() <= MAX_COMMAND, "saved custody capacity invalid")?;
        let value = parse_json(json)?;
        ensure(
            canonical_json(&value)? == json && digest(&value)? == sha256,
            "saved custody integrity failure",
        )?;
        exact_fields(
            &value,
            &[
                "schema_version",
                "owner",
                "reference",
                "revision",
                "sha256",
                "verifier",
                "profile",
                "attempt_id",
                "native_result_sha256",
                "session_id",
                "observation_sha256",
                "artifact_references",
            ],
            "saved custody",
        )?;
        ensure(
            field(&value, "schema_version")? == &JsValue::Number(1.0),
            "saved custody schema invalid",
        )?;
        let profile = self
            .store
            .bootstrap
            .trusted_custody
            .as_ref()
            .ok_or_else(|| ClaimError::new("trusted custody absent"))?;
        for key in ["owner", "verifier", "profile"] {
            ensure(
                field(&value, key)? == field(profile, key)?,
                "saved custody owner/profile mismatch",
            )?;
        }
        required_text(&value, "reference")?;
        required_text(&value, "revision")?;
        value_sha(field(&value, "sha256")?, "saved owner digest")?;
        required_text(&value, "attempt_id")?;
        value_sha(
            field(&value, "native_result_sha256")?,
            "saved result digest",
        )?;
        let references = array(field(&value, "artifact_references")?, "artifact_references")?
            .iter()
            .map(|reference| {
                reference
                    .nonempty_text("artifact_references")?
                    .to_string_checked()
            })
            .collect::<ClaimResult<Vec<_>>>()?;
        if let Some(obs) = observation {
            ensure(
                field(&value, "observation_sha256")? == &JsValue::text(&digest(obs)?),
                "saved observation custody mismatch",
            )?;
            ensure(
                field(&value, "session_id")? == field(field(obs, "continuity")?, "sessionId")?,
                "saved session mismatch",
            )?;
            let artifacts = array(field(obs, "artifacts")?, "artifacts")?;
            let observed = artifacts
                .iter()
                .map(|a| required_text(a, "reference"))
                .collect::<ClaimResult<Vec<_>>>()?;
            ensure(
                same_artifact_references(&references, &observed),
                "saved artifact custody mismatch",
            )?;
        } else {
            ensure(
                field(&value, "observation_sha256")? == &JsValue::Null
                    && field(&value, "session_id")? == &JsValue::Null
                    && references.is_empty(),
                "saved absence custody mismatch",
            )?;
        }
        Ok(())
    }
    fn production_stored_bytes(&self) -> ClaimResult<i64> {
        self.store.conn.query_row(
            "SELECT coalesce((SELECT sum(length(CAST(event_identity||observation_id||observation_sha256||observation_json||request_sha256||admission_sha256||grant_id||grant_sha256||custody_json||custody_sha256 AS BLOB))) FROM production_path_observations),0)+coalesce((SELECT sum(length(CAST(operation_id||establishment_id||claim_revision||claim_sha256||coalesce(observation_id,'')||establishment_sha256||establishment_json||request_sha256||admission_sha256||grant_id||grant_sha256||custody_sha256||custody_json AS BLOB))) FROM production_path_establishments),0)",
            [], |r| r.get(0),
        ).map_err(row_error)
    }
    fn production_grant(&self, id: &str, stage: ProductionPathStage) -> ClaimResult<JsValue> {
        self.store.check_root()?;
        ensure(
            self.store.bootstrap.trusted_custody.is_some(),
            "production custody bootstrap absent",
        )?;
        let grant = self
            .store
            .bootstrap
            .grants
            .iter()
            .find(|g| text_field(g, "grant_id").ok().as_deref() == Some(id))
            .ok_or_else(|| ClaimError::new("production grant not registered"))?;
        ensure(
            required_text(grant, "profile")? == PRODUCTION_PATH_PROFILE,
            "production grant profile mismatch",
        )?;
        ensure(
            strings(grant, "permissions", true)?.contains(&stage.permission().to_string()),
            "production permission absent",
        )?;
        Ok(grant.clone())
    }
    fn production_locator(
        &self,
        request: &ProductionPathRequest,
    ) -> ClaimResult<ProductionPathLocator> {
        let (grant_id, binding, stage, key) = request_parts(request);
        ensure(!key.trim().is_empty(), "production key absent")?;
        binding.validate()?;
        ensure(
            binding.stage == stage,
            "production admission stage mismatch",
        )?;
        let payload = request_value(request)?;
        ensure(
            canonical_json(&payload)?.len() <= MAX_COMMAND,
            "production command exceeds capacity",
        )?;
        let payload_sha256 = digest(&payload)?;
        ensure(
            payload_sha256 == binding.child_request_sha256,
            "production request admission mismatch",
        )?;
        let grant = self.production_grant(grant_id, stage)?;
        let envelope = JsValue::object([
            ("schema_version", JsValue::Number(1.0)),
            ("request", payload),
            ("admission", binding.value()),
            ("grant_id", JsValue::text(grant_id)),
            ("grant_sha256", JsValue::text(&grant_digest(&grant))),
        ]);
        ensure(
            canonical_json(&envelope)?.len() <= MAX_COMMAND,
            "production command exceeds capacity",
        )?;
        Ok(ProductionPathLocator {
            root: self.store.identity.clone(),
            stage,
            key,
            payload_sha256,
            grant_sha256: grant_digest(&grant),
            admission_sha256: binding.original_digest(),
            custody_sha256: None,
        })
    }
    pub fn observation_locator(
        &self,
        command: &ObservationCommand,
    ) -> ClaimResult<ProductionPathLocator> {
        validate_production_path_observation(&command.observation)?;
        self.production_locator(&ProductionPathRequest::RecordObservation(command.clone()))
    }
    pub fn establishment_locator(
        &self,
        command: &EstablishmentCommand,
    ) -> ClaimResult<ProductionPathLocator> {
        validate_production_path_claim(&command.claim)?;
        self.production_locator(&ProductionPathRequest::EstablishClaim(command.clone()))
    }
    fn checked_custody(
        &self,
        binding: &ProductionPathAdmissionBinding,
        obs: Option<&JsValue>,
        custody: CustodyEvidence,
    ) -> ClaimResult<(String, String)> {
        let config = self
            .store
            .bootstrap
            .trusted_custody
            .as_ref()
            .ok_or_else(|| ClaimError::new("trusted custody absent"))?;
        ensure(
            custody.owner == required_text(config, "owner")?
                && custody.verifier == required_text(config, "verifier")?
                && custody.profile == required_text(config, "profile")?,
            "custody owner/profile mismatch",
        )?;
        ensure(
            !custody.reference.is_empty() && !custody.revision.is_empty(),
            "custody reference absent",
        )?;
        sha(&custody.sha256, "custody digest")?;
        ensure(
            strings(config, "reference_schemes", true)?
                .iter()
                .any(|p| custody.reference.starts_with(p)),
            "custody reference scheme invalid",
        )?;
        ensure(
            custody.attempt_id == binding.attempt_id
                && custody.native_result_sha256 == binding.native_result_sha256,
            "custody attempt/result mismatch",
        )?;
        ensure(
            custody.session_id == binding.session_id,
            "custody session mismatch",
        )?;
        match obs {
            Some(obs) => {
                ensure(
                    custody.observation_sha256.as_deref() == Some(digest(obs)?.as_str()),
                    "custody observation mismatch",
                )?;
                ensure(
                    required_text(field(obs, "execution")?, "attemptId")? == binding.attempt_id
                        && required_text(field(obs, "execution")?, "resultDigest")?
                            == binding.native_result_sha256,
                    "observation attempt/result mismatch",
                )?;
                ensure(
                    Some(required_text(field(obs, "continuity")?, "sessionId")?)
                        == binding.session_id,
                    "observation session mismatch",
                )?;
                ensure(
                    required_text(obs, "obligationId")? == binding.obligation_id,
                    "observation obligation mismatch",
                )?;
                ensure(
                    required_text(field(obs, "subject")?, "reviewEpisodeId")?
                        == binding.review_episode_id,
                    "observation episode mismatch",
                )?;
                ensure(
                    value_sha(
                        field(field(obs, "selection")?, "revision")?,
                        "selection revision",
                    )? == binding.selection_digest,
                    "observation selection mismatch",
                )?;
                let artifacts = array(field(obs, "artifacts")?, "artifacts")?;
                let schemes = strings(config, "artifact_schemes", true)?;
                let observed = artifacts
                    .iter()
                    .map(|artifact| required_text(artifact, "reference"))
                    .collect::<ClaimResult<Vec<_>>>()?;
                for reference in &observed {
                    ensure(
                        schemes.iter().any(|p| reference.starts_with(p)),
                        "artifact custody mismatch",
                    )?;
                }
                ensure(
                    same_artifact_references(&custody.artifact_references, &observed),
                    "custody artifact set mismatch",
                )?;
            }
            None => {
                ensure(
                    custody.observation_sha256.is_none()
                        && custody.artifact_references.is_empty()
                        && custody.session_id.is_none(),
                    "absence custody mismatch",
                )?;
            }
        }
        let json = canonical_json(&custody.value())?;
        ensure(
            json.len() <= MAX_COMMAND,
            "custody envelope exceeds capacity",
        )?;
        Ok((json, digest(&custody.value())?))
    }
    pub fn record_observation(
        &mut self,
        command: ObservationCommand,
    ) -> Result<ProductionPathPublication, ProductionPathWriteError> {
        let locator = self
            .observation_locator(&command)
            .map_err(ProductionPathWriteError::NoEffect)?;
        let request = ProductionPathRequest::RecordObservation(command.clone());
        let lease = self
            .port
            .acquire_production_path(&command.admission, &request)
            .map_err(ProductionPathWriteError::NoEffect)?;
        let custody = lease
            .read_custody(Some(&command.observation))
            .map_err(ProductionPathWriteError::NoEffect)?;
        let (custody_json, custody_sha) = self
            .checked_custody(&command.admission, Some(&command.observation), custody)
            .map_err(ProductionPathWriteError::NoEffect)?;
        let locator = ProductionPathLocator {
            custody_sha256: Some(custody_sha.clone()),
            ..locator
        };
        let json =
            canonical_json(&command.observation).map_err(ProductionPathWriteError::NoEffect)?;
        ensure(
            json.len() + custody_json.len() <= MAX_COMMAND,
            "observation row exceeds capacity",
        )
        .map_err(ProductionPathWriteError::NoEffect)?;
        let id = required_text(&command.observation, "id")
            .map_err(ProductionPathWriteError::NoEffect)?;
        let sha = digest(&command.observation).map_err(ProductionPathWriteError::NoEffect)?;
        let row_size = locator.key.len()
            + id.len()
            + sha.len()
            + json.len()
            + locator.payload_sha256.len()
            + locator.admission_sha256.len()
            + command.grant_id.len()
            + locator.grant_sha256.len()
            + custody_json.len()
            + custody_sha.len();
        ensure(row_size <= MAX_COMMAND, "observation row exceeds capacity")
            .map_err(ProductionPathWriteError::NoEffect)?;
        self.store
            .check_root()
            .map_err(ProductionPathWriteError::NoEffect)?;
        self.store
            .conn
            .execute_batch("BEGIN IMMEDIATE")
            .map_err(|e| ProductionPathWriteError::NoEffect(row_error(e)))?;
        let outcome = (|| -> ClaimResult<ProductionPathPublication> {
            self.preflight_observation_row(&locator.key, false)?;
            let existing:Option<ObservationReplayRow>=self.store.conn.query_row(
                "SELECT observation_id,observation_sha256,observation_json,request_sha256,admission_sha256,grant_id,grant_sha256,custody_json,custody_sha256 FROM production_path_observations WHERE event_identity=?",
                [&locator.key],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?))).optional().map_err(row_error)?;
            let receipt = ProductionPathReceipt {
                id,
                sha256: sha,
                locator: locator.clone(),
            };
            if let Some(row) = existing {
                ensure(
                    row.0 == receipt.id
                        && row.1 == receipt.sha256
                        && row.2 == json
                        && row.3 == locator.payload_sha256
                        && row.4 == locator.admission_sha256
                        && row.5 == command.grant_id
                        && row.6 == locator.grant_sha256
                        && row.7 == custody_json
                        && row.8 == custody_sha,
                    "observation replay conflict",
                )?;
                return Ok(ProductionPathPublication::Replayed(receipt));
            }
            let total = self.production_stored_bytes()?;
            ensure(
                total + row_size as i64 <= MAX_TOTAL_ROWS,
                "production root capacity exceeded",
            )?;
            self.store
                .conn
                .execute(
                    "INSERT INTO production_path_observations VALUES(?,?,?,?,?,?,?,?,?,?)",
                    params![
                        locator.key,
                        receipt.id,
                        receipt.sha256,
                        json,
                        locator.payload_sha256,
                        locator.admission_sha256,
                        command.grant_id,
                        locator.grant_sha256,
                        custody_json,
                        custody_sha
                    ],
                )
                .map_err(row_error)?;
            Ok(ProductionPathPublication::Applied(receipt))
        })();
        self.finish_production_write(outcome, locator, "observation", None)
    }
    fn finish_production_write(
        &self,
        outcome: ClaimResult<ProductionPathPublication>,
        locator: ProductionPathLocator,
        cut: &str,
        claim: Option<&JsValue>,
    ) -> Result<ProductionPathPublication, ProductionPathWriteError> {
        match outcome {
            Ok(publication) => {
                crate::fault_cut(&format!("before_{cut}_commit"));
                if let Err(error) = crate::store::harden_sidecars(
                    &self
                        .store
                        .identity
                        .absolute_path
                        .join("claim-evidence.sqlite3"),
                ) {
                    return if self.store.conn.execute_batch("ROLLBACK").is_ok() {
                        Err(ProductionPathWriteError::NoEffect(error))
                    } else {
                        Err(ProductionPathWriteError::OutcomeUnknown(Box::new(locator)))
                    };
                }
                if self.store.conn.execute_batch("COMMIT").is_err() {
                    return Err(ProductionPathWriteError::OutcomeUnknown(Box::new(locator)));
                }
                crate::fault_cut(&format!("after_{cut}_commit"));
                let receipt = match &publication {
                    ProductionPathPublication::Applied(r)
                    | ProductionPathPublication::Replayed(r) => r,
                };
                let result = match locator.stage {
                    ProductionPathStage::RecordObservation => self.reconcile_observation(&locator),
                    _ => self
                        .reconcile_establishment(&locator, claim.expect("establishment has claim")),
                };
                match result {
                    ProductionPathReconciliation::Committed(found) if *found == *receipt => {
                        Ok(publication)
                    }
                    _ => Err(ProductionPathWriteError::OutcomeUnknown(Box::new(locator))),
                }
            }
            Err(error) => {
                if self.store.conn.execute_batch("ROLLBACK").is_ok() {
                    Err(ProductionPathWriteError::NoEffect(error))
                } else {
                    Err(ProductionPathWriteError::OutcomeUnknown(Box::new(locator)))
                }
            }
        }
    }
    fn observation_row(&self, id: &str) -> ClaimResult<Option<(JsValue, String, String)>> {
        self.store.check_root()?;
        self.preflight_observation_row(id, true)?;
        let row:Option<ObservationSqlRow>=self.store.conn.query_row(
            "SELECT event_identity,observation_id,observation_sha256,observation_json,request_sha256,admission_sha256,grant_id,grant_sha256,custody_json,custody_sha256 FROM production_path_observations WHERE observation_id=?",
            [id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?,r.get(9)?))).optional().map_err(row_error)?;
        let Some(row) = row else { return Ok(None) };
        ensure(
            row.3.len() + row.8.len() <= MAX_COMMAND,
            "observation row capacity invalid",
        )?;
        let obs = parse_json(&row.3)?;
        validate_production_path_observation(&obs)?;
        ensure(
            canonical_json(&obs)? == row.3
                && digest(&obs)? == row.2
                && required_text(&obs, "id")? == row.1
                && required_text(&obs, "event_identity")? == row.0,
            "observation row integrity failure",
        )?;
        let expected_request = digest(&JsValue::object([
            ("stage", JsValue::text("record_observation")),
            ("observation", obs.clone()),
        ]))?;
        ensure(
            row.4 == expected_request,
            "observation request integrity failure",
        )?;
        sha(&row.4, "observation request")?;
        sha(&row.5, "observation admission")?;
        let grant = self.production_grant(&row.6, ProductionPathStage::RecordObservation)?;
        ensure(grant_digest(&grant) == row.7, "observation grant changed")?;
        let custody = parse_json(&row.8)?;
        ensure(
            canonical_json(&custody)? == row.8 && digest(&custody)? == row.9,
            "observation custody integrity failure",
        )?;
        Ok(Some((obs, row.8, row.9)))
    }
    pub fn establish_claim(
        &mut self,
        command: EstablishmentCommand,
    ) -> Result<ProductionPathPublication, ProductionPathWriteError> {
        let locator = self
            .establishment_locator(&command)
            .map_err(ProductionPathWriteError::NoEffect)?;
        let request = ProductionPathRequest::EstablishClaim(command.clone());
        let lease = self
            .port
            .acquire_production_path(&command.admission, &request)
            .map_err(ProductionPathWriteError::NoEffect)?;
        let observation = if let Some(id) = &command.observation_id {
            Some(
                self.observation_row(id)
                    .map_err(ProductionPathWriteError::NoEffect)?
                    .ok_or_else(|| {
                        ProductionPathWriteError::NoEffect(ClaimError::new(
                            "referenced observation absent",
                        ))
                    })?
                    .0,
            )
        } else {
            None
        };
        let custody = lease
            .read_custody(observation.as_ref())
            .map_err(ProductionPathWriteError::NoEffect)?;
        let (custody_json, custody_sha) = self
            .checked_custody(&command.admission, observation.as_ref(), custody)
            .map_err(ProductionPathWriteError::NoEffect)?;
        if let Some(id) = &command.observation_id {
            let row = self
                .observation_row(id)
                .map_err(ProductionPathWriteError::NoEffect)?
                .ok_or_else(|| {
                    ProductionPathWriteError::NoEffect(ClaimError::new(
                        "referenced observation absent",
                    ))
                })?;
            ensure(
                row.2 == custody_sha,
                "establishment custody differs from observation",
            )
            .map_err(ProductionPathWriteError::NoEffect)?;
        }
        let locator = ProductionPathLocator {
            custody_sha256: Some(custody_sha),
            ..locator
        };
        let value = establishment(&command.operation_id, &command.claim, observation.as_ref())
            .map_err(ProductionPathWriteError::NoEffect)?;
        let json = canonical_json(&value).map_err(ProductionPathWriteError::NoEffect)?;
        ensure(
            json.len() <= MAX_COMMAND,
            "establishment row exceeds capacity",
        )
        .map_err(ProductionPathWriteError::NoEffect)?;
        let id = required_text(&value, "id").map_err(ProductionPathWriteError::NoEffect)?;
        let sha = digest(&value).map_err(ProductionPathWriteError::NoEffect)?;
        let claim_sha = digest(&command.claim).map_err(ProductionPathWriteError::NoEffect)?;
        let row_size = command.operation_id.len()
            + id.len()
            + required_text(&command.claim, "revision")
                .map_err(ProductionPathWriteError::NoEffect)?
                .len()
            + claim_sha.len()
            + command.observation_id.as_ref().map_or(0, String::len)
            + sha.len()
            + json.len()
            + locator.payload_sha256.len()
            + locator.admission_sha256.len()
            + command.grant_id.len()
            + locator.grant_sha256.len()
            + locator.custody_sha256.as_ref().map_or(0, String::len)
            + custody_json.len();
        ensure(
            row_size <= MAX_COMMAND,
            "establishment row exceeds capacity",
        )
        .map_err(ProductionPathWriteError::NoEffect)?;
        self.store
            .check_root()
            .map_err(ProductionPathWriteError::NoEffect)?;
        self.store
            .conn
            .execute_batch("BEGIN IMMEDIATE")
            .map_err(|e| ProductionPathWriteError::NoEffect(row_error(e)))?;
        let outcome = (|| -> ClaimResult<ProductionPathPublication> {
            if let Some(id) = &command.observation_id {
                let current = self
                    .observation_row(id)?
                    .ok_or_else(|| ClaimError::new("referenced observation absent"))?;
                ensure(
                    Some(&current.0) == observation.as_ref()
                        && current.2 == locator.custody_sha256.clone().unwrap_or_default(),
                    "observation changed during establishment",
                )?;
            }
            self.preflight_establishment_row(&command.operation_id, false)?;
            let existing:Option<EstablishmentReplayRow>=self.store.conn.query_row(
                "SELECT establishment_id,claim_revision,claim_sha256,establishment_sha256,establishment_json,observation_id,request_sha256,admission_sha256,grant_id,grant_sha256,operation_id,custody_sha256,custody_json FROM production_path_establishments WHERE operation_id=?",
                [&command.operation_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?,r.get(9)?,r.get(10)?,r.get(11)?,r.get(12)?))).optional().map_err(row_error)?;
            let receipt = ProductionPathReceipt {
                id,
                sha256: sha,
                locator: locator.clone(),
            };
            if let Some(row) = existing {
                ensure(
                    row.0 == receipt.id
                        && row.1 == required_text(&command.claim, "revision")?
                        && row.2 == claim_sha
                        && row.3 == receipt.sha256
                        && row.4 == json
                        && row.5 == command.observation_id
                        && row.6 == locator.payload_sha256
                        && row.7 == locator.admission_sha256
                        && row.8 == command.grant_id
                        && row.9 == locator.grant_sha256
                        && row.10 == command.operation_id
                        && Some(row.11) == locator.custody_sha256
                        && row.12 == custody_json,
                    "establishment replay conflict",
                )?;
                return Ok(ProductionPathPublication::Replayed(receipt));
            }
            let total = self.production_stored_bytes()?;
            ensure(
                total + row_size as i64 <= MAX_TOTAL_ROWS,
                "production root capacity exceeded",
            )?;
            self.store
                .conn
                .execute(
                    "INSERT INTO production_path_establishments VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?)",
                    params![
                        command.operation_id,
                        receipt.id,
                        required_text(&command.claim, "revision")?,
                        claim_sha,
                        command.observation_id,
                        receipt.sha256,
                        json,
                        locator.payload_sha256,
                        locator.admission_sha256,
                        command.grant_id,
                        locator.grant_sha256,
                        locator.custody_sha256,
                        custody_json
                    ],
                )
                .map_err(row_error)?;
            Ok(ProductionPathPublication::Applied(receipt))
        })();
        self.finish_production_write(outcome, locator, "establishment", Some(&command.claim))
    }
    pub fn read_claim_admission(
        &self,
        request: AdmissionReadRequest,
    ) -> ClaimResult<CheckedClaimEvidence> {
        validate_production_path_claim(&request.claim)?;
        for (name, value) in [
            ("establishment", &request.establishment_sha256),
            (
                "establishment request",
                &request.establishment_request_sha256,
            ),
            (
                "establishment admission",
                &request.establishment_admission_sha256,
            ),
        ] {
            sha(value, name)?;
        }
        ensure(
            request.observation_id.is_some() == request.observation_sha256.is_some(),
            "observation reference incomplete",
        )?;
        if let Some(value) = &request.observation_sha256 {
            sha(value, "observation")?;
        }
        let operation = ProductionPathRequest::ReadAdmission(request.clone());
        let _locator = self.production_locator(&operation)?;
        let lease = self
            .port
            .acquire_production_path(&request.admission, &operation)?;
        self.store.check_root()?;
        self.preflight_establishment_row(&request.establishment_id, true)?;
        let row:Option<EstablishmentReadRow>=self.store.conn.query_row(
            "SELECT establishment_id,claim_revision,claim_sha256,observation_id,establishment_sha256,establishment_json,request_sha256,admission_sha256,grant_id,grant_sha256,custody_sha256,custody_json FROM production_path_establishments WHERE establishment_id=?",
            [&request.establishment_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?,r.get(9)?,r.get(10)?,r.get(11)?))).optional().map_err(row_error)?;
        let row = row.ok_or_else(|| ClaimError::new("establishment absent"))?;
        ensure(
            row.5.len() <= MAX_COMMAND,
            "establishment row capacity invalid",
        )?;
        ensure(
            row.0 == request.establishment_id
                && row.1 == required_text(&request.claim, "revision")?
                && row.2 == digest(&request.claim)?
                && row.3 == request.observation_id
                && row.4 == request.establishment_sha256
                && row.6 == request.establishment_request_sha256
                && row.7 == request.establishment_admission_sha256,
            "establishment claim/observation/admission mismatch",
        )?;
        let grant = self.production_grant(&row.8, ProductionPathStage::EstablishClaim)?;
        ensure(row.9 == grant_digest(&grant), "establishment grant changed")?;
        sha(&row.6, "establishment request")?;
        sha(&row.7, "establishment admission")?;
        let value = parse_json(&row.5)?;
        ensure(
            canonical_json(&value)? == row.5
                && digest(&value)? == row.4
                && required_text(&value, "id")? == row.0,
            "establishment row integrity failure",
        )?;
        let expected_request = digest(&JsValue::object([
            ("stage", JsValue::text("establish_claim")),
            ("operation_id", field(&value, "operationId")?.clone()),
            ("claim", request.claim.clone()),
            (
                "observation_id",
                row.3.as_ref().map_or(JsValue::Null, |s| JsValue::text(s)),
            ),
        ]))?;
        ensure(
            row.6 == expected_request,
            "establishment request integrity failure",
        )?;
        let observation = if let Some(id) = &request.observation_id {
            let observed = self
                .observation_row(id)?
                .ok_or_else(|| ClaimError::new("referenced observation absent"))?;
            ensure(
                Some(digest(&observed.0)?) == request.observation_sha256,
                "observation reference mismatch",
            )?;
            Some(observed.0)
        } else {
            None
        };
        let expected = establishment(
            &required_text(&value, "operationId")?,
            &request.claim,
            observation.as_ref(),
        )?;
        ensure(value == expected, "establishment evaluation mismatch")?;
        let custody = lease.read_custody(observation.as_ref())?;
        let (custody_json, custody_sha) =
            self.checked_custody(&request.admission, observation.as_ref(), custody.clone())?;
        ensure(
            row.10 == custody_sha && row.11 == custody_json,
            "establishment custody mismatch",
        )?;
        if let Some(id) = &request.observation_id {
            let row = self
                .observation_row(id)?
                .ok_or_else(|| ClaimError::new("referenced observation absent"))?;
            ensure(
                row.1 == custody_json && row.2 == custody_sha,
                "readback custody mismatch",
            )?;
        }
        let total = canonical_json(&JsValue::object([
            ("schema_version", JsValue::Number(1.0)),
            (
                "root",
                JsValue::object([
                    (
                        "path",
                        JsValue::text(&self.store.identity.absolute_path.to_string_lossy()),
                    ),
                    ("root_id", JsValue::text(&self.store.identity.root_id)),
                    ("profile", JsValue::text(&self.store.identity.profile)),
                ]),
            ),
            ("admission", request.admission.value()),
            ("grant_id", JsValue::text(&request.grant_id)),
            ("claim", request.claim.clone()),
            ("establishment", value.clone()),
            ("observation", observation.clone().unwrap_or(JsValue::Null)),
            ("custody", custody.value()),
            (
                "establishment_ref",
                JsValue::object([
                    ("id", JsValue::text(&request.establishment_id)),
                    ("sha256", JsValue::text(&request.establishment_sha256)),
                    (
                        "request_sha256",
                        JsValue::text(&request.establishment_request_sha256),
                    ),
                    (
                        "admission_sha256",
                        JsValue::text(&request.establishment_admission_sha256),
                    ),
                ]),
            ),
            (
                "observation_sha256",
                request
                    .observation_sha256
                    .as_ref()
                    .map_or(JsValue::Null, |s| JsValue::text(s)),
            ),
        ]))?
        .len();
        ensure(total <= MAX_READBACK, "checked readback exceeds capacity")?;
        Ok(CheckedClaimEvidence {
            claim: request.claim,
            observation,
            establishment: value,
            custody,
            binding: request.admission,
            root: self.store.identity.clone(),
        })
    }
    pub fn reconcile_observation(
        &self,
        locator: &ProductionPathLocator,
    ) -> ProductionPathReconciliation {
        if locator.stage != ProductionPathStage::RecordObservation {
            return ProductionPathReconciliation::Conflicting;
        }
        self.reconcile_row(locator, false, None)
    }
    pub fn reconcile_establishment(
        &self,
        locator: &ProductionPathLocator,
        claim: &JsValue,
    ) -> ProductionPathReconciliation {
        if locator.stage != ProductionPathStage::EstablishClaim {
            return ProductionPathReconciliation::Conflicting;
        }
        self.reconcile_row(locator, true, Some(claim))
    }
    fn reconcile_row(
        &self,
        locator: &ProductionPathLocator,
        est: bool,
        claim: Option<&JsValue>,
    ) -> ProductionPathReconciliation {
        let result = (|| -> ClaimResult<Option<ProductionPathReceipt>> {
            self.store.check_root()?;
            ensure(
                locator.root == self.store.identity,
                "reconciliation root mismatch",
            )?;
            let (id, sha, json, request_sha, admission_sha, grant_id, grant_sha, custody_sha): (
                String,
                String,
                String,
                String,
                String,
                String,
                String,
                Option<String>,
            ) = if est {
                self.preflight_establishment_row(&locator.key, false)?;
                let row:Option<EstablishmentReconcileRow>=self.store.conn.query_row(
                    "SELECT establishment_id,establishment_sha256,establishment_json,request_sha256,admission_sha256,grant_id,grant_sha256,observation_id,custody_sha256,claim_revision,claim_sha256,custody_json FROM production_path_establishments WHERE operation_id=?",[&locator.key],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?,r.get(9)?,r.get(10)?,r.get(11)?))).optional().map_err(row_error)?;
                let Some(row) = row else { return Ok(None) };
                let claim = claim.ok_or_else(|| {
                    ClaimError::new("full claim required for establishment reconciliation")
                })?;
                validate_production_path_claim(claim)?;
                ensure(
                    row.9 == required_text(claim, "revision")? && row.10 == digest(claim)?,
                    "establishment claim binding mismatch",
                )?;
                let obs = if let Some(id) = &row.7 {
                    let observed = self
                        .observation_row(id)?
                        .ok_or_else(|| ClaimError::new("referenced observation absent"))?;
                    self.validate_saved_custody(&row.11, &row.8, Some(&observed.0))?;
                    ensure(
                        row.11 == observed.1 && row.8 == observed.2,
                        "establishment custody differs from observation",
                    )?;
                    Some(observed.0)
                } else {
                    self.validate_saved_custody(&row.11, &row.8, None)?;
                    None
                };
                let value = parse_json(&row.2)?;
                ensure(
                    value == establishment(&locator.key, claim, obs.as_ref())?,
                    "establishment evaluation mismatch",
                )?;
                let expected_request = digest(&JsValue::object([
                    ("stage", JsValue::text("establish_claim")),
                    ("operation_id", JsValue::text(&locator.key)),
                    ("claim", claim.clone()),
                    (
                        "observation_id",
                        row.7.as_ref().map_or(JsValue::Null, |id| JsValue::text(id)),
                    ),
                ]))?;
                ensure(
                    row.3 == expected_request,
                    "establishment request integrity failure",
                )?;
                (row.0, row.1, row.2, row.3, row.4, row.5, row.6, Some(row.8))
            } else {
                self.preflight_observation_row(&locator.key, false)?;
                let row:Option<ObservationReconcileRow>=self.store.conn.query_row(
                    "SELECT observation_id,observation_sha256,observation_json,request_sha256,admission_sha256,grant_id,grant_sha256,custody_sha256 FROM production_path_observations WHERE event_identity=?",[&locator.key],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?))).optional().map_err(row_error)?;
                let Some(row) = row else { return Ok(None) };
                validate_production_path_observation(&parse_json(&row.2)?)?;
                self.observation_row(&row.0)?
                    .ok_or_else(|| ClaimError::new("observation row absent"))?;
                (row.0, row.1, row.2, row.3, row.4, row.5, row.6, Some(row.7))
            };
            ensure(json.len() <= MAX_COMMAND, "reconciliation row too large")?;
            let parsed = parse_json(&json)?;
            ensure(
                canonical_json(&parsed)? == json
                    && digest(&parsed)? == sha
                    && required_text(&parsed, "id")? == id,
                "reconciliation row integrity failure",
            )?;
            let grant = self.production_grant(&grant_id, locator.stage)?;
            ensure(
                grant_sha == grant_digest(&grant),
                "reconciliation grant changed",
            )?;
            if request_sha != locator.payload_sha256
                || admission_sha != locator.admission_sha256
                || grant_sha != locator.grant_sha256
                || (locator.custody_sha256.is_some() && custody_sha != locator.custody_sha256)
            {
                return Err(ClaimError::new("reconciliation identity conflict"));
            }
            Ok(Some(ProductionPathReceipt {
                id,
                sha256: sha,
                locator: locator.clone(),
            }))
        })();
        match result {
            Ok(None) => ProductionPathReconciliation::Absent,
            Ok(Some(receipt)) => ProductionPathReconciliation::Committed(Box::new(receipt)),
            Err(e) if e.message.contains("conflict") || e.message.contains("mismatch") => {
                ProductionPathReconciliation::Conflicting
            }
            Err(e) => ProductionPathReconciliation::Unresolved(e.message),
        }
    }
}
