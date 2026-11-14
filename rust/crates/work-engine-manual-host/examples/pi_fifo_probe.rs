//! Fixed-public-fixture FIFO mechanism probe. No SDK or candidate admission.
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::{
        fd::AsRawFd,
        unix::{
            fs::{FileTypeExt, MetadataExt, OpenOptionsExt},
            process::CommandExt,
        },
    },
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use work_engine_manual_host::pi_docker::{
    fifo::{self, Codec, FIXTURE},
    native_memory::{disable_process_dumps, observe_backing},
};
type R<T> = Result<T, &'static str>;
const NAME: &str = "we-pi-fifo-01a1db17";
const IMAGE: &str = "sha256:4b150b48b4116e2a22f2c39cc27025fbf4f153e2078fd3b14d159fdb5c7279da";
fn checked(s: &str) -> R<()> {
    if s.as_bytes()
        .windows(b"WORK_ENGINE_EXCLUSION_SENTINEL".len())
        .any(|w| w == b"WORK_ENGINE_EXCLUSION_SENTINEL")
    {
        Err("fixture_check")
    } else {
        Ok(())
    }
}
fn command(args: &[String]) -> R<Vec<u8>> {
    for a in args {
        checked(a)?;
    }
    let mut child = Command::new("/usr/bin/docker")
        .args(args)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", "/nonexistent")
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| "controller_spawn")?;
    use std::os::fd::IntoRawFd;
    let mut pipes = [
        child.stdout.take().unwrap().into_raw_fd(),
        child.stderr.take().unwrap().into_raw_fd(),
    ];
    // Keep owned pipe objects alive while polling; never create raw output files.
    // Reattach through owned File descriptors, transferring the original ownership.
    let mut files = Vec::new();
    for fd in &mut pipes {
        if unsafe { libc::fcntl(*fd, libc::F_SETFL, libc::O_NONBLOCK) } < 0 {
            unsafe {
                libc::kill(-(child.id() as i32), libc::SIGKILL);
            }
            let _ = child.wait();
            return Err("controller_nonblocking");
        }
        use std::os::fd::FromRawFd;
        files.push(unsafe { File::from_raw_fd(*fd) });
    }
    let start = Instant::now();
    let mut result = Vec::new();
    let mut failed = false;
    let mut total = 0usize;
    let mut exit = None;
    loop {
        for (i, f) in files.iter_mut().enumerate() {
            let mut buf = [0; 4096];
            loop {
                match f.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        total += n;
                        if total > 65536 {
                            failed = true;
                            break;
                        }
                        if i == 0 {
                            result.extend_from_slice(&buf[..n]);
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(_) => {
                        failed = true;
                        break;
                    }
                }
            }
        }
        if failed || start.elapsed() > Duration::from_secs(5) {
            unsafe {
                libc::kill(-(child.id() as i32), libc::SIGKILL);
            }
            let _ = child.wait();
            return Err("controller_bound");
        }
        if let Some(success) = exit {
            return if success {
                Ok(result)
            } else {
                Err("controller_refused")
            };
        }
        match child.try_wait() {
            Ok(Some(s)) => exit = Some(s.success()),
            Ok(None) => {}
            Err(_) => return Err("controller_wait"),
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}
fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|s| s.to_string()).collect()
}
fn until_read(f: &mut File, n: usize) -> R<Vec<u8>> {
    let mut out = vec![0; n];
    let mut got = 0;
    let start = Instant::now();
    while got < n {
        match f.read(&mut out[got..]) {
            Ok(0) => {}
            Ok(k) => got += k,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => return Err("fifo_read"),
        };
        if start.elapsed() > Duration::from_secs(3) {
            return Err("fifo_deadline");
        };
        std::thread::sleep(Duration::from_millis(1));
    }
    Ok(out)
}
fn write_bounded(f: &mut File, bytes: &[u8]) -> R<()> {
    let start = Instant::now();
    let mut sent = 0;
    while sent < bytes.len() {
        match f.write(&bytes[sent..]) {
            Ok(0) => return Err("fifo_write_zero"),
            Ok(n) => sent += n,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => return Err("fifo_write"),
        };
        if start.elapsed() > Duration::from_secs(3) {
            return Err("fifo_deadline");
        }
    }
    Ok(())
}
fn inspect(id: &str) -> R<serde_json::Value> {
    let b = command(&args(&["inspect", id]))?;
    let v: serde_json::Value = serde_json::from_slice(&b).map_err(|_| "inspect_json")?;
    Ok(v[0].clone())
}
fn verify(v: &serde_json::Value, id: &str, dir: &Path) -> R<()> {
    if v["Id"] != id
        || v["Name"] != format!("/{NAME}")
        || v["Config"]["Image"] != IMAGE
        || v["Config"]["User"] != "1000:1000"
        || v["HostConfig"]["LogConfig"]["Type"] != "none"
        || v["HostConfig"]["NetworkMode"] != "none"
        || v["HostConfig"]["ReadonlyRootfs"] != true
        || v["HostConfig"]["Memory"] != 134217728
        || v["HostConfig"]["MemorySwap"] != 134217728
        || v["HostConfig"]["Privileged"] != false
        || v["HostConfig"]["CapDrop"] != serde_json::json!(["ALL"])
    {
        return Err("container_profile");
    }
    let mounts = v["Mounts"].as_array().ok_or("mounts")?;
    if mounts.len() != 3 {
        return Err("mount_count");
    }
    for name in ["in", "out", "bootstrap"] {
        let m = mounts
            .iter()
            .find(|m| m["Destination"] == format!("/control/{name}"))
            .ok_or("mount_missing")?;
        let source = if name == "bootstrap" {
            dir.join("bootstrap")
        } else {
            dir.join(name)
        };
        if m["Source"] != source.to_str().unwrap() || m["RW"] != false || m["Type"] != "bind" {
            return Err("mount_profile");
        }
    }
    Ok(())
}
fn exercise(id: &str, dir: &Path, mode: &str) -> R<()> {
    let mut output = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW)
        .open(dir.join("out"))
        .map_err(|_| "open_output")?;
    println!("INTENT_START");
    command(&args(&["start", id]))?;
    let start = Instant::now();
    let mut input = loop {
        match OpenOptions::new()
            .write(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW)
            .open(dir.join("in"))
        {
            Ok(f) => break f,
            Err(_) => {
                if start.elapsed() > Duration::from_secs(3) {
                    return Err("open_deadline");
                };
                std::thread::sleep(Duration::from_millis(2));
            }
        }
    };
    if until_read(&mut output, 14)? != b"FIFO_BOUND_V1\n" {
        return Err("bound_stage");
    }
    let v = inspect(id)?;
    verify(&v, id, dir)?;
    let pid = v["State"]["Pid"].as_u64().ok_or("pid")? as u32;
    let proc = PathBuf::from(format!("/proc/{pid}"));
    let mut seen = Vec::new();
    for entry in fs::read_dir(proc.join("fd")).map_err(|_| "external_fd_unavailable")? {
        let e = entry.map_err(|_| "fd_entry")?;
        let fd = e
            .file_name()
            .to_str()
            .ok_or("fd_name")?
            .parse::<u32>()
            .map_err(|_| "fd_number")?;
        seen.push(fd);
        if fd < 2 {
            let actual = fs::metadata(e.path()).map_err(|_| "fd_stat")?;
            let expect = fs::metadata(dir.join(if fd == 0 { "in" } else { "out" }))
                .map_err(|_| "fifo_stat")?;
            if actual.dev() != expect.dev()
                || actual.ino() != expect.ino()
                || !actual.file_type().is_fifo()
            {
                return Err("fd_identity");
            }
        } else if fd == 2 {
            if fs::read_link(e.path()).map_err(|_| "stderr_link")? != Path::new("/dev/null") {
                return Err("stderr_route");
            }
        }
    }
    seen.sort();
    if seen != [0, 1, 2] {
        return Err("fd_alias");
    }
    let cgroup = observe_backing(pid).map_err(|_| "container_backing_unknown")?;
    if !cgroup.swap_disabled || cgroup.swap_current != 0 {
        return Err("container_swap");
    }
    let status = fs::read_to_string(proc.join("status")).map_err(|_| "status")?;
    if !status
        .lines()
        .any(|l| l.starts_with("NSpid:") && l.split_whitespace().last() == Some("1"))
    {
        return Err("namespace_pid");
    }
    let host_ns = fs::read_link("/proc/self/ns/pid").map_err(|_| "host_namespace")?;
    let container_ns = fs::read_link(proc.join("ns/pid")).map_err(|_| "container_namespace")?;
    if host_ns == container_ns {
        return Err("namespace_shared");
    }
    let inode_pairs: Vec<_> = ["in", "out"]
        .iter()
        .map(|n| {
            let m = fs::metadata(dir.join(n)).unwrap();
            (m.dev(), m.ino())
        })
        .collect();
    let observation = serde_json::json!({"event":"external_fd_namespace_backing", "container_id":id,"pid":pid,
        "pid_namespace":container_ns.to_str(),"fds":[0,1,2],"fifo_dev_inodes":inode_pairs,"backing":cgroup,
        "readonly_object_mounts":3,"fixture_mode":mode}).to_string();
    checked(&observation)?;
    println!("{observation}");
    write_bounded(&mut input, b"P")?;
    if until_read(&mut output, 14)? != b"FIFO_READY_V1\n" {
        return Err("ready_stage");
    }
    let status = fs::read_to_string(proc.join("status")).map_err(|_| "filtered_status")?;
    if !status.contains("NoNewPrivs:\t1") || !status.contains("Seccomp:\t2") {
        return Err("seccomp_status");
    }
    let mut codec = Codec::new(FIXTURE);
    for seq in 0..2 {
        let frame = fifo::encode(FIXTURE, seq, b"PUBLIC_FIXTURE_V1").map_err(|_| "encode")?;
        write_bounded(&mut input, &frame)?;
        let bytes = until_read(&mut output, frame.len())?;
        if bytes != frame
            || codec.accept(&bytes[4..]).map_err(|_| "decode")? != b"PUBLIC_FIXTURE_V1"
        {
            return Err("echo");
        }
    }
    let expected_exit = match mode {
        "clean" => 0,
        "partial-prefix" => {
            write_bounded(&mut input, &[0])?;
            2
        }
        "partial-body" => {
            write_bounded(&mut input, &[0, 0, 0, 100, 0])?;
            2
        }
        "stale-generation" => {
            let mut binding = FIXTURE;
            binding.generation = 2;
            write_bounded(
                &mut input,
                &fifo::encode(binding, 2, b"PUBLIC_FIXTURE_V1").unwrap(),
            )?;
            2
        }
        "duplicate" => {
            write_bounded(
                &mut input,
                &fifo::encode(FIXTURE, 1, b"PUBLIC_FIXTURE_V1").unwrap(),
            )?;
            2
        }
        "compressed" => {
            let mut f = fifo::encode(FIXTURE, 2, b"PUBLIC_FIXTURE_V1").unwrap();
            f[4] = 1;
            write_bounded(&mut input, &f)?;
            2
        }
        "oversize" => {
            write_bounded(&mut input, &((fifo::MAX_FRAME + 1) as u32).to_be_bytes())?;
            2
        }
        "cancel" => {
            println!("INTENT_CANCEL");
            command(&args(&["kill", "--signal", "KILL", id]))?;
            137
        }
        "read-deadline" => {
            if until_read(&mut output, 1) != Err("fifo_deadline") {
                return Err("read_deadline_control");
            };
            println!("READ_DEADLINE_VERIFIED");
            return Ok(());
        }
        "backpressure" => {
            if unsafe { libc::fcntl(output.as_raw_fd(), libc::F_SETPIPE_SZ, 4096) } != 4096
                || unsafe { libc::fcntl(input.as_raw_fd(), libc::F_SETPIPE_SZ, 4096) } != 4096
            {
                return Err("pipe_capacity");
            }
            let mut bounded = false;
            for seq in 2..8 {
                let f = fifo::encode(FIXTURE, seq, &[0; fifo::MAX_FRAME - fifo::HEADER]).unwrap();
                if write_bounded(&mut input, &f) == Err("fifo_deadline") {
                    bounded = true;
                    break;
                }
            }
            if !bounded {
                return Err("backpressure_control");
            };
            println!("WRITE_BACKPRESSURE_DEADLINE_VERIFIED");
            return Ok(());
        }
        _ => return Err("fixture_mode"),
    };
    drop(input);
    let start = Instant::now();
    loop {
        let v = inspect(id)?;
        if v["State"]["Running"] == false {
            if v["State"]["ExitCode"] != expected_exit {
                return Err("partial_exit");
            };
            break;
        }
        if start.elapsed() > Duration::from_secs(3) {
            return Err("exit_deadline");
        };
        std::thread::sleep(Duration::from_millis(5));
    }
    println!("TWO_BOUND_FRAMES_TERMINAL_CONTROL_VERIFIED");
    Ok(())
}
fn run() -> R<()> {
    for fixture in [
        "PUBLIC_FIXTURE_V1",
        "FIFO_BOUND_V1\n",
        "FIFO_READY_V1\n",
        "P",
        "INTENT_CREATE",
        "INTENT_START",
        "INTENT_REMOVE",
        "FIXED_MECHANISM_PASS_CANDIDATE_ADMISSION_UNESTABLISHED",
    ] {
        checked(fixture)?;
    }
    let mode = std::env::args().nth(2).ok_or("fixture_mode")?;
    if ![
        "clean",
        "partial-prefix",
        "partial-body",
        "stale-generation",
        "duplicate",
        "compressed",
        "oversize",
        "cancel",
        "read-deadline",
        "backpressure",
    ]
    .contains(&mode.as_str())
    {
        return Err("fixture_mode");
    }
    disable_process_dumps().map_err(|_| "host_dump")?;
    let backing = observe_backing(std::process::id()).map_err(|_| "host_backing")?;
    if !backing.swap_disabled || backing.swap_current != 0 {
        return Err("host_swap");
    }
    let dir = PathBuf::from(std::env::args().nth(1).ok_or("fixture_directory")?);
    checked(dir.to_str().ok_or("fixture_path")?)?;
    if !dir.is_absolute() || fs::canonicalize(&dir).map_err(|_| "fixture_dir")? != dir {
        return Err("fixture_directory");
    }
    for name in ["in", "out"] {
        let path = dir.join(name);
        let c = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
        if unsafe { libc::mkfifo(c.as_ptr(), 0o600) } != 0 {
            return Err("fifo_create");
        }
    }
    let identities: Vec<_> = ["in", "out"]
        .iter()
        .map(|n| {
            let m = fs::symlink_metadata(dir.join(n)).unwrap();
            (m.dev(), m.ino())
        })
        .collect();
    let mut create = args(&[
        "create",
        "--pull",
        "never",
        "--name",
        NAME,
        "--network",
        "none",
        "--read-only",
        "--log-driver",
        "none",
        "--cap-drop",
        "ALL",
        "--security-opt",
        "no-new-privileges",
        "--memory",
        "134217728",
        "--memory-swap",
        "134217728",
        "--pids-limit",
        "8",
        "--user",
        "1000:1000",
        "--entrypoint",
        "/control/bootstrap",
    ]);
    for name in ["in", "out", "bootstrap"] {
        create.extend(args(&[
            "--mount",
            &format!(
                "type=bind,src={},dst=/control/{name},readonly",
                dir.join(name).display()
            ),
        ]));
    }
    create.push(IMAGE.into());
    println!("INTENT_CREATE");
    let created = command(&create)?;
    let id = String::from_utf8(created)
        .map_err(|_| "create_id")?
        .trim()
        .to_string();
    if id.len() != 64 || !id.bytes().all(|v| v.is_ascii_hexdigit()) {
        return Err("create_unknown");
    }
    let result = exercise(&id, &dir, &mode);
    println!("INTENT_REMOVE");
    let current = inspect(&id)?;
    if current["Id"] != id || current["Name"] != format!("/{NAME}") {
        return Err("cleanup_identity");
    }
    let removed = command(&args(&["rm", "--force", &id])).is_ok();
    for (index, name) in ["in", "out"].iter().enumerate() {
        let p = dir.join(name);
        let m = fs::symlink_metadata(&p).map_err(|_| "cleanup_fifo")?;
        if !m.file_type().is_fifo()
            || m.uid() != 1000
            || m.mode() & 0o777 != 0o600
            || (m.dev(), m.ino()) != identities[index]
        {
            return Err("cleanup_fifo_identity");
        };
        fs::remove_file(p).map_err(|_| "cleanup_fifo_remove")?;
    }
    if !removed {
        return Err("remove_unknown");
    };
    println!("EXACT_CONTAINER_REMOVED_FIFO_CLOSED");
    result
}
fn main() {
    match run() {
        Ok(()) => println!("FIXED_MECHANISM_PASS_CANDIDATE_ADMISSION_UNESTABLISHED"),
        Err(reason) => {
            println!("FIXED_MECHANISM_REFUSAL:{reason}");
            std::process::exit(2)
        }
    }
}
