//! Domain-separated, length-prefixed digests. No clock, no randomness.
use sha2::{Digest as _, Sha256};

pub fn sha256_hex(bytes: &[u8]) -> String {
    let d = Sha256::digest(bytes);
    d.iter().map(|b| format!("{b:02x}")).collect()
}

/// `H(tag, field...)` with 8-byte big-endian length prefixes.
pub fn digest_fields(tag: &str, fields: &[&[u8]]) -> String {
    let mut h = Sha256::new();
    h.update((tag.len() as u64).to_be_bytes());
    h.update(tag.as_bytes());
    for f in fields {
        h.update((f.len() as u64).to_be_bytes());
        h.update(f);
    }
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// `operation_id = H("p1f.operation.v1", admission_id, obligation_id, execution_id)`.
pub fn derived_operation_id(admission_id: &str, obligation_id: &str, execution_id: &str) -> String {
    digest_fields(
        "p1f.operation.v1",
        &[
            admission_id.as_bytes(),
            obligation_id.as_bytes(),
            execution_id.as_bytes(),
        ],
    )
}
