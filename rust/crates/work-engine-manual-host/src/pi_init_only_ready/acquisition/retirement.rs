//! One unselected Linux/x86-64 natural-exit evidence reader. A successful
//! cgroup_rmdir establishes kernel subtree retirement, not task exit status,
//! containment before removal, or closure of the runtime recovery frontier.

use super::{Custody, Kind, kernel::KernelTask, within};
use crate::{Result, refuse};
use serde::Serialize;
use serde_json::json;
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    os::{
        fd::AsRawFd,
        unix::{
            ffi::OsStrExt,
            fs::{MetadataExt, OpenOptionsExt},
        },
    },
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

const TRACEFS: &str = "/sys/kernel/tracing";
const TRACEFS_MAGIC: libc::c_long = 0x7472_6163;
const EVENT: &str = "cgroup:cgroup_rmdir";
const EVENT_DIR: &str = "events/cgroup/cgroup_rmdir";
const MAX_TEXT: u64 = 8192;
const MAX_CPUS: usize = 256;
const MAX_TRACE: u64 = 32 * 1024;
const FILTER: &str = "root == 0";
const RETIREMENT_READ_INTERVAL: Duration = Duration::from_millis(25);

pub(super) struct RetirementCapture {
    instance: Instance,
    journal: PathBuf,
    arm: u64,
    cpus: Vec<u32>,
    mask: String,
}

/// Positions in the same protected archive. This is not a verdict or permit.
#[derive(Serialize)]
pub(super) struct SubtreeRetired {
    pub(super) arm: u64,
    pub(super) capture: u64,
}

// Once durably armed, an unfinalized instance is evidence, not scratch.
struct Instance {
    path: PathBuf,
    retained: bool,
    identity: (u64, u64),
    trusted_gid0: bool,
}

fn root_controls(uid: u32, gid: u32, mode: u32, trusted_gid0: bool) -> bool {
    let outside_write = if gid == 0 && trusted_gid0 {
        0o002
    } else {
        0o022
    };
    uid == 0 && mode & outside_write == 0
}

fn private_instance(gid: u32, mode: u32, trusted_gid0: bool) -> bool {
    mode & 0o007 == 0 && (mode & 0o070 == 0 || gid == 0 && trusted_gid0)
}

fn checked_node(path: &Path, directory: bool, trusted_gid0: bool) -> Result<fs::Metadata> {
    let meta = fs::symlink_metadata(path)?;
    if !(if directory {
        meta.is_dir()
    } else {
        meta.is_file()
    }) || !root_controls(meta.uid(), meta.gid(), meta.mode(), trusted_gid0)
    {
        return refuse("unprotected tracefs node");
    }
    let path = std::ffi::CString::new(path.as_os_str().as_bytes()).map_err(io::Error::other)?;
    // Reject ambiguous ACL metadata rather than changing permissions.
    let mut attrs = [0u8; 4096];
    let n = unsafe { libc::llistxattr(path.as_ptr(), attrs.as_mut_ptr().cast(), attrs.len()) };
    if n < 0 {
        let e = io::Error::last_os_error();
        if e.raw_os_error() != Some(libc::EOPNOTSUPP) {
            return Err(e.into());
        }
    } else if attrs[..n as usize]
        .split(|b| *b == 0)
        .any(|x| x == b"system.posix_acl_access" || x == b"system.posix_acl_default")
    {
        return refuse("tracefs node has an unadjudicated POSIX ACL");
    }
    Ok(meta)
}

