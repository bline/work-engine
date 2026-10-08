use crate::contract::{CampaignIdentity, CampaignRevision, NativeReviewRequestRef};
use crate::recovery::{OperationReceipt, RecoveryLocator};

/// Process-local capability issued only for a newly committed initial request.
/// It cannot be cloned, serialized, or reconstructed from a reference.
#[derive(Debug)]
pub struct AdmissionHandle {
    pub(crate) owner_epoch: String,
    pub(crate) root_id: String,
    pub(crate) identity: CampaignIdentity,
    pub(crate) obligation_id: String,
    pub(crate) operation_id: String,
    pub(crate) request_digest: String,
    pub(crate) prepared_revision: CampaignRevision,
}

#[derive(Debug)]
pub struct PreparedInitial {
    pub request: NativeReviewRequestRef,
    pub handle: AdmissionHandle,
    pub receipt: OperationReceipt,
}

#[derive(Debug)]
pub enum Preparation {
    Applied(Box<PreparedInitial>),
    Replayed {
        request: Box<NativeReviewRequestRef>,
        receipt: OperationReceipt,
    },
    NoEffect(String),
    OutcomeUnknown(RecoveryLocator),
}
