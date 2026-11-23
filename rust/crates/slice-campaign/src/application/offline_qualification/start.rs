//! Typed data for the explicit offline-only fixture. The roles below are
//! synthetic substitutes for separate serving/runtime, kernel, and listener
//! custody. None of these inputs, even a complete set, is installed evidence.

use serde::Serialize;
use serde_json::json;

use super::{AttemptBinding, JoinedMarkReadback, QualificationPort, check_owner};
use crate::codec::campaign_digest;
use crate::store::offline_qualification::refused;
use crate::store::task_free_b::{Digest, Mark};
use crate::{Result, require_sha};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FixtureObject {
    pub full_id: String,
    pub serving_generation: String,
}

/// Selected serving object's independent synthetic readback, not the Create reply.
#[derive(Debug, Serialize)]
pub struct FixtureCreateReadback {
    pub request_digest: String,
    pub object: FixtureObject,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JoinedStartRequest {
    pub create_request_digest: String,
    pub create_mark_digest: String,
    pub target: FixtureObject,
    pub request_digest: String,
}

/// Actual task and exec generation seen by the inert Start-side source. Task
/// ID (even a full one) alone does not anchor the kernel lifetime or exec.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FixtureRunning {
    pub start_request_digest: String,
    pub object: FixtureObject,
    pub task_id: String,
    pub lifetime: String,
    pub exec_generation: String,
}

/// Synthetic runtime/kernel executable readback. Both objects are observed
/// at this execution, rather than inferred from the requested recipe.
#[derive(Debug, Serialize)]
pub struct FixtureExecReadback {
    pub running: FixtureRunning,
    pub executable_sha256: String,
    pub startup_closure_sha256: String,
}

#[derive(Debug, Serialize)]
pub enum FixtureCheckpoint {
    HeldBeforeWorkload,
    Lost,
    NotHeld,
}

/// Protected-listener surrogate: accepted peer identity is a separate reading
/// from the init's checkpoint. A checkpoint alone cannot establish its sender.
#[derive(Debug, Serialize)]
pub struct FixtureReadyPeer {
    pub listener: String,
    pub peer: FixtureRunning,
    pub checkpoint: FixtureCheckpoint,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FixtureCredentials {
    pub uids: [u32; 4],
    pub gids: [u32; 4],
    pub supplementary_groups: Vec<u32>,
    pub capability_masks: [u64; 5],
    pub no_new_privileges: bool,
    pub dumpable: bool,
    pub umask: u16,
}

/// Digests stand for bound *effective* inventories and applied profiles, read
/// independently of the init and of the requested config. In this fixture
/// each inventory stands for one complete, canonical, synthetic observation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FixtureIsolation {
    pub namespaces_mounts_cgroup: String,
    pub network_and_devices: String,
    pub seccomp_and_lsm: String,
    pub inherited_handles_and_control_paths: String,
}

#[derive(Debug, Serialize)]
pub struct FixtureEffectiveReadback {
    pub running: FixtureRunning,
    pub credentials: Option<FixtureCredentials>,
    pub isolation: Option<FixtureIsolation>,
}

#[derive(Debug, Serialize)]
pub enum FixtureConfinement {
    Denied,
    Missing,
    Observed(FixtureEffectiveReadback),
}

