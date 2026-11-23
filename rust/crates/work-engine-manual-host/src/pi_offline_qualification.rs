//! Isolated Host driver for offline qualification composition tests. There is
//! no Job, CLI, Docker adapter, recovery send, or terminal interpretation here.

use std::sync::Mutex;

use serde_json::json;
use slice_campaign::codec::campaign_digest;
use slice_campaign::offline_qualification::{
    AttemptBinding, JoinedMarkReadback, OperationKind, SpentQualification,
};
use slice_campaign::{CampaignError, Result};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Admission {
    pub binding: AttemptBinding,
    pub operation_id: String,
    pub recipe_sha256: String,
    pub serving_sha256: String,
}

impl Admission {
    pub fn matches(&self, expected: &AttemptBinding) -> bool {
        &self.binding == expected
            && self.operation_id == expected.operation_id()
            && self.recipe_sha256 == expected.recipe_sha256()
            && self.serving_sha256 == expected.serving_sha256()
    }
}

#[derive(PartialEq, Eq)]
enum CollectorState {
    Ready,
    Admitted,
    Closed,
}

/// Short per-operation fence. Cancel/close and the mark decision order under
/// this lock. A send that already won its decision may still take effect.
pub struct CollectorFence(Mutex<CollectorState>);

impl CollectorFence {
    pub fn new_fixture() -> Self {
        Self(Mutex::new(CollectorState::Ready))
    }

    pub fn admit_once(&self) -> bool {
        let Ok(mut state) = self.0.lock() else {
            return false;
        };
        if *state != CollectorState::Ready {
            return false;
        }
        *state = CollectorState::Admitted;
        true
    }

    pub fn close(&self) {
        if let Ok(mut state) = self.0.lock() {
            *state = CollectorState::Closed;
        }
    }

