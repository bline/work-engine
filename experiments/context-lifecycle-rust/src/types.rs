use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

macro_rules! id {
    ($($name:ident),+ $(,)?) => {$ (
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
        pub struct $name(pub(crate) String);
        impl $name {
            pub fn new(value: impl Into<String>) -> Self { Self(value.into()) }
            pub fn as_str(&self) -> &str { &self.0 }
        }
    )+};
}
id!(
    ContextId,
    TransitionId,
    InputId,
    DeliveryId,
    EffectId,
    CommandId,
    RuntimeId
);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Epoch(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Revision(pub u64);
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stamp {
    pub context: ContextId,
    pub epoch: Epoch,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapturePermit {
    pub context: ContextId,
    pub epoch: Epoch,
    pub transition: TransitionId,
    pub deadline: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Continuation {
    pub objective: String,
    pub authority: String,
    pub commitments: Vec<String>,
    pub references: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Input {
    pub id: InputId,
    pub payload: Vec<u8>,
    pub digest: String,
    pub lineage: String,
    pub delivery: Option<DeliveryId>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    Hold,
    Succeed,
    Fail,
    ObserveCodex,
    CodexText,
    CodexTextLive,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceDomain {
    FakeSeal,
    CodexObservation,
    CodexTextTurnCompletion,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectPurpose {
    Input,
    Readiness {
        transition: TransitionId,
        package_revision: Revision,
        package_digest: String,
        expected_ack: String,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntheticAuthority {
    pub approval: String,
    pub deadline: u64,
    pub mode: Mode,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    Success(String),
    Failure(String),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectState {
    Prepared,
    Entered,
    Unknown,
    Success,
    Failure,
    Conflict,
    ExpiredBeforeEntry,
}
impl EffectState {
    pub fn unresolved(&self) -> bool {
        !matches!(
            self,
            Self::Success | Self::Failure | Self::ExpiredBeforeEntry
        )
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryOutcome {
    Pending,
    Unknown,
    Success,
    Failure,
    Conflict,
    ExpiredBeforeEntry,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderRequest {
    pub effect: EffectId,
    pub context: ContextId,
    pub epoch: Epoch,
    pub delivery: DeliveryId,
    pub input: InputId,
    pub payload: Vec<u8>,
    pub input_digest: String,
    pub lineage: String,
    pub authority: SyntheticAuthority,
    pub purpose: EffectPurpose,
    pub profile_digest: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evidence {
    pub effect: EffectId,
    pub request_digest: String,
    pub outcome: Outcome,
    pub sealed: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Effect {
    pub id: EffectId,
    pub context: ContextId,
    pub delivery: DeliveryId,
    pub request: ProviderRequest,
    pub domain: EvidenceDomain,
    pub request_digest: String,
    pub state: EffectState,
    pub evidence: Vec<Evidence>,
    #[serde(default)]
    pub codex: Option<crate::codex::Observation>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Delivery {
    pub id: DeliveryId,
    pub input: InputId,
    pub effect: EffectId,
    pub admitted_epoch: Epoch,
    pub outcome: DeliveryOutcome,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaptureState {
    Pending,
    Ready,
    Failed(String),
    Unknown(String),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transition {
    pub permit: CapturePermit,
    pub capture: CaptureState,
    pub continuation: Option<Continuation>,
    pub attestation: Option<Attestation>,
    pub completed: bool,
    #[serde(default)]
    pub readiness_package: Option<Bootstrap>,
    #[serde(default)]
    pub readiness_effect: Option<EffectId>,
    #[serde(default)]
    pub limited_adequacy: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Context {
    pub id: ContextId,
    pub epoch: Epoch,
    pub active: bool,
    pub fenced: bool,
    pub queue: Vec<Input>,
    pub transition: Option<TransitionId>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bootstrap {
    pub context: ContextId,
    pub transition: TransitionId,
    pub successor: Epoch,
    pub revision: Revision,
    pub continuation: Continuation,
    pub queue: Vec<Input>,
    pub unresolved: Vec<Effect>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attestation {
    pub context: ContextId,
    pub transition: TransitionId,
    pub successor: Epoch,
    pub revision: Revision,
    pub digest: String,
    pub runtime: RuntimeId,
    pub model_context: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalExit {
    pub context: ContextId,
    pub epoch: Epoch,
    pub pid: u32,
    pub status: String,
    pub observed_at: u64,
    pub unsafe_at_time: bool,
    pub unresolved: Vec<EffectId>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaptureEnd {
    Failed(String),
    Unknown(String),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Command {
    pub key: CommandId,
    pub action: Action,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Register {
        context: ContextId,
    },
    Enqueue {
        owner: Stamp,
        input: InputId,
        payload: Vec<u8>,
        lineage: String,
    },
    Admit {
        owner: Stamp,
        input: InputId,
        delivery: DeliveryId,
        effect: EffectId,
        authority: SyntheticAuthority,
    },
    Replace {
        owner: Stamp,
        transition: TransitionId,
        deadline: u64,
    },
    Capture {
        permit: CapturePermit,
        continuation: Continuation,
    },
    BeginReadiness {
        permit: CapturePermit,
        effect: EffectId,
        delivery: DeliveryId,
        authority: SyntheticAuthority,
    },
    EndCapture {
        permit: CapturePermit,
        outcome: CaptureEnd,
    },
    Activate {
        attestation: Attestation,
    },
    ActivateText {
        attestation: Attestation,
        limited_adequacy: String,
    },
    ObserveLocalExit {
        owner: Stamp,
        pid: u32,
        status: String,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Reply {
    Owner(Stamp),
    Capture(CapturePermit),
    Ticket(EffectId),
    Recorded,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct JournalEntry {
    pub command: Command,
    pub reply: Result<Reply, String>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct State {
    pub revision: u64,
    pub contexts: BTreeMap<ContextId, Context>,
    pub transitions: BTreeMap<TransitionId, Transition>,
    pub effects: BTreeMap<EffectId, Effect>,
    pub deliveries: BTreeMap<DeliveryId, Delivery>,
    pub exits: Vec<LocalExit>,
    pub commands: BTreeMap<CommandId, JournalEntry>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub revision: Revision,
    pub contexts: Vec<Context>,
    pub transitions: Vec<Transition>,
    pub effects: Vec<Effect>,
    pub deliveries: Vec<Delivery>,
    pub exits: Vec<LocalExit>,
    pub barrier: Option<Milestone>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stage {
    BeforeEntry,
    AfterEntry,
    AfterSend,
    AfterResult,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Milestone {
    pub stage: Stage,
    pub effect: EffectId,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Condition {
    Barrier,
    Effect(EffectId, EffectState),
    RevisionAtLeast(u64),
    Native(EffectId, crate::codex::ObservationState),
}
pub fn digest(value: &impl Serialize) -> String {
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(value).expect("serializable protocol value"))
    )
}
