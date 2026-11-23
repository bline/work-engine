//! Public setup staging only. No Job, ingress, release or qualification.
use super::{LIMIT, OwnedChild, docker_command, exact_name_list, pin_is_hash, single_inspect};
use crate::{Artifact, Error, Result, STOP, file_digest_within, group_members, kill_group, refuse};
use serde_json::{Value, json};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read},
    os::{fd::AsRawFd, unix::fs::PermissionsExt},
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        atomic::Ordering,
        mpsc::{Receiver, RecvTimeoutError},
    },
    time::{Duration, Instant},
};

const INIT: &str = "/work-engine-hold-init";
const PLATFORM: &str = "linux/amd64";
const ATTEMPT: &str = "work-engine.public-hold-attempt";
const INIT_HASH: &str = "work-engine.public-hold-init-sha256";
const COMMAND_LIMIT: Duration = Duration::from_secs(3);
// Monotonic budget from immediately before submitting start, not from after
// inspect or from receipt of a signal. Leave room for a quick outside sample
// without spending the fixed init's 30-second self-limit on this wait.
const OBSERVATION_LIMIT: Duration = Duration::from_secs(5);
const OBSERVATION_CANCEL_SLICE: Duration = Duration::from_millis(25);

struct ObservationSignal {
    attempt: String,
    container_id: String,
    outcome: ObservationOutcome,
}

// No installed producer. These are scheduling signals, not authenticated evidence.
#[allow(dead_code)]
enum ObservationOutcome {
    Complete,
    Failed,
    EarlyExit,
}

fn wait_for_observation(
    signals: Option<&Receiver<ObservationSignal>>,
    attempt: &str,
    id: &str,
    deadline: Instant,
    now: &impl Fn() -> Instant,
    cancelled: &impl Fn() -> bool,
) -> &'static str {
    loop {
        if cancelled() {
            return "cancelled";
        }
        let remaining = deadline.saturating_duration_since(now());
        if remaining.is_zero() {
            return "deadline";
        }
        let Some(signals) = signals else {
            return "unavailable";
        };
        // No observer code or spawned helper on the cleanup path. Bound the
        // blocking operation itself so silence cannot hide cancellation.
        let signal = match signals.recv_timeout(remaining.min(OBSERVATION_CANCEL_SLICE)) {
            Err(RecvTimeoutError::Timeout) => continue,
            result => result,
        };
        if cancelled() {
            return "cancelled";
        }
        if now() >= deadline {
            return "deadline";
        }
        return match signal {
            Err(_) => "unavailable",
            Ok(signal) if signal.attempt != attempt || signal.container_id != id => "misbound",
            Ok(signal) => match signal.outcome {
                ObservationOutcome::Complete => "signal_complete",
                ObservationOutcome::Failed => "failed",
                ObservationOutcome::EarlyExit => "early_exit",
            },
        };
    }
}

/// Closed recipe facts, not a permit or an acquisition interface. Only a
/// future installed owner can supply these in a protected namespace. In
/// particular, client and image identity checks do not establish endpoint,
/// runtime or backing storage custody.
pub(super) struct Launch {
    pub(super) attempt: String,
    pub(super) fence_root: PathBuf,
    pub(super) docker: Artifact,
    /// Complete local image object ID, not a repository reference or tag.
    pub(super) image_id: String,
    /// Expected init-content identity for correlation, not measured task bytes.
    pub(super) init_sha256: String,
}

