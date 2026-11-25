use super::*;
use crate::pi_docker::init_only_ready::{STOCK_STATIC_DIRS, tests::Synthetic};

fn thermal(cpu: u32) -> String {
    format!("/sys/devices/system/cpu/cpu{cpu}/thermal_throttle")
}

const FIXED: &str = concat!(
    "11 1 0:10 / / ro,relatime - overlay overlay rw,lowerdir=layers\n",
    "12 11 0:12 / /proc rw,nosuid,nodev,noexec,relatime - proc proc rw\n",
    "13 11 0:13 / /dev rw,nosuid - tmpfs tmpfs rw,size=65536k,mode=755,inode64\n",
    "14 13 0:14 / /dev/mqueue rw,nosuid,nodev,noexec,relatime - mqueue mqueue rw\n",
    "15 13 0:15 / /dev/pts rw,nosuid,noexec,relatime - devpts devpts rw,gid=5,mode=620\n",
    "16 13 0:16 / /dev/shm rw,nosuid,nodev,noexec,relatime - tmpfs shm rw,size=65536k\n",
    "17 11 0:17 / /sys ro,nosuid,nodev,noexec,relatime - sysfs sysfs ro\n",
    "18 17 0:18 /system.slice/docker-full.scope /sys/fs/cgroup ro,nosuid,nodev,noexec,relatime - cgroup2 cgroup rw\n",
    "19 11 8:1 /var/lib/docker/containers/full/hostname /etc/hostname ro,relatime - ext4 /dev/sda1 rw\n",
    "20 11 8:1 /var/lib/docker/containers/full/hosts /etc/hosts ro,relatime - ext4 /dev/sda1 rw\n",
    "21 11 8:1 /var/lib/docker/containers/full/resolv.conf /etc/resolv.conf ro,relatime - ext4 /dev/sda1 rw\n",
);
const ABSENT: [&str; 2] = ["/proc/sched_debug", "/proc/timer_stats"];
const MASK_OPTS: &str = "ro,size=4k,nr_inodes=1,inode64";

struct Layout {
    raw: String,
    nodes: BTreeMap<String, MaskNode>,
    masked: Vec<String>,
    cpus: String,
}

fn sample(mount_id: u64, device: (u32, u32), inode: u64, mode: u32) -> MaskNode {
    MaskNode {
        mount_id,
        device,
        inode,
        mode,
        rdev: (0, 0),
    }
}

impl Layout {
    fn new(n: u32) -> Self {
        let mut raw = FIXED.to_string();
        let mut nodes = BTreeMap::from([
            ("/".into(), sample(11, (0, 10), 40, libc::S_IFDIR | 0o755)),
            ("/sys".into(), sample(17, (0, 17), 1, libc::S_IFDIR | 0o555)),
            (
                "/dev/null".into(),
                MaskNode {
                    rdev: (1, 3),
                    ..sample(13, (0, 13), 3, libc::S_IFCHR | 0o666)
                },
            ),
        ]);
        for (p, id) in GENERATED_FILES.into_iter().zip([20, 19, 21]) {
            nodes.insert(p.into(), sample(id, (8, 1), id + 10, libc::S_IFREG | 0o644));
        }
        let mut id = 22u64;
        for p in PROC_READONLY {
            raw += &format!(
                "{id} 12 0:12 {} {p} ro,nosuid,nodev,noexec,relatime - proc proc rw\n",
                &p[5..]
            );
            id += 1;
        }
        for p in FILE_MASKS.into_iter().filter(|p| !ABSENT.contains(p)) {
            raw += &format!(
                "{id} 12 0:13 /null {p} rw,nosuid - tmpfs tmpfs rw,size=65536k,mode=755,inode64\n"
            );
            nodes.insert(
                p.into(),
                MaskNode {
                    mount_id: id,
                    ..nodes["/dev/null"]
                },
            );
            id += 1;
        }
        let mut masked: Vec<_> = FILE_MASKS.into_iter().map(str::to_string).collect();
        for (i, p) in STOCK_STATIC_DIRS
            .into_iter()
            .map(str::to_string)
            .chain((0..n).map(thermal))
            .enumerate()
        {
            let parent = if p.starts_with("/sys/") { 17 } else { 12 };
            // Most rows share a superblock/root inode, one exercises individual
            // tmpfs fallback, but each has its own mount identity.
            let minor = if i == 2 { 98 } else { 99 };
            raw +=
                &format!("{id} {parent} 0:{minor} / {p} ro,relatime - tmpfs tmpfs {MASK_OPTS}\n");
            nodes.insert(p.clone(), sample(id, (0, minor), 1, libc::S_IFDIR | 0o755));
            masked.push(p);
            id += 1;
        }
        Self {
            raw,
            nodes,
            masked,
            cpus: format!("0-{}\n", n - 1),
        }
    }

    fn projected(&self) -> io::Result<StockMountCut> {
        project(
            self.raw.as_bytes(),
            self.cpus.clone(),
            &self.masked,
            51,
            self.nodes["/"],
            |p| Ok(self.nodes.get(p).copied()),
        )
    }