impl Instance {
    fn create(attempt: &str, trusted_gid0: bool) -> Result<Self> {
        // Public traversal of the shared mount is not evidence access.
        // The actual attempt owner must bind the GID 0 premise separately;
        // an attestation for a task-free fixture does not carry forward.
        let root = Path::new(TRACEFS);
        let instances = root.join("instances");
        if unsafe { libc::geteuid() } != 0 || fs::canonicalize(&instances)? != instances {
            return refuse("administrator-owned canonical tracefs unavailable");
        }
        for parent in instances.ancestors() {
            checked_node(parent, true, trusted_gid0)?;
        }
        let dir = File::open(root)?;
        let mut info = std::mem::MaybeUninit::<libc::statfs>::uninit();
        // Safety: dir stays open and the buffer is initialized on success.
        if unsafe { libc::fstatfs(dir.as_raw_fd(), info.as_mut_ptr()) } != 0
            || unsafe { info.assume_init() }.f_type != TRACEFS_MAGIC
        {
            return refuse("administrator-owned tracefs unavailable");
        }
        let path = instances.join(format!("work-engine-b-{attempt}"));
        fs::create_dir(&path)?;
        let meta = match checked_node(&path, true, trusted_gid0) {
            Ok(m) if private_instance(m.gid(), m.mode(), trusted_gid0) => m,
            other => {
                // No control writes, even cleanup writes, before protection.
                fs::remove_dir(&path)?;
                return other.and_then(|_| refuse("fresh tracefs instance is not private"));
            }
        };
        Ok(Self {
            path,
            retained: false,
            identity: (meta.dev(), meta.ino()),
            trusted_gid0,
        })
    }

    fn node(&self, name: &str, directory: bool) -> Result<PathBuf> {
        let instances = self
            .path
            .parent()
            .ok_or_else(|| io::Error::other("missing tracefs parent"))?;
        for parent in instances.ancestors() {
            checked_node(parent, true, self.trusted_gid0)?;
        }
        let base = checked_node(&self.path, true, self.trusted_gid0)?;
        if (base.dev(), base.ino()) != self.identity
            || !private_instance(base.gid(), base.mode(), self.trusted_gid0)
        {
            return refuse("tracefs instance identity or protection changed");
        }
        let path = self.path.join(name);
        if !name.is_empty() {
            for parent in path.ancestors().skip(1).take_while(|p| *p != self.path) {
                checked_node(parent, true, self.trusted_gid0)?;
            }
            checked_node(&path, directory, self.trusted_gid0)?;
        }
        Ok(path)
    }

    fn read(&self, name: &str, max: u64) -> Result<String> {
        let mut text = String::new();
        OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(self.node(name, false)?)?
            .take(max + 1)
            .read_to_string(&mut text)?;
        if text.len() as u64 > max {
            return refuse("tracefs record exceeds capture bound");
        }
        Ok(text)
    }

    fn control(&self, name: &str, data: &str) -> Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(self.node(name, false)?)?;
        // A control command is one write, not a sequence of partial commands.
        if file.write(data.as_bytes())? != data.len() {
            return refuse("tracefs control write incomplete");
        }
        Ok(())
    }

    fn expect(&self, name: &str, value: &str) -> Result<()> {
        if self.read(name, MAX_TEXT)?.trim() != value {
            return refuse("tracefs control readback differs");
        }
        Ok(())
    }

    fn cpu_coverage(&self) -> Result<(Vec<u32>, String)> {
        let mut possible = String::new();
        File::open("/sys/devices/system/cpu/possible")?
            .take(513)
            .read_to_string(&mut possible)?;
        let expected = possible_cpus(&possible)
            .ok_or_else(|| io::Error::other("possible CPU set unavailable or outside bound"))?;
        // tracefs's cpumask callback rejects a short buffer, even on an
        // EOF probe. Request its complete bounded line in one read.
        let mut raw = [0u8; 513];
        let len = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(self.node("tracing_cpumask", false)?)?
            .read(&mut raw)?;
        if !(1..=512).contains(&len) || raw[len - 1] != b'\n' {
            return refuse("tracefs CPU mask unavailable or outside bound");
        }
        let mask = std::str::from_utf8(&raw[..len])
            .map_err(io::Error::other)?
            .to_owned();
        let mut cpus = Vec::new();
        for entry in fs::read_dir(self.node("per_cpu", true)?)? {
            let name = entry?.file_name();
            let cpu = name
                .to_str()
                .and_then(|s| s.strip_prefix("cpu"))
                .and_then(decimal);
            if cpus.len() == MAX_CPUS || cpu.is_none() {
                return refuse("tracefs CPU inventory unavailable or outside bound");
            }
            cpus.push(cpu.expect("checked"));
        }
        cpus.sort_unstable();
        if cpus != expected || !cpus.iter().all(|&cpu| in_mask(&mask, cpu)) {
            return refuse("tracefs does not cover every possible CPU");
        }
        Ok((cpus, mask))
    }

    fn stats(&self, cpus: &[u32]) -> Result<Vec<(u32, String)>> {
        cpus.iter()
            .map(|cpu| Ok((*cpu, self.read(&format!("per_cpu/cpu{cpu}/stats"), 512)?)))
            .collect()
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        if !self.path.as_os_str().is_empty() {
            let _ = self.control("tracing_on", "0\n");
            if !self.retained
                && let Ok(path) = self.node("", true)
            {
                let _ = fs::remove_dir(path);
            }
        }
    }
}

