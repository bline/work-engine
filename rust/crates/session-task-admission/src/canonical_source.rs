//! Exact source continuity across this owner's acknowledged canonical writes.
//! This capability carries source provenance, never human/action authority.
use crate::source;
pub use crate::source::{Pin, Program};
use serde_json::Value;
use std::{ffi::OsStr, time::Duration};
type Result<T> = std::result::Result<T, &'static str>;

/// Not deserializable. Restart cannot silently enroll a changed source.
pub struct CanonicalSource {
    #[cfg(test)]
    pub(crate) check_elapsed: std::cell::Cell<Option<time::Duration>>,
    #[cfg(test)]
    pub(crate) before_check_delay: std::cell::Cell<Option<std::time::Duration>>,
    initial: Pin,
    current: Pin,
    authority: String,
    scope: Program,
    ledger: Program,
}
pub struct Envelope<'a> {
    pub operation: &'a str,
    pub actor: &'a str,
    pub task: &'a str,
    pub parent: &'a str,
    pub parent_sha256: &'a str,
    pub maximum: &'a str,
    pub reserve: &'a str,
    pub request: &'a str,
    pub proposal: &'a str,
    pub mode: &'a str,
    pub profile: &'a str,
    pub native: &'a str,
}
impl CanonicalSource {
    pub fn new(scope: Program, ledger: Program, initial: Pin) -> Result<Self> {
        if !source::pinned(&initial)
            || std::fs::canonicalize(&initial.path).ok().as_ref() != Some(&initial.path)
            || scope.python != ledger.python
            || !scope.dependencies.contains(&ledger.script)
        {
            return Err("canonical_source_pin");
        }
        let args = [
            OsStr::new("envelope-source"),
            OsStr::new("--record"),
            initial.path.as_os_str(),
            OsStr::new("--record-sha256"),
            OsStr::new(&initial.sha256),
        ];
        let raw = source::run_bounded(&scope, &args, Duration::from_secs(40))
            .ok_or("canonical_source_unavailable")?;
        let receipt: Value =
            serde_json::from_slice(&raw).map_err(|_| "canonical_source_receipt")?;
        let authority = receipt["authority_sha256"]
            .as_str()
            .ok_or("canonical_source_receipt")?;
        if receipt["schema"] != "canonical-envelope-source/v1"
            || receipt["path"] != initial.path.to_str().ok_or("canonical_source_pin")?
            || receipt["before_sha256"] != initial.sha256
            || receipt["after_sha256"] != initial.sha256
            || authority.len() != 64
            || !source::pinned(&initial)
        {
            return Err("canonical_source_receipt");
        }
        Ok(Self {
            #[cfg(test)]
            before_check_delay: std::cell::Cell::new(None),
            #[cfg(test)]
            check_elapsed: std::cell::Cell::new(None),
            current: initial.clone(),
            initial,
            authority: authority.into(),
            scope,
            ledger,
        })
    }
    pub fn initial(&self) -> &Pin {
        &self.initial
    }
    pub fn current_sha256(&self) -> &str {
        &self.current.sha256
    }
    pub fn authority_sha256(&self) -> &str {
        &self.authority
    }
    pub fn check_current(&self) -> Result<()> {
        #[cfg(test)]
        if let Some(delay) = self.before_check_delay.take() {
            std::thread::sleep(delay);
        }
        if !source::pinned(&self.current)
            || !source::pinned(&self.ledger.python)
            || !source::pinned(&self.ledger.script)
            || !source::pinned(&self.scope.python)
            || !source::pinned(&self.scope.script)
            || !self.scope.dependencies.iter().all(source::pinned)
            || !self.ledger.dependencies.iter().all(source::pinned)
        {
            return Err("canonical_source_pin");
        }
        Ok(())
    }
    pub(crate) fn current(&self) -> &Pin {
        &self.current
    }
    pub(crate) fn matches(&self, initial: &Pin, scope: &Program, ledger: &Program) -> bool {
        self.initial == *initial && self.scope == *scope && self.ledger == *ledger
    }
    pub fn transition(&mut self, action: &str, b: &Envelope<'_>) -> Result<Value> {
        if !["check", "reserve", "fence", "recheck-fenced"].contains(&action) {
            return Err("canonical_action");
        }
        self.check_current()?;
        let pairs = [
            (
                "--record",
                self.current.path.to_str().ok_or("canonical_source_pin")?,
            ),
            ("--source-contract", "canonical-envelope-source/v1"),
            ("--record-sha256", &self.current.sha256),
            ("--authority-sha256", &self.authority),
            ("--action", action),
            ("--operation-id", b.operation),
            ("--actor", b.actor),
            ("--task-sha256", b.task),
            ("--parent-operation-id", b.parent),
            ("--parent-reservation-sha256", b.parent_sha256),
            ("--maximum-usd", b.maximum),
            ("--reserve-usd", b.reserve),
            ("--request-sha256", b.request),
            ("--proposal-sha256", b.proposal),
            ("--mode-sha256", b.mode),
            ("--profile-sha256", b.profile),
            ("--native-binding-sha256", b.native),
            ("--ledger-sha256", &self.ledger.script.sha256),
        ];
        let mut args = vec![OsStr::new("envelope")];
        for (k, v) in &pairs {
            args.push(OsStr::new(k));
            args.push(OsStr::new(v));
        }
        let raw = source::run_bounded(&self.scope, &args, Duration::from_secs(40))
            .ok_or("canonical_held_or_reply_unknown")?;
        let receipt: Value = serde_json::from_slice(&raw).map_err(|_| "canonical_reply_unknown")?;
        let s = &receipt["source"];
        let after = s["after_sha256"]
            .as_str()
            .ok_or("canonical_source_receipt")?;
        let next = Pin {
            path: self.current.path.clone(),
            sha256: after.into(),
        };
        if s["schema"] != "canonical-envelope-source/v1"
            || s["path"] != pairs[0].1
            || s["before_sha256"] != self.current.sha256
            || s["authority_sha256"] != self.authority
            || !source::pinned(&next)
        {
            return Err("canonical_source_receipt");
        }
        if action == "recheck-fenced"
            && (after != self.current.sha256
                || receipt["readonly"] != true
                || receipt["effect_state"] != "effect_unknown"
                || receipt["authority_status"] != "not_established")
        {
            return Err("canonical_readonly_receipt");
        }
        // Lost/invalid ACK: retain the old pin and durable unknown exposure.
        self.current = next;
        self.check_current()?;
        Ok(receipt)
    }
}
