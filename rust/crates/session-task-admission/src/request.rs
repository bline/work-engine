//! Parsing for the future gated admission operation. These fields are lookup
//! and consistency terms only; they do not authenticate the caller or grant
//! authority, and the crate exposes no writable service endpoint.
use crate::Ref;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdmissionRequest {
    pub schema: String,
    pub request_id: String,
    pub task: Ref,
    pub decision: Ref,
    pub expected_task_revision: String,
}
#[derive(Debug, PartialEq, Eq)]
pub struct InvalidRequest;
impl AdmissionRequest {
    /// Validate against refs obtained independently from protected state.
    pub fn parse(
        raw: &str,
        current_task: &Ref,
        current_decision: &Ref,
    ) -> Result<Self, InvalidRequest> {
        let value: Self = serde_json::from_str(raw).map_err(|_| InvalidRequest)?;
        if value.schema != "dot.admission-request/v1-proposal"
            || !crate::nonempty(&value.request_id)
            || value.request_id.len() > 256
            || !crate::valid_ref(&value.task)
            || !crate::valid_ref(&value.decision)
            || value.task != *current_task
            || value.decision != *current_decision
            || value.expected_task_revision != value.task.revision
        {
            return Err(InvalidRequest);
        }
        Ok(value)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn rejects_stale_and_unknown_request_fields() {
        let r = Ref {
            id: "t".into(),
            revision: "r1".into(),
            sha256: "0".repeat(64),
        };
        let mut v = json!({"schema":"dot.admission-request/v1-proposal","request_id":"req","task":r,"decision":r,"expected_task_revision":"r1"});
        assert!(AdmissionRequest::parse(&v.to_string(), &r, &r).is_ok());
        v["expected_task_revision"] = "r2".into();
        assert!(AdmissionRequest::parse(&v.to_string(), &r, &r).is_err());
        v["expected_task_revision"] = "r1".into();
        v["actor"] = "self-granted".into();
        assert!(AdmissionRequest::parse(&v.to_string(), &r, &r).is_err());
    }
}
