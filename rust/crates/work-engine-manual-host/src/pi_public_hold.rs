//! Public setup staging only. No Job, ingress, release or qualification.
use super::{
    IMAGE, LIMIT, OwnedChild, docker_command, exact_name_list, pin_is_hash, single_inspect,
};
use crate::{Artifact, Error, Result, STOP, file_digest_within, group_members, kill_group, refuse};
use serde_json::{Value, json};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read},
    os::{fd::AsRawFd, unix::fs::PermissionsExt},
    path::{Path, PathBuf},
    process::Stdio,
    sync::atomic::Ordering,
    time::{Duration, Instant},
};

const INIT: &str = "/work-engine-hold-init";
const ATTEMPT: &str = "work-engine.public-hold-attempt";
const INIT_HASH: &str = "work-engine.public-hold-init-sha256";
const COMMAND_LIMIT: Duration = Duration::from_secs(3);

/// Closed recipe facts, not a permit or an acquisition interface. Only a
/// future installed owner can supply these in a protected namespace. In
/// particular, digest reads below do not prove install custody or eliminate
/// replacement of daemon-visible paths by an actor with equivalent access.
pub(super) struct Launch {
    pub(super) attempt: String,
    pub(super) fence_root: PathBuf,
    pub(super) docker: Artifact,
    pub(super) init: Artifact,
    /// Local image object ID corresponding to the fixed repository digest.
    pub(super) image_id: String,
}

impl Launch {
    fn check(&self) -> Result<()> {
        if !pin_is_hash(&self.attempt)
            || !self
                .image_id
                .strip_prefix("sha256:")
                .is_some_and(pin_is_hash)
            || !self.fence_root.is_absolute()
            || self.fence_root.canonicalize()? != self.fence_root
            || !self.fence_root.is_dir()
        {
            return refuse("invalid public hold recipe");
        }
        for pin in [&self.docker, &self.init] {
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
        }
        if self.init.path == self.docker.path {
            return refuse("public hold init cannot be the Docker executable");
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

    fn source(&self) -> &str {
        self.init.path.to_str().expect("checked launch path")
    }

    fn create(&self) -> Vec<String> {
        strings(&[
            "create",
            "--pull=never",
            "--name",
            &self.name(),
            "--label",
            &format!("{ATTEMPT}={}", self.attempt),
            "--label",
            &format!("{INIT_HASH}={}", self.init.sha256),
            "--network=none",
            "--ipc=private",
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
            "--mount",
            &format!(
                "type=bind,src={},dst={INIT},readonly,bind-propagation=rprivate",
                self.source()
            ),
            "--entrypoint",
            INIT,
            IMAGE,
            "hold-only",
        ])
    }

    fn inspect_identity<'a>(&self, v: &'a Value, known: Option<&str>) -> Result<&'a str> {
        let id = v["Id"].as_str().unwrap_or("");
        if !pin_is_hash(id)
            || known.is_some_and(|previous| previous != id)
            || v["Name"] != format!("/{}", self.name())
            || v["Config"]["Labels"][ATTEMPT] != self.attempt
            || v["Config"]["Labels"][INIT_HASH] != self.init.sha256
        {
            return refuse("public hold container/attempt identity conflict");
        }
        Ok(id)
    }

    fn inspect_launch(&self, v: &Value) -> Result<()> {
        let config = &v["Config"];
        let host = &v["HostConfig"];
        let empty = |v: &Value| v.is_null() || v.as_array().is_some_and(Vec::is_empty);
        if v["Image"] != self.image_id
            || v["Path"] != INIT
            || v["Args"] != json!(["hold-only"])
            || config["Image"] != IMAGE
            || config["Entrypoint"] != json!([INIT])
            || config["Cmd"] != json!(["hold-only"])
            || config["Healthcheck"]["Test"] != json!(["NONE"])
            || config["User"] != "1000:1000"
            || config["WorkingDir"] != "/"
            || ["OpenStdin", "StdinOnce", "AttachStdin", "Tty"]
                .iter()
                .any(|k| config[*k] != false)
            || !config["Env"].as_array().is_some_and(|env| {
                env.iter().all(|e| {
                    // Only the fixed pinned Node image's inert build metadata and
                    // baseline environment may remain; no loader/provider input.
                    e.as_str()
                        .and_then(|s| s.split_once('='))
                        .is_some_and(|(key, _)| {
                            matches!(
                                key,
                                "PATH" | "NODE_VERSION" | "YARN_VERSION" | "HOME" | "HOSTNAME"
                            )
                        })
                })
            })
            || host["NetworkMode"] != "none"
            || host["IpcMode"] != "private"
            || host["PidMode"] != ""
            || host["ReadonlyRootfs"] != true
            || host["Privileged"] != false
            || !empty(&host["CapAdd"])
            || !empty(&host["GroupAdd"])
            || !empty(&host["Devices"])
            || !empty(&host["DeviceRequests"])
            || !empty(&host["DeviceCgroupRules"])
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
        let Some([mount]) = v["Mounts"].as_array().map(Vec::as_slice) else {
            return refuse("public hold requires exactly its read-only init bind");
        };
        if mount["Source"] != self.source()
            || mount["Destination"] != INIT
            || mount["RW"] != false
            || mount["Propagation"] != "rprivate"
            || mount["Type"] != "bind"
        {
            return refuse("public hold init bind conflict");
        }
        Ok(())
    }
}

fn strings(args: &[&str]) -> Vec<String> {
    args.iter().map(|s| (*s).into()).collect()
}

pub(super) fn run(
    setup: Option<&Launch>,
    mut invoke: impl FnMut(&Path, &[String]) -> Result<(bool, Vec<u8>)>,
) -> Result<Value> {
    let setup = setup.ok_or_else(|| {
        Error::Refused("public inference setup has no installed launch selection".into())
    })?;
    setup.check()?;
    not_cancelled()?;
    setup.reserve_once()?;
    not_cancelled()?;

    let name = setup.name();
    let mut call = |args: Vec<String>| invoke(&setup.docker.path, &args);
    let mut known = None;
    let launch = (|| -> Result<()> {
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
        Ok(())
    })();

    // The attempt is permanently fenced. The remaining source path cannot
    // create/start, even after an uncertain start, timeout, or cancellation.
    // Fence daemon work by removing only the exact immutable object ID. A
    // delayed start for the removed ID cannot switch to a reused name.
    let cleanup = (|| -> Result<()> {
        let target = known.as_deref().unwrap_or(&name);
        let container = inspected(call(strings(&["container", "inspect", target]))?)?;
        let id = setup.inspect_identity(&container, known.as_deref())?;
        if known.is_none() {
            // A name after uncertain allocation alone is not cleanup custody.
            setup.inspect_launch(&container)?;
        }
        let (ok, _) = call(strings(&["rm", "--force", id]))?;
        if !ok {
            return refuse("public hold force removal uncertain");
        }
        let (ok, bytes) = call(exact_name_list(&name))?;
        if !ok || !bytes.iter().all(u8::is_ascii_whitespace) {
            return refuse("public hold container absence not established");
        }
        Ok(())
    })();
    Ok(json!({
        "attempt": setup.attempt,
        "disposition": "hold_stop_only",
        "hold_inspected": launch.is_ok(),
        "launch_error": launch.err().map(|e| e.to_string()),
        "fenced": true,
        "cleanup_confirmed": cleanup.is_ok(),
        "cleanup_error": cleanup.err().map(|e| e.to_string()),
    }))
}

fn not_cancelled() -> Result<()> {
    if STOP.load(Ordering::SeqCst) {
        return refuse("public hold cancelled before advancing");
    }
    Ok(())
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
