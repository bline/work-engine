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

pub const DTO_SCHEMA_KINDS: [&str; 2] = ["exact-reliance-ref-v1", "exact-revision-projection-v1"];

pub fn dto_schema(kind: &str) -> ClaimResult<JsValue> {
    let body = match kind {
        "exact-reliance-ref-v1" => exact_reliance(),
        "exact-revision-projection-v1" => projection(),
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
