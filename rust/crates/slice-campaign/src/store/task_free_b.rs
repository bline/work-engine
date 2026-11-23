//! Task-free B lifecycle records. No spend, send, or qualification authority.
//!
//! A's owner-only committed-spend view must project precisely committed task-free
//! spends, including an immutable receipt and their opening Store epoch. The view
//! is checked in the *same transaction* on every entry, including historical reads.
//! A separately owns the live fence for every send and must abandon on any failed
//! mark call, even where there is no detectable persisted marker. No mark readback
//! can substitute for that fence or for A's moved, process-local handoff.

use rusqlite::{Transaction, TransactionBehavior, params};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use super::Store;
use crate::{CampaignError, Result};

pub(crate) const MIGRATION_SQL: &str = include_str!("task_free_b.sql");
pub(crate) const PURPOSE: &str = "b-task-free-qualification-v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub(crate) struct Digest(String);

impl TryFrom<String> for Digest {
    type Error = CampaignError;

    fn try_from(value: String) -> Result<Self> {
        Self::new(value)
    }
}

impl Digest {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn new(value: String) -> Result<Self> {
        if value.len() != 64
            || !value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(contract("noncanonical B digest"));
        }
        Ok(Self(value))
    }
}

/// Exact join data, never an admission handle. The selected head binds the
/// admitted purpose; the separate pins identify the scoped dispatch and sources.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Binding {
    pub(crate) root: String,
    pub(crate) profile: String,
    pub(crate) opening_epoch: String,
    pub(crate) issuer: String,
    pub(crate) grant: String,
    pub(crate) attempt: String,
    pub(crate) spend_receipt: String,
    pub(crate) head: Digest,
    pub(crate) canonical_selection: Digest,
    pub(crate) recipe: Digest,
    pub(crate) server_operation: String,
    pub(crate) serving_generation: Digest,
    /// Selection pin for the authenticated evidence ingress, not a credential.
    pub(crate) evidence_ingress: Digest,
}

impl Binding {
    fn mark_scope(&self) -> Result<Digest> {
        let bytes = format!("task-free-b-lifecycle-mark-v1\0{}", encode(self)?);
        Ok(Digest(crate::codec::raw_sha256(bytes.as_bytes())))
    }

