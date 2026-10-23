//! File-byte sha256 identity. No canonical codec is introduced (contract S1).

use sha2::{Digest, Sha256};
use std::fmt::Write;

/// Lowercase hexadecimal sha256 of `bytes`.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for byte in digest.iter() {
        // Writing to a String cannot fail.
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// True for exactly 64 lowercase hexadecimal characters.
pub fn is_sha256_hex(text: &str) -> bool {
    text.len() == 64
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn hex_shape() {
        assert!(is_sha256_hex(&sha256_hex(b"x")));
        assert!(!is_sha256_hex(&sha256_hex(b"x").to_uppercase()));
        assert!(!is_sha256_hex("abc"));
    }
}
