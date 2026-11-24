use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use std::time::{SystemTime, UNIX_EPOCH};

use super::task_free_b::{self, Digest, Kind, Mark, PURPOSE, Target, joined_fixture};
use super::{Store, verify_sidecars};
use crate::application::offline_qualification::{
    AttemptBinding, FixtureCreateReadback, FixtureHeldReadbacks, FixtureRunning, HoldFailure,
    JoinedMarkReadback, OfflineLiveSubtree, OfflineStopSubject, QualificationHistoryKey,
};
use crate::{CampaignError, Result, codec::campaign_digest};
use serde_json::json;

const SCHEMA: &str = include_str!("../../migrations/offline_qualification_v1.sql");
const COMMITTED_SPEND: &str = "
    CREATE VIEW task_free_b_committed_spend_v1 AS
    SELECT q.issuer_id, s.grant_id, q.root_id, q.store_profile, q.opening_epoch, s.attempt_id,
           s.spend_receipt, q.purpose, q.head AS head_digest, q.binding_digest AS selection_digest,
           q.recipe_digest, q.server_operation, q.serving_generation, q.evidence_ingress
      FROM offline_qualification_spend s JOIN offline_qualification_selection q USING
           (grant_id, attempt_id, binding_digest)
     WHERE q.entered=1
";

fn store_error(error: rusqlite::Error) -> CampaignError {
    CampaignError::Integrity(format!(
        "offline qualification store not confirmed: {error}"
    ))
}

pub(crate) fn refused(reason: &str) -> CampaignError {
    CampaignError::Conflict(format!("offline qualification: {reason}"))
}

#[derive(Debug, PartialEq, Eq)]
struct State {
    entered: bool,
    revoked: bool,
    spent: bool,
}

fn schema_joined(conn: &Connection) -> Result<bool> {
    conn.query_row(
        "SELECT joined FROM offline_qualification_schema WHERE version=1",
        [],
        |row| row.get(0),
    )
    .map_err(store_error)
}

fn state(conn: &Connection, b: &AttemptBinding) -> Result<State> {
    schema_joined(conn)?;
    let (root, identity, head, attempt, digest, intent, expiry, entered, revoked, spend) = conn
        .query_row(
            "SELECT q.root_id,q.identity_key,q.head,q.attempt_id,q.binding_digest,q.intent_digest,
                    q.expires_at,q.entered,q.revoked,s.attempt_id,s.binding_digest,s.spend_receipt
             FROM offline_qualification_selection q
             LEFT JOIN offline_qualification_spend s ON s.grant_id=q.grant_id
             WHERE q.grant_id=?1",
            [b.grant_id()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, bool>(7)?,
                    row.get::<_, bool>(8)?,
                    (
                        row.get::<_, Option<String>>(9)?,
                        row.get::<_, Option<String>>(10)?,
                        row.get::<_, Option<String>>(11)?,
                    ),
                ))
            },
        )
        .optional()
        .map_err(store_error)?
        .ok_or_else(|| refused("selection absent"))?;
    if root != b.root_id()
        || identity != b.identity_key()
        || head != b.head().as_str()
        || attempt != b.attempt_id()
        || digest != b.binding_digest()
        || intent != b.intent_sha256()
        || expiry != b.expires_at()
    {
        return Err(refused("selection binding differs"));
    }
    let spent = match spend {
        (None, None, None) => false,
        (Some(attempt), Some(digest), Some(receipt))
            if entered
                && attempt == b.attempt_id()
                && digest == b.binding_digest()
                && receipt == b.lifecycle().spend_receipt =>
        {
            true
        }
        _ => return Err(refused("spent binding differs")),
    };
    Ok(State {
        entered,
        revoked,
        spent,
    })
}

#[cfg(test)]
thread_local! {
    pub(crate) static FIXTURE_NOW_MS: std::cell::Cell<Option<i64>> = const { std::cell::Cell::new(None) };
}

fn owner_now_ms() -> Result<i64> {
    #[cfg(test)]
    if let Some(now) = FIXTURE_NOW_MS.get() {
        return Ok(now);
    }
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|t| i64::try_from(t.as_millis()).ok())
        .ok_or_else(|| refused("owner clock unavailable"))
}