    fn replace_row(&mut self, path: &str, change: impl FnOnce(&str) -> String) {
        let row = self
            .raw
            .lines()
            .find(|l| l.split_whitespace().nth(4) == Some(path))
            .unwrap()
            .to_string();
        self.raw = self.raw.replacen(&format!("{row}\n"), &change(&row), 1);
    }
}

#[test]
fn stock_mount_inventory_reconciles_complete_historical_and_larger_layouts() {
    // #6411 retained the exact 37 mount targets but not backing/statx.
    // The complete target set is its regression input; coordinates/options
    // here are modeled from pinned Moby/runc, not a new host observation.
    for n in [12, 48] {
        let layout = Layout::new(n);
        let cut = layout.projected().unwrap();
        assert_eq!(cut.mounts.len(), (25 + n) as usize);
        assert_eq!(cut.directory_masks.len(), (5 + n) as usize);
        assert_eq!(cut.generated_files.len(), 3);
        assert_eq!(
            cut.mounts
                .iter()
                .filter(|m| matches!(m.kind, MountKind::RuntimeMaskedFile) && !m.read_only)
                .count(),
            5
        );
        let mut fixture = Synthetic::new();
        let sample = fixture.cut();
        sample.launch.runtime = "runc".into();
        sample.launch.masked_paths = layout.masked;
        sample.launch.proc_readonly_paths = PROC_READONLY.into_iter().map(str::to_string).collect();
        sample.launch.root_mount_id = 11;
        let inv = sample.effective.inventory.as_mut().unwrap();
        inv.mounts = cut.mounts;
        assert!(inv.closed(&sample.launch, 51, true));
        assert!(!inv.closed(&sample.launch, 51, false)); // no new Stop authority
    }
}

#[test]
fn stock_mount_inventory_rejects_unknown_or_contradictory_complete_rows() {
    let p = thermal(0);
    for (old, new) in [
        (" ro,relatime", " rw,relatime"),
        (" ro,relatime", " ro,rw,relatime"),
        ("- tmpfs tmpfs ro,", "- tmpfs tmpfs rw,"),
        ("- tmpfs tmpfs ro,", "- ext4 tmpfs ro,"),
        ("- tmpfs tmpfs ro,", "- tmpfs substitute ro,"),
        ("nr_inodes=1", "nr_inodes=2"),
        (" 17 0:99 / ", " 12 0:99 / "),
        (" 17 0:99 / ", " 17 0:99 /child "),
    ] {
        let mut x = Layout::new(12);
        x.replace_row(&p, |l| {
            assert!(l.contains(old));
            format!("{}\n", l.replacen(old, new, 1))
        });
        assert!(x.projected().is_err(), "accepted {old} => {new}");
    }
    for missing in [
        p.as_str(),
        "/dev/pts",
        "/sys/fs/cgroup",
        "/proc/fs",
        "/etc/hosts",
    ] {
        let mut x = Layout::new(12);
        x.replace_row(missing, |_| String::new());
        assert!(x.projected().is_err(), "missing {missing}");
    }
    let mut duplicate = Layout::new(12);
    duplicate.replace_row(&p, |l| format!("{l}\n{l}\n"));
    assert!(duplicate.projected().is_err());
    let mut duplicate_id = Layout::new(12);
    duplicate_id.replace_row(&p, |l| format!("17 {}\n", l.split_once(' ').unwrap().1));
    assert!(duplicate_id.projected().is_err());
    let mut unknown = Layout::new(12);
    unknown.raw += "200 11 0:90 / /unexpected ro - tmpfs tmpfs ro,size=4k,nr_inodes=1\n";
    assert!(unknown.projected().is_err());
    let mut unrelated_writable = Layout::new(12);
    unrelated_writable.replace_row("/etc/hosts", |l| {
        format!("{}\n", l.replacen("ro,relatime", "rw,relatime", 1))
    });
    assert!(unrelated_writable.projected().is_err());
    let mut wrong_cpu = Layout::new(12);
    wrong_cpu.cpus = "0-10\n".to_string();
    assert!(wrong_cpu.projected().is_err());
    let mut not_selected = Layout::new(12);
    not_selected.masked.retain(|v| v != &p);
    assert!(not_selected.projected().is_err());
    let mut duplicate_selection = Layout::new(12);
    duplicate_selection.masked.push(p.clone());
    assert!(duplicate_selection.projected().is_err());
    let mut wrong_kind = Layout::new(12);
    wrong_kind.nodes.get_mut(&p).unwrap().mode = libc::S_IFREG;
    assert!(wrong_kind.projected().is_err());
    let mut wrong_mount = Layout::new(12);
    wrong_mount.nodes.get_mut(&p).unwrap().mount_id = 17;
    assert!(wrong_mount.projected().is_err());
    let mut inconsistent = Layout::new(12);
    inconsistent.nodes.get_mut(&p).unwrap().inode += 1;
    assert!(inconsistent.projected().is_err());
    let mut false_absence = Layout::new(12);
    false_absence.replace_row("/proc/acpi", |_| String::new());
    assert!(false_absence.projected().is_err());
    false_absence.nodes.remove("/proc/acpi");
    assert!(false_absence.projected().is_ok());
    let mut null_substitute = Layout::new(12);
    null_substitute.nodes.get_mut("/proc/kcore").unwrap().rdev = (1, 5);
    assert!(null_substitute.projected().is_err());
}
