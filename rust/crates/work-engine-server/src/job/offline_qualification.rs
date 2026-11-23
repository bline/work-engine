//! Private, job-free offline collector. Not selected by any Server route.

use slice_campaign::offline_qualification::{
    AttemptBinding, JoinedMarkReadback, QualificationPort, SpentQualification,
};
use work_engine_manual_host::pi_docker::offline_qualification::{
    Admission, CollectorFence, EffectObservation, EffectRequest, MarkerFixture, Outcome,
    run_fixture, run_joined_fixture,
};

#[derive(Debug, PartialEq, Eq)]
enum Collected {
    ScopeRefused,
    ClosedOrAlreadyAdmitted,
    EntryNotConfirmed,
    SpendNotConfirmed,
    Host(Outcome),
}

struct Collector {
    binding: AttemptBinding,
    owner: QualificationPort,
    fence: CollectorFence,
}

impl Collector {
    fn stage(owner: QualificationPort, selected: Option<AttemptBinding>) -> Option<Self> {
        Some(Self {
            binding: selected?,
            owner,
            fence: CollectorFence::new_fixture(),
        })
    }

    fn collect(
        &self,
        request: &Admission,
        marker: &mut MarkerFixture,
        fake: impl FnMut(&EffectRequest) -> EffectObservation,
    ) -> Collected {
        self.collect_once(request, |spent| {
            run_fixture(spent, request, &self.fence, marker, fake)
        })
    }

    fn collect_joined(
        &self,
        request: &Admission,
        readback: JoinedMarkReadback,
        fake: impl FnOnce(&EffectRequest) -> EffectObservation,
    ) -> Collected {
        self.collect_once(request, |spent| {
            run_joined_fixture(spent, request, &self.fence, readback, fake)
        })
    }

    fn collect_once(
        &self,
        request: &Admission,
        drive: impl FnOnce(SpentQualification) -> Outcome,
    ) -> Collected {
        if !request.matches(&self.binding) {
            return Collected::ScopeRefused;
        }
        if !self.fence.admit_once() {
            return Collected::ClosedOrAlreadyAdmitted;
        }
        let result = match self.owner.enter(&self.binding) {
            Err(_) => Collected::EntryNotConfirmed,
            Ok(entered) => match entered.consume() {
                Err(_) => Collected::SpendNotConfirmed,
                Ok(spent) => Collected::Host(drive(spent)),
            },
        };
        self.close();
        result
    }

    fn close(&self) {
        self.fence.close();
    }
}

#[cfg(test)]
mod tests;
