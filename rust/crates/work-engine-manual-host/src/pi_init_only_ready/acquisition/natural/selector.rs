//! The opt-in operator boundary for one already-selected Campaign attempt.
//! This is not enrolled in Server and cannot issue a grant, resume or replay.

use super::super::{
    AppliedCoverage, AppliedWitness, HeldCapture, Phase, ScopeWitness, StartFence, StopExchange,
    StopFence, StopResolution,
};
use super::{
    AbortApproval, AbortSubject, Attach, Attempt, AttemptBinding, Endpoint, EndpointIdentity,
    ExecutableNode, Identity, OfflineContainment, Path, ProtectedSource, QualificationPort,
    Recipe, Result, SelectedTask, Serving, StartRpc, TaskKey, Verdict,
    boot_key, campaign_digest, fs, full_hex, invalid, json, lifetime, refuse, run_natural, within,
};
use serde::Deserialize;
use slice_campaign::offline_qualification::{OfflineStopSubject, QualificationPurpose};
use std::{
    fs::{File, OpenOptions},
    io::{self, Read},
    os::{
        fd::BorrowedFd,
        unix::fs::{MetadataExt, OpenOptionsExt},
    },
    time::Instant,
};

const CAPTURES: &str =
    "/var/lib/work-engine/contract-b-init-qualification-v4/natural-exit/captures";
const DOCKER: &str = "/run/docker.sock";
const TASKS: &str = "/run/containerd/containerd.sock";
const EXEC: &str = "/usr/local/libexec/work-engine-b-init-qualify-v4";
const MAX_ABORT: u64 = 16 * 1024;

/// Describes the approved installed subject; decoding does not grant authority.
/// The owner must independently commission the exact serving and confinement
/// boundary, and select these digests in Campaign for one NaturalExit binding.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NaturalExitSelection {
    controller_sha256: String,
    docker: EndpointIdentity,
    docker_boot: [u8; 16],
    docker_ticks: [u8; 8],
    docker_sha256: String,
    natural: Identity,
}

fn stock_endpoint(identity: &EndpointIdentity, path: &str) -> Result<Endpoint> {
    if identity.path_bytes != path.as_bytes()
        || identity.inode_be == [0; 8]
        || identity.listener_pid < 1
        || identity.listener_uid != 0
    {
        return refuse("natural selector requested a non-stock endpoint");
    }
    Ok(Endpoint {
        path: path.into(),
        device: u64::from_be_bytes(identity.device_be),
        inode: u64::from_be_bytes(identity.inode_be),
        listener_pid: identity.listener_pid,
        listener_uid: identity.listener_uid,
        listener_gid: identity.listener_gid,
    })
}

impl NaturalExitSelection {
    fn endpoints(&self) -> Result<(Endpoint, Endpoint)> {
        let docker = stock_endpoint(&self.docker, DOCKER)?;
        let tasks = stock_endpoint(&self.natural.tasks, TASKS)?;
        self.natural.runtime.check(&tasks)?;
        if self.natural.docker_pid == tasks.listener_pid {
            return refuse("Docker process and containerd listener overlap");
        }
        Ok((docker, tasks))
    }

    fn serving(&self) -> Result<Serving> {
        self.endpoints()?;
        let runtime_generation = campaign_digest(&json!([
            "b-natural-stock-runtime-generation-v1",
            "aad11006b869517fcd3009450b6f82da282e1a9b",
            &self.natural.tasks,
            self.natural.runtime.start_ticks.to_be_bytes(),
            &self.natural.runtime.artifact_sha256,
            &self.natural.oci,
        ]))
        .map_err(invalid)?;
        Ok(Serving {
            boot: self.docker_boot,
            docker_socket_inode: u64::from_be_bytes(self.docker.inode_be),
            daemon_process_start_ticks: u64::from_be_bytes(self.docker_ticks),
            daemon_artifact_sha256: self.docker_sha256.clone(),
            moby_semantics_revision: super::super::super::MOBY.into(),
            runtime_generation,
            api: super::super::super::API.into(),
        })
    }

    /// Owner readback for grant selection, not an RPC or an installation test.
    pub fn serving_sha256(&self) -> Result<String> {
        self.serving()?.digest().map_err(invalid)
    }

    /// Bind the running controller/reader, exact sockets, runtime pins and
    /// authority report, capture-directory filesystem identity, and budgets.
    pub fn intent_sha256(&self) -> Result<String> {
        self.intent_at(Path::new(CAPTURES))
    }

    fn intent_at(&self, root: &Path) -> Result<String> {
        let (docker, tasks) = self.endpoints()?;
        let n = &self.natural;
        let attempt = Attempt::new(
            &docker,
            n.docker_pid,
            &n.authority_sha256,
            &tasks,
            &n.runtime,
            &n.oci,
            &n.curl_sha256,
            n.trusted_gid0,
        )?;
        attempt
            .identity()
            .intent(root, &self.docker, &self.controller_sha256)
    }

