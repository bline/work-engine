use work_engine_manual_host::pi_docker::native_memory::{
    CandidateArena, MemoryRefusal, disable_process_dumps, verify_process_dump_protection,
};

#[test]
fn bounded_locked_mapping_and_kernel_flags() {
    let mut arena = CandidateArena::new(8192).expect("host must support bounded mlock");
    arena.append(b"synthetic-candidate").unwrap();
    let address = arena.bytes().as_ptr() as usize;
    let smaps = std::fs::read_to_string("/proc/self/smaps").unwrap();
    let mut selected = false;
    let mut locked = false;
    let mut flags = false;
    for line in smaps.lines() {
        if let Some(range) = line.split_whitespace().next().filter(|v| v.contains('-')) {
            if let Some((a, b)) = range.split_once('-') {
                if let (Ok(a), Ok(b)) = (usize::from_str_radix(a, 16), usize::from_str_radix(b, 16))
                {
                    selected = a <= address && address < b;
                }
            }
        }
        if selected && line.starts_with("Locked:") {
            locked = line
                .split_whitespace()
                .nth(1)
                .unwrap()
                .parse::<usize>()
                .unwrap()
                >= 8;
        }
        if selected && line.starts_with("VmFlags:") {
            let values: Vec<_> = line.split_whitespace().collect();
            flags = ["lo", "dd", "dc"].iter().all(|f| values.contains(f));
        }
    }
    assert!(
        locked && flags,
        "actual mapping must be locked, dontdump, dontfork"
    );
    assert_eq!(arena.bytes(), b"synthetic-candidate");
    assert_eq!(arena.append(&[0; 8192]), Err(MemoryRefusal::Overflow));
    assert_eq!(arena.bytes(), b"synthetic-candidate");
    arena.clear();
    assert!(arena.bytes().is_empty());
    assert_eq!(CandidateArena::new(0).err(), Some(MemoryRefusal::Capacity));
    assert_eq!(
        CandidateArena::new(usize::MAX).err(),
        Some(MemoryRefusal::Capacity)
    );
}

#[test]
fn dump_control_exec_reset() {
    const KEY: &str = "WORK_ENGINE_DUMP_PROBE";
    if std::env::var_os(KEY).is_some() {
        // This is AFTER execve, before any candidate data is acquired.
        assert_eq!(
            verify_process_dump_protection(),
            Err(MemoryRefusal::Dumpable)
        );
        disable_process_dumps().unwrap();
        assert!(verify_process_dump_protection().is_ok());
        return;
    }
    use std::os::unix::process::CommandExt;
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "dump_control_exec_reset"])
        .env_clear()
        .env(KEY, "1");
    unsafe {
        command.pre_exec(|| {
            if libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    assert!(command.status().unwrap().success());
}
