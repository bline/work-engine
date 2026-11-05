//! Repository-fact identity and profile validation shared by admission and reads.
use crate::codec::{JsString, JsValue, exact_fields, field};
use crate::contract::{ClaimResult, ensure};
use crate::findings::{
    legacy_text_field, stable_claim_id, text_field, validate_common_revision_payload,
};

pub(crate) const REPOSITORY_FACT_PROFILE: &str = "repository-fact-v1";

/// Validate the producer-assigned subject and its first or successor payload.
/// Comparing with the predecessor makes the assertion scope immutable.
pub(crate) fn validate_fact(
    repository_id: &str,
    subject: &JsValue,
    statement_identity: &JsString,
    payload: &JsValue,
    scope: &str,
    predecessor_payload: Option<&JsValue>,
) -> ClaimResult<()> {
    ensure(!repository_id.is_empty(), "repository identity empty")?;
    stable_claim_id(subject)?;
    let key = legacy_text_field(subject, "stable_subject_id")?;
    ensure(
        text_field(subject, "namespace")? == format!("repository-fact:{repository_id}")
            && text_field(subject, "subject_kind")? == "repository_fact"
            && key == statement_identity,
        "repository subject identity mismatch",
    )?;
    validate_common_revision_payload(payload, scope)?;
    let profile = field(payload, "profile_payload")?;
    exact_fields(
        profile,
        &["repository_id", "assertion_key", "assertion_scope"],
        "repository fact payload",
    )?;
    ensure(
        text_field(profile, "repository_id")? == repository_id,
        "repository payload mismatch",
    )?;
    ensure(
        legacy_text_field(profile, "assertion_key")? == key,
        "assertion key mismatch",
    )?;
    legacy_text_field(profile, "assertion_scope")?;
    if let Some(previous) = predecessor_payload {
        ensure(
            text_field(previous, "decision_scope")? == scope,
            "repository predecessor decision scope mismatch",
        )?;
        let previous = field(previous, "profile_payload")?;
        for name in ["repository_id", "assertion_key", "assertion_scope"] {
            ensure(
                field(profile, name)? == field(previous, name)?,
                "immutable assertion identity changed",
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::parse_json;

    fn subject() -> JsValue {
        parse_json(r#"{"namespace":"repository-fact:repo-a","subject_kind":"repository_fact","stable_subject_id":"assertion-1","evidence_baseline":{"owner":"capture","reference":"ref","revision":"rev","integrity_sha256":"0000000000000000000000000000000000000000000000000000000000000000","freshness":"exact","status":"verified"},"content_set":["src"]}"#).unwrap()
    }
    fn payload(assertion_scope: &str) -> JsValue {
        parse_json(&format!(r#"{{"proposition":"p","support_qualification":"attributed","assumptions":[],"limitations":[],"confidence":{{"label":"high"}},"evidence_references":[],"sensitivity_references":[],"evidence_mode":"capture","judgment_kind":"fact","decision_scope":"scope","profile_payload":{{"repository_id":"repo-a","assertion_key":"assertion-1","assertion_scope":"{assertion_scope}"}},"reopening_conditions":[],"tombstone":false}}"#)).unwrap()
    }
    #[test]
    fn binds_identity_and_immutable_scope() {
        let key = JsString::new("assertion-1");
        let prior = payload("bounded scope");
        assert!(validate_fact("repo-a", &subject(), &key, &prior, "scope", None).is_ok());
        assert!(validate_fact("repo-b", &subject(), &key, &prior, "scope", None).is_err());
        assert!(
            validate_fact(
                "repo-a",
                &subject(),
                &JsString::new("other"),
                &prior,
                "scope",
                None
            )
            .is_err()
        );
        assert!(
            validate_fact(
                "repo-a",
                &subject(),
                &key,
                &payload("changed"),
                "scope",
                Some(&prior)
            )
            .is_err()
        );
        assert!(validate_fact("repo-a", &subject(), &key, &prior, "other", None).is_err());
    }
}