impl RetirementCapture {
    pub(super) fn arm_position(&self) -> u64 {
        self.arm
    }

    /// Required before recording the Start arm or writing any Start bytes.
    /// The exact original ID is not yet available; never change/reset the
    /// capture when it is learned. Hierarchy 0 is the unified v2 hierarchy.
    pub(super) fn arm(
        journal: &mut Custody,
        trusted_gid0: bool,
        deadline: Instant,
    ) -> Result<Self> {
        within(deadline)?;
        let path = journal.attempt_path().to_owned();
        let attempt = path
            .file_name()
            .and_then(|p| p.to_str())
            .filter(|id| super::full_hex(id))
            .ok_or_else(|| io::Error::other("invalid retirement attempt directory"))?;
        let mut instance = Instance::create(attempt, trusted_gid0)?;
        instance.control("tracing_on", "0\n")?;
        instance.expect("tracing_on", "0")?;
        instance.expect("current_tracer", "nop")?;
        instance.expect("set_event", "")?;
        // No free-form comm prefix or overwrite of an early retirement.
        instance.control("options/context-info", "0\n")?;
        instance.control("options/overwrite", "0\n")?;
        // The Linux trace reader otherwise may stop recording on open.
        // Observe progress without consuming the ring or suppressing events.
        instance.control("options/pause-on-trace", "0\n")?;
        instance.expect("options/context-info", "0")?;
        instance.expect("options/overwrite", "0")?;
        instance.expect("options/pause-on-trace", "0")?;
        let format = instance.read(&format!("{EVENT_DIR}/format"), MAX_TEXT)?;
        let event_id = event_format(&format)
            .ok_or_else(|| io::Error::other("unexpected cgroup_rmdir format"))?;
        instance.control(&format!("{EVENT_DIR}/filter"), &format!("{FILTER}\n"))?;
        instance.expect(&format!("{EVENT_DIR}/filter"), FILTER)?;
        instance.control(&format!("{EVENT_DIR}/enable"), "1\n")?;
        instance.expect("set_event", EVENT)?;
        let (cpus, mask) = instance.cpu_coverage()?;
        let baseline = instance.stats(&cpus)?;
        if entry_count(&baseline, &cpus) != Some(0) {
            return refuse("new tracefs instance has nonempty or incomplete counters");
        }
        instance.control("tracing_on", "1\n")?;
        instance.expect("tracing_on", "1")?;
        let arm = journal.record(
            Kind::SubtreeRetirementArm,
            json!({
                "schema": "b-init-original-rmdir-v3", "instance": instance.path,
                "pause_on_trace": false,
                "instance_dev_be": instance.identity.0.to_be_bytes(),
                "instance_ino_be": instance.identity.1.to_be_bytes(),
                "trusted_gid0": trusted_gid0,
                "hierarchy": 0, "event_id": event_id, "format": format, "filter": FILTER,
                "cpus": cpus, "mask": mask, "baseline": baseline,
            }),
        )?;
        instance.retained = true;
        within(deadline)?;
        Ok(Self {
            instance,
            journal: path,
            arm,
            cpus,
            mask,
        })
    }

