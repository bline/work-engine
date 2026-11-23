//! Isolated Host driver for offline qualification composition tests. There is
//! no Job, CLI, Docker adapter, recovery send, or terminal interpretation here.

use std::sync::Mutex;

use serde_json::json;
use slice_campaign::codec::campaign_digest;
use slice_campaign::offline_qualification::{
    AttemptBinding, FixtureCreateReadback, FixtureHeldReadbacks, FixtureRunning, HoldFailure,
    JoinedMarkReadback, JoinedStartRequest, OperationKind, SpentQualification,
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

    fn if_admitted<T, E>(
        &self,
        closed: impl FnOnce() -> E,
        check: impl FnOnce() -> std::result::Result<T, E>,
    ) -> std::result::Result<T, E> {
        // Collector -> Campaign; the guard survives the decision.
        match self.0.lock() {
            Ok(state) if *state == CollectorState::Admitted => check(),
            _ => Err(closed()),
        }
    }

    fn mark<T>(&self, mark: impl FnOnce() -> Result<T>) -> Result<T> {
        self.if_admitted(|| refused("collector not open for send"), mark)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectRequest {
    pub version: u8,
    pub sequence: u8,
    pub kind: OperationKind,
    pub binding: AttemptBinding,
    pub predecessor_digest: Option<String>,
    pub predecessor_mark_digest: Option<String>,
    pub request_digest: String,
    pub target_id: Option<String>,
}

impl EffectRequest {
    fn joined_start(binding: &AttemptBinding, start: &JoinedStartRequest) -> Self {
        Self {
            version: 1,
            sequence: 2,
            kind: OperationKind::Start,
            binding: binding.clone(),
            predecessor_digest: Some(start.create_request_digest.clone()),
            predecessor_mark_digest: Some(start.create_mark_digest.clone()),
            request_digest: start.request_digest.clone(),
            target_id: Some(start.target.full_id.clone()),
        }
    }

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
            predecessor_mark_digest: None,
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

#[derive(Debug)]
pub enum EffectObservation {
    Created {
        request_digest: String,
        full_id: String,
    },
    JoinedCreated {
        request_digest: String,
        full_id: String,
        serving_readback: FixtureCreateReadback,
    },
    Acknowledged {
        request_digest: String,
    },
    JoinedRunning {
        running: FixtureRunning,
        held: Box<FixtureHeldReadbacks>,
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
    /// Matching full object reply only. No serving readback, no Start.
    InertObjectCreated,
    /// Inert operation ACK only: no independently bound running task or held evidence.
    StartAcknowledged,
    StartedWithoutHeld(HoldFailure),
    InertInitHeld,
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

#[derive(Clone, Copy)]
pub struct JoinedReadbacks {
    pub create_mark: JoinedMarkReadback,
    pub start_mark: JoinedMarkReadback,
}

impl From<JoinedMarkReadback> for JoinedReadbacks {
    fn from(create_mark: JoinedMarkReadback) -> Self {
        Self {
            create_mark,
            start_mark: JoinedMarkReadback::Check,
        }
    }
}

/// Same winning collector and moved handoff for both inert operations. No
/// independent Start entry, recovery, Close, delivery or release branch.
pub fn run_joined_fixture(
    spent: SpentQualification,
    selection: &Admission,
    fence: &CollectorFence,
    readbacks: impl Into<JoinedReadbacks>,
    mut observe: impl FnMut(&EffectRequest) -> EffectObservation,
) -> Outcome {
    use EffectObservation as Seen;
    use OperationKind::{Create, Start};
    if !selection.matches(spent.binding()) {
        return Outcome::ScopeRefused;
    }
    let readbacks = readbacks.into();
    let Ok(op) = EffectRequest::new(spent.binding(), Create, None, None) else {
        return Outcome::NotSubmitted(Create);
    };
    let Ok(marked) =
        fence.mark(|| spent.mark_joined_create(&op.request_digest, readbacks.create_mark))
    else {
        return Outcome::NotSubmitted(Create);
    };
    // Both effects run after exact committed readback, outside both gates.
    let next = match observe(&op) {
        Seen::JoinedCreated {
            request_digest,
            full_id,
            serving_readback,
        } => match marked.confirm_created(&request_digest, &full_id, serving_readback) {
            Ok(next) => next,
            Err(_) => return Outcome::Unresolved(Create),
        },
        Seen::Created {
            request_digest,
            full_id,
        } if request_digest == op.request_digest && full_identity(&full_id) => {
            return Outcome::InertObjectCreated;
        }
        Seen::Refused { request_digest } if request_digest == op.request_digest => {
            return Outcome::ConfirmedRefusal(Create);
        }
        _ => return Outcome::Unresolved(Create),
    };
    let start = EffectRequest::joined_start(&op.binding, next.request());
    let Ok(marked) = fence.mark(|| next.mark(readbacks.start_mark)) else {
        // Only a local non-delegation; a committed mark may still exist.
        return Outcome::NotSubmitted(Start);
    };
    match observe(&start) {
        Seen::JoinedRunning { running, held } => {
            let Ok(confirmed) = marked.confirm_running(running) else {
                return Outcome::Unresolved(Start);
            };
            match fence.if_admitted(|| HoldFailure::NotCurrent, || confirmed.verify_held(*held)) {
                Ok(()) => Outcome::InertInitHeld,
                Err(reason) => Outcome::StartedWithoutHeld(reason),
            }
        }
        Seen::Acknowledged { request_digest } if request_digest == start.request_digest => {
            Outcome::StartAcknowledged
        }
        Seen::Refused { request_digest } if request_digest == start.request_digest => {
            Outcome::ConfirmedRefusal(Start)
        }
        _ => Outcome::Unresolved(Start),
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
            .mark(|| spent.mark_inert_send(op.kind, || marker.record_exact(&op)))
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