fn require_current(conn: &Connection, b: &AttemptBinding) -> Result<()> {
    let now = owner_now_ms()?;
    let head_matches = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM campaign_state WHERE identity_key=?1 AND revision=?2)",
            params![b.identity_key(), b.head().as_str()],
            |row| row.get::<_, bool>(0),
        )
        .map_err(store_error)?;
    if !head_matches || now >= b.expires_at() {
        return Err(refused("head changed or selection expired"));
    }
    Ok(())
}

const CURRENT_SPEND: State = State {
    entered: true,
    revoked: false,
    spent: true,
};

fn require_current_spend(conn: &Connection, b: &AttemptBinding) -> Result<()> {
    require_current(conn, b)?;
    if state(conn, b)? != CURRENT_SPEND {
        return Err(refused("exact current spend not available for send"));
    }
    Ok(())
}

impl Store {
    pub(crate) fn qualification_scope(&self) -> (String, String) {
        (self.marker.profile.clone(), self.epoch.clone())
    }

    fn verify_qualification_opening(&self, b: &AttemptBinding) -> Result<()> {
        self.verify_binding()?;
        if b.root_id() != self.root_id
            || b.lifecycle().opening_epoch != self.epoch
            || b.lifecycle().profile != self.marker.profile
        {
            return Err(refused("selection Store opening differs"));
        }
        Ok(())
    }

    pub(crate) fn attach_offline_qualification_fixture(&mut self, joined: bool) -> Result<()> {
        self.verify_binding()?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(store_error)?;
        tx.execute_batch(SCHEMA).map_err(store_error)?;
        tx.execute(
            "INSERT INTO offline_qualification_schema VALUES (1, ?1)",
            [joined],
        )
        .map_err(store_error)?;
        if joined {
            tx.execute_batch(task_free_b::MIGRATION_SQL)
                .map_err(store_error)?;
            tx.execute_batch(COMMITTED_SPEND).map_err(store_error)?;
        }
        tx.commit().map_err(store_error)?;
        self.verify_binding()?;
        if schema_joined(&self.conn)? != joined {
            return Err(refused("fixture schema readback differs"));
        }
        Ok(())
    }

    pub(crate) fn register_qualification(&mut self, b: &AttemptBinding) -> Result<()> {
        self.verify_qualification_opening(b)?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(store_error)?;
        schema_joined(&tx)?;
        require_current(&tx, b)?;
        let lifecycle = b.lifecycle();
        // No upsert: neither the same key nor its alternate binding can be reselected.
        tx.execute(
            "INSERT INTO offline_qualification_selection
             (grant_id,root_id,identity_key,head,attempt_id,binding_digest,expires_at,
              store_profile,opening_epoch,issuer_id,purpose,recipe_digest,server_operation,
              serving_generation,evidence_ingress,intent_digest)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
            params![
                b.grant_id(),
                b.root_id(),
                b.identity_key(),
                b.head().as_str(),
                b.attempt_id(),
                b.binding_digest(),
                b.expires_at(),
                lifecycle.profile,
                lifecycle.opening_epoch,
                lifecycle.issuer,
                PURPOSE,
                lifecycle.recipe.as_str(),
                lifecycle.server_operation,
                lifecycle.serving_generation.as_str(),
                lifecycle.evidence_ingress.as_str(),
                b.intent_sha256(),
            ],
        )
        .map_err(store_error)?;
        tx.commit().map_err(store_error)?;
        self.readback(
            b,
            State {
                entered: false,
                revoked: false,
                spent: false,
            },
        )
    }

    pub(crate) fn enter_qualification(&mut self, b: &AttemptBinding) -> Result<()> {
        self.verify_qualification_opening(b)?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(store_error)?;
        require_current(&tx, b)?;
        if state(&tx, b)?
            != (State {
                entered: false,
                revoked: false,
                spent: false,
            })
        {
            return Err(refused("entry unavailable"));
        }
        if tx.execute("UPDATE offline_qualification_selection SET entered=1 WHERE grant_id=?1 AND entered=0 AND revoked=0", [b.grant_id()]).map_err(store_error)? != 1 {
            return Err(refused("entry not applied"));
        }
        tx.commit().map_err(store_error)?;
        self.readback(
            b,
            State {
                entered: true,
                revoked: false,
                spent: false,
            },
        )
    }

