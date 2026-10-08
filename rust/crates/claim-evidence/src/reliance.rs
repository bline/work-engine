use crate::application::AdmissionBinding;
use crate::codec::JsValue;
use crate::contract::{ClaimResult, OperationLocator};
use crate::findings::{ClaimId, FindingRevisionId, OperationId};

#[derive(Clone, Debug)]
pub struct RelianceCommand {
    pub operation_id: OperationId,
    pub grant_id: String,
    pub admission: AdmissionBinding,
    pub claim_id: ClaimId,
    pub revision_id: FindingRevisionId,
    pub consumer: String,
    pub consumer_revision: String,
    pub decision_scope: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RelianceId(String);
impl RelianceId {
    pub(crate) fn new(value: String) -> ClaimResult<Self> {
        crate::contract::ensure(
            value.starts_with("reliance@")
                && value.len() == 73
                && value[9..]
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "reliance identity invalid",
        )?;
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactFindingRevisionRef {
    pub claim_id: ClaimId,
    pub revision_id: FindingRevisionId,
    pub revision_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactRelianceRef {
    pub id: RelianceId,
    pub sha256: String,
    pub finding: ExactFindingRevisionRef,
    pub consumer: String,
    pub consumer_revision: String,
    pub decision_scope: String,
}
impl ExactRelianceRef {
    pub fn to_js_value(&self) -> JsValue {
        JsValue::object([
            ("schema_version", JsValue::Number(1.0)),
            ("kind", JsValue::text("exact_finding_reliance")),
            ("id", JsValue::text(self.id.as_str())),
            ("sha256", JsValue::text(&self.sha256)),
            ("claim_id", JsValue::text(self.finding.claim_id.as_str())),
            (
                "claim_revision_id",
                JsValue::text(self.finding.revision_id.as_str()),
            ),
            (
                "claim_revision_sha256",
                JsValue::text(&self.finding.revision_sha256),
            ),
            ("consumer", JsValue::text(&self.consumer)),
            ("consumer_revision", JsValue::text(&self.consumer_revision)),
            ("decision_scope", JsValue::text(&self.decision_scope)),
        ])
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelianceReceipt {
    pub exact: ExactRelianceRef,
    pub locator: OperationLocator,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReliancePublication {
    Applied(RelianceReceipt),
    Replayed(RelianceReceipt),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RelianceReconciliation {
    Absent,
    Committed(Box<RelianceReceipt>),
    Conflicting,
    Unresolved(String),
}
