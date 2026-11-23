//! The only non-test child that constructs B facts, reachable solely under
//! the explicit offline fixture feature. Scope and semantic checks remain at
//! B's ingress. A's Campaign transaction/guard owns currentness and sends.

use super::*;
use crate::application::offline_qualification::{
    AttemptBinding, FixtureCreateReadback, FixtureHeldReadbacks, FixtureRunning, HoldFailure,
};
use crate::codec::campaign_digest;
use serde_json::json;

mod terminal;

pub(in crate::store) fn object(readback: &FixtureCreateReadback) -> Result<ObjectTarget> {
    Ok(ObjectTarget {
        full_id: Digest::new(readback.object.full_id.clone())?,
        serving_generation: Digest::new(readback.object.serving_generation.clone())?,
    })
}

fn task(running: &FixtureRunning) -> Result<TaskIdentity> {
    Ok(TaskIdentity {
        task: running.task_id.clone(),
        lifetime: Digest::new(running.lifetime.clone())?,
        exec_generation: Digest::new(running.exec_generation.clone())?,
    })
}

fn fact(
    binding: &Binding,
    mark: &Mark,
    source: Digest,
    slot: Slot,
    task: Option<TaskIdentity>,
    observation: Observation,
) -> AuthenticatedFact {
    AuthenticatedFact {
        binding: binding.clone(),
        fact: Fact {
            ingress: binding.evidence_ingress.clone(),
            source,
            slot,
            mark: mark.clone(),
            task,
            observation,
        },
    }
}

fn created(
    binding: &Binding,
    mark: &Mark,
    readback: &FixtureCreateReadback,
) -> Result<AuthenticatedFact> {
    let target = object(readback)?;
    if mark.kind != Kind::Create
        || mark.request.as_str() != readback.request_digest
        || target.serving_generation != binding.serving_generation
    {
        return Err(conflict("fixture Create serving readback differs"));
    }
    let source = Digest::new(campaign_digest(&json!([
        "synthetic-selected-serving-object-v1",
        mark.digest()?,
        readback
    ]))?)?;
    Ok(fact(
        binding,
        mark,
        source,
        Slot::Operation(Kind::Create),
        None,
        Observation::Created(target),
    ))
}

fn started(binding: &Binding, mark: &Mark, running: &FixtureRunning) -> Result<AuthenticatedFact> {
    let Some(Target::Start(target)) = &mark.target else {
        return Err(conflict("fixture Start target absent"));
    };
    if mark.kind != Kind::Start
        || mark.request.as_str() != running.start_request_digest
        || target.full_id.as_str() != running.object.full_id
        || target.serving_generation.as_str() != running.object.serving_generation
    {
        return Err(conflict("fixture Start runtime readback differs"));
    }
    let source = Digest::new(campaign_digest(&json!([
        "synthetic-start-runtime-task-v1",
        mark.digest()?,
        running
    ]))?)?;
    Ok(fact(
        binding,
        mark,
        source,
        Slot::Operation(Kind::Start),
        Some(task(running)?),
        Observation::Started,
    ))
}

fn exact_fact(tx: &Transaction<'_>, binding: &Binding, expected: &AuthenticatedFact) -> Result<()> {
    check_spend(tx, binding)?;
    let history = load(tx, binding)?;
    check_exact_fact(&history, expected, &expected.fact.mark)
}

fn check_exact_fact(history: &History, expected: &AuthenticatedFact, tip: &Mark) -> Result<()> {
    let fact = &expected.fact;
    if history.tip() != Some(tip)
        || history.contested()
        || !history.facts[Slot::Terminal.index()].is_empty()
        || history.facts[fact.slot.index()].as_slice() != std::slice::from_ref(fact)
    {
        return Err(conflict(
            "fixture predecessor disposition missing, crossed, or contested",
        ));
    }
    Ok(())
}

pub(in crate::store) fn require_created(
    tx: &Transaction<'_>,
    binding: &Binding,
    mark: &Mark,
    readback: &FixtureCreateReadback,
) -> Result<()> {
    exact_fact(tx, binding, &created(binding, mark, readback)?)
}

impl Store {
    pub(in crate::store) fn confirm_joined_start_mark(
        &mut self,
        binding: &Binding,
        start: &Mark,
        create: &Mark,
        readback: &FixtureCreateReadback,
    ) -> Result<()> {
        let history = self.read_b_history(binding)?;
        check_exact_fact(&history, &created(binding, create, readback)?, start)
    }

