//! Original-task namespace and network readback for the natural-only Held cut.

use super::super::{Effective, Result, refuse, within};
use super::{Anchor, KernelTask, bounded, proc_path};
use serde::Serialize;
use std::{
    fs::{self, File},
    os::unix::fs::MetadataExt,
    time::Instant,
};

#[derive(Serialize)]
pub(in super::super) struct IsolationCut {
    pub(in super::super) task: [u64; 6],
    pub(in super::super) host: [u64; 6],
    interfaces: Vec<u8>,
    ipv4_routes: Vec<u8>,
    ipv6_routes: Vec<u8>,
}

fn closed_net(dev: &[u8], route4: &[u8], route6: &[u8]) -> bool {
    let (Ok(dev), Ok(route4), Ok(route6)) = (
        std::str::from_utf8(dev),
        std::str::from_utf8(route4),
        std::str::from_utf8(route6),
    ) else {
        return false;
    };
    let mut interfaces = dev.lines();
    let mut routes4 = route4.lines();
    if !dev.ends_with('\n')
        || !route4.ends_with('\n')
        || (!route6.is_empty() && !route6.ends_with('\n'))
        || !interfaces.next().is_some_and(|s| {
            s.split('|')
                .map(str::trim)
                .eq(["Inter-", "Receive", "Transmit"])
        })
        || !interfaces.next().is_some_and(|s| s.starts_with(" face |"))
        || !routes4.next().is_some_and(|s| {
            s.split_ascii_whitespace().eq([
                "Iface",
                "Destination",
                "Gateway",
                "Flags",
                "RefCnt",
                "Use",
                "Metric",
                "Mask",
                "MTU",
                "Window",
                "IRTT",
            ])
        })
    {
        return false;
    }
    let loopback = |line: &str| {
        let Some((interface, counters)) = line.split_once(':') else {
            return false;
        };
        let counters = counters.split_ascii_whitespace().collect::<Vec<_>>();
        interface.trim() == "lo"
            && counters.len() == 16
            && counters.iter().all(|c| c.parse::<u64>().is_ok())
    };
    interfaces.next().is_some_and(loopback)
        && interfaces.next().is_none()
        && routes4.all(|line| {
            let columns = line.split_ascii_whitespace().collect::<Vec<_>>();
            columns.len() == 11
                && columns[0] == "lo"
                && columns[1..]
                    .iter()
                    .zip([16, 16, 16, 10, 10, 10, 16, 10, 10, 10])
                    .all(|(value, radix)| u64::from_str_radix(value, radix).is_ok())
        })
        && route6.lines().all(|line| {
            let columns = line.split_ascii_whitespace().collect::<Vec<_>>();
            columns.len() == 10
                && columns[9] == "lo"
                && columns[..9]
                    .iter()
                    .zip([32, 2, 32, 2, 32, 8, 8, 8, 8])
                    .all(|(value, len)| {
                        value.len() == len && value.bytes().all(|c| c.is_ascii_hexdigit())
                    })
        })
}

fn namespaces(pid: u32) -> std::io::Result<[u64; 6]> {
    let mut result = [0; 6];
    for (name, id) in ["pid", "mnt", "ipc", "net", "cgroup", "uts"]
        .into_iter()
        .zip(&mut result)
    {
        *id = fs::metadata(proc_path(pid, &format!("ns/{name}")))?.ino();
    }
    Ok(result)
}

impl KernelTask {
    pub(in super::super) fn stock_isolation(
        &self,
        effective: &Effective,
        deadline: Instant,
    ) -> Result<IsolationCut> {
        let cut = self.observe_stock_isolation(deadline)?;
        if effective.task != self.key
            || cut.task != effective.namespaces.task
            || cut.host != effective.namespaces.host
        {
            return refuse("namespace readback crossed the applied witness");
        }
        Ok(cut)
    }

    pub(in super::super) fn observe_stock_isolation(
        &self,
        deadline: Instant,
    ) -> Result<IsolationCut> {
        self.live_events()?;
        within(deadline)?;
        let p = self.key.pid;
        let task = namespaces(p)?;
        let host = namespaces(1)?;
        if task[0] != self.key.pid_namespace
            || task
                .into_iter()
                .zip(host)
                .any(|(t, h)| t == 0 || h == 0 || t == h)
        {
            return refuse("observed namespace identities did not join the closed original task");
        }
        let read = |name| bounded(File::open(proc_path(p, name))?, 16 * 1024);
        let interfaces = read("net/dev")?;
        let ipv4_routes = read("net/route")?;
        let ipv6_routes = read("net/ipv6_route")?;
        if !closed_net(&interfaces, &ipv4_routes, &ipv6_routes)
            || namespaces(p)? != task
            || namespaces(1)? != host
        {
            return refuse(
                "original task interface/route readback is not positively loopback-only",
            );
        }
        self.live_events()?;
        within(deadline)?;
        Ok(IsolationCut {
            task,
            host,
            interfaces,
            ipv4_routes,
            ipv6_routes,
        })
    }
}

#[test]
fn stock_isolation_requires_complete_loopback_interfaces_and_routes() {
    let interfaces = b"Inter-|   Receive                                                |  Transmit\n face |by rows\n   lo: 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0\n";
    let v4 = b"Iface Destination Gateway Flags RefCnt Use Metric Mask MTU Window IRTT\n";
    let v6 = b"00000000000000000000000000000000 00 00000000000000000000000000000000 00 00000000000000000000000000000000 ffffffff 00000001 00000000 00200200 lo\n";
    assert!(closed_net(interfaces, v4, v6));
    let foreign_link = [&interfaces[..], b"eth0: 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0\n"].concat();
    let foreign_v4 = [
        &v4[..],
        b"eth0 00000000 00000000 0001 0 0 0 00000000 0 0 0\n",
    ]
    .concat();
    let foreign_v6 = std::str::from_utf8(v6).unwrap().replace(" lo\n", " eth0\n");
    assert!(!closed_net(&foreign_link, v4, v6));
    assert!(!closed_net(interfaces, &foreign_v4, v6));
    assert!(!closed_net(interfaces, v4, foreign_v6.as_bytes()));
    assert!(!closed_net(interfaces, &v4[..v4.len() - 1], v6));
}