    fn validate(&self) -> Result<()> {
        for id in [
            &self.root,
            &self.profile,
            &self.opening_epoch,
            &self.issuer,
            &self.grant,
            &self.attempt,
            &self.spend_receipt,
            &self.server_operation,
        ] {
            validate_id(id)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TaskIdentity {
    pub(crate) task: String,
    pub(crate) lifetime: Digest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i64)]
pub(crate) enum Kind {
    Create = 1,
    Start = 2,
    Close = 3,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Mark {
    scope: Digest,
    sequence: i64,
    kind: Kind,
    request: Digest,
    task: Option<TaskIdentity>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Slot {
    Operation(Kind),
    Hold,
    Terminal,
}

impl Slot {
    fn index(self) -> usize {
        match self {
            Self::Operation(kind) => kind as usize - 1,
            Self::Hold => 3,
            Self::Terminal => 4,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Observation {
    Created(TaskIdentity),
    Started,
    Closed,
    Held,
    /// One coherent attestation, not two claims composed from different lifetimes.
    /// The provenance digests are supplied by the authenticated ingress. Neither
    /// predicate prescribes a Docker mechanism or a Delete acknowledgment.
    Terminal {
        workload_and_subtree_exit: Digest,
        accepted_and_recovered_work_fenced: Digest,
    },
    Failed,
    Lost,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Fact {
    ingress: Digest,
    source: Digest,
    slot: Slot,
    mark: Mark,
    task: Option<TaskIdentity>,
    observation: Observation,
}

/// Not deserializable or constructible by sibling modules or by callers. Only
/// an explicitly wired child of this module may produce authenticated facts.
/// This offline cut has no production producer; the descendant fixture is test-only.
pub(crate) struct AuthenticatedFact {
    binding: Binding,
    fact: Fact,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Recorded {
    Stored,
    Duplicate,
    /// Once quarantined we retain the first two conflicting immutable records.
    Contested,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Evidence<'a> {
    Absent,
    Failed(&'a Digest),
    Lost(&'a Digest),
    Unknown(&'a Digest),
    /// The source reported hold but start disposition is not confirmed.
    UnresolvedHold(&'a Digest),
    Contested,
    Observed {
        source: &'a Digest,
        observation: &'a Observation,
    },
}

#[derive(Default)]
pub(crate) struct History {
    marks: Vec<Mark>,
    facts: [Vec<Fact>; 5],
}

impl History {
    pub(crate) fn tip(&self) -> Option<&Mark> {
        self.marks.last()
    }

    pub(crate) fn mark(&self, kind: Kind) -> Option<&Mark> {
        self.marks.iter().find(|mark| mark.kind == kind)
    }

    pub(crate) fn evidence(&self, slot: Slot) -> Evidence<'_> {
        // Even an individually good fact cannot qualify a contested attempt.
        if self.contested() {
            return Evidence::Contested;
        }
        let Some(fact) = self.facts[slot.index()].first() else {
            return Evidence::Absent;
        };
        match &fact.observation {
            Observation::Failed => Evidence::Failed(&fact.source),
            Observation::Lost => Evidence::Lost(&fact.source),
            Observation::Unknown => Evidence::Unknown(&fact.source),
            Observation::Held
                if !matches!(
                    self.evidence(Slot::Operation(Kind::Start)),
                    Evidence::Observed {
                        observation: Observation::Started,
                        ..
                    }
                ) =>
            {
                Evidence::UnresolvedHold(&fact.source)
            }
            positive => Evidence::Observed {
                source: &fact.source,
                observation: positive,
            },
        }
    }

    fn contested(&self) -> bool {
        self.facts.iter().any(|slot| slot.len() > 1)
    }

    fn original_task(&self) -> Option<&TaskIdentity> {
        // Needed to inspect exact historical anchors even after the create slot
        // is contested. Callers never get a positive projection in that case.
        match &self.facts[Slot::Operation(Kind::Create).index()]
            .first()?
            .observation
        {
            Observation::Created(task) => Some(task),
            _ => None,
        }
    }

    fn check_mark(&self, mark: &Mark, predecessor: Option<&Mark>) -> Result<()> {
        if self.tip() != predecessor
            || mark.sequence != self.marks.len() as i64 + 1
            || !matches!(
                (self.tip().map(|mark| mark.kind), mark.kind),
                (None, Kind::Create)
                    | (Some(Kind::Create), Kind::Start | Kind::Close)
                    | (Some(Kind::Start), Kind::Close)
            )
        {
            return Err(conflict(
                "B operation predecessor/order changed or already marked",
            ));
        }
        match (mark.kind, &mark.task) {
            (Kind::Create, None) => Ok(()),
            (Kind::Start | Kind::Close, Some(task)) if self.original_task() == Some(task) => Ok(()),
            _ => Err(conflict(
                "B operation lacks its exact original task/lifetime",
            )),
        }
    }

    fn check_fact(&self, binding: &Binding, fact: &Fact) -> Result<()> {
        if fact.ingress != binding.evidence_ingress || !self.marks.contains(&fact.mark) {
            return Err(conflict("B fact ingress or operation anchor differs"));
        }
        match fact.slot {
            Slot::Operation(kind) if fact.mark.kind == kind && fact.task == fact.mark.task => {}
            Slot::Hold if fact.mark.kind == Kind::Start && fact.task == fact.mark.task => {}
            Slot::Terminal
                if self.tip() == Some(&fact.mark)
                    && fact.task.is_some()
                    && fact.task.as_ref() == self.original_task() => {}
            _ => {
                return Err(conflict(
                    "B fact is stale, mis-scoped or has the wrong task/lifetime",
                ));
            }
        }
        match (&fact.slot, &fact.observation) {
            (Slot::Operation(Kind::Create), Observation::Created(task)) => validate_id(&task.task),
            (Slot::Operation(Kind::Start), Observation::Started)
            | (Slot::Operation(Kind::Close), Observation::Closed)
            | (Slot::Hold, Observation::Held)
            | (Slot::Terminal, Observation::Terminal { .. })
            | (_, Observation::Failed | Observation::Lost | Observation::Unknown) => Ok(()),
            _ => Err(contract("B observation is not admissible for this slot")),
        }
    }
}

impl Store {
    /// Record possible issue before the corresponding external action. Returns
    /// no capability or recovered/replayed success. A's live fence is mandatory.
    #[cfg(test)]
    pub(crate) fn mark_b_possible_issue(
        &mut self,
        binding: &Binding,
        predecessor: Option<&Mark>,
        kind: Kind,
        request: Digest,
        task: Option<TaskIdentity>,
    ) -> Result<()> {
        let tx = self.b_transaction(binding, true)?;
        let mark = Self::write_b_mark(&tx, binding, predecessor, kind, request, task)?;
        tx.commit().map_err(sql)?;
        self.confirm_b_mark(binding, &mark)
    }

    /// Campaign owns the immediate transaction and the live send decision.
    /// This mutation requires the exact A spend in that same transaction and
    /// returns only expected readback data, never permission to delegate.
    pub(super) fn write_b_mark(
        tx: &Transaction<'_>,
        binding: &Binding,
        predecessor: Option<&Mark>,
        kind: Kind,
        request: Digest,
        task: Option<TaskIdentity>,
    ) -> Result<Mark> {
        binding.validate()?;
        check_spend(tx, binding)?;
        let history = load(tx, binding)?;
        if history.contested() || !history.facts[Slot::Terminal.index()].is_empty() {
            return Err(conflict("B attempt has contested or terminal evidence"));
        }
        let mark = Mark {
            scope: binding.mark_scope()?,
            sequence: history.marks.len() as i64 + 1,
            kind,
            request,
            task,
        };
        history.check_mark(&mark, predecessor)?;
        if mark.sequence == 1 {
            tx.execute(
                "INSERT INTO b_lifecycle_attempt VALUES (?1, ?2, ?3, ?4)",
                params![
                    binding.attempt,
                    binding.issuer,
                    binding.grant,
                    encode(binding)?
                ],
            )
            .map_err(sql)?;
        }
        tx.execute(
            "INSERT INTO b_lifecycle_mark VALUES (?1, ?2, ?3, ?4)",
            params![binding.attempt, mark.sequence, kind as i64, encode(&mark)?],
        )
        .map_err(sql)?;
        Ok(mark)
    }

    pub(super) fn confirm_b_mark(&mut self, binding: &Binding, mark: &Mark) -> Result<()> {
        let history = self.read_b_history(binding)?;
        if history.tip() != Some(mark) {
            return Err(integrity("B committed mark differs at readback"));
        }
        Ok(())
    }

    pub(crate) fn record_b_fact(
        &mut self,
        binding: &Binding,
        authenticated: AuthenticatedFact,
    ) -> Result<Recorded> {
        if authenticated.binding != *binding {
            return Err(conflict("B authenticated fact binds another spend/attempt"));
        }
        let tx = self.b_transaction(binding, true)?;
        let history = load(&tx, binding)?;
        let fact = authenticated.fact;
        history.check_fact(binding, &fact)?;
        let index = fact.slot.index();
        let slot = &history.facts[index];
        if slot.contains(&fact) {
            return Ok(Recorded::Duplicate);
        }
        if slot.len() == 2 {
            return Ok(Recorded::Contested);
        }
        tx.execute(
            "INSERT INTO b_lifecycle_fact VALUES (?1, ?2, ?3, ?4)",
            params![
                binding.attempt,
                index as i64 + 1,
                slot.len() as i64,
                encode(&fact)?
            ],
        )
        .map_err(sql)?;
        tx.commit().map_err(sql)?;
        if !self.read_b_history(binding)?.facts[index].contains(&fact) {
            return Err(integrity("B committed fact differs at readback"));
        }
        Ok(if slot.is_empty() {
            Recorded::Stored
        } else {
            Recorded::Contested
        })
    }

    /// Checked historical evidence only, including after loss/reopen. No mutation
    /// is allowed from a new Store epoch in this first offline cut.
    pub(crate) fn read_b_history(&mut self, binding: &Binding) -> Result<History> {
        let tx = self.b_transaction(binding, false)?;
        let history = load(&tx, binding)?;
        tx.commit().map_err(sql)?;
        Ok(history)
    }

    fn b_transaction(&mut self, binding: &Binding, write: bool) -> Result<Transaction<'_>> {
        binding.validate()?;
        self.verify_binding()?;
        super::verify_sidecars(&self.root)?;
        if binding.root != self.root_id
            || binding.profile != self.marker.profile
            || (write && binding.opening_epoch != self.epoch)
        {
            return Err(conflict(
                "B Store root, profile or live opening epoch differs",
            ));
        }
        let tx = self
            .conn
            .transaction_with_behavior(if write {
                TransactionBehavior::Immediate
            } else {
                TransactionBehavior::Deferred
            })
            .map_err(sql)?;
        check_spend(&tx, binding)?;
        Ok(tx)
    }
}

fn check_spend(tx: &Transaction<'_>, b: &Binding) -> Result<()> {
    // Require exactly one owner row for the *stable* issuer/grant, rather than
    // picking a matching row among conflicting rows/generations. An absent view
    // (not attached by the owner yet), no row, or a mismatch all fail closed.
    let mut stmt = tx
        .prepare(
            "SELECT root_id, store_profile, opening_epoch, attempt_id, spend_receipt, purpose,
                head_digest, selection_digest, recipe_digest, server_operation, serving_generation,
                evidence_ingress
           FROM task_free_b_committed_spend_v1 WHERE issuer_id=?1 AND grant_id=?2",
        )
        .map_err(sql)?;
    let mut rows = stmt.query(params![b.issuer, b.grant]).map_err(sql)?;
    let row = rows
        .next()
        .map_err(sql)?
        .ok_or_else(|| conflict("B committed spend absent"))?;
    for (i, expected) in [
        b.root.as_str(),
        &b.profile,
        &b.opening_epoch,
        &b.attempt,
        &b.spend_receipt,
        PURPOSE,
        &b.head.0,
        &b.canonical_selection.0,
        &b.recipe.0,
        &b.server_operation,
        &b.serving_generation.0,
        &b.evidence_ingress.0,
    ]
    .into_iter()
    .enumerate()
    {
        if row.get::<_, String>(i).map_err(sql)? != expected {
            return Err(conflict("B committed spend binding differs"));
        }
    }
    if rows.next().map_err(sql)?.is_some() {
        return Err(integrity("B committed spend is not unique"));
    }
    Ok(())
}

fn load(tx: &Transaction<'_>, binding: &Binding) -> Result<History> {
    // Read by attempt OR stable spend unit to catch aliases instead of returning
    // an empty history for a rebinding of the same consumed grant.
    let mut stmt = tx
        .prepare(
            "SELECT attempt_id, issuer_id, grant_id, binding_json FROM b_lifecycle_attempt
          WHERE attempt_id=?1 OR (issuer_id=?2 AND grant_id=?3)",
        )
        .map_err(sql)?;
    let mut rows = stmt
        .query(params![binding.attempt, binding.issuer, binding.grant])
        .map_err(sql)?;
    let mut history = History::default();
    let Some(row) = rows.next().map_err(sql)? else {
        return Ok(history);
    };
    if row.get::<_, String>(0).map_err(sql)? != binding.attempt
        || row.get::<_, String>(1).map_err(sql)? != binding.issuer
        || row.get::<_, String>(2).map_err(sql)? != binding.grant
        || decode::<Binding>(&row.get::<_, String>(3).map_err(sql)?)? != *binding
        || rows.next().map_err(sql)?.is_some()
    {
        return Err(integrity("B attempt binding is not immutable and exact"));
    }

    // Decode marks first; then typed facts in slot order to resolve the original
    // create lifetime. No qualification is inferred from well-shaped rows.
    let mut stmt = tx.prepare(
        "SELECT sequence, kind, mark_json FROM b_lifecycle_mark WHERE attempt_id=?1 ORDER BY sequence",
    ).map_err(sql)?;
    let mut rows = stmt.query([&binding.attempt]).map_err(sql)?;
    let scope = binding.mark_scope()?;
    while let Some(row) = rows.next().map_err(sql)? {
        let mark: Mark = decode(&row.get::<_, String>(2).map_err(sql)?)?;
        if mark.scope != scope
            || mark.sequence != row.get::<_, i64>(0).map_err(sql)?
            || mark.kind as i64 != row.get::<_, i64>(1).map_err(sql)?
        {
            return Err(integrity("B mark row identity differs"));
        }
        history.marks.push(mark);
    }
    if history.marks.is_empty() {
        return Err(integrity("B attempt missing its first mark"));
    }
    let mut stmt = tx.prepare(
        "SELECT slot, ordinal, fact_json FROM b_lifecycle_fact WHERE attempt_id=?1 ORDER BY slot, ordinal",
    ).map_err(sql)?;
    let mut rows = stmt.query([&binding.attempt]).map_err(sql)?;
    while let Some(row) = rows.next().map_err(sql)? {
        let fact: Fact = decode(&row.get::<_, String>(2).map_err(sql)?)?;
        let i = fact.slot.index();
        let slot = &history.facts[i];
        if i as i64 + 1 != row.get::<_, i64>(0).map_err(sql)?
            || slot.len() as i64 != row.get::<_, i64>(1).map_err(sql)?
            || slot.contains(&fact)
        {
            return Err(integrity("B fact row identity differs"));
        }
        history
            .check_fact(binding, &fact)
            .map_err(|_| integrity("B stored fact does not match its scope"))?;
        history.facts[i].push(fact);
    }
    let marks = std::mem::take(&mut history.marks);
    for mark in marks {
        history
            .check_mark(&mark, history.tip())
            .map_err(|_| integrity("B stored mark order or task binding differs"))?;
        history.marks.push(mark);
    }
    Ok(history)
}

fn validate_id(id: &str) -> Result<()> {
    if id.is_empty() || id.len() > 128 || !id.bytes().all(|b| b.is_ascii_graphic()) {
        return Err(contract("invalid B binding identifier"));
    }
    Ok(())
}

fn encode(value: &impl Serialize) -> Result<String> {
    serde_json::to_string(value).map_err(|e| CampaignError::Contract(e.to_string()))
}

fn decode<T: DeserializeOwned>(json: &str) -> Result<T> {
    if json.len() > 4096 {
        return Err(integrity("B record exceeds size limit"));
    }
    serde_json::from_str(json).map_err(|e| CampaignError::Integrity(e.to_string()))
}

fn conflict(message: &str) -> CampaignError {
    CampaignError::Conflict(message.into())
}
fn contract(message: &str) -> CampaignError {
    CampaignError::Contract(message.into())
}
fn integrity(message: &str) -> CampaignError {
    CampaignError::Integrity(message.into())
}
fn sql(error: rusqlite::Error) -> CampaignError {
    // A failed call, including an ambiguous commit or later readback, is never
    // a proof of rollback and never a signal to call the external operation.
    integrity(&format!("B lifecycle transaction not confirmed: {error}"))
}

#[cfg(test)]
mod tests;
