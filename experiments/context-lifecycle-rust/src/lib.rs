//! A fake-only context lifecycle kernel, not a production engine.
//!
//! Local ownership is moved, not copied:
//! ```compile_fail
//! use context_lifecycle_experiment::{Client, Owner, CommandId, TransitionId};
//! fn invalid(c: &Client, owner: Owner) {
//!     c.replace(CommandId::new("r"), owner, TransitionId::new("t"), u64::MAX);
//!     let stale = owner.stamp(); // use after move
//! }
//! ```
//! A capability cannot be fabricated:
//! ```compile_fail
//! use context_lifecycle_experiment::{Owner, Stamp};
//! fn fabricate(stamp: Stamp) -> Owner { Owner { stamp } }
//! ```
//! Identity types are not interchangeable:
//! ```compile_fail
//! use context_lifecycle_experiment::{InputId, EffectId};
//! let effect: EffectId = InputId::new("same bytes");
//! ```
//! Observers have no advancement API:
//! ```compile_fail
//! use context_lifecycle_experiment::{Observer, Command};
//! fn advance(observer: &Observer, command: Command) { observer.execute(command); }
//! ```
//! Capture grants no ordinary delivery capability:
//! ```compile_fail
//! use context_lifecycle_experiment::*;
//! fn misuse(client: &Client, capture: CapturePermit, authority: SyntheticAuthority) {
//!     client.admit(CommandId::new("a"), &capture, InputId::new("i"),
//!         DeliveryId::new("d"), EffectId::new("e"), authority);
//! }
//! ```

pub mod codex;
mod kernel;
pub mod process;
pub mod provider;
mod storage;
mod types;
pub mod vertical;
use kernel::Coordinator;
pub use kernel::{Clock, SystemClock};
pub use types::*;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::net::SocketAddr;

/// Local capability. Its fields and construction are private; it is not Clone.
pub struct Owner {
    stamp: Stamp,
}
impl Owner {
    pub fn stamp(&self) -> Stamp {
        self.stamp.clone()
    }
}

/// Trusted synthetic controller connection. This is not a security boundary.
#[derive(Clone)]
pub struct Client {
    address: SocketAddr,
}
impl Client {
    pub fn new(address: SocketAddr) -> Self {
        Self { address }
    }
    pub fn execute(&self, command: Command) -> Result<Reply> {
        process::rpc(self.address, &Request::Command(command))
    }
    pub fn observer(&self) -> Observer {
        Observer {
            address: self.address,
        }
    }
    pub fn admit(
        &self,
        key: CommandId,
        owner: &Owner,
        input: InputId,
        delivery: DeliveryId,
        effect: EffectId,
        authority: SyntheticAuthority,
    ) -> Result<EffectId> {
        match self.execute(Command {
            key,
            action: Action::Admit {
                owner: owner.stamp(),
                input,
                delivery,
                effect,
                authority,
            },
        })? {
            Reply::Ticket(effect) => Ok(effect),
            _ => bail!("unexpected admission reply"),
        }
    }
    pub fn register(&self, key: CommandId, context: ContextId) -> Result<Owner> {
        match self.execute(Command {
            key,
            action: Action::Register { context },
        })? {
            Reply::Owner(stamp) => Ok(Owner { stamp }),
            _ => bail!("unexpected register reply"),
        }
    }
    pub fn replace(
        &self,
        key: CommandId,
        owner: Owner,
        transition: TransitionId,
        deadline: u64,
    ) -> Result<CapturePermit> {
        match self.execute(Command {
            key,
            action: Action::Replace {
                owner: owner.stamp,
                transition,
                deadline,
            },
        })? {
            Reply::Capture(permit) => Ok(permit),
            _ => bail!("unexpected replace reply"),
        }
    }
    pub fn activate(&self, key: CommandId, attestation: Attestation) -> Result<Owner> {
        match self.execute(Command {
            key,
            action: Action::Activate { attestation },
        })? {
            Reply::Owner(stamp) => Ok(Owner { stamp }),
            _ => bail!("unexpected activate reply"),
        }
    }
    pub fn begin_readiness(
        &self,
        key: CommandId,
        permit: CapturePermit,
        effect: EffectId,
        delivery: DeliveryId,
        authority: SyntheticAuthority,
    ) -> Result<EffectId> {
        match self.execute(Command {
            key,
            action: Action::BeginReadiness {
                permit,
                effect,
                delivery,
                authority,
            },
        })? {
            Reply::Ticket(effect) => Ok(effect),
            _ => bail!("unexpected readiness reply"),
        }
    }
    pub fn activate_text(
        &self,
        key: CommandId,
        attestation: Attestation,
        limited_adequacy: String,
    ) -> Result<Owner> {
        match self.execute(Command {
            key,
            action: Action::ActivateText {
                attestation,
                limited_adequacy,
            },
        })? {
            Reply::Owner(stamp) => Ok(Owner { stamp }),
            _ => bail!("unexpected text activation reply"),
        }
    }
    /// Releases an explicitly configured experiment barrier, never grants delivery authority.
    pub fn release_barrier(&self) -> Result<()> {
        let _: Reply = process::rpc(self.address, &Request::ReleaseBarrier)?;
        Ok(())
    }
}