    /// Only the protected operator executable may invoke this private entry.
    /// The type carries no caller-provided reader or kernel/effect adapter. The
    /// protected Campaign binding and normal spend/mark checks remain required.
    pub fn run(
        self,
        port: &QualificationPort,
        binding: &AttemptBinding,
        recipe: &Recipe,
        manifest: Vec<u8>,
        image_config: Vec<u8>,
    ) -> Result<Verdict> {
        if binding.purpose() != QualificationPurpose::NaturalExit {
            return refuse("private natural exit requires a NaturalExit Campaign binding");
        }
        let (docker, tasks) = self.endpoints()?;
        let serving = self.serving()?;
        let n = &self.natural;
        let mut source = StockSource {
            digest: &self.controller_sha256,
            serving,
            docker: &docker,
            docker_pid: n.docker_pid,
            root: Path::new(CAPTURES),
        };
        run_natural(
            port,
            binding,
            Path::new(CAPTURES),
            &docker,
            n.docker_pid,
            &n.authority_sha256,
            &tasks,
            &n.runtime,
            &n.oci,
            &n.curl_sha256,
            n.trusted_gid0,
            recipe,
            manifest,
            image_config,
            &mut source,
        )
    }
}

struct StockSource<'a> {
    digest: &'a str,
    serving: Serving,
    docker: &'a Endpoint,
    docker_pid: i32,
    root: &'a Path,
}

fn controller(digest: &str, deadline: Instant) -> Result<()> {
    // SAFETY: geteuid has no preconditions.
    if unsafe { libc::geteuid() } != 0 || !full_hex(digest) {
        return refuse("natural exit requires the commissioned administrator executable");
    }
    let path = Path::new(EXEC);
    for dir in path.ancestors().skip(1) {
        super::authority::directory(dir)?;
    }
    let loaded = File::open("/proc/self/exe")?;
    let installed = fs::symlink_metadata(path)?;
    let node = ExecutableNode::observe(&loaded.metadata()?)?;
    if installed.nlink() != 1
        || installed.mode() & 0o7100 != 0o0100
        || node != ExecutableNode::observe(&installed)?
    {
        return refuse("running controller is not the commissioned installed executable");
    }
    node.check_hash(&loaded, digest, deadline)?;
    if node != ExecutableNode::observe(&fs::symlink_metadata(path)?)? {
        return refuse("installed controller changed during its identity readback");
    }
    within(deadline)
}

// Start/Held come from the committed natural path's own adapter, not this
// trait's legacy seam. None of these separate one-Stop reads can succeed.
enum Inapplicable {}
impl StartFence for Inapplicable {
    fn confirm_cut(&mut self, _: &HeldCapture, _: Instant, _: Instant) -> Result<AppliedCoverage> {
        match *self {}
    }
}
impl StopFence for Inapplicable {
    fn confirm_end(&mut self, _: (Instant, Instant), _: Instant) -> Result<OfflineContainment> {
        match *self {}
    }
}

impl ProtectedSource for StockSource<'_> {
    type StartFence = Inapplicable;
    type StopFence = Inapplicable;

    fn identity(&self) -> &str {
        self.digest
    }

    fn preflight(&mut self, _: &AttemptBinding, _: &Endpoint, deadline: Instant) -> Result<()> {
        controller(self.digest, deadline)
    }

    fn serving(&mut self, endpoint: &Endpoint, deadline: Instant) -> Result<Serving> {
        endpoint.check_path()?;
        let boot = fs::read_to_string("/proc/sys/kernel/random/boot_id")?;
        let pid = u32::try_from(self.docker_pid).map_err(invalid)?;
        if EndpointIdentity::from(endpoint) != EndpointIdentity::from(self.docker)
            || boot.trim().replace('-', "") != boot_key(&self.serving.boot)
            || lifetime(pid)?.0 != self.serving.daemon_process_start_ticks
        {
            return refuse("original stock Docker serving coordinates differ from the selection");
        }
        // The natural adapter additionally retains the exact loaded Docker
        // and containerd pidfds/artifact pins, and authenticates wire bytes.
        within(deadline)?;
        Ok(self.serving.clone())
    }

    fn fence_start(
        &mut self,
        _: &StartRpc,
        _: BorrowedFd<'_>,
        _: BorrowedFd<'_>,
        _: Instant,
    ) -> Result<(Attach, Inapplicable)> {
        refuse("inherited Start source is not selected")
    }
    fn selected(&mut self, _: &StartRpc, _: Instant) -> Result<SelectedTask> {
        refuse("inherited task source is not selected")
    }
    fn applied(
        &mut self,
        _: &StartRpc,
        _: &Attach,
        _: &TaskKey,
        _: Instant,
    ) -> Result<AppliedWitness> {
        refuse("inherited Applied source is not selected")
    }
    fn fence_stop(
        &mut self,
        _: &StopExchange,
        _: BorrowedFd<'_>,
        _: Instant,
    ) -> Result<(StopResolution, Inapplicable)> {
        refuse("Stop is not part of natural exit")
    }
    fn settlement(
        &mut self,
        _: &OfflineStopSubject,
        _: &TaskKey,
        _: Phase,
        _: Instant,
    ) -> Result<ScopeWitness> {
        refuse("Stop settlement is not part of natural exit")
    }

    fn natural_abort_approval(
        &mut self,
        subject: &AbortSubject,
        deadline: Instant,
    ) -> Result<Option<AbortApproval>> {
        // Kept outside the numbered attempt archive, in the same protected
        // root. The adjacent file conveys no launch authority. An emergency
        // requires an administrator-authored exact decision while the old
        // pidfd is still retained; a deadline alone supplies no approval.
        within(deadline)?;
        super::super::custody::check_root(self.root)?;
        if !full_hex(&subject.selection) {
            return refuse("invalid emergency subject");
        }
        let path = self.root.join(format!("{}.abort.json", subject.selection));
        let file = match OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(path)
        {
            Ok(file) => file,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let m = file.metadata()?;
        if !m.is_file()
            || m.uid() != 0
            || m.nlink() != 1
            || m.mode() & 0o7177 != 0
            || m.len() == 0
            || m.len() > MAX_ABORT
        {
            return refuse("unprotected emergency decision");
        }
        let mut raw = String::new();
        file.take(MAX_ABORT + 1).read_to_string(&mut raw)?;
        within(deadline)?;
        parse_abort(raw, subject).map(Some)
    }
}