    pub(crate) fn read_qualification_spend(
        &self,
        k: &QualificationHistoryKey,
        identity: &str,
    ) -> Result<bool> {
        self.verify_binding()?;
        verify_sidecars(&self.root)?;
        if k.root != self.root_id || k.profile != self.marker.profile {
            return Err(refused("historical selection is outside this owner"));
        }
        schema_joined(&self.conn)?;
        // Compare the stored original opening, never the current opening or head.
        let (entered, spent) = self
            .conn
            .query_row(
                "SELECT q.entered,s.attempt_id,s.binding_digest,s.spend_receipt
               FROM offline_qualification_selection q
               LEFT JOIN offline_qualification_spend s ON s.grant_id=q.grant_id
              WHERE q.grant_id=?1 AND q.root_id=?2 AND q.identity_key=?3 AND q.attempt_id=?4
                AND q.binding_digest=?5 AND q.store_profile=?6 AND q.opening_epoch=?7
                AND q.intent_digest=?8 AND q.purpose=?9",
                params![
                    k.grant,
                    k.root,
                    identity,
                    k.attempt,
                    k.selection,
                    k.profile,
                    k.original_opening,
                    k.intent,
                    PURPOSE
                ],
                |row| {
                    Ok((
                        row.get::<_, bool>(0)?,
                        (
                            row.get::<_, Option<String>>(1)?,
                            row.get::<_, Option<String>>(2)?,
                            row.get::<_, Option<String>>(3)?,
                        ),
                    ))
                },
            )
            .optional()
            .map_err(store_error)?
            .ok_or_else(|| refused("exact historical selection absent"))?;
        match spent {
            (None, None, None) => Ok(false),
            (Some(attempt), Some(selection), Some(receipt))
                if entered
                    && attempt == k.attempt
                    && selection == k.selection
                    && receipt
                        == campaign_digest(&json!([PURPOSE, "spend-receipt-v1", k.selection]))? =>
            {
                Ok(true)
            }
            _ => Err(refused("historical spend binding differs")),
        }
    }

    pub(crate) fn spend_qualification(&mut self, b: &AttemptBinding, confirm: bool) -> Result<()> {
        self.verify_qualification_opening(b)?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(store_error)?;
        require_current(&tx, b)?;
        if state(&tx, b)?
            != (State {
                entered: true,
                revoked: false,
                spent: false,
            })
        {
            return Err(refused("one-shot spend unavailable"));
        }
        tx.execute(
            "INSERT INTO offline_qualification_spend(grant_id,attempt_id,binding_digest,spend_receipt)
             VALUES (?1,?2,?3,?4)",
            params![b.grant_id(), b.attempt_id(), b.binding_digest(), b.lifecycle().spend_receipt],
        ).map_err(store_error)?;
        tx.commit().map_err(store_error)?;
        if !confirm {
            // Offline unit cut: even a durable commit without an acknowledgment
            // to the live winner must never issue a permit on later readback.
            return Err(refused("spend confirmation lost"));
        }
        self.readback(
            b,
            State {
                entered: true,
                revoked: false,
                spent: true,
            },
        )
    }

    pub(crate) fn revoke_qualification(&mut self, b: &AttemptBinding) -> Result<()> {
        self.verify_binding()?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(store_error)?;
        let prior = state(&tx, b)?;
        if !prior.revoked && tx.execute("UPDATE offline_qualification_selection SET revoked=1 WHERE grant_id=?1 AND revoked=0", [b.grant_id()]).map_err(store_error)? != 1 {
            return Err(refused("revoke not applied"));
        }
        tx.commit().map_err(store_error)?;
        self.readback(
            b,
            State {
                revoked: true,
                ..prior
            },
        )
    }

    pub(crate) fn mark_qualification_fixture(
        &mut self,
        b: &AttemptBinding,
        request: Digest,
        readback: JoinedMarkReadback,
    ) -> Result<Mark> {
        self.mark_joined_send(b, request, readback, None)
    }

    pub(crate) fn mark_qualification_start(
        &mut self,
        b: &AttemptBinding,
        create: &Mark,
        observed: &FixtureCreateReadback,
        request: Digest,
        readback: JoinedMarkReadback,
    ) -> Result<Mark> {
        self.mark_joined_send(b, request, readback, Some((create, observed)))
    }