impl Launch {
    fn check(&self) -> Result<()> {
        if !pin_is_hash(&self.attempt)
            || !pin_is_hash(&self.init_sha256)
            || !self
                .image_id
                .strip_prefix("sha256:")
                .is_some_and(|id| pin_is_hash(id) && !id.bytes().any(|b| b.is_ascii_uppercase()))
            || !self.fence_root.is_absolute()
            || self.fence_root.canonicalize()? != self.fence_root
            || !self.fence_root.is_dir()
        {
            return refuse("invalid public hold recipe");
        }
        let pin = &self.docker;
        let safe_path = pin.path.to_str().is_some_and(|p| {
            p.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"/._-".contains(&b))
        });
        if !safe_path
            || !pin.path.is_absolute()
            || pin.path.canonicalize()? != pin.path
            || !pin_is_hash(&pin.sha256)
            || fs::metadata(&pin.path)?.permissions().mode() & 0o111 == 0
            || file_digest_within(&pin.path, None, 32 * LIMIT)? != pin.sha256
        {
            return refuse("public hold executable pin mismatch");
        }
        Ok(())
    }

    fn name(&self) -> String {
        format!("we-public-hold-{}", self.attempt)
    }

    fn reserve_once(&self) -> Result<()> {
        // This single durable bit is never erased, even after removal. It is
        // scoped to a private attempt namespace, not an effects/receipt journal.
        // If we recover with only this bit, cleanup is unknown, not replayable.
        let parent = File::open(&self.fence_root)?;
        let bit = match OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(self.fence_root.join(self.name()))
        {
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                return refuse("public hold attempt fenced; prior cleanup not established here");
            }
            other => other?,
        };
        bit.sync_all()?;
        parent.sync_all()?;
        Ok(())
    }

    fn inspect_image(&self, v: &Value) -> Result<()> {
        if v["Id"] != self.image_id
            || v["Os"] != "linux"
            || v["Architecture"] != "amd64"
            || !unset_or(&v["Variant"], "")
        {
            return refuse("public hold local image identity/platform conflict");
        }
        let config = &v["Config"];
        // Do not create if it could allocate an implicit volume, or if Docker
        // could inherit extra environment or startup configuration. The init
        // path and argument are supplied explicitly in create regardless.
        if !config.is_object()
            || !["Volumes", "ExposedPorts", "Labels"]
                .iter()
                .all(|k| no_map(&config[*k]))
            || !["Env", "Shell", "OnBuild"]
                .iter()
                .all(|k| no_list(&config[*k]))
            || !(no_list(&config["Entrypoint"]) || config["Entrypoint"] == json!([INIT]))
            || !(no_list(&config["Cmd"]) || config["Cmd"] == json!(["hold-only"]))
            || !config["Healthcheck"].is_null()
            || !config["StopTimeout"].is_null()
            || !unset_or(&config["StopSignal"], "")
            || !unset_or(&config["ArgsEscaped"], false)
            || !(unset_or(&config["WorkingDir"], "") || config["WorkingDir"] == "/")
            || !(unset_or(&config["User"], "") || config["User"] == "1000:1000")
        {
            return refuse("public hold local image configuration conflict");
        }
        Ok(())
    }

    fn create(&self) -> Vec<String> {
        strings(&[
            "create",
            "--pull=never",
            "--platform",
            PLATFORM,
            "--name",
            &self.name(),
            "--label",
            &format!("{ATTEMPT}={}", self.attempt),
            "--label",
            &format!("{INIT_HASH}={}", self.init_sha256),
            "--network=none",
            "--ipc=private",
            "--cgroupns=private",
            "--read-only",
            "--cap-drop=ALL",
            "--security-opt=no-new-privileges",
            "--pids-limit=16",
            "--memory=64m",
            "--cpus=1",
            "--user=1000:1000",
            "--workdir=/",
            "--no-healthcheck",
            "--restart=no",
            "--init=false",
            "--log-driver=none",
            "--entrypoint",
            INIT,
            &self.image_id,
            "hold-only",
        ])
    }

    fn inspect_identity<'a>(&self, v: &'a Value, known: Option<&str>) -> Result<&'a str> {
        let id = v["Id"].as_str().unwrap_or("");
        if !pin_is_hash(id)
            || known.is_some_and(|previous| previous != id)
            || v["Name"] != format!("/{}", self.name())
            || v["Image"] != self.image_id
            || v["Config"]["Labels"][ATTEMPT] != self.attempt
            || v["Config"]["Labels"][INIT_HASH] != self.init_sha256
        {
            return refuse("public hold container/attempt identity conflict");
        }
        Ok(id)
    }

    fn inspect_launch(&self, v: &Value) -> Result<()> {
        let config = &v["Config"];
        let host = &v["HostConfig"];
        if v["Path"] != INIT
            || v["Args"] != json!(["hold-only"])
            || config["Image"] != self.image_id
            || config["Labels"] != json!({(ATTEMPT): self.attempt, (INIT_HASH): self.init_sha256})
            || config["Entrypoint"] != json!([INIT])
            || config["Cmd"] != json!(["hold-only"])
            || config["Healthcheck"]["Test"] != json!(["NONE"])
            || config["User"] != "1000:1000"
            || config["WorkingDir"] != "/"
            || ["OpenStdin", "StdinOnce", "AttachStdin", "Tty"]
                .iter()
                .any(|k| config[*k] != false)
            // Checks configured env, not implicit engine defaults or the task's
            // runtime environment, which this inspect does not observe.
            || !no_list(&config["Env"])
            || !no_map(&config["Volumes"])
            || !no_map(&config["ExposedPorts"])
            || !config["StopTimeout"].is_null()
            || !unset_or(&config["StopSignal"], "")
            || !unset_or(&config["ArgsEscaped"], false)
            || host["NetworkMode"] != "none"
            || host["IpcMode"] != "private"
            || host["PidMode"] != ""
            || !unset_or(&host["UTSMode"], "")
            || !unset_or(&host["UsernsMode"], "")
            || host["CgroupnsMode"] != "private"
            || host["ReadonlyRootfs"] != true
            || host["Privileged"] != false
            || ![
                "CapAdd", "GroupAdd", "Devices", "DeviceRequests", "DeviceCgroupRules",
                "Binds", "Mounts", "VolumesFrom", "Links", "ExtraHosts",
            ]
            .iter()
            .all(|k| no_list(&host[*k]))
            || !["Tmpfs", "PortBindings", "Sysctls", "StorageOpt"]
                .iter()
                .all(|k| no_map(&host[*k]))
            || !unset_or(&host["PublishAllPorts"], false)
            || host["CapDrop"] != json!(["ALL"])
            || host["SecurityOpt"] != json!(["no-new-privileges"])
            || host["PidsLimit"] != 16
            || host["Memory"] != 67_108_864_u64
            || host["NanoCpus"] != 1_000_000_000_u64
            || host["Init"] != false
            || host["AutoRemove"] != false
            || host["RestartPolicy"]["Name"] != "no"
            || host["LogConfig"]["Type"] != "none"
        {
            return refuse("public hold launch/containment conflict");
        }
        if !v["Mounts"].as_array().is_some_and(Vec::is_empty) {
            return refuse("public hold requires empty container mounts");
        }
        Ok(())
    }
}

