//! Versioned public consumer DTO schemas generated from the closed Rust wire
//! shapes. Schema generation has no runtime Serde conversion of legacy strings.
use crate::codec::JsValue;
use crate::contract::{ClaimError, ClaimResult};

fn object(properties: Vec<(&str, JsValue)>) -> JsValue {
    let required = properties
        .iter()
        .map(|(key, _)| JsValue::text(key))
        .collect();
    JsValue::object([
        ("type", JsValue::text("object")),
        ("required", JsValue::Array(required)),
        ("properties", JsValue::object(properties)),
        ("additionalProperties", JsValue::Bool(false)),
    ])
}
fn text() -> JsValue {
    JsValue::object([
        ("type", JsValue::text("string")),
        ("minLength", JsValue::Number(1.0)),
    ])
}
fn hash() -> JsValue {
    JsValue::object([
        ("type", JsValue::text("string")),
        ("pattern", JsValue::text("^[0-9a-f]{64}$")),
    ])
}
fn constant(value: JsValue) -> JsValue {
    JsValue::object([("const", value)])
}
fn array(item: JsValue, min: usize, max: Option<usize>) -> JsValue {
    let mut properties = vec![
        ("type", JsValue::text("array")),
        ("items", item),
        ("minItems", JsValue::Number(min as f64)),
    ];
    if let Some(max) = max {
        properties.push(("maxItems", JsValue::Number(max as f64)));
    }
    JsValue::object(properties)
}
fn arbitrary_object() -> JsValue {
    JsValue::object([("type", JsValue::text("object"))])
}
fn reference() -> JsValue {
    object(vec![
        ("owner", text()),
        ("reference", text()),
        ("revision", text()),
        ("integrity_sha256", hash()),
        ("freshness", text()),
        ("status", text()),
    ])
}
fn exact_reliance() -> JsValue {
    object(vec![
        ("schema_version", constant(JsValue::Number(1.0))),
        ("kind", constant(JsValue::text("exact_finding_reliance"))),
        ("id", text()),
        ("sha256", hash()),
        ("claim_id", text()),
        ("claim_revision_id", text()),
        ("claim_revision_sha256", hash()),
        ("consumer", text()),
        ("consumer_revision", text()),
        ("decision_scope", text()),
    ])
}
fn projection() -> JsValue {
    let consumer = object(vec![
        ("identity", text()),
        ("revision", text()),
        ("decision_scope", text()),
    ]);
    let projection_identity = object(vec![
        ("path", constant(JsValue::text("canonical/store.json"))),
        ("sha256", hash()),
        (
            "source_watermark",
            JsValue::object([(
                "type",
                JsValue::Array(vec![JsValue::text("string"), JsValue::text("null")]),
            )]),
        ),
    ]);
    let provenance = object(vec![
        ("projection_schema_version", constant(JsValue::Number(1.0))),
        (
            "build_version",
            constant(JsValue::text("claim-evidence-rust-v1")),
        ),
        ("projection_identity", projection_identity),
        ("freshness", text()),
        ("completeness", text()),
        ("actual_content_set", text()),
        ("excluded_inputs", array(JsValue::Bool(true), 0, None)),
        ("failed_inputs", array(JsValue::Bool(true), 0, None)),
        ("unresolved_references", array(arbitrary_object(), 0, None)),
    ]);
    let claim = object(vec![
        ("id", text()),
        ("profile", text()),
        ("subject", arbitrary_object()),
        ("statement_identity", text()),
    ]);
    let selected = object(vec![
        ("revision_id", text()),
        ("revision_sha256", hash()),
        ("selection_reason", text()),
        ("claim", claim),
        ("authority_ref", text()),
        ("authority_reference", reference()),
        ("current_reliances", array(exact_reliance(), 0, None)),
    ]);
    let authority = object(
        [
            "claimPublicationAuthorized",
            "claimSelectionAuthorized",
            "findingEvaluationAuthorized",
            "reviewAcceptanceAuthorized",
            "campaignAcceptanceAuthorized",
            "mutationAuthorized",
        ]
        .into_iter()
        .map(|key| (key, constant(JsValue::Bool(false))))
        .collect(),
    );
    object(vec![
        ("schema_version", constant(JsValue::Number(1.0))),
        (
            "context_kind",
            constant(JsValue::text(crate::projection::PROJECTION_CONTEXT_KIND)),
        ),
        ("request_id", text()),
        ("consumer", consumer),
        ("projection", provenance),
        (
            "relevant_exact_revisions",
            array(arbitrary_object(), 1, Some(100)),
        ),
        ("selection_metadata", array(selected, 1, Some(100))),
        ("limitations", array(text(), 1, None)),
        ("authority", authority),
    ])
}