    fn confirm_joined_disposition(
        &mut self,
        binding: &Binding,
        auth: AuthenticatedFact,
    ) -> Result<()> {
        let tx = self.b_transaction(binding, true)?;
        let history = load(&tx, binding)?;
        if history.tip() != Some(&auth.fact.mark)
            || history.contested()
            || !history.facts[Slot::Terminal.index()].is_empty()
        {
            return Err(conflict(
                "fixture disposition anchor is not exact and unambiguous",
            ));
        }
        let disposition = Self::write_b_fact(&tx, binding, &auth)?;
        tx.commit().map_err(sql)?;
        if disposition != Recorded::Stored {
            return Err(conflict("fixture disposition was not newly recorded"));
        }
        self.confirm_b_fact(binding, &auth)
    }

    pub(crate) fn confirm_fixture_created(
        &mut self,
        attempt: &AttemptBinding,
        mark: &Mark,
        readback: &FixtureCreateReadback,
    ) -> Result<()> {
        let binding = attempt.lifecycle();
        self.confirm_joined_disposition(binding, created(binding, mark, readback)?)
    }

    pub(crate) fn confirm_fixture_running(
        &mut self,
        attempt: &AttemptBinding,
        mark: &Mark,
        running: &FixtureRunning,
    ) -> Result<()> {
        let binding = attempt.lifecycle();
        self.confirm_joined_disposition(binding, started(binding, mark, running)?)
    }
}

pub(in crate::store) fn write_held(
    tx: &Transaction<'_>,
    attempt: &AttemptBinding,
    mark: &Mark,
    running: &FixtureRunning,
    inputs: &FixtureHeldReadbacks,
    capture_sha256: Option<&str>,
) -> std::result::Result<AuthenticatedFact, HoldFailure> {
    let binding = attempt.lifecycle();
    let map = |_| HoldFailure::FactNotConfirmed;
    exact_fact(tx, binding, &started(binding, mark, running).map_err(map)?).map_err(map)?;
    let auth = held_fact(attempt, mark, running, inputs, capture_sha256)?;
    if Store::write_b_fact(tx, binding, &auth).map_err(map)? != Recorded::Stored {
        return Err(HoldFailure::FactNotConfirmed);
    }
    Ok(auth)
}

fn held_fact(
    attempt: &AttemptBinding,
    mark: &Mark,
    running: &FixtureRunning,
    inputs: &FixtureHeldReadbacks,
    capture_sha256: Option<&str>,
) -> std::result::Result<AuthenticatedFact, HoldFailure> {
    let binding = attempt.lifecycle();
    let map = |_| HoldFailure::FactNotConfirmed;
    inputs.check(attempt, running)?;
    let mark_digest = mark.digest().map_err(map)?;
    let source = match capture_sha256 {
        Some(digest) => {
            Digest::new(digest.to_owned()).map_err(map)?;
            json!([
                "synthetic-init-only-held-capture-v1",
                mark_digest,
                inputs,
                digest
            ])
        }
        None => json!([
            "synthetic-held-exec-listener-effective-state-join-v1",
            mark_digest,
            inputs
        ]),
    };
    let source = Digest::new(campaign_digest(&source).map_err(map)?).map_err(map)?;
    Ok(fact(
        binding,
        mark,
        source,
        Slot::Hold,
        Some(task(running).map_err(map)?),
        Observation::Held,
    ))
}

pub(in crate::store) fn confirm_held(
    store: &mut Store,
    attempt: &AttemptBinding,
    auth: &AuthenticatedFact,
    running: &FixtureRunning,
) -> std::result::Result<(), HoldFailure> {
    let binding = attempt.lifecycle();
    let history = store
        .read_b_history(binding)
        .map_err(|_| HoldFailure::FactNotConfirmed)?;
    let mark = &auth.fact.mark;
    let started = started(binding, mark, running).map_err(|_| HoldFailure::FactNotConfirmed)?;
    check_exact_fact(&history, &started, mark).map_err(|_| HoldFailure::FactNotConfirmed)?;
    check_exact_fact(&history, auth, mark).map_err(|_| HoldFailure::FactNotConfirmed)
}