    /// The original Wait reply can precede the exit monitor's runc cleanup.
    /// A progress read is not the terminal witness: after a matching hint,
    /// `finish` still joins the original pidfd and validates the stopped ring.
    pub(super) fn await_original(&self, task: &KernelTask, deadline: Instant) -> Result<()> {
        await_hint(task.retirement_key().subtree_inode, deadline, || {
            self.instance.expect("tracing_on", "1")?;
            self.instance.expect("options/pause-on-trace", "0")?;
            self.instance.read("trace", MAX_TRACE)
        })
    }

    /// After any possible Start, also finish on refusal/unbound/expired paths
    /// to preserve the sample. A positive witness requires the original pin;
    /// exact runtime completion and acceptance/recovery remain caller proofs.
    pub(super) fn finish(
        mut self,
        journal: &mut Custody,
        task: Option<&KernelTask>,
        deadline: Instant,
    ) -> Result<SubtreeRetired> {
        if journal.attempt_path() != self.journal {
            return refuse("retirement capture crossed attempt archives");
        }
        let exited = task.map(KernelTask::original_exited).transpose();
        let sample = self.snapshot();
        let key = task.map(KernelTask::retirement_key);
        let valid = matches!(exited, Ok(Some(true)))
            && sample.as_ref().is_ok_and(|s| {
                key.is_some_and(|key| matches_retirement(s, &self.cpus, key.subtree_inode))
            });
        // Sync the raw sample (even if unbound or lossy) before destroying
        // its source. On custody failure leave the named instance with the
        // administrator instead of erasing the only potential evidence.
        let complete_read = sample.is_ok();
        let position = record_snapshot(journal, self.arm, key, sample, &exited)?;
        if !complete_read {
            return refuse(
                "tracefs sample unavailable; leave named instance for the administrator",
            );
        }
        fs::remove_dir(self.instance.node("", true)?)?;
        self.instance.path.clear();
        within(deadline)?;
        if !valid {
            return refuse("original subtree retirement absent, crossed or capture incomplete");
        }
        Ok(SubtreeRetired {
            arm: self.arm,
            capture: position,
        })
    }

    fn snapshot(&self) -> Result<Snapshot> {
        self.instance.control("tracing_on", "0\n")?;
        self.instance.expect("tracing_on", "0")?;
        let trace = self.instance.read("trace", MAX_TRACE)?;
        // Check without throwing away the trace that was already read.
        let stats = self.instance.stats(&self.cpus);
        let coverage = (|| -> Result<()> {
            self.instance.expect("current_tracer", "nop")?;
            self.instance.expect("set_event", EVENT)?;
            self.instance
                .expect(&format!("{EVENT_DIR}/filter"), FILTER)?;
            self.instance.expect("options/context-info", "0")?;
            self.instance.expect("options/overwrite", "0")?;
            self.instance.expect("options/pause-on-trace", "0")?;
            let (cpus, mask) = self.instance.cpu_coverage()?;
            if cpus != self.cpus || mask != self.mask {
                return refuse("tracefs CPU coverage changed");
            }
            Ok(())
        })();
        let coverage_error = coverage
            .err()
            .as_ref()
            .or_else(|| stats.as_ref().err())
            .map(ToString::to_string);
        Ok(Snapshot {
            trace,
            stats: stats.unwrap_or_default(),
            coverage_error,
        })
    }
}

#[derive(Serialize)]
struct Snapshot {
    trace: String,
    stats: Vec<(u32, String)>,
    coverage_error: Option<String>,
}

