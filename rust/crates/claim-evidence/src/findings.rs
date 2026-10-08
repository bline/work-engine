use crate::codec::{JsString, JsValue, exact_fields, field, validate_transport};
use crate::contract::{ClaimError, ClaimResult, ensure};

macro_rules! identity {
    ($name:ident) => {
        #[derive(Clone, Debug, PartialEq, Eq, Hash)]
        pub struct $name(String);
        impl $name {
            pub fn new(value: impl Into<String>) -> ClaimResult<Self> {
                let value = value.into();
                ensure(!value.is_empty(), concat!(stringify!($name), " is empty"))?;
                Ok(Self(value))
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}
identity!(ClaimId);
identity!(FindingRevisionId);
identity!(OperationId);

pub(crate) fn text(value: &JsValue, label: &str) -> ClaimResult<String> {
    let result = value.as_text()?.to_string_checked()?;
    ensure(!result.is_empty(), &format!("{label} is empty"))?;
    Ok(result)
}
pub(crate) fn text_field(value: &JsValue, key: &str) -> ClaimResult<String> {
    text(field(value, key)?, key)
}
pub(crate) fn legacy_text<'a>(value: &'a JsValue, label: &str) -> ClaimResult<&'a JsString> {
    let string = value.as_text()?;
    ensure(!string.0.is_empty(), &format!("{label} is empty"))?;
    Ok(string)
}
pub(crate) fn legacy_text_field<'a>(value: &'a JsValue, key: &str) -> ClaimResult<&'a JsString> {
    legacy_text(field(value, key)?, key)
}
pub(crate) fn array<'a>(value: &'a JsValue, label: &str) -> ClaimResult<&'a [JsValue]> {
    value
        .as_array()
        .map_err(|_| ClaimError::new(format!("{label} is not an array")))
}

pub fn stable_claim_id(subject: &JsValue) -> ClaimResult<ClaimId> {
    exact_fields(
        subject,
        &[
            "namespace",
            "subject_kind",
            "stable_subject_id",
            "evidence_baseline",
            "content_set",
        ],
        "subject",
    )?;
    let namespace = legacy_text_field(subject, "namespace")?;
    let kind = legacy_text_field(subject, "subject_kind")?;
    let stable = legacy_text_field(subject, "stable_subject_id")?;
    validate_reference(field(subject, "evidence_baseline")?)?;
    let content = array(field(subject, "content_set")?, "content_set")?;
    ensure(!content.is_empty(), "content_set is empty")?;
    for item in content {
        legacy_text(item, "content_set entry")?;
    }
    let identity = JsValue::object([
        ("namespace", JsValue::String(namespace.clone())),
        ("subject_kind", JsValue::String(kind.clone())),
        ("stable_subject_id", JsValue::String(stable.clone())),
    ]);
    ClaimId::new(format!("claim-v1@{}", crate::codec::digest(&identity)?))
}

pub(crate) fn revision_id(revision: &JsValue) -> ClaimResult<FindingRevisionId> {
    let claim_id = text_field(revision, "claim_id")?;
    FindingRevisionId::new(format!("{}@{}", claim_id, crate::codec::digest(revision)?))
}

pub(crate) fn validate_reference(reference: &JsValue) -> ClaimResult<()> {
    exact_fields(
        reference,
        &[
            "owner",
            "reference",
            "revision",
            "integrity_sha256",
            "freshness",
            "status",
        ],
        "reference",
    )?;
    for key in ["owner", "reference", "revision", "freshness"] {
        legacy_text_field(reference, key)?;
    }
    let hash = text_field(reference, "integrity_sha256")?;
    ensure(
        hash.len() == 64
            && hash
                .bytes()
                .all(|x| x.is_ascii_hexdigit() && !x.is_ascii_uppercase()),
        "invalid reference integrity",
    )?;
    ensure(
        [
            "verified",
            "unavailable",
            "moved_resolvable",
            "excluded",
            "integrity_mismatch",
        ]
        .contains(&text_field(reference, "status")?.as_str()),
        "invalid reference status",
    )
}

pub(crate) fn validate_revision_payload(payload: &JsValue, scope: &str) -> ClaimResult<()> {
    exact_fields(
        payload,
        &[
            "proposition",
            "support_qualification",
            "assumptions",
            "limitations",
            "confidence",
            "evidence_references",
            "sensitivity_references",
            "evidence_mode",
            "judgment_kind",
            "decision_scope",
            "profile_payload",
            "reopening_conditions",
            "tombstone",
        ],
        "revision payload",
    )?;
    validate_transport(payload)?;
    for key in [
        "proposition",
        "support_qualification",
        "evidence_mode",
        "judgment_kind",
    ] {
        legacy_text_field(payload, key)?;
    }
    ensure(
        text_field(payload, "decision_scope")? == scope,
        "authority decision scope mismatch",
    )?;
    for key in ["assumptions", "limitations", "reopening_conditions"] {
        for item in array(field(payload, key)?, key)? {
            legacy_text(item, key)?;
        }
    }
    for key in ["evidence_references", "sensitivity_references"] {
        for item in array(field(payload, key)?, key)? {
            validate_reference(item)?;
        }
    }
    exact_fields(
        field(payload, "profile_payload")?,
        &["finding_id", "severity", "episode", "outcome"],
        "finding payload",
    )?;
    for key in ["finding_id", "severity", "episode", "outcome"] {
        legacy_text_field(field(payload, "profile_payload")?, key)?;
    }
    ensure(
        matches!(field(payload, "tombstone")?, JsValue::Bool(_)),
        "tombstone is not bool",
    )
}

pub(crate) fn make_revision(
    claim_id: &ClaimId,
    predecessor: Option<&FindingRevisionId>,
    payload: &JsValue,
    actor: &str,
    grant_id: &str,
) -> ClaimResult<JsValue> {
    let mut map = payload.as_object()?.clone();
    map.insert("schema_version".into(), JsValue::Number(1.0));
    map.insert("claim_id".into(), JsValue::text(claim_id.as_str()));
    map.insert(
        "predecessor_revision".into(),
        predecessor.map_or(JsValue::Null, |p| JsValue::text(p.as_str())),
    );
    map.insert("producer".into(), JsValue::text(actor));
    map.insert("authority_ref".into(), JsValue::text(grant_id));
    let without_id = JsValue::Object(map);
    let id = revision_id(&without_id)?;
    let mut full = without_id.as_object()?.clone();
    full.insert("id".into(), JsValue::text(id.as_str()));
    Ok(JsValue::Object(full))
}