fn enum_text(values: &[&str]) -> JsValue {
    JsValue::object([(
        "enum",
        JsValue::Array(values.iter().map(|s| JsValue::text(s)).collect()),
    )])
}
fn optional_text() -> JsValue {
    JsValue::object([(
        "type",
        JsValue::Array(vec![JsValue::text("string"), JsValue::text("null")]),
    )])
}
fn unique_texts(min: usize) -> JsValue {
    let mut map = array(text(), min, None)
        .as_object()
        .expect("schema object")
        .clone();
    map.insert("uniqueItems".into(), JsValue::Bool(true));
    JsValue::Object(map)
}
fn production_claim() -> JsValue {
    let candidate = object(vec![
        ("commit", text()),
        ("tree", text()),
        ("patchIdentity", text()),
    ]);
    let subject = object(vec![("candidate", candidate), ("reviewEpisodeId", text())]);
    let acceptance = object(vec![
        ("owner", text()),
        ("source", text()),
        ("unestablishedRoute", text()),
    ]);
    let profile = object(vec![
        ("id", constant(JsValue::text("production-path-v1"))),
        (
            "revision",
            constant(JsValue::text("production-path-profile-v1")),
        ),
        ("allowedMechanisms", unique_texts(1)),
        ("admissibleObservers", unique_texts(1)),
        (
            "integrityRequired",
            JsValue::object([("type", JsValue::text("boolean"))]),
        ),
        ("requiredRealization", text()),
        ("requiredCapabilities", unique_texts(0)),
        ("continuity", enum_text(&["fresh_initial", "retained"])),
    ]);
    object(vec![
        ("schemaVersion", constant(JsValue::Number(1.0))),
        ("claimId", text()),
        ("revision", text()),
        ("proposition", text()),
        ("subject", subject),
        ("coveredState", text()),
        (
            "consumptionBoundary",
            enum_text(&["builder_projection", "campaign_terminalization"]),
        ),
        ("consumer", text()),
        ("acceptance", acceptance),
        ("profile", profile),
    ])
}
fn production_observation() -> JsValue {
    let candidate = object(vec![
        ("commit", text()),
        ("tree", text()),
        ("patchIdentity", text()),
    ]);
    let subject = object(vec![("candidate", candidate), ("reviewEpisodeId", text())]);
    let artifact = object(vec![
        ("owner", text()),
        ("reference", text()),
        ("digest", optional_text()),
        ("status", enum_text(&["verified", "unavailable"])),
    ]);
    object(vec![
        ("schema_version", constant(JsValue::Number(2.0))),
        ("id", text()),
        ("event_identity", text()),
        ("kind", constant(JsValue::text("production_path"))),
        (
            "selection",
            object(vec![("id", text()), ("revision", hash())]),
        ),
        ("obligationId", text()),
        ("subject", subject),
        ("coveredState", text()),
        (
            "execution",
            object(vec![("attemptId", text()), ("resultDigest", hash())]),
        ),
        (
            "realization",
            object(vec![("requested", text()), ("observed", text())]),
        ),
        (
            "capabilityEnvelope",
            object(vec![
                ("capabilities", unique_texts(0)),
                (
                    "mutationAuthorized",
                    JsValue::object([("type", JsValue::text("boolean"))]),
                ),
            ]),
        ),
        (
            "continuity",
            object(vec![
                ("mode", enum_text(&["fresh_initial", "same_session_resume"])),
                ("sessionId", text()),
            ]),
        ),
        (
            "transport",
            object(vec![("mechanism", text()), ("digest", hash())]),
        ),
        (
            "observer",
            object(vec![("identity", text()), ("kind", text())]),
        ),
        ("observedAt", text()),
        ("adapterVersion", text()),
        ("artifacts", array(artifact, 0, None)),
    ])
}
fn production_establishment() -> JsValue {
    object(vec![
        ("schemaVersion", constant(JsValue::Number(1.0))),
        ("id", text()),
        ("operationId", text()),
        ("claimId", text()),
        ("claimRevision", text()),
        ("observationId", text()),
        ("observationDigest", hash()),
        (
            "evaluator",
            constant(JsValue::text("claim-evidence.production-path-v1")),
        ),
        (
            "profileRevision",
            constant(JsValue::text("production-path-profile-v1")),
        ),
        (
            "status",
            enum_text(&["established", "false", "unestablished"]),
        ),
        ("reasons", unique_texts(0)),
        ("predecessor", constant(JsValue::Null)),
    ])
}
fn bootstrap_config_v2() -> JsValue {
    let reference = object(vec![
        ("owner", text()),
        ("reference", text()),
        ("revision", text()),
        ("integrity_sha256", hash()),
        ("freshness", text()),
        ("status", constant(JsValue::text("verified"))),
    ]);
    let grant = |profile: &str, permissions: &[&str]| {
        object(vec![
            ("schema_version", constant(JsValue::Number(1.0))),
            ("grant_id", text()),
            ("actor", text()),
            ("profile", constant(JsValue::text(profile))),
            (
                "permissions",
                JsValue::object([
                    ("type", JsValue::text("array")),
                    ("minItems", JsValue::Number(1.0)),
                    ("uniqueItems", JsValue::Bool(true)),
                    ("items", enum_text(permissions)),
                ]),
            ),
            ("decision_scope", text()),
            ("authority_reference", reference.clone()),
        ])
    };
    let grants = JsValue::object([(
        "oneOf",
        JsValue::Array(vec![
            grant(
                "revision-bound-review-finding-v1",
                &["create_claim", "publish_revision", "record_reliance"],
            ),
            grant(
                "production-path-v1",
                &["record_observation", "establish_claim", "read_admission"],
            ),
        ]),
    )]);
    let custody = object(vec![
        ("owner", text()),
        ("verifier", text()),
        ("profile", text()),
        ("artifact_schemes", unique_texts(1)),
        ("reference_schemes", unique_texts(1)),
    ]);
    object(vec![
        ("schema_version", constant(JsValue::Number(2.0))),
        (
            "profile",
            constant(JsValue::text("native-review-claims-v1")),
        ),
        ("root_id", text()),
        ("grants", array(grants, 1, Some(64))),
        ("trusted_custody", custody),
    ])
}
fn bootstrap_source_v2() -> JsValue {
    object(vec![
        ("schema_version", constant(JsValue::Number(2.0))),
        ("owner", text()),
        ("reference", text()),
        ("revision", text()),
        ("freshness", text()),
        (
            "profile",
            constant(JsValue::text("native-review-claims-v1")),
        ),
        ("actors", unique_texts(1)),
        ("decision_scopes", unique_texts(1)),
        ("grant_ids", unique_texts(1)),
        ("custody_config_sha256", hash()),
    ])
}
pub const DTO_SCHEMA_KINDS: [&str; 8] = [
    "exact-reliance-ref-v1",
    "exact-revision-projection-v1",
    "bootstrap-config-v2",
    "bootstrap-source-v2",
    "production-path-claim-v1",
    "production-path-observation-v2",
    "production-path-establishment-v1",
    "production-path-reference-v1",
];

pub fn dto_schema(kind: &str) -> ClaimResult<JsValue> {
    let body = match kind {
        "exact-reliance-ref-v1" => exact_reliance(),
        "exact-revision-projection-v1" => projection(),
        "bootstrap-config-v2" => bootstrap_config_v2(),
        "bootstrap-source-v2" => bootstrap_source_v2(),
        "production-path-claim-v1" => production_claim(),
        "production-path-observation-v2" => production_observation(),
        "production-path-establishment-v1" => production_establishment(),
        "production-path-reference-v1" => object(vec![
            ("owner", constant(JsValue::text("claim-evidence"))),
            ("reference", text()),
            ("revision", text()),
            ("sha256", hash()),
            (
                "freshness",
                constant(JsValue::text("exact immutable revision")),
            ),
        ]),
        _ => return Err(ClaimError::new("unknown claim DTO schema")),
    };
    let mut fields = body.as_object()?.clone();
    fields.insert(
        "$schema".into(),
        JsValue::text("https://json-schema.org/draft/2020-12/schema"),
    );
    fields.insert(
        "$id".into(),
        JsValue::text(&format!("urn:work-engine:claim-evidence:{kind}")),
    );
    Ok(JsValue::Object(fields))
}
