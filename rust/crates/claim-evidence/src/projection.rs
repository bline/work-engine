use std::collections::HashSet;

use crate::application::AdmissionBinding;
use crate::codec::{JsString, JsValue, digest_unchecked as digest, field};
use crate::contract::{ClaimError, ClaimResult, RootIdentity, ensure};
use crate::findings::{ClaimId, FindingRevisionId, text_field};
use crate::reliance::{ExactFindingRevisionRef, ExactRelianceRef, RelianceId};
use crate::store::{Store, records};

pub const PROJECTION_BUILD_VERSION: &str = "claim-evidence-rust-v1";
pub const PROJECTION_CONTEXT_KIND: &str = "rust_reviewer_exact_claim_projection_v1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectionConsumer {
    pub identity: String,
    pub revision: String,
    pub decision_scope: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectionSelection {
    pub revision_id: FindingRevisionId,
    pub selection_reason: JsString,
}

#[derive(Clone, Debug)]
pub struct ProjectionRequest {
    pub request_id: String,
    pub admission: AdmissionBinding,
    pub consumer: ProjectionConsumer,
    pub selections: Vec<ProjectionSelection>,
    pub limitations: Vec<JsString>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProjectionProvenance {
    pub projection_schema_version: u8,
    pub build_version: &'static str,
    pub canonical_input_path: &'static str,
    pub canonical_input_sha256: String,
    pub source_watermark: Option<JsString>,
    pub freshness: JsString,
    pub completeness: JsString,
    pub actual_content_set: JsString,
    pub excluded_inputs: Vec<JsValue>,
    pub failed_inputs: Vec<JsValue>,
    pub unresolved_references: Vec<JsValue>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProjectedRevision {
    pub selection_reason: JsString,
    pub exact: ExactFindingRevisionRef,
    pub claim: JsValue,
    pub revision: JsValue,
    pub authority_ref: String,
    pub authority_reference: JsValue,
    /// Only active reliance by this exact consumer on a current head appears here.
    pub current_reliances: Vec<ExactRelianceRef>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClaimProjection {
    request_id: String,
    consumer: ProjectionConsumer,
    root: RootIdentity,
    store_revision: i64,
    provenance: ProjectionProvenance,
    relevant_exact_revisions: Vec<ProjectedRevision>,
    limitations: Vec<JsString>,
}
impl ClaimProjection {
    pub fn request_id(&self) -> &str {
        &self.request_id
    }
    pub fn consumer(&self) -> &ProjectionConsumer {
        &self.consumer
    }
    pub fn provenance(&self) -> &ProjectionProvenance {
        &self.provenance
    }
    pub fn relevant_exact_revisions(&self) -> &[ProjectedRevision] {
        &self.relevant_exact_revisions
    }
    pub fn limitations(&self) -> &[JsString] {
        &self.limitations
    }
    /// Authority-free versioned consumer DTO. Rust root path and admission
    /// data are deliberately absent from this representation.
    pub fn to_js_value(&self) -> JsValue {
        let projection = &self.provenance;
        JsValue::object([
            ("schema_version", JsValue::Number(1.0)),
            ("context_kind", JsValue::text(PROJECTION_CONTEXT_KIND)),
            ("request_id", JsValue::text(&self.request_id)),
            (
                "consumer",
                JsValue::object([
                    ("identity", JsValue::text(&self.consumer.identity)),
                    ("revision", JsValue::text(&self.consumer.revision)),
                    (
                        "decision_scope",
                        JsValue::text(&self.consumer.decision_scope),
                    ),
                ]),
            ),
            (
                "projection",
                JsValue::object([
                    (
                        "projection_schema_version",
                        JsValue::Number(f64::from(projection.projection_schema_version)),
                    ),
                    ("build_version", JsValue::text(projection.build_version)),
                    (
                        "projection_identity",
                        JsValue::object([
                            ("path", JsValue::text(projection.canonical_input_path)),
                            ("sha256", JsValue::text(&projection.canonical_input_sha256)),
                            (
                                "source_watermark",
                                projection
                                    .source_watermark
                                    .as_ref()
                                    .map_or(JsValue::Null, |v| JsValue::String(v.clone())),
                            ),
                        ]),
                    ),
                    ("freshness", JsValue::String(projection.freshness.clone())),
                    (
                        "completeness",
                        JsValue::String(projection.completeness.clone()),
                    ),
                    (
                        "actual_content_set",
                        JsValue::String(projection.actual_content_set.clone()),
                    ),
                    (
                        "excluded_inputs",
                        JsValue::Array(projection.excluded_inputs.clone()),
                    ),
                    (
                        "failed_inputs",
                        JsValue::Array(projection.failed_inputs.clone()),
                    ),
                    (
                        "unresolved_references",
                        JsValue::Array(projection.unresolved_references.clone()),
                    ),
                ]),
            ),
            (
                "relevant_exact_revisions",
                JsValue::Array(
                    self.relevant_exact_revisions
                        .iter()
                        .map(|selected| selected.revision.clone())
                        .collect(),
                ),
            ),
            (
                "selection_metadata",
                JsValue::Array(
                    self.relevant_exact_revisions
                        .iter()
                        .map(|selected| {
                            JsValue::object([
                                (
                                    "revision_id",
                                    JsValue::text(selected.exact.revision_id.as_str()),
                                ),
                                (
                                    "revision_sha256",
                                    JsValue::text(&selected.exact.revision_sha256),
                                ),
                                (
                                    "selection_reason",
                                    JsValue::String(selected.selection_reason.clone()),
                                ),
                                ("claim", selected.claim.clone()),
                                ("authority_ref", JsValue::text(&selected.authority_ref)),
                                ("authority_reference", selected.authority_reference.clone()),
                                (
                                    "current_reliances",
                                    JsValue::Array(
                                        selected
                                            .current_reliances
                                            .iter()
                                            .map(ExactRelianceRef::to_js_value)
                                            .collect(),
                                    ),
                                ),
                            ])
                        })
                        .collect(),
                ),
            ),
            (
                "limitations",
                JsValue::Array(
                    self.limitations
                        .iter()
                        .cloned()
                        .map(JsValue::String)
                        .collect(),
                ),
            ),
            (
                "authority",
                JsValue::object([
                    ("claimPublicationAuthorized", JsValue::Bool(false)),
                    ("claimSelectionAuthorized", JsValue::Bool(false)),
                    ("findingEvaluationAuthorized", JsValue::Bool(false)),
                    ("reviewAcceptanceAuthorized", JsValue::Bool(false)),
                    ("campaignAcceptanceAuthorized", JsValue::Bool(false)),
                    ("mutationAuthorized", JsValue::Bool(false)),
                ]),
            ),
        ])
    }
}

fn request_value(request: &ProjectionRequest) -> ClaimResult<JsValue> {
    ensure(
        !request.request_id.is_empty(),
        "projection request ID absent",
    )?;
    for (name, value) in [
        ("consumer identity", &request.consumer.identity),
        ("consumer revision", &request.consumer.revision),
        ("consumer scope", &request.consumer.decision_scope),
    ] {
        ensure(!value.is_empty(), &format!("{name} absent"))?;
    }
    ensure(
        (1..=100).contains(&request.selections.len()),
        "projection selections require 1 through 100 items",
    )?;
    let mut seen = HashSet::new();
    let selections = request
        .selections
        .iter()
        .map(|selection| {
            ensure(
                seen.insert(selection.revision_id.as_str()),
                "duplicate projection revision",
            )?;
            ensure(
                selection.selection_reason.is_nonempty_text(),
                "projection selection reason absent",
            )?;
            Ok(JsValue::object([
                ("revision_id", JsValue::text(selection.revision_id.as_str())),
                (
                    "selection_reason",
                    JsValue::String(selection.selection_reason.clone()),
                ),
            ]))
        })
        .collect::<ClaimResult<Vec<_>>>()?;
    ensure(
        !request.limitations.is_empty()
            && request.limitations.iter().all(JsString::is_nonempty_text),
        "projection limitations absent",
    )?;
    let value = JsValue::object([
        ("schema_version", JsValue::Number(1.0)),
        ("request_id", JsValue::text(&request.request_id)),
        ("operation", JsValue::text("project_relevant_revisions")),
        (
            "consumer",
            JsValue::object([
                ("identity", JsValue::text(&request.consumer.identity)),
                ("revision", JsValue::text(&request.consumer.revision)),
                (
                    "decision_scope",
                    JsValue::text(&request.consumer.decision_scope),
                ),
            ]),
        ),
        ("selections", JsValue::Array(selections)),
        (
            "limitations",
            JsValue::Array(
                request
                    .limitations
                    .iter()
                    .cloned()
                    .map(JsValue::String)
                    .collect(),
            ),
        ),
    ]);
    crate::codec::canonical_json(&value)?;
    Ok(value)
}

/// Domain admission digest for an exact projection request, including consumer,
/// selections and limitations. This is not a historical read-service request ID.
pub fn projection_request_sha256(request: &ProjectionRequest) -> ClaimResult<String> {
    crate::codec::digest(&request_value(request)?)
}

fn required_text(value: &JsValue, key: &str) -> ClaimResult<JsString> {
    Ok(field(value, key)?.as_text()?.clone())
}

pub(crate) fn build(store: &Store, request: &ProjectionRequest) -> ClaimResult<ClaimProjection> {
    let (store_revision, state) = store.read_state()?;
    let boundary = field(&state, "projection_boundary")?;
    ensure(
        field(boundary, "completeness")? != &JsValue::text("unavailable"),
        "projection unavailable",
    )?;
    let revisions = records(&state, "revisions")?;
    let claims = records(&state, "claims")?;
    let grants = records(&state, "authorities")?;
    let reliances = records(&state, "reliances")?;
    let mut selected = Vec::with_capacity(request.selections.len());
    let mut seen = HashSet::new();
    for selection in &request.selections {
        ensure(
            seen.insert(selection.revision_id.as_str()),
            "duplicate projection revision",
        )?;
        let revision = revisions
            .iter()
            .find(|r| text_field(r, "id").ok().as_deref() == Some(selection.revision_id.as_str()))
            .ok_or_else(|| ClaimError::new("relevant revision not found"))?;
        ensure(
            text_field(revision, "decision_scope")? == request.consumer.decision_scope,
            "projection decision scope mismatch",
        )?;
        let claim_id = ClaimId::new(text_field(revision, "claim_id")?)?;
        let claim = claims
            .iter()
            .find(|c| text_field(c, "id").ok().as_deref() == Some(claim_id.as_str()))
            .ok_or_else(|| ClaimError::new("projection claim absent"))?;
        let authority_ref = text_field(revision, "authority_ref")?;
        let grant = grants
            .iter()
            .find(|g| text_field(g, "grant_id").ok().as_deref() == Some(&authority_ref))
            .ok_or_else(|| ClaimError::new("projection authority provenance absent"))?;
        let is_head = !revisions.iter().any(|r| {
            text_field(r, "predecessor_revision").ok().as_deref()
                == Some(selection.revision_id.as_str())
        });
        let mut current_reliances = Vec::new();
        if is_head {
            for reliance in reliances.iter().filter(|r| {
                text_field(r, "claim_revision_id").ok().as_deref()
                    == Some(selection.revision_id.as_str())
                    && text_field(r, "consumer").ok().as_deref() == Some(&request.consumer.identity)
                    && text_field(r, "consumer_revision").ok().as_deref()
                        == Some(&request.consumer.revision)
                    && text_field(r, "decision_scope").ok().as_deref()
                        == Some(&request.consumer.decision_scope)
            }) {
                current_reliances.push(ExactRelianceRef {
                    id: RelianceId::new(text_field(reliance, "id")?)?,
                    sha256: digest(reliance),
                    finding: ExactFindingRevisionRef {
                        claim_id: claim_id.clone(),
                        revision_id: selection.revision_id.clone(),
                        revision_sha256: digest(revision),
                    },
                    consumer: request.consumer.identity.clone(),
                    consumer_revision: request.consumer.revision.clone(),
                    decision_scope: request.consumer.decision_scope.clone(),
                });
            }
        }
        selected.push(ProjectedRevision {
            selection_reason: selection.selection_reason.clone(),
            exact: ExactFindingRevisionRef {
                claim_id,
                revision_id: selection.revision_id.clone(),
                revision_sha256: digest(revision),
            },
            claim: JsValue::object([
                ("id", field(claim, "id")?.clone()),
                ("profile", field(claim, "profile")?.clone()),
                ("subject", field(claim, "subject")?.clone()),
                (
                    "statement_identity",
                    field(claim, "statement_identity")?.clone(),
                ),
            ]),
            revision: revision.clone(),
            authority_ref,
            authority_reference: field(grant, "authority_reference")?.clone(),
            current_reliances,
        });
    }
    let mut unresolved = Vec::new();
    for reference in grants
        .iter()
        .map(|g| field(g, "authority_reference"))
        .chain(
            claims
                .iter()
                .map(|c| field(field(c, "subject")?, "evidence_baseline")),
        )
    {
        let reference = reference?;
        if field(reference, "status")? != &JsValue::text("verified") {
            unresolved.push(reduced_reference(reference)?);
        }
    }
    for revision in revisions {
        for key in ["evidence_references", "sensitivity_references"] {
            for reference in field(revision, key)?.as_array()? {
                if field(reference, "status")? != &JsValue::text("verified") {
                    unresolved.push(reduced_reference(reference)?);
                }
            }
        }
    }
    unresolved.sort_by_key(crate::codec::canonical_json_unchecked);
    let watermark = field(boundary, "source_watermark")?;
    let source_watermark = if watermark == &JsValue::Null {
        None
    } else {
        Some(watermark.as_text()?.clone())
    };
    let result = ClaimProjection {
        request_id: request.request_id.clone(),
        consumer: request.consumer.clone(),
        root: store.identity.clone(),
        store_revision,
        provenance: ProjectionProvenance {
            projection_schema_version: 1,
            build_version: PROJECTION_BUILD_VERSION,
            canonical_input_path: "canonical/store.json",
            canonical_input_sha256: digest(&state),
            source_watermark,
            freshness: required_text(boundary, "freshness")?,
            completeness: required_text(boundary, "completeness")?,
            actual_content_set: required_text(boundary, "actual_content_set")?,
            excluded_inputs: field(boundary, "excluded_inputs")?.as_array()?.to_vec(),
            failed_inputs: field(boundary, "failed_inputs")?.as_array()?.to_vec(),
            unresolved_references: unresolved,
        },
        relevant_exact_revisions: selected,
        limitations: request.limitations.clone(),
    };
    // The public DTO must fit the same checked codec even at CE1's retained
    // maximum accepted stored depth; report failure before returning success.
    crate::codec::canonical_json(&result.to_js_value())?;
    Ok(result)
}

fn reduced_reference(reference: &JsValue) -> ClaimResult<JsValue> {
    Ok(JsValue::object([
        ("owner", field(reference, "owner")?.clone()),
        ("reference", field(reference, "reference")?.clone()),
        ("revision", field(reference, "revision")?.clone()),
        ("status", field(reference, "status")?.clone()),
    ]))
}

pub(crate) fn verify_fresh(store: &Store, projection: &ClaimProjection) -> ClaimResult<()> {
    let (revision, state) = store.read_state()?;
    ensure(
        projection.root == store.identity
            && revision == projection.store_revision
            && projection.provenance.canonical_input_sha256 == digest(&state),
        "projection is stale for the canonical store",
    )
}