fn record_snapshot(
    journal: &mut Custody,
    arm: u64,
    task: Option<&super::TaskKey>,
    sample: Result<Snapshot>,
    exited: &io::Result<Option<bool>>,
) -> Result<u64> {
    journal.record(
        Kind::SubtreeRetirementCapture,
        json!({
            "arm": arm, "task": task, "original_exited": exited.as_ref().ok().copied().flatten(),
            "pidfd_error": exited.as_ref().err().map(ToString::to_string),
            "sample": sample.as_ref().ok(), "capture_error": sample.err().map(|e| e.to_string()),
        }),
    )
}

fn decimal<T: std::str::FromStr>(s: &str) -> Option<T> {
    (!s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
        .then(|| s.parse().ok())
        .flatten()
}

fn unique<T>(mut values: impl Iterator<Item = T>) -> Option<T> {
    let v = values.next()?;
    values.next().is_none().then_some(v)
}

fn event_format(text: &str) -> Option<u32> {
    fn field(text: &str, name: &str, size: &str, signed: &str) -> Option<()> {
        let fields = unique(
            text.lines()
                .filter_map(|line| line.trim().strip_prefix(name)),
        )?;
        let mut parts = fields.split(';').map(str::trim);
        decimal::<u32>(parts.next()?.strip_prefix("offset:")?)?;
        (parts.next()?.strip_prefix("size:")? == size
            && parts.next()?.strip_prefix("signed:")? == signed)
            .then_some(())
    }
    (text.lines().next()? == "name: cgroup_rmdir").then_some(())?;
    field(text, "field:int root;", "4", "1")?;
    field(text, "field:u64 id;", "8", "0")?;
    let id = decimal(unique(
        text.lines()
            .filter_map(|line| line.strip_prefix("ID:").map(str::trim)),
    )?)?;
    (id != 0).then_some(id)
}

pub(super) fn possible_cpus(text: &str) -> Option<Vec<u32>> {
    if text.len() > 512 {
        return None;
    }
    let mut cpus = Vec::new();
    for part in text.trim_end().split(',') {
        let (start, end) = part.split_once('-').unwrap_or((part, part));
        let (start, end) = (decimal::<u32>(start)?, decimal::<u32>(end)?);
        if end.checked_sub(start)? >= MAX_CPUS as u32
            || cpus.last().is_some_and(|&last| last >= start)
        {
            return None;
        }
        cpus.extend(start..=end);
        if cpus.len() > MAX_CPUS {
            return None;
        }
    }
    Some(cpus)
}

fn in_mask(mask: &str, cpu: u32) -> bool {
    let group = |s: &str| !s.is_empty() && s.len() <= 8 && s.bytes().all(|b| b.is_ascii_hexdigit());
    let mut parts = mask.trim_end().split(',');
    if !parts.next().is_some_and(group) || !parts.all(|s| s.len() == 8 && group(s)) {
        return false;
    }
    mask.trim_end()
        .rsplit(',')
        .nth((cpu / 32) as usize)
        .and_then(|s| u32::from_str_radix(s, 16).ok())
        .is_some_and(|bits| bits & (1 << (cpu % 32)) != 0)
}

fn entry_count(stats: &[(u32, String)], cpus: &[u32]) -> Option<u64> {
    fn stat(s: &str, name: &str) -> Option<u64> {
        decimal(unique(s.lines().filter_map(|line| {
            line.split_once(':')
                .filter(|(n, _)| *n == name)
                .map(|(_, v)| v.trim())
        }))?)
    }
    if cpus.is_empty() || !stats.iter().map(|(cpu, _)| cpu).eq(cpus) {
        return None;
    }
    stats.iter().try_fold(0u64, |sum, (_, raw)| {
        for name in ["overrun", "commit overrun", "dropped events", "read events"] {
            (stat(raw, name)? == 0).then_some(())?;
        }
        sum.checked_add(stat(raw, "entries")?)
    })
}

fn parse_id(line: &str) -> Option<u64> {
    let data = line.strip_prefix("cgroup_rmdir: root=0 id=")?;
    // The kernel prints the numeric fields before the string. The final
    // verifier counts every line, including any injected newlines.
    let (fields, path) = data.split_once(" path=")?;
    let (id, level) = fields.split_once(" level=")?;
    if !path.starts_with('/') || path.contains('\r') {
        return None;
    }
    decimal::<u32>(level)?;
    decimal(id)
}

fn await_hint(id: u64, deadline: Instant, mut trace: impl FnMut() -> Result<String>) -> Result<()> {
    loop {
        within(deadline)?;
        let observed = trace()?;
        within(deadline)?;
        if observed.lines().any(|row| parse_id(row) == Some(id)) {
            return Ok(());
        }
        std::thread::sleep(
            RETIREMENT_READ_INTERVAL.min(deadline.saturating_duration_since(Instant::now())),
        );
    }
}

fn matches_retirement(snap: &Snapshot, cpus: &[u32], id: u64) -> bool {
    let Some(body) = snap.trace.strip_prefix("# tracer: nop\n#\n") else {
        return false;
    };
    if snap.coverage_error.is_some() || !body.ends_with('\n') {
        return false;
    }
    let Some(entries) = entry_count(&snap.stats, cpus) else {
        return false;
    };
    let mut count = 0u64;
    let mut matched = 0u64;
    for line in body.split_terminator('\n') {
        let Some(observed) = parse_id(line) else {
            return false;
        };
        count += 1;
        matched += u64::from(observed == id);
    }
    count == entries && matched == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: u64 = 12_345_678_901; // Must not truncate to 32 bits.
    const FORMAT: &str = "name: cgroup_rmdir\nID: 496\n\tfield:int root; offset:8; size:4; signed:1;\n\tfield:u64 id; offset:16; size:8; signed:0;\n";
    fn stats(entries: u32) -> String {
        format!(
            "entries: {entries}\noverrun: 0\ncommit overrun: 0\ndropped events: 0\nread events: 0\n"
        )
    }
    fn sample() -> Snapshot {
        Snapshot {
            trace: format!(
                "# tracer: nop\n#\ncgroup_rmdir: root=0 id=43 level=2 path=/unrelated\ncgroup_rmdir: root=0 id={ID} level=2 path=/system.slice/original.scope\n"
            ),
            stats: vec![(0, stats(2)), (5, stats(0))],
            coverage_error: None,
        }
    }

    #[test]
    fn shared_tracefs_is_not_the_private_instance_boundary() {
        assert!(root_controls(0, 0, 0o755, false));
        assert!(!private_instance(0, 0o755, true));
        assert!(private_instance(0, 0o750, true));
        assert!(!private_instance(0, 0o750, false));
        assert!(!private_instance(42, 0o750, true));
        assert!(private_instance(42, 0o700, false));
        for (uid, gid, mode) in [(1, 0, 0o755), (0, 2, 0o775), (0, 0, 0o757)] {
            assert!(!root_controls(uid, gid, mode, true));
        }
    }

    #[test]
    fn possible_cpu_set_must_be_fully_enabled_in_mask() {
        assert_eq!(possible_cpus("0-1,4,33\n"), Some(vec![0, 1, 4, 33]));
        for text in ["0-256", "0,0", "1-0", "", "0-1,1-3"] {
            assert_eq!(possible_cpus(text), None);
        }
        assert!(in_mask("2,00000013\n", 33));
        assert!(in_mask("2,00000013\n", 4));
        assert!(!in_mask("0,00000013\n", 33));
        assert!(!in_mask("2,13\n", 33));
    }

    #[test]
    fn checks_format_without_assuming_event_number() {
        assert_eq!(event_format(FORMAT), Some(496));
        assert_eq!(event_format(&FORMAT.replace("496", "505")), Some(505));
        assert_eq!(event_format(&FORMAT.replace("size:8", "size:4")), None);
        assert_eq!(
            event_format(&FORMAT.replace("cgroup_rmdir", "cgroup_notify_populated")),
            None
        );
    }

    #[test]
    fn attributes_one_full_id_among_prebinding_records() {
        let cpus = &[0, 5];
        assert!(matches_retirement(&sample(), cpus, ID));
        assert!(!matches_retirement(&sample(), cpus, u64::from(ID as u32)));
        for (old, new) in [
            ("root=0", "root=1"),
            ("cgroup_rmdir:", "cgroup_notify_populated:"),
            ("original.scope\n", "original.scope"),
            ("/unrelated", "/unrelated\n# forged header"),
        ] {
            let mut s = sample();
            s.trace = s.trace.replace(old, new);
            assert!(!matches_retirement(&s, cpus, ID), "accepted {new}");
        }
        let event = format!("cgroup_rmdir: root=0 id={ID} level=2 path=/injected\n");
        let mut s = sample();
        s.trace.push_str(&event);
        s.stats[0].1 = stats(3);
        assert!(!matches_retirement(&s, cpus, ID)); // two matching records
        let mut s = sample();
        s.trace = s.trace.replace(&format!("id={ID}"), "id=44");
        s.trace.push_str(&event);
        assert!(!matches_retirement(&s, cpus, ID)); // injected matching line, too many rows
    }

    #[test]
    fn waits_through_early_exit_until_original_retirement_or_cutoff() {
        let mut reads = 0;
        let target = sample().trace;
        let unrelated = target.replace(&format!("id={ID}"), "id=44");
        await_hint(ID, Instant::now() + Duration::from_secs(1), || {
            reads += 1;
            Ok(if reads == 1 { unrelated.clone() } else { target.clone() })
        })
        .unwrap();
        assert_eq!(reads, 2);
        assert!(await_hint(ID, Instant::now(), || panic!("read past cutoff")).is_err());
    }

    #[test]
    fn refuses_loss_accounting_gaps() {
        let cpus = &[0, 5];
        let baseline = vec![(0, stats(0)), (5, stats(0))];
        assert_eq!(entry_count(&baseline, cpus), Some(0));
        assert!(!matches_retirement(&sample(), &[0, 1, 5], ID));
        assert!(!matches_retirement(&sample(), &[0], ID));
        for name in ["overrun", "commit overrun", "dropped events", "read events"] {
            let mut s = sample();
            s.stats[1].1 = stats(0).replace(&format!("\n{name}: 0"), &format!("\n{name}: 1"));
            assert!(!matches_retirement(&s, cpus, ID), "accepted loss: {name}");
        }
        let mut s = sample();
        s.stats[0].1 = s.stats[0].1.replace("commit overrun: 0\n", "");
        assert!(!matches_retirement(&s, cpus, ID));
        let mut s = sample();
        s.coverage_error = Some("mask crossed".into());
        assert!(!matches_retirement(&s, cpus, ID));
    }

    #[test]
    fn retains_unbound_raw_capture_in_existing_private_hash_chain() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let root_path = root.path().canonicalize().unwrap();
        fs::set_permissions(&root_path, fs::Permissions::from_mode(0o700)).unwrap();
        let mut journal = Custody::begin(&root_path, &"a".repeat(64)).unwrap();
        let arm = journal
            .record(Kind::SubtreeRetirementArm, json!({"filter": FILTER}))
            .unwrap();
        assert_eq!(
            record_snapshot(&mut journal, arm, None, Ok(sample()), &Ok(None)).unwrap(),
            2
        );
        let file = |pos| journal.attempt_path().join(format!("{pos:03}.json"));
        let read = |pos| fs::read(file(pos)).unwrap();
        let saved: serde_json::Value = serde_json::from_slice(&read(2)).unwrap();
        assert_eq!(saved["previous"], crate::digest(&read(1)));
        assert_eq!(saved["data"]["arm"], arm);
        assert!(saved["data"]["task"].is_null());
        assert_eq!(saved["data"]["sample"]["trace"], sample().trace);
        assert_eq!(fs::metadata(file(2)).unwrap().mode() & 0o777, 0o600);
    }
}
