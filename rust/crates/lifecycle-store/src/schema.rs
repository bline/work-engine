//! Physical schema8/9 add diagnostics to semantic schema6/7 respectively, and
//! schema10/11 add direct semantic control to 8/9.
//! This mapping is for behavior dispatch only; physical identity remains visible.
pub(crate) fn behavior_version(version: i64) -> i64 {
    match version {
        8 | 10 => 6,
        9 | 11 => 7,
        other => other,
    }
}
