use crate::codec::{JsString, JsValue};
use crate::findings::{ClaimId, FindingRevisionId, OperationId};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("{message}")]
pub struct ClaimError {
    pub message: String,
}
impl ClaimError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}
pub type ClaimResult<T> = Result<T, ClaimError>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RootIdentity {
    pub absolute_path: PathBuf,
    pub root_id: String,
    pub profile: String,
}

#[derive(Clone, Debug)]
pub enum FindingMutation {
    Create {
        subject: JsValue,
        statement_identity: JsString,
        initial_revision: JsValue,
    },
    Revise {
        claim_id: ClaimId,
        predecessor: FindingRevisionId,
        revision: JsValue,
    },
}

#[derive(Clone, Debug)]
pub struct FindingCommand {
    pub operation_id: OperationId,
    pub grant_id: String,
    pub admission: crate::application::AdmissionBinding,
    pub mutation: FindingMutation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationLocator {
    pub root: RootIdentity,
    pub operation_id: OperationId,
    pub action: &'static str,
    pub payload_sha256: String,
    pub grant_sha256: String,
    pub admission_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FindingReceipt {
    pub claim_id: ClaimId,
    pub revision_id: FindingRevisionId,
    pub revision_sha256: String,
    pub locator: OperationLocator,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Publication {
    Applied(FindingReceipt),
    Replayed(FindingReceipt),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WriteError {
    NoEffect(ClaimError),
    OutcomeUnknown(Box<OperationLocator>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reconciliation {
    Absent,
    Committed(Box<FindingReceipt>),
    Conflicting,
    Unresolved(String),
}

pub(crate) fn ensure(condition: bool, message: &str) -> ClaimResult<()> {
    if condition {
        Ok(())
    } else {
        Err(ClaimError::new(message))
    }
}
