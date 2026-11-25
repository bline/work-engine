//! One commissioned admission scope, not a new grant or a claim inferred
//! from socket permissions. Only the trusted administrator can commission it;
//! Campaign selects the exact report for one grant. The controller checks
//! the live coordinates it can read, archives the operator's actual readout,
//! and relies on the reviewed installation boundary for delegated ingress
//! and the trusted principals' no-out-of-band commitment.

use super::super::EndpointIdentity;
use super::{AttemptBinding, Endpoint, Path, Result, Serving, boot_key, full_hex, invalid, refuse};
use serde::{Deserialize, Serialize};
use std::{
    ffi::CString,
    fs::{self, OpenOptions},
    io::{self, Read},
    os::unix::{
        ffi::OsStrExt,
        fs::{MetadataExt, OpenOptionsExt},
    },
};

const FILE_NAME: &str = "natural-exit-authority.json";
const MAX_REPORT: u64 = 32 * 1024;

#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct Node {
    dev_be: [u8; 8],
    ino_be: [u8; 8],
    uid: u32,
    gid: u32,
    mode: u32,
}

impl From<&fs::Metadata> for Node {
    fn from(m: &fs::Metadata) -> Self {
        Self {
            dev_be: m.dev().to_be_bytes(),
            ino_be: m.ino().to_be_bytes(),
            uid: m.uid(),
            gid: m.gid(),
            mode: m.mode(),
        }
    }
}

fn no_acl(path: &Path, name: &std::ffi::CStr) -> Result<()> {
    let path = CString::new(path.as_os_str().as_bytes()).map_err(invalid)?;
    // SAFETY: both strings are null terminated. A null buffer with size
    // zero queries existence/size, not the attribute contents.
    let n = unsafe { libc::lgetxattr(path.as_ptr(), name.as_ptr(), std::ptr::null_mut(), 0) };
    if n < 0 && io::Error::last_os_error().raw_os_error() == Some(libc::ENODATA) {
        Ok(())
    } else {
        refuse("scope path ACL exists or its absence is unverified")
    }
}

pub(super) fn directory(path: &Path) -> Result<Node> {
    let m = fs::symlink_metadata(path)?;
    if !m.is_dir() || m.uid() != 0 || m.mode() & 0o022 != 0 {
        return refuse("natural scope ancestry is not owned and protected by root");
    }
    no_acl(path, c"system.posix_acl_access")?;
    Ok(Node::from(&m))
}

#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct SocketAccess {
    // Order: socket, immediate parent, ... filesystem root. Inodes are
    // lossless. Compare the socket ctime as well, not parent ctimes (which
    // unrelated work, such as creation of an unrelated file, can change).
    nodes: Vec<Node>,
    ctime_be: [u8; 8],
    ctime_nsec: i64,
}