    fn if_admitted(&self, mark: impl FnOnce() -> Result<()>) -> Result<()> {
        let state = self.0.lock().map_err(|_| refused("collector poisoned"))?;
        if *state != CollectorState::Admitted {
            return Err(refused("collector not open for send"));
        }
        // Lock order is collector -> Campaign, and no effect code runs here.
        mark()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectRequest {
    pub version: u8,
    pub sequence: u8,
    pub kind: OperationKind,
    pub binding: AttemptBinding,
    pub predecessor_digest: Option<String>,
    pub request_digest: String,
    pub target_id: Option<String>,
}

impl EffectRequest {
    fn new(
        binding: &AttemptBinding,
        kind: OperationKind,
        prior: Option<&Self>,
        target: Option<&str>,
    ) -> Result<Self> {
        let (sequence, name) = match kind {
            OperationKind::Create => (1, "create"),
            OperationKind::Start => (2, "start"),
            OperationKind::Close => (3, "close"),
        };
        let version = 1;
        let predecessor_digest = prior.map(|op| op.request_digest.clone());
        let request_digest = if kind == OperationKind::Create {
            if prior.is_some() || target.is_some() {
                return Err(refused("Create may not have a predecessor or target"));
            }
            binding.create_request_digest()?
        } else {
            campaign_digest(&json!([
                "b-offline-operation",
                version,
                sequence,
                name,
                binding.root_id(),
                binding.grant_id(),
                binding.attempt_id(),
                binding.operation_id(),
                binding.binding_digest(),
                predecessor_digest,
                target
            ]))?
        };
        Ok(Self {
            version,
            sequence,
            kind,
            binding: binding.clone(),
            predecessor_digest,
            request_digest,
            target_id: target.map(str::to_owned),
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum MarkerCase {
    Fresh,
    Missing,
    Preexisting,
    WrongBinding,
    Unknown,
}

/// In-memory simulation of the mark-before-effect contract, NOT B's store.
/// Non-fresh decisions never delegate even if a possible mark was recorded.
pub struct MarkerFixture {
    cases: [MarkerCase; 3],
    marked: Vec<EffectRequest>,
}

impl MarkerFixture {
    pub fn new(cases: [MarkerCase; 3]) -> Self {
        Self {
            cases,
            marked: Vec::new(),
        }
    }
    pub fn marked(&self) -> &[EffectRequest] {
        &self.marked
    }

    fn record_exact(&mut self, request: &EffectRequest) -> Result<()> {
        if self.marked.iter().any(|r| r.sequence == request.sequence) {
            return Err(refused("pre-existing operation mark"));
        }
        match self.cases[usize::from(request.sequence - 1)] {
            MarkerCase::Fresh => {
                self.marked.push(request.clone());
                Ok(())
            }
            MarkerCase::Missing => Err(refused("mark unavailable")),
            MarkerCase::Preexisting => Err(refused("pre-existing operation mark")),
            MarkerCase::WrongBinding => Err(refused("mark readback differs")),
            MarkerCase::Unknown => {
                self.marked.push(request.clone());
                Err(refused("mark outcome unknown"))
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum EffectObservation {
    Created {
        request_digest: String,
        full_id: String,
    },
    Acknowledged {
        request_digest: String,
    },
    Refused {
        request_digest: String,
    },
    Unknown,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    ScopeRefused,
    NotSubmitted(OperationKind),
    ConfirmedRefusal(OperationKind),
    Unresolved(OperationKind),
    InertSequenceAcknowledged,
    /// Joined Create got a matching full object ID, not a task or terminal fact.
    InertObjectCreated,
}

fn refused(reason: &str) -> CampaignError {
    CampaignError::Conflict(format!("offline fixture: {reason}"))
}

fn full_identity(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Run the joined inert Create only. No selected task/lifetime evidence exists
/// here. Consuming the fresh handoff at the mark leaves no Start/Close path.
pub fn run_joined_fixture(
    spent: SpentQualification,
    selection: &Admission,
    fence: &CollectorFence,
    readback: JoinedMarkReadback,
    observe: impl FnOnce(&EffectRequest) -> EffectObservation,
) -> Outcome {
    use OperationKind::Create;
    if !selection.matches(spent.binding()) {
        return Outcome::ScopeRefused;
    }
    let Ok(op) = EffectRequest::new(spent.binding(), Create, None, None) else {
        return Outcome::NotSubmitted(Create);
    };
    if fence
        .if_admitted(|| spent.mark_joined_create(&op.request_digest, readback))
        .is_err()
    {
        return Outcome::NotSubmitted(Create);
    }
    // Outside both gates. The observation cannot publish authenticated B facts.
    match observe(&op) {
        EffectObservation::Created {
            request_digest,
            full_id,
        } if request_digest == op.request_digest && full_identity(&full_id) => {
            Outcome::InertObjectCreated
        }
        EffectObservation::Refused { request_digest } if request_digest == op.request_digest => {
            Outcome::ConfirmedRefusal(Create)
        }
        _ => Outcome::Unresolved(Create),
    }
}

/// Owns the fresh ticket for the A-only simulated sequence. `observe` is a fake
/// below both gates, available only with the offline fixture feature. A clean
/// response to Close is *not* terminal evidence.
pub fn run_fixture(
    mut spent: SpentQualification,
    selection: &Admission,
    fence: &CollectorFence,
    marker: &mut MarkerFixture,
    mut observe: impl FnMut(&EffectRequest) -> EffectObservation,
) -> Outcome {
    if !selection.matches(spent.binding()) {
        return Outcome::ScopeRefused;
    }
    use EffectObservation as Seen;
    use OperationKind::{Close, Create, Start};

    let mut previous = None;
    let mut object_id = None;
    for kind in [Create, Start, Close] {
        let Ok(op) = EffectRequest::new(
            spent.binding(),
            kind,
            previous.as_ref(),
            object_id.as_deref(),
        ) else {
            return Outcome::NotSubmitted(kind);
        };
        if fence
            .if_admitted(|| spent.mark_inert_send(op.kind, || marker.record_exact(&op)))
            .is_err()
        {
            return Outcome::NotSubmitted(kind);
        }
        match observe(&op) {
            Seen::Created {
                request_digest,
                full_id,
            } if kind == Create
                && request_digest == op.request_digest
                && full_identity(&full_id) =>
            {
                object_id = Some(full_id);
            }
            Seen::Acknowledged { request_digest }
                if kind != Create && request_digest == op.request_digest => {}
            Seen::Refused { request_digest } if request_digest == op.request_digest => {
                return Outcome::ConfirmedRefusal(kind);
            }
            _ => return Outcome::Unresolved(kind),
        }
        previous = Some(op);
    }
    Outcome::InertSequenceAcknowledged
}
