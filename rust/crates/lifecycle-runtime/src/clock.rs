use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use lifecycle_core::{AuthorityExpiresAt, ClockSample, WaitBudgetMs, WallTimeMs};
use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum ClockError {
    #[error("system wall time is outside the supported millisecond range")]
    WallRange,
    #[error("monotonic wait deadline is outside the process clock range")]
    WaitRange,
}

pub trait ClockSource: Send + Sync {
    fn wall_time(&self) -> Result<WallTimeMs, ClockError>;
    fn monotonic_now(&self) -> Instant;

    fn sample(&self, wait_budget: WaitBudgetMs) -> Result<ClockSample, ClockError> {
        Ok(ClockSample {
            wall: self.wall_time()?,
            wait_budget,
        })
    }
}

#[derive(Clone, Copy, Default)]
pub struct SystemClock;

impl ClockSource for SystemClock {
    fn wall_time(&self) -> Result<WallTimeMs, ClockError> {
        let since_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ClockError::WallRange)?;
        let millis = i64::try_from(since_epoch.as_millis()).map_err(|_| ClockError::WallRange)?;
        Ok(WallTimeMs::new(millis))
    }

    fn monotonic_now(&self) -> Instant {
        Instant::now()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeadlineStatus {
    OnTime,
    WallExpired,
    WaitExpired,
    BothExpired,
}

#[derive(Clone, Copy, Debug)]
pub struct MonotonicDeadline {
    cutoff: Instant,
}

impl MonotonicDeadline {
    pub fn new(start: Instant, budget: WaitBudgetMs) -> Result<Self, ClockError> {
        let cutoff = start
            .checked_add(Duration::from_millis(budget.get()))
            .ok_or(ClockError::WaitRange)?;
        Ok(Self { cutoff })
    }

    pub fn cutoff(self) -> Instant {
        self.cutoff
    }

    /// Recheck after every awaited external operation and before final success.
    pub fn status(
        self,
        now: Instant,
        wall: WallTimeMs,
        authority: AuthorityExpiresAt,
    ) -> DeadlineStatus {
        let wall_expired = !authority.allows_new_entry(ClockSample {
            wall,
            wait_budget: WaitBudgetMs::new(0),
        });
        let wait_expired = now >= self.cutoff;
        match (wall_expired, wait_expired) {
            (false, false) => DeadlineStatus::OnTime,
            (true, false) => DeadlineStatus::WallExpired,
            (false, true) => DeadlineStatus::WaitExpired,
            (true, true) => DeadlineStatus::BothExpired,
        }
    }

    pub fn remaining(self, now: Instant) -> Duration {
        self.cutoff.saturating_duration_since(now)
    }
}