fn parse_abort(raw: String, subject: &AbortSubject) -> Result<AbortApproval> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Decision {
        action: String,
        subject: serde_json::Value,
        reason: String,
    }
    if raw.len() as u64 > MAX_ABORT {
        return refuse("oversize emergency decision");
    }
    let decision: Decision = serde_json::from_str(&raw)?;
    if decision.action != "abort-exact-original-by-pidfd"
        || decision.subject != serde_json::to_value(subject)?
        || decision.reason.trim().is_empty()
    {
        return refuse("emergency decision does not select this original attempt");
    }
    Ok(AbortApproval {
        subject: subject.clone(),
        operator_approval_sha256: crate::digest(raw.as_bytes()),
        raw_operator_decision: raw,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selected() -> NaturalExitSelection {
        let endpoint = |path: &str, inode: u64, pid| EndpointIdentity {
            path_bytes: path.as_bytes().into(),
            device_be: 6_u64.to_be_bytes(),
            inode_be: inode.to_be_bytes(),
            listener_pid: pid,
            listener_uid: 0,
            listener_gid: 0,
        };
        NaturalExitSelection {
            controller_sha256: "a".repeat(64),
            docker: endpoint(DOCKER, (1 << 54) + 1, 1),
            docker_boot: [3; 16],
            docker_ticks: 344_u64.to_be_bytes(),
            docker_sha256: "b".repeat(64),
            natural: Identity {
                docker_pid: 9,
                tasks: endpoint(TASKS, (1 << 54) + 1, 14),
                runtime: super::super::ContainerdPin {
                    artifact_sha256: "c".repeat(64),
                    start_ticks: 72,
                },
                oci: super::super::OciPin {
                    shim_sha256: "d".repeat(64),
                    runc_sha256: "e".repeat(64),
                },
                curl_sha256: "f".repeat(64),
                authority_sha256: "0".repeat(64),
                trusted_gid0: true,
            },
        }
    }

    #[test]
    fn natural_selector_binds_runtime_controller_scope_and_archive_without_daemon_contact() {
        use std::os::unix::fs::PermissionsExt;
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        for path in [first.path(), second.path()] {
            fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let mut s = selected();
        let intent = s.intent_at(first.path()).unwrap();
        let serving = s.serving_sha256().unwrap();
        assert_ne!(s.intent_at(second.path()).unwrap(), intent);
        s.controller_sha256 = "f".repeat(64);
        assert_ne!(s.intent_at(first.path()).unwrap(), intent);
        s.controller_sha256 = "a".repeat(64);
        s.natural.authority_sha256 = "f".repeat(64);
        assert_ne!(s.intent_at(first.path()).unwrap(), intent);
        s.natural.authority_sha256 = "0".repeat(64);
        s.natural.tasks.inode_be = ((1 << 54) + 2_u64).to_be_bytes();
        assert_ne!(s.intent_at(first.path()).unwrap(), intent);
        assert_ne!(s.serving_sha256().unwrap(), serving);
        s.natural.tasks.path_bytes = b"/tmp/containerd.sock".to_vec();
        assert!(s.intent_at(first.path()).is_err());
        assert!(s.serving_sha256().is_err());
    }

    #[test]
    fn natural_selector_accepts_only_explicit_emergency_for_exact_original() {
        let serving = selected().serving().unwrap();
        let task = super::super::super::super::tests::task(&serving, &"a".repeat(64));
        let subject = AbortSubject {
            selection: "b".repeat(64),
            start_arm: 22,
            original: task,
        };
        let mut decision = json!({
            "action": "abort-exact-original-by-pidfd", "subject": subject, "reason": "test authorization",
        });
        assert!(parse_abort(decision.to_string(), &subject).is_ok());
        decision["subject"]["start_arm"] = json!(23);
        assert!(parse_abort(decision.to_string(), &subject).is_err());
        decision["subject"]["start_arm"] = json!(22);
        decision["action"] = json!("retry");
        assert!(parse_abort(decision.to_string(), &subject).is_err());
    }
}