impl SocketAccess {
    fn observe(endpoint: &Endpoint, gid: Option<u32>) -> Result<Self> {
        endpoint.check_path()?;
        let path = &endpoint.path;
        let m = fs::symlink_metadata(path)?;
        if (m.dev(), m.ino(), m.uid(), m.mode() & 0o7777)
            != (endpoint.device, endpoint.inode, 0, 0o660)
            || gid.is_some_and(|g| m.gid() != g)
        {
            return refuse("stock natural endpoint differs from its protected access boundary");
        }
        no_acl(path, c"system.posix_acl_access")?;
        let mut nodes = vec![Node::from(&m)];
        for path in path.ancestors().skip(1) {
            nodes.push(directory(path)?);
        }
        let after = fs::symlink_metadata(path)?;
        if Node::from(&after) != nodes[0]
            || (after.ctime(), after.ctime_nsec()) != (m.ctime(), m.ctime_nsec())
        {
            return refuse("scope socket changed during access readback");
        }
        Ok(Self {
            nodes,
            ctime_be: m.ctime().to_be_bytes(),
            ctime_nsec: m.ctime_nsec(),
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Commissioning {
    selection: ScopeDecision,
    grant: String,
    boot: String,
    serving_sha256: String,
    docker_endpoint: EndpointIdentity,
    tasks_endpoint: EndpointIdentity,
    docker_access: SocketAccess,
    tasks_access: SocketAccess,
    // The actual reviewed administrator readout, not request booleans.
    // A digest establishes correspondence and custody, not the meaning of
    // these observations. Do not select a report with unresolved reachability.
    serving_listeners_and_activation: String,
    effective_principals_and_credentials: String,
    inherited_and_delegated_ingress: String,
    protected_artifacts_policy_and_subtree_authority: String,
}

#[derive(Deserialize)]
enum ScopeDecision {
    NaturalExitOnlyNoOutOfBandForThisGrant,
}

impl Commissioning {
    fn for_selection(
        raw: &str,
        grant: &str,
        serving_sha256: &str,
        boot: &str,
        docker: &Endpoint,
        tasks: &Endpoint,
    ) -> Result<Self> {
        let approved: Self = serde_json::from_str(raw)?;
        let ScopeDecision::NaturalExitOnlyNoOutOfBandForThisGrant = approved.selection;
        if approved.grant != grant
            || approved.serving_sha256 != serving_sha256
            || approved.boot != boot
            || approved.docker_endpoint != EndpointIdentity::from(docker)
            || approved.tasks_endpoint != EndpointIdentity::from(tasks)
            || [
                &approved.serving_listeners_and_activation,
                &approved.effective_principals_and_credentials,
                &approved.inherited_and_delegated_ingress,
                &approved.protected_artifacts_policy_and_subtree_authority,
            ]
            .into_iter()
            .any(|s| s.trim().is_empty())
        {
            return refuse("operator scope is incomplete or crossed from a different selection");
        }
        Ok(approved)
    }
}

pub(super) struct Scope {
    approved: Commissioning,
    raw: String,
}

impl Scope {
    pub(super) fn load(
        root: &Path,
        approval_sha: &str,
        binding: &AttemptBinding,
        docker: &Endpoint,
        tasks: &Endpoint,
        serving: &Serving,
    ) -> Result<Self> {
        // SAFETY: geteuid has no preconditions.
        if unsafe { libc::geteuid() } != 0
            || !full_hex(approval_sha)
            || root != fs::canonicalize(root)?
        {
            return refuse("missing or unprotected natural-exit authority selection");
        }
        for dir in root.ancestors() {
            directory(dir)?;
        }
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(root.join(FILE_NAME))?;
        let m = file.metadata()?;
        if !m.is_file()
            || m.uid() != 0
            || m.nlink() != 1
            || m.mode() & 0o7177 != 0
            || m.len() == 0
            || m.len() > MAX_REPORT
        {
            return refuse("operator scope is not in protected single-file custody");
        }
        let mut raw = String::new();
        file.take(MAX_REPORT + 1).read_to_string(&mut raw)?;
        if raw.len() as u64 > MAX_REPORT || crate::digest(raw.as_bytes()) != approval_sha {
            return refuse("operator scope does not match the Campaign-selected digest");
        }
        let approved = Commissioning::for_selection(
            &raw,
            binding.grant_id(),
            binding.serving_sha256(),
            &boot_key(&serving.boot),
            docker,
            tasks,
        )?;
        let scope = Self { approved, raw };
        scope.check(docker, tasks)?;
        Ok(scope)
    }

    pub(super) fn readout(&self) -> &str {
        &self.raw
    }

    pub(super) fn check(&self, docker: &Endpoint, tasks: &Endpoint) -> Result<()> {
        let boot = fs::read_to_string("/proc/sys/kernel/random/boot_id")?;
        if boot.trim().replace('-', "") != self.approved.boot
            || SocketAccess::observe(docker, None)? != self.approved.docker_access
            || SocketAccess::observe(tasks, Some(0))? != self.approved.tasks_access
        {
            return Err(invalid(
                "commissioned boot or stock endpoint access has changed",
            ));
        }
        Ok(())
    }
}

#[test]
fn natural_scope_rejects_crossed_boot_endpoints_grant_and_incomplete_report() {
    use serde_json::json;

    let sock = |name: &str, inode: u64, pid| Endpoint {
        path: name.into(),
        device: 8,
        inode,
        listener_pid: pid,
        listener_uid: 0,
        listener_gid: 0,
    };
    let docker = sock("/run/docker.sock", (1 << 54) + 1, 1);
    let tasks = sock("/run/containerd/containerd.sock", (1 << 54) + 3, 11);
    let crossed = sock("/run/containerd/containerd.sock", (1 << 54) + 4, 11);
    let boot = "a".repeat(32);
    let serving = "b".repeat(64);
    let grant = "c".repeat(64);
    // This fixture exercises the join, not production custody or actual DAC.
    let ctime = [0u8; 8];
    let mock_access = json!({"nodes":[],"ctime_be":ctime,"ctime_nsec":0});
    let mut report = json!({
        "selection":"NaturalExitOnlyNoOutOfBandForThisGrant", "grant":grant,
        "boot":boot,"serving_sha256":serving,
        "docker_endpoint":EndpointIdentity::from(&docker), "tasks_endpoint":EndpointIdentity::from(&tasks),
        "docker_access":mock_access,"tasks_access":mock_access,
        "serving_listeners_and_activation":"local test fixture",
        "effective_principals_and_credentials":"local test fixture",
        "inherited_and_delegated_ingress":"local test fixture",
        "protected_artifacts_policy_and_subtree_authority":"local test fixture",
    });
    let check = |value: &serde_json::Value, g: &str, b: &str, s: &str, t: &Endpoint| {
        Commissioning::for_selection(&value.to_string(), g, s, b, &docker, t)
    };
    assert!(check(&report, &grant, &boot, &serving, &tasks).is_ok());
    assert!(check(&report, &grant, &boot, &serving, &crossed).is_err());
    assert!(check(&report, "adjacent grant", &boot, &serving, &tasks).is_err());
    assert!(check(&report, &grant, "different boot", &serving, &tasks).is_err());
    assert!(check(&report, &grant, &boot, "different serving", &tasks).is_err());
    report["inherited_and_delegated_ingress"] = json!(" ");
    assert!(check(&report, &grant, &boot, &serving, &tasks).is_err());
}