fn no_list(v: &Value) -> bool {
    v.is_null() || v.as_array().is_some_and(Vec::is_empty)
}

fn no_map(v: &Value) -> bool {
    v.is_null() || v.as_object().is_some_and(serde_json::Map::is_empty)
}

fn unset_or<T>(v: &Value, expected: T) -> bool
where
    Value: PartialEq<T>,
{
    v.is_null() || *v == expected
}

fn strings(args: &[&str]) -> Vec<String> {
    args.iter().map(|s| (*s).into()).collect()
}

pub(super) fn run(
    setup: Option<&Launch>,
    invoke: impl FnMut(&Path, &[String]) -> Result<(bool, Vec<u8>)>,
) -> Result<Value> {
    // The installed observer (including how its signals are acquired and
    // isolated) is not selected here. A missing reader must not delay cleanup.
    run_attempt(setup, invoke, None, Instant::now, || {
        STOP.load(Ordering::SeqCst)
    })
}

fn run_attempt(
    setup: Option<&Launch>,
    mut invoke: impl FnMut(&Path, &[String]) -> Result<(bool, Vec<u8>)>,
    signals: Option<&Receiver<ObservationSignal>>,
    now: impl Fn() -> Instant,
    cancelled: impl Fn() -> bool,
) -> Result<Value> {
    let setup = setup.ok_or_else(|| {
        Error::Refused("public inference setup has no installed launch selection".into())
    })?;
    let not_cancelled = || {
        if cancelled() {
            return refuse("public hold cancelled before advancing");
        }
        Ok(())
    };
    setup.check()?;
    not_cancelled()?;
    setup.reserve_once()?;
    not_cancelled()?;

    let name = setup.name();
    let mut call = |args: Vec<String>| invoke(&setup.docker.path, &args);
    let preflight = (|| -> Result<()> {
        let (ok, bytes) = call(strings(&["image", "inspect", &setup.image_id]))?;
        if !ok {
            return refuse("public hold local image inspect unavailable");
        }
        setup.inspect_image(&single_inspect(&bytes)?)?;
        not_cancelled()
    })();
    let create_attempted = preflight.is_ok();
    let mut known = None;
    let launch = preflight.and_then(|()| {
        let (ok, bytes) = call(setup.create())?;
        if !ok {
            return refuse("public hold create uncertain");
        }
        let id = std::str::from_utf8(&bytes).unwrap_or("").trim();
        if !pin_is_hash(id) {
            return refuse("public hold create returned invalid identity");
        }
        known = Some(id.to_owned());
        let container = inspected(call(strings(&["container", "inspect", id]))?)?;
        setup.inspect_identity(&container, Some(id))?;
        setup.inspect_launch(&container)?;
        if container["State"]["Status"] != "created" || container["State"]["Running"] != false {
            return refuse("public hold container was not freshly created");
        }
        not_cancelled()?;
        let deadline = now() + OBSERVATION_LIMIT;
        let (ok, bytes) = call(strings(&["start", id]))?;
        if !ok || std::str::from_utf8(&bytes).unwrap_or("").trim() != id {
            return refuse("public hold init-only start uncertain");
        }
        let container = inspected(call(strings(&["container", "inspect", id]))?)?;
        setup.inspect_identity(&container, Some(id))?;
        setup.inspect_launch(&container)?;
        not_cancelled()?;
        if container["State"]["Status"] != "running" || container["State"]["Running"] != true {
            return refuse("public hold running state not established by Docker inspect");
        }
        Ok(wait_for_observation(
            signals,
            &setup.attempt,
            id,
            deadline,
            &now,
            &cancelled,
        ))
    });

    // The attempt is permanently fenced. No remaining source path can
    // create/start. Clean up by the checked object ID, never by a name
    // substituted for a known ID. Docker can itself fall back to name lookup
    // after an ID miss; the CLI argument is not a daemon replacement fence.
    // None of the following readbacks independently settles daemon work.
    let target = create_attempted.then(|| -> Result<String> {
        let target = known.as_deref().unwrap_or(&name);
        let container = inspected(call(strings(&["container", "inspect", target]))?)?;
        let id = setup.inspect_identity(&container, known.as_deref())?;
        if known.is_none() {
            // A name after uncertain allocation alone is not cleanup custody.
            setup.inspect_launch(&container)?;
        }
        Ok(id.to_owned())
    });
    let remove = if let Some(Ok(id)) = &target {
        Some(call(strings(&["rm", "--force", id])).map(|(ok, _)| ok))
    } else {
        None
    };
    let listed = matches!(remove, Some(Ok(true))).then(|| {
        let (ok, bytes) = call(exact_name_list(&name))?;
        if !ok {
            return refuse("public hold Docker exact-name query exited unsuccessfully");
        }
        Ok(bytes.iter().all(u8::is_ascii_whitespace))
    });
    Ok(json!({
        "attempt": setup.attempt,
        "disposition": "hold_stop_only",
        "create_attempted": create_attempted,
        "hold_inspected": launch.is_ok(),
        "observation_window": launch.as_ref().copied().unwrap_or("not_opened"),
        "launch_error": launch.err().map(|e| e.to_string()),
        "fenced": true,
        "cleanup_target_error": target.and_then(Result::err).map(|e| e.to_string()),
        "docker_remove_cli_success": remove.as_ref().and_then(|r| r.as_ref().ok()),
        "docker_remove_cli_error": remove.and_then(Result::err).map(|e| e.to_string()),
        "docker_exact_name_list_empty": listed.as_ref().and_then(|r| r.as_ref().ok()),
        "docker_exact_name_list_error": listed.and_then(Result::err).map(|e| e.to_string()),
    }))
}

