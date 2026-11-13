//! Digest framing (contract section 17.5).
//!
//! `sha256("work-engine.organization" 0x00 kind 0x00 "v0" 0x00 canonical bytes)`, where kind is
//! `requirements` or `candidate`. This crate is the only owner of these two kinds.

use std::fmt;

use sha2::{Digest as _, Sha256};

use crate::error::CodecError;

/// The domain separator of every organizational digest.
pub const CODEC_DOMAIN: &str = "work-engine.organization";
/// The version of the canonical form and the framing.
pub const CODEC_VERSION: &str = "v0";

/// The two digest kinds owned by this crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DigestKind {
    Requirements,
    Candidate,
}

impl DigestKind {
    /// The kind spelling that enters the framing.
    pub fn as_str(self) -> &'static str {
        match self {
            DigestKind::Requirements => "requirements",
            DigestKind::Candidate => "candidate",
        }
    }

    /// Resolves a declared codec (domain, kind, version). Anything this crate does not own is
    /// refused: other kinds (`acceptance`, `admission`, ...) belong to the authority crate, and
    /// another version is another codec.
    pub fn from_declaration(
        domain: &str,
        kind: &str,
        version: &str,
    ) -> Result<DigestKind, CodecError> {
        let resolved = match kind {
            "requirements" => Some(DigestKind::Requirements),
            "candidate" => Some(DigestKind::Candidate),
            _ => None,
        };
        match resolved {
            Some(resolved) if domain == CODEC_DOMAIN && version == CODEC_VERSION => Ok(resolved),
            _ => Err(CodecError::UnknownCodec {
                domain: domain.to_owned(),
                kind: kind.to_owned(),
                version: version.to_owned(),
            }),
        }
    }
}

/// A SHA-256 organizational digest.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Digest([u8; 32]);

impl Digest {
    /// Frames and hashes already-canonical bytes.
    pub(crate) fn frame(kind: DigestKind, canonical_bytes: &[u8]) -> Digest {
        let mut hasher = Sha256::new();
        hasher.update(CODEC_DOMAIN.as_bytes());
        hasher.update([0u8]);
        hasher.update(kind.as_str().as_bytes());
        hasher.update([0u8]);
        hasher.update(CODEC_VERSION.as_bytes());
        hasher.update([0u8]);
        hasher.update(canonical_bytes);
        Digest(hasher.finalize().into())
    }

    /// Parses the text form of section 3.1 inputs: exactly 64 lowercase hexadecimal characters.
    pub(crate) fn from_hex(text: &str) -> Option<Digest> {
        let bytes = text.as_bytes();
        if bytes.len() != 64 {
            return None;
        }
        let mut out = [0u8; 32];
        for (index, pair) in bytes.chunks(2).enumerate() {
            let nibble = |byte: u8| match byte {
                b'0'..=b'9' => Some(byte - b'0'),
                b'a'..=b'f' => Some(byte - b'a' + 10),
                _ => None,
            };
            out[index] = nibble(pair[0])? << 4 | nibble(pair[1])?;
        }
        Some(Digest(out))
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Lowercase hexadecimal, 64 characters. The textual form used inside other documents
    /// (for example a `sha256:` prefix) is not decided by this crate.
    pub fn to_hex(&self) -> String {
        self.0.iter().map(|byte| format!("{byte:02x}")).collect()
    }
}

impl fmt::Debug for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Digest({})", self.to_hex())
    }
}

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}
