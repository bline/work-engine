//! Read-only child of custody parent. Descriptor provenance belongs to parent.
use review_execution_evidence::pi::{ExpectedSeal, SealedReader};
use std::os::fd::FromRawFd;
fn main() {
    let r = (|| {
        let a: Vec<_> = std::env::args().collect();
        let fd: i32 = a.get(2).ok_or("fd")?.parse().map_err(|_| "fd")?;
        if fd < 3 {
            return Err("fd");
        }
        let f = unsafe { std::fs::File::from_raw_fd(fd) };
        let expected = ExpectedSeal::from_custody_descriptor(&f)?;
        let reader = SealedReader::open(std::path::Path::new(a.get(1).ok_or("root")?), expected)?;
        reader.read(
            a.get(3)
                .map(|s| s.parse())
                .transpose()
                .map_err(|_| "ordinal")?
                .unwrap_or(2),
        )?;
        Ok::<(), &'static str>(())
    })();
    if r.is_err() {
        std::process::exit(1);
    }
}