#[derive(Debug, Serialize)]
pub struct FixtureHeldReadbacks {
    pub exec: Option<FixtureExecReadback>,
    pub ready: Option<FixtureReadyPeer>,
    pub confinement: FixtureConfinement,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FixtureInitPolicy {
    pub executable_sha256: String,
    pub startup_closure_sha256: String,
    pub credentials: FixtureCredentials,
    pub isolation: FixtureIsolation,
}

/// One fixed owner-side offline policy; the collector cannot select another
/// or treat one of its readbacks as the approved source of truth.
pub fn fixture_init_policy() -> FixtureInitPolicy {
    FixtureInitPolicy {
        executable_sha256: "1".repeat(64),
        startup_closure_sha256: "2".repeat(64),
        credentials: FixtureCredentials {
            uids: [65534; 4],
            gids: [65534; 4],
            supplementary_groups: vec![],
            capability_masks: [0; 5],
            no_new_privileges: true,
            dumpable: false,
            umask: 0o077,
        },
        isolation: FixtureIsolation {
            namespaces_mounts_cgroup: "3".repeat(64),
            network_and_devices: "4".repeat(64),
            seccomp_and_lsm: "5".repeat(64),
            inherited_handles_and_control_paths: "6".repeat(64),
        },
    }
}

pub(super) fn fixture_ingress_digest() -> Result<String> {
    campaign_digest(&json!([
        "offline-only-joined-fixture-ingress-v1",
        "serving-object-and-operation-surrogate",
        "runtime-kernel-executable-surrogate",
        "protected-readiness-listener-surrogate",
        "independent-effective-state-surrogate",
        fixture_init_policy()
    ]))
}

pub fn fixture_listener_id(binding: &AttemptBinding, start_request: &str) -> Result<String> {
    campaign_digest(&json!([
        "offline-protected-listener-v1",
        binding.binding_digest(),
        start_request
    ]))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HoldFailure {
    NotCurrent,
    MissingExec,
    WrongExec,
    LostReadiness,
    WrongPeer,
    ConfinementDenied,
    ConfinementMissing,
    ConfinementMismatch,
    FactNotConfirmed,
}

impl FixtureHeldReadbacks {
    pub(crate) fn check(
        &self,
        binding: &AttemptBinding,
        running: &FixtureRunning,
    ) -> std::result::Result<(), HoldFailure> {
        use HoldFailure::*;
        let policy = fixture_init_policy();
        let exec = self.exec.as_ref().ok_or(MissingExec)?;
        if &exec.running != running
            || exec.executable_sha256 != policy.executable_sha256
            || exec.startup_closure_sha256 != policy.startup_closure_sha256
        {
            return Err(WrongExec);
        }
        let ready = self.ready.as_ref().ok_or(LostReadiness)?;
        if &ready.peer != running
            || fixture_listener_id(binding, &running.start_request_digest)
                .as_ref()
                .map_err(|_| WrongPeer)?
                != &ready.listener
        {
            return Err(WrongPeer);
        }
        if !matches!(ready.checkpoint, FixtureCheckpoint::HeldBeforeWorkload) {
            return Err(LostReadiness);
        }
        let effective = match &self.confinement {
            FixtureConfinement::Denied => return Err(ConfinementDenied),
            FixtureConfinement::Missing => return Err(ConfinementMissing),
            FixtureConfinement::Observed(effective) => effective,
        };
        if &effective.running != running {
            return Err(ConfinementMismatch);
        }
        let credentials = effective.credentials.as_ref().ok_or(ConfinementMissing)?;
        let isolation = effective.isolation.as_ref().ok_or(ConfinementMissing)?;
        if credentials != &policy.credentials || isolation != &policy.isolation {
            return Err(ConfinementMismatch);
        }
        Ok(())
    }
}

/// Continuation of the original mark readback only; no entry point from disk.
pub struct MarkedCreate {
    pub(super) port: QualificationPort,
    pub(super) binding: AttemptBinding,
    pub(super) mark: Mark,
    pub(super) request: String,
}

impl MarkedCreate {
    pub fn confirm_created(
        self,
        replied_request: &str,
        replied_id: &str,
        readback: FixtureCreateReadback,
    ) -> Result<FreshStart> {
        if self.request != replied_request
            || readback.request_digest != self.request
            || readback.object.full_id != replied_id
            || readback.object.serving_generation != self.binding.serving_sha256()
        {
            return Err(refused("Create reply and serving-object readback differ"));
        }
        require_sha(replied_id, "fixture Create full object ID")?;
        {
            let mut owner = self.port.owner()?;
            check_owner(&owner, &self.binding)?;
            owner
                .store
                .confirm_fixture_created(&self.binding, &self.mark, &readback)?;
        }
        let creator = self.mark.digest()?;
        let request_digest = campaign_digest(&json!([
            "b-offline-operation",
            1,
            2,
            "start",
            self.binding.binding_digest(),
            self.request,
            creator,
            readback.object
        ]))?;
        Ok(FreshStart {
            port: self.port,
            binding: self.binding,
            create_mark: self.mark,
            request: JoinedStartRequest {
                create_request_digest: self.request,
                create_mark_digest: creator,
                target: readback.object.clone(),
                request_digest,
            },
            created: readback,
        })
    }
}

pub struct FreshStart {
    port: QualificationPort,
    binding: AttemptBinding,
    create_mark: Mark,
    request: JoinedStartRequest,
    created: FixtureCreateReadback,
}

impl FreshStart {
    pub fn request(&self) -> &JoinedStartRequest {
        &self.request
    }

    pub fn mark(self, readback: JoinedMarkReadback) -> Result<MarkedStart> {
        let request = Digest::new(self.request.request_digest.clone())?;
        let mark = {
            let mut owner = self.port.owner()?;
            check_owner(&owner, &self.binding)?;
            owner.store.mark_qualification_start(
                &self.binding,
                &self.create_mark,
                &self.created,
                request,
                readback,
            )?
        };
        Ok(MarkedStart {
            port: self.port,
            binding: self.binding,
            request: self.request,
            mark,
        })
    }
}

/// Only records this Start disposition. No further send method or release.
pub struct MarkedStart {
    port: QualificationPort,
    binding: AttemptBinding,
    request: JoinedStartRequest,
    mark: Mark,
}

impl MarkedStart {
    pub fn confirm_running(self, running: FixtureRunning) -> Result<ConfirmedRunning> {
        if self.request.request_digest != running.start_request_digest
            || self.request.target != running.object
        {
            return Err(refused(
                "running task does not bind the exact Start and Create object",
            ));
        }
        {
            let mut owner = self.port.owner()?;
            check_owner(&owner, &self.binding)?;
            owner
                .store
                .confirm_fixture_running(&self.binding, &self.mark, &running)?;
        }
        Ok(ConfirmedRunning {
            port: self.port,
            binding: self.binding,
            mark: self.mark,
            running,
        })
    }
}

pub struct ConfirmedRunning {
    port: QualificationPort,
    binding: AttemptBinding,
    mark: Mark,
    running: FixtureRunning,
}

impl ConfirmedRunning {
    pub fn verify_held(self, inputs: FixtureHeldReadbacks) -> std::result::Result<(), HoldFailure> {
        self.verify_held_fact(&inputs, None)
    }

    /// Bind the checked Host fixture capture at the same live owner gate as
    /// Held. The capture digest is descriptive evidence, never an issuer.
    pub fn verify_init_only_held(
        self,
        port: &QualificationPort,
        binding: &AttemptBinding,
        running: &FixtureRunning,
        inputs: &FixtureHeldReadbacks,
        capture_sha256: &str,
    ) -> std::result::Result<(), HoldFailure> {
        if !std::sync::Arc::ptr_eq(&self.port.inner, &port.inner)
            || self.binding != *binding
            || self.running != *running
        {
            return Err(HoldFailure::NotCurrent);
        }
        self.verify_held_fact(inputs, Some(capture_sha256))
    }

    fn verify_held_fact(
        self,
        inputs: &FixtureHeldReadbacks,
        capture_sha256: Option<&str>,
    ) -> std::result::Result<(), HoldFailure> {
        let mut owner = self
            .port
            .owner()
            .map_err(|_| HoldFailure::FactNotConfirmed)?;
        check_owner(&owner, &self.binding).map_err(|_| HoldFailure::NotCurrent)?;
        owner.store.confirm_fixture_held(
            &self.binding,
            &self.mark,
            &self.running,
            inputs,
            capture_sha256,
        )
    }
}