/// Read-only observation. Dropping it or a pending request changes no durable work.
#[derive(Clone)]
pub struct Observer {
    address: SocketAddr,
}
impl Observer {
    pub fn snapshot(&self) -> Result<Snapshot> {
        process::rpc(self.address, &Request::Snapshot)
    }
    pub fn bootstrap(&self, transition: TransitionId) -> Result<Bootstrap> {
        process::rpc(self.address, &Request::Bootstrap(transition))
    }
    pub fn readiness_package(&self, transition: TransitionId) -> Result<Bootstrap> {
        self.snapshot()?
            .transitions
            .into_iter()
            .find(|candidate| candidate.permit.transition == transition)
            .and_then(|candidate| candidate.readiness_package)
            .ok_or_else(|| anyhow::anyhow!("readiness package unavailable"))
    }
    pub fn result(&self, effect: EffectId) -> Result<Effect> {
        process::rpc(self.address, &Request::Result(effect))
    }
    /// A bounded wait in the server, serviced without owning execution responsibility.
    pub fn wait(&self, condition: Condition, timeout_ms: u64) -> Result<Snapshot> {
        process::rpc(
            self.address,
            &Request::Wait {
                condition,
                timeout_ms,
            },
        )
    }
}

/// Two synthetic model contexts can live in this one runtime process.
/// Loading is an exact-byte synthetic attestation, not proof of understanding.
pub struct SyntheticRuntime {
    id: RuntimeId,
    contexts: BTreeMap<(ContextId, Epoch), Option<Bootstrap>>,
}
impl SyntheticRuntime {
    pub fn new(id: RuntimeId, initial: &Owner) -> Self {
        Self {
            id,
            contexts: BTreeMap::from([(
                (initial.stamp.context.clone(), initial.stamp.epoch),
                None,
            )]),
        }
    }
    pub fn load(&mut self, package: Bootstrap) -> Attestation {
        let attestation = Attestation {
            context: package.context.clone(),
            transition: package.transition.clone(),
            successor: package.successor,
            revision: package.revision,
            digest: digest(&package),
            runtime: self.id.clone(),
            model_context: format!("{}:{}", package.context.0, package.successor.0),
        };
        self.contexts
            .insert((package.context.clone(), package.successor), Some(package));
        attestation
    }
    pub fn context_count(&self) -> usize {
        self.contexts.len()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Request {
    Command(Command),
    Snapshot,
    Bootstrap(TransitionId),
    Result(EffectId),
    Wait {
        condition: Condition,
        timeout_ms: u64,
    },
    ReleaseBarrier,
}
