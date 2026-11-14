//! Memory primitives for the native Pi boundary. These do not admit a workload.
//! In particular, locked arenas do not protect copies made by parsers or SDKs.
use std::ptr::NonNull;

/// Fixed-capacity, privately owned, non-swapping, non-core candidate storage.
/// No Debug/Clone/Serialize implementation may expose the contained bytes.
/// Borrowed views cannot outlive this owner; callers must not copy candidates
/// into ordinary heap buffers before exclusion checking.
pub struct CandidateArena {
    ptr: NonNull<u8>,
    capacity: usize,
    length: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum MemoryRefusal {
    #[error("candidate capacity outside supported bound")]
    Capacity,
    #[error("private memory mapping unavailable")]
    Mapping,
    #[error("candidate memory locking unavailable")]
    Lock,
    #[error("candidate dump or fork exclusion unavailable")]
    Exclusion,
    #[error("candidate aggregate capacity exceeded")]
    Overflow,
    #[error("process remains dumpable")]
    Dumpable,
    #[error("process dump protection unavailable")]
    DumpControl,
}

impl CandidateArena {
    pub const MAX_CAPACITY: usize = 1024 * 1024;
    pub fn new(capacity: usize) -> Result<Self, MemoryRefusal> {
        if capacity == 0 || capacity > Self::MAX_CAPACITY {
            return Err(MemoryRefusal::Capacity);
        }
        // MAP_ANONYMOUS supplies zeroed, page-aligned private backing. No input
        // is accepted until all protections have succeeded.
        let raw = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                capacity,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        if raw == libc::MAP_FAILED {
            return Err(MemoryRefusal::Mapping);
        }
        let result = unsafe {
            if libc::mlock(raw, capacity) != 0 {
                Err(MemoryRefusal::Lock)
            } else if libc::madvise(raw, capacity, libc::MADV_DONTDUMP) != 0
                || libc::madvise(raw, capacity, libc::MADV_DONTFORK) != 0
            {
                Err(MemoryRefusal::Exclusion)
            } else {
                Ok(())
            }
        };
        if let Err(e) = result {
            unsafe {
                libc::munmap(raw, capacity);
            }
            return Err(e);
        }
        Ok(Self {
            ptr: NonNull::new(raw.cast()).ok_or(MemoryRefusal::Mapping)?,
            capacity,
            length: 0,
        })
    }
    pub fn append(&mut self, bytes: &[u8]) -> Result<(), MemoryRefusal> {
        let end = self
            .length
            .checked_add(bytes.len())
            .filter(|n| *n <= self.capacity)
            .ok_or(MemoryRefusal::Overflow)?;
        unsafe {
            std::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                self.ptr.as_ptr().add(self.length),
                bytes.len(),
            );
        }
        self.length = end;
        Ok(())
    }
    pub fn bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.length) }
    }
    pub fn clear(&mut self) {
        // Volatile stores prevent dead-store elimination before unmapping.
        for offset in 0..self.capacity {
            unsafe {
                self.ptr.as_ptr().add(offset).write_volatile(0);
            }
        }
        self.length = 0;
    }
}
impl Drop for CandidateArena {
    fn drop(&mut self) {
        self.clear();
        // Unmap while still locked: never expose a populated unlocked interval.
        unsafe {
            libc::munmap(self.ptr.as_ptr().cast(), self.capacity);
        }
    }
}

/// Process-local control, to be applied in the final executable BEFORE candidate
/// acquisition. execve resets dumpability: a launcher setting is insufficient.
/// This is deliberately not a runtime admission capability.
pub fn disable_process_dumps() -> Result<(), MemoryRefusal> {
    if unsafe { libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0) } != 0 {
        return Err(MemoryRefusal::DumpControl);
    }
    verify_process_dump_protection()
}
pub fn verify_process_dump_protection() -> Result<(), MemoryRefusal> {
    match unsafe { libc::prctl(libc::PR_GET_DUMPABLE, 0, 0, 0, 0) } {
        0 => Ok(()),
        -1 => Err(MemoryRefusal::DumpControl),
        _ => Err(MemoryRefusal::Dumpable),
    }
}

#[derive(Debug, serde::Serialize)]
pub struct BackingObservation {
    pub pid: u32,
    pub start_ticks: u64,
    pub swap_disabled: bool,
    pub swap_current: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, thiserror::Error)]
#[serde(rename_all = "snake_case")]
pub enum ProfileRefusal {
    #[error("process identity unavailable or changed")]
    ProcessIdentity,
    #[error("memory backing observation unavailable")]
    BackingUnknown,
    #[error("controller memory can swap")]
    ControllerSwap,
    #[error("Docker attachment relay memory can swap")]
    RelaySwap,
    #[error("Docker attachment relay dump policy unestablished")]
    RelayDumpUnknown,
    #[error("Docker local peer identity unavailable")]
    DockerPeer,
}

