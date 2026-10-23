use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

macro_rules! record {
    ($name:ident { $($field:ident : $ty:ty),* $(,)? }) => {
        #[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
        #[serde(deny_unknown_fields)]
        pub struct $name { $(pub $field: $ty),* }
    };
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    pub host: String,
    pub session: String,
    pub agent: Option<String>,
}

#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord,
)]
#[serde(rename_all = "snake_case")]
pub enum Authority {
    Admit,
    Verify,
    Amend,
    Intervene,
    Close,
    Inspect,
}

#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord,
)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Read,
    Publish,
    Send,
    Receive,
    Dispatch,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Grant {
    pub action: Action,
    pub resource: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CoverageMode {
    Managed,
    Cooperative,
    Uncontrolled,
}
record!(Coverage {
    mode: CoverageMode,
    limitation: String
});
record!(Evidence {
    reference: String,
    sha256: String
});
record!(Artifact {
    sha256: String,
    data_base64: String,
    provenance: String,
    resource: String
});
record!(Environment { artifacts: BTreeSet<String>, limitations: Vec<String> });
record!(Profile {
    arm: Option<String>, environment: String, projection: String,
    grants: Vec<Grant>, allow_carryover: bool, require_observed_exposure: bool,
});
record!(Specification {
    experiment: String, owner: Identity, authority_reference: String,
    delegates: BTreeMap<Authority, Vec<Identity>>, arms: BTreeSet<String>,
    phases: BTreeMap<String, String>, artifacts: BTreeMap<String, Artifact>,
    environments: BTreeMap<String, Environment>, profiles: BTreeMap<String, Profile>,
    coverage: BTreeMap<String, Coverage>, inspect_after: BTreeSet<String>,
});

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BindingStatus {
    Prepared,
    Bound,
    Verified,
    Active,
    Retired,
    Cancelled,
}
record!(Exposure {
    identity_evidence: Evidence, observation_evidence: Option<Evidence>,
    supplied_artifacts: BTreeSet<String>, observed_artifacts: BTreeSet<String>,
    prior_bindings: BTreeSet<String>, inherited_context: Vec<Evidence>,
});
record!(Binding {
    handle: String, spec: String, phase: String, profile: String, unit: String,
    branch: String, parent: Option<String>, replaces: Option<String>,
    continuation: BTreeSet<String>, status: BindingStatus, paused: bool,
    identity: Option<Identity>, exposure: Option<Exposure>, generation: u64,
    prepared_by: Identity,
    verified_by: Option<Identity>, verification: Option<Evidence>,
});
record!(Branch { current: Option<String>, last_owner: Option<String>, generation: u64, next_message: usize });

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptKind {
    Startup,
    Artifact,
    Messages,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptStatus {
    Issued,
    Collected,
    Disposed,
}
record!(Receipt {
    id: String, local_id: String, handle: String, kind: ReceiptKind, status: ReceiptStatus,
    issued_by: Identity, artifact_ids: BTreeSet<String>, next_message: Option<usize>,
    evidence: Option<Evidence>, disposition: Option<String>,
});
record!(Message {
    id: String, source: Identity, sender: Option<String>, resource: String,
    data_base64: String, sha256: String, provenance: String,
});
record!(Audit {
    id: String,
    actor: Identity,
    observed_at: String,
    command: serde_json::Value
});
record!(State {
    schema: u32, experiment: String, specs: BTreeMap<String, Specification>,
    bindings: BTreeMap<String, Binding>, branches: BTreeMap<String, Branch>,
    closed_phases: BTreeSet<String>, sealed_admissions: BTreeSet<String>, revoked_delegates: BTreeSet<String>, receipts: BTreeMap<String, Receipt>,
    inboxes: BTreeMap<String, Vec<Message>>, audit: Vec<Audit>,
    imports: BTreeMap<String, BTreeMap<String, Artifact>>,
});

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Register {
        specification: Specification,
    },
    Amend {
        from_spec: String,
        specification: Specification,
        evidence: Evidence,
    },
    Prepare {
        handle: String,
        spec: String,
        phase: String,
        profile: String,
        unit: String,
        branch: String,
        parent: Option<String>,
        replaces: Option<String>,
        continuation: BTreeSet<String>,
    },
    Startup {
        handle: String,
        receipt: String,
    },
    Bind {
        handle: String,
        identity: Identity,
        exposure: Exposure,
    },
    Verify {
        handle: String,
        evidence: Evidence,
    },
    Activate {
        handle: String,
        expected_generation: u64,
        settlement: Option<Evidence>,
    },
    Evaluate {
        handle: String,
        generation: u64,
        action: Action,
        resource: String,
    },
    Deliver {
        handle: String,
        generation: u64,
        artifacts: BTreeSet<String>,
        receipt: String,
    },
    Import {
        spec: String,
        artifact_id: String,
        artifact: Artifact,
        evidence: Evidence,
        producer: Option<String>,
        generation: Option<u64>,
    },
    Send {
        handle: String,
        generation: u64,
        id: String,
        targets: BTreeSet<String>,
        resource: String,
        data_base64: String,
        provenance: String,
    },
    Receive {
        handle: String,
        generation: u64,
        receipt: String,
        limit: usize,
    },
    Acknowledge {
        receipt: String,
        evidence: Evidence,
    },
    Dispose {
        receipt: String,
        evidence: Evidence,
        disposition: String,
    },
    Pause {
        handle: String,
        evidence: Evidence,
    },
    Resume {
        handle: String,
        evidence: Evidence,
        disposition: String,
    },
    Retire {
        handle: String,
        evidence: Evidence,
    },
    ClosePhase {
        spec: String,
        phase: String,
        evidence: Evidence,
    },
    SealAdmissions {
        spec: String,
        phase: String,
        evidence: Evidence,
    },
    RevokeDelegate {
        spec: String,
        identity: Identity,
        evidence: Evidence,
    },
    Status {
        handle: String,
    },
    Inspect {
        spec: String,
    },
}
record!(Request {
    id: String,
    actor: Identity,
    observed_at: String,
    command: Command
});
record!(Input { state: Option<State>, request: Request });
record!(Output {
    state: State,
    result: serde_json::Value,
    changed: bool
});
