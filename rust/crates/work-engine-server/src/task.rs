//! Thin transport adapter over the accepted synthetic task-answer owner.
//!
//! The trusted startup profile selects the database, repository, board program,
//! key policy and actor. Request arguments only carry the owner's own `task_id`
//! and `Consume` inputs; they cannot select any of that configuration. The
//! owner alone decides signature, deadline, revocation, replay and currentness.
use serde::Deserialize;
use serde_json::{Value, json};
use session_task_admission::owner_boundary::{
    Consume, Error, Pin, Program, SyntheticConfig, SyntheticOwner,
};
use std::{path::PathBuf, sync::Mutex};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

pub const CAPABILITY: &str = "synthetic-owner-v1";
pub const OPERATIONS: [&str; 3] = [
    "project_owner_task",
    "consume_owner_answer",
    "read_owner_answer",
];

pub fn operation(name: &str) -> bool {
    OPERATIONS.contains(&name)
}

/// Trusted local startup file. Never accepted from a caller.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    version: u8,
    database: PathBuf,
    repository: PathBuf,
    board: Program,
    policy: Pin,
    credential_id: String,
    key_fingerprint: String,
    owner: String,
    actor: String,
    session_id: String,
    rp_id: String,
    origin: String,
    epoch: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Project {
    task_id: String,
}

pub struct Owner {
    inner: Mutex<SyntheticOwner>,
    /// Trusted-profile path of the configured board interpreter, kept only for
    /// the readiness refutation below.
    board_python: PathBuf,
}

impl Owner {
    pub fn open(profile: Profile) -> Result<Self, &'static str> {
        let epoch =
            OffsetDateTime::parse(&profile.epoch, &Rfc3339).map_err(|_| "invalid_task_profile")?;
        let board_python = profile.board.python.path.clone();
        if profile.version != 1 {
            return Err("invalid_task_profile");
        }
        let owner = SyntheticOwner::open_synthetic(SyntheticConfig {
            database: profile.database,
            repository: profile.repository,
            board: profile.board,
            policy: profile.policy,
            credential_id: profile.credential_id,
            key_fingerprint: profile.key_fingerprint,
            owner: profile.owner,
            actor: profile.actor,
            session_id: profile.session_id,
            rp_id: profile.rp_id,
            origin: profile.origin,
            epoch,
            scoped_control: None,
        })
        .map_err(|_| "invalid_task_profile")?;
        Ok(Self {
            inner: Mutex::new(owner),
            board_python,
        })
    }

    /// Readiness of the configured task capability, distinct from liveness.
    ///
    /// The accepted owner offers no nonmutating availability probe: `project`
    /// and `consume` write, `read_exact` needs a real signed selection, and its
    /// pin check is crate-private. Construction therefore proves only that the
    /// profile parsed and the owner opened, never that operations can run.
    /// `task_owner` is thus never `true` here. The one cheap, bounded evidence
    /// is the configured board interpreter: absent, readiness is attributed
    /// `unavailable`; otherwise it stays `unestablished`. No owner call is made
    /// and no signature, decision or hash logic is performed by the transport.
    pub fn readiness(&self) -> Value {
        let (state, reason) = if self.board_python.is_file() {
            ("unestablished", "no_nonmutating_owner_probe")
        } else {
            ("unavailable", "board_executable_missing")
        };
        json!({"task_owner":false,"task_owner_readiness":{"state":state,"reason":reason,"capability":CAPABILITY}})
    }

    /// Blocking: call from `spawn_blocking` holding an owner-concurrency permit.
    /// An uncertain consume is returned as `owner_unknown`/`owner_unavailable`;
    /// it is never retried here. `read_owner_answer` is the reconciliation read.
    pub fn call(&self, operation: &str, arguments: Value) -> Result<Value, &'static str> {
        let mut owner = self.inner.lock().map_err(|_| "owner_unavailable")?;
        let value = match operation {
            "project_owner_task" => {
                let p: Project =
                    serde_json::from_value(arguments).map_err(|_| "invalid_arguments")?;
                serde_json::to_value(owner.project(&p.task_id).map_err(code)?)
            }
            "consume_owner_answer" => {
                let c: Consume =
                    serde_json::from_value(arguments).map_err(|_| "invalid_arguments")?;
                serde_json::to_value(owner.consume(&c).map_err(code)?)
            }
            "read_owner_answer" => {
                let c: Consume =
                    serde_json::from_value(arguments).map_err(|_| "invalid_arguments")?;
                serde_json::to_value(owner.read_exact(&c).map_err(code)?)
            }
            _ => return Err("invalid_request"),
        };
        value.map_err(|_| "owner_unavailable")
    }
}

fn code(e: Error) -> &'static str {
    match e {
        Error::Unavailable => "owner_unavailable",
        Error::Invalid => "owner_invalid",
        Error::Conflict => "owner_conflict",
        Error::Held => "owner_held",
        Error::Unknown => "owner_unknown",
    }
}

pub fn tools() -> Vec<Value> {
    let signed = json!({"type":"object","additionalProperties":false,"properties":{"answer_json":{"type":"string"},"credential_id":{"type":"string"},"authenticator_data":{"type":"string"},"client_data_json":{"type":"string"},"signature":{"type":"string"}},"required":["answer_json","credential_id","authenticator_data","client_data_json","signature"]});
    let reference = json!({"type":"object","additionalProperties":false,"properties":{"id":{"type":"string"},"revision":{"type":"string"},"sha256":{"type":"string"}},"required":["id","revision","sha256"]});
    let selection = json!({"type":"object","additionalProperties":false,"properties":{"task":reference,"proposal":reference,"task_generation":{"type":"integer"},"proposal_generation":{"type":"integer"},"request_sequence":{"type":"integer"},"board_revision":{"type":"string"}},"required":["task","proposal","task_generation","proposal_generation","request_sequence","board_revision"]});
    let consume = json!({"type":"object","additionalProperties":false,"properties":{"selection":selection,"signed":signed},"required":["selection","signed"]});
    vec![
        json!({"name":"project_owner_task","description":"Project the current synthetic owner task request","inputSchema":{"type":"object","additionalProperties":false,"properties":{"task_id":{"type":"string"}},"required":["task_id"]}}),
        json!({"name":"consume_owner_answer","description":"Durably consume an exact signed owner answer","inputSchema":consume}),
        json!({"name":"read_owner_answer","description":"Reconcile an exact signed owner answer without consuming it","inputSchema":consume}),
    ]
}
