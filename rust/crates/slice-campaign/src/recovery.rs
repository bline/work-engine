use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::contract::{CampaignIdentity, CampaignRevision};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OperationReceipt {
    pub root_id: String,
    pub identity: CampaignIdentity,
    pub operation_id: String,
    pub kind: String,
    pub request_digest: String,
    pub profile_digest: String,
    pub prior_revision: Option<CampaignRevision>,
    pub result_revision: CampaignRevision,
    pub result_reference: String,
}

#[derive(Clone, Debug)]
pub enum Effect<T> {
    Applied { value: T, receipt: OperationReceipt },
    Replayed { value: T, receipt: OperationReceipt },
    NoEffect(String),
    OutcomeUnknown(RecoveryLocator),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecoveryLocator {
    pub root_id: String,
    pub anchored_root: PathBuf,
    pub identity: CampaignIdentity,
    pub operation_id: String,
    pub kind: String,
    pub request_digest: String,
    pub expected_revision: Option<CampaignRevision>,
    pub profile_digest: String,
}

#[derive(Clone, Debug)]
pub enum Reconcile {
    Absent,
    Committed(OperationReceipt),
    Conflicting(OperationReceipt),
    Unresolved(String),
}