    fn mark_joined_send(
        &mut self,
        b: &AttemptBinding,
        request: Digest,
        readback: JoinedMarkReadback,
        start_after: Option<(&Mark, &FixtureCreateReadback)>,
    ) -> Result<Mark> {
        self.verify_qualification_opening(b)?;
        verify_sidecars(&self.root)?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(store_error)?;
        if !schema_joined(&tx)? {
            return Err(refused("joined fixture absent"));
        }
        require_current_spend(&tx, b)?;
        // No callbacks here. B receives the same immediate transaction that
        // ordered currentness/revoke against this mark, not another grant.
        let (predecessor, kind, target) = match start_after {
            None => (None, Kind::Create, None),
            Some((mark, observation)) => {
                joined_fixture::require_created(&tx, b.lifecycle(), mark, observation)?;
                (
                    Some(mark),
                    Kind::Start,
                    Some(Target::Start(joined_fixture::object(observation)?)),
                )
            }
        };
        let mark = Self::write_b_mark(&tx, b.lifecycle(), predecessor, kind, request, target)?;
        tx.commit().map_err(store_error)?;
        if readback == JoinedMarkReadback::LostAfterCommit {
            return Err(refused("possible-issue confirmation lost"));
        }
        self.readback(b, CURRENT_SPEND)?;
        if let Some((create, observation)) = start_after {
            self.confirm_joined_start_mark(b.lifecycle(), &mark, create, observation)?;
        } else {
            self.confirm_b_mark(b.lifecycle(), &mark)?;
        }
        Ok(mark)
    }

    pub(crate) fn mark_offline_scoped_stop(
        &mut self,
        b: &AttemptBinding,
        live: &OfflineLiveSubtree,
        readback: JoinedMarkReadback,
    ) -> Result<OfflineStopSubject> {
        self.verify_qualification_opening(b)?;
        verify_sidecars(&self.root)?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(store_error)?;
        if !schema_joined(&tx)? {
            return Err(refused("joined fixture absent"));
        }
        require_current_spend(&tx, b)?;
        let expected = Self::write_offline_stop_mark(&tx, b, live)?;
        tx.commit().map_err(store_error)?;
        if readback == JoinedMarkReadback::LostAfterCommit {
            return Err(refused("scoped Stop possible-issue confirmation lost"));
        }
        self.readback(b, CURRENT_SPEND)?;
        if self.read_offline_scoped_stop(b, live)? != expected {
            return Err(refused("scoped Stop mark differs at readback"));
        }
        Ok(expected)
    }

    pub(crate) fn confirm_fixture_held(
        &mut self,
        b: &AttemptBinding,
        mark: &Mark,
        running: &FixtureRunning,
        evidence: &FixtureHeldReadbacks,
        capture_sha256: Option<&str>,
    ) -> std::result::Result<(), HoldFailure> {
        let uncertain = |_| HoldFailure::FactNotConfirmed;
        self.verify_qualification_opening(b).map_err(uncertain)?;
        verify_sidecars(&self.root).map_err(uncertain)?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| HoldFailure::FactNotConfirmed)?;
        if !schema_joined(&tx).map_err(uncertain)? {
            return Err(HoldFailure::FactNotConfirmed);
        }
        require_current_spend(&tx, b).map_err(|_| HoldFailure::NotCurrent)?;
        let fact = joined_fixture::write_held(&tx, b, mark, running, evidence, capture_sha256)?;
        tx.commit().map_err(|_| HoldFailure::FactNotConfirmed)?;
        joined_fixture::confirm_held(self, b, &fact, running)
    }

    pub(crate) fn mark_isolated_fixture<R>(
        &mut self,
        b: &AttemptBinding,
        inert_mark: impl FnOnce() -> Result<R>,
    ) -> Result<R> {
        self.verify_qualification_opening(b)?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(store_error)?;
        if schema_joined(&tx)? {
            return Err(refused("no simulated marker in joined fixture"));
        }
        require_current_spend(&tx, b)?;
        let mark = inert_mark()?;
        tx.commit().map_err(store_error)?;
        self.verify_binding()?;
        Ok(mark)
    }

    fn readback(&self, b: &AttemptBinding, expected: State) -> Result<()> {
        self.verify_binding()?;
        if state(&self.conn, b)? != expected {
            return Err(refused("commit readback differs"));
        }
        Ok(())
    }
}