fn inspected((ok, bytes): (bool, Vec<u8>)) -> Result<Value> {
    if !ok {
        return refuse("public hold Docker inspect unavailable; cleanup/identity unknown");
    }
    single_inspect(&bytes)
}

/// Job-free, noninteractive use of the Host's owned Docker command machinery.
/// Killing a timed-out CLI gives no daemon settlement; the caller must fence.
pub(super) fn docker(program: &Path, args: &[String]) -> Result<(bool, Vec<u8>)> {
    let mut child = OwnedChild(
        docker_command(program, args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?,
    );
    let output = child.stdout.take().expect("Docker stdout piped");
    // Only the parent read end is nonblocking; the child's writes remain normal.
    if unsafe { libc::fcntl(output.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK) } == -1 {
        return Err(io::Error::last_os_error().into());
    }
    let mut output = output.take(LIMIT + 1);
    let mut bytes = Vec::new();
    let deadline = Instant::now() + COMMAND_LIMIT;
    loop {
        let drained = match output.read_to_end(&mut bytes) {
            Ok(_) => true,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => false,
            Err(e) => return Err(e.into()),
        };
        if output.limit() == 0 || Instant::now() >= deadline {
            return refuse("public hold Docker command exceeded its time/output bound");
        }
        if let Some(status) = child.try_wait()? {
            if !group_members(child.id()).is_empty() {
                kill_group(child.id());
                return refuse("public hold Docker command left owned processes");
            }
            // The CLI may have exited between the preceding read and try_wait.
            if !drained {
                output.read_to_end(&mut bytes)?;
                if output.limit() == 0 {
                    return refuse("public hold Docker command exceeded its output bound");
                }
            }
            return Ok((status.success(), bytes));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(test)]
#[path = "pi_public_hold_tests.rs"]
mod tests;