fn start_ticks(pid: u32) -> Result<u64, ProfileRefusal> {
    let s = std::fs::read_to_string(format!("/proc/{pid}/stat"))
        .map_err(|_| ProfileRefusal::ProcessIdentity)?;
    // comm may contain spaces or parentheses; fields after its LAST ')' begin
    // at field3. starttime is field22, hence offset19.
    s.rsplit_once(')')
        .and_then(|(_, tail)| tail.split_whitespace().nth(19))
        .and_then(|n| n.parse().ok())
        .ok_or(ProfileRefusal::ProcessIdentity)
}
fn cgroup_relative(text: &str) -> Result<&str, ProfileRefusal> {
    let mut paths = text.lines().filter_map(|line| line.strip_prefix("0::"));
    let path = paths.next().ok_or(ProfileRefusal::BackingUnknown)?;
    if paths.next().is_some()
        || !path.starts_with('/')
        || path.contains('\0')
        || std::path::Path::new(path).components().any(|c| {
            matches!(
                c,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        })
    {
        return Err(ProfileRefusal::BackingUnknown);
    }
    Ok(path)
}
/// Read-only observation of one process; this is NOT an admission capability.
/// No cgroup setting is modified and unavailable evidence is never zero.
pub fn observe_backing(pid: u32) -> Result<BackingObservation, ProfileRefusal> {
    let before = start_ticks(pid)?;
    let membership = std::fs::read_to_string(format!("/proc/{pid}/cgroup"))
        .map_err(|_| ProfileRefusal::BackingUnknown)?;
    let relative = cgroup_relative(&membership)?;
    let root = std::path::Path::new("/sys/fs/cgroup");
    let dir = root.join(relative.trim_start_matches('/'));
    if std::fs::canonicalize(&dir).map_err(|_| ProfileRefusal::BackingUnknown)? != dir {
        return Err(ProfileRefusal::BackingUnknown);
    }
    let swap_current = std::fs::read_to_string(dir.join("memory.swap.current"))
        .map_err(|_| ProfileRefusal::BackingUnknown)?
        .trim()
        .parse()
        .map_err(|_| ProfileRefusal::BackingUnknown)?;
    let mut swap_disabled = false;
    let mut ancestor = Some(dir.as_path());
    while let Some(path) = ancestor {
        if path == root {
            break;
        }
        match std::fs::read_to_string(path.join("memory.swap.max")) {
            Ok(value) if value.trim() == "0" => swap_disabled = true,
            Ok(value) if value.trim() == "max" || value.trim().parse::<u64>().is_ok() => {}
            _ => return Err(ProfileRefusal::BackingUnknown),
        }
        ancestor = path.parent();
    }
    if start_ticks(pid)? != before
        || std::fs::read_to_string(format!("/proc/{pid}/cgroup"))
            .map_err(|_| ProfileRefusal::BackingUnknown)?
            != membership
    {
        return Err(ProfileRefusal::ProcessIdentity);
    }
    Ok(BackingObservation {
        pid,
        start_ticks: before,
        swap_disabled,
        swap_current,
    })
}

/// Observe the local Unix socket peer. A socket-activation peer may be PID1;
/// this does not identify the process serving Docker requests.
/// This opens no container, sends no request and never returns peer-controlled
/// diagnostic strings. A daemon-provided DTO cannot establish this observation.
pub fn observe_local_docker_socket_peer() -> Result<BackingObservation, ProfileRefusal> {
    use std::os::fd::AsRawFd;
    let socket = std::os::unix::net::UnixStream::connect("/var/run/docker.sock")
        .map_err(|_| ProfileRefusal::DockerPeer)?;
    let mut credentials: libc::ucred = unsafe { std::mem::zeroed() };
    let mut size = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    if unsafe {
        libc::getsockopt(
            socket.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&mut credentials as *mut libc::ucred).cast(),
            &mut size,
        )
    } != 0
        || size as usize != std::mem::size_of::<libc::ucred>()
        || credentials.pid <= 0
    {
        return Err(ProfileRefusal::DockerPeer);
    }
    observe_backing(credentials.pid as u32)
}

/// Deployed-profile diagnostic for the Docker service process. This does not
/// rely on SO_PEERCRED being the daemon: systemd socket activation may own it.
/// A fixed service cgroup, exact process comm and stable process identity are
/// observations only, never publication or transport admission authority.
pub fn observe_docker_service_relay() -> Result<BackingObservation, ProfileRefusal> {
    let list = std::fs::read_to_string("/sys/fs/cgroup/system.slice/docker.service/cgroup.procs")
        .map_err(|_| ProfileRefusal::DockerPeer)?;
    let mut found = None;
    for item in list.split_whitespace() {
        let pid: u32 = item.parse().map_err(|_| ProfileRefusal::DockerPeer)?;
        if std::fs::read_to_string(format!("/proc/{pid}/comm"))
            .map_err(|_| ProfileRefusal::ProcessIdentity)?
            .trim()
            == "dockerd"
        {
            if found.is_some() {
                return Err(ProfileRefusal::DockerPeer);
            }
            let before = start_ticks(pid)?;
            let observation = observe_backing(pid)?;
            if before != observation.start_ticks {
                return Err(ProfileRefusal::ProcessIdentity);
            }
            found = Some(observation);
        }
    }
    found.ok_or(ProfileRefusal::DockerPeer)
}

/// Diagnostic only: even a swap-disabled peer lacks dump-policy/descendant
/// evidence. No successful runtime permit is issued by this bounded leaf.
pub fn diagnose_attach_backing() -> Result<(), ProfileRefusal> {
    let controller = observe_backing(std::process::id())?;
    if !controller.swap_disabled || controller.swap_current != 0 {
        return Err(ProfileRefusal::ControllerSwap);
    }
    let relay = observe_docker_service_relay()?;
    if !relay.swap_disabled || relay.swap_current != 0 {
        return Err(ProfileRefusal::RelaySwap);
    }
    Err(ProfileRefusal::RelayDumpUnknown)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cgroup_paths_refuse_ambiguous_or_escaping_membership() {
        for text in ["", "0::relative", "0::/a/../b", "0::/a\n0::/b"] {
            assert_eq!(cgroup_relative(text), Err(ProfileRefusal::BackingUnknown));
        }
        assert_eq!(cgroup_relative("0::/user.slice/x"), Ok("/user.slice/x"));
    }
}
