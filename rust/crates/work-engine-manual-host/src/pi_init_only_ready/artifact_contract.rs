//! Fixed Linux image-native B qualification subject (not a workload).
pub const ENTRYPOINT: &str = "/work-engine-b-init-only";
pub const USER: u32 = 65534;
pub const GROUP: u32 = 65534;
pub const HOLD_SECONDS: u64 = 30;
pub const PRIVATE_UMASK: u16 = 0o077;
pub const CHECKPOINT: &[u8] = b"work-engine.b.init-only-held.v1\n";

pub fn reduced_groups(groups: &[u32]) -> bool {
    // Some OCI launchers explicitly include the primary group. This grants
    // no additional group authority. No other group is accepted.
    matches!(groups, [] | [GROUP])
}
