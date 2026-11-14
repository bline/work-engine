//! Bounded plain FIFO framing. Codec state is not a dispatch/publication permit.
use std::fmt;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    Bound,
    Partial,
    Identity,
    Sequence,
    Encoding,
}
impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Bound => "frame_bound",
            Self::Partial => "partial_frame",
            Self::Identity => "frame_identity",
            Self::Sequence => "frame_sequence",
            Self::Encoding => "frame_encoding",
        })
    }
}
impl std::error::Error for Refusal {}
pub const HEADER: usize = 53;
pub const MAX_FRAME: usize = 4096;
pub const MAX_TOTAL: usize = 32768;
pub const MAX_FRAMES: u64 = 8;
#[derive(Clone, Copy)]
pub struct Binding {
    pub native: [u8; 16],
    pub attempt: u64,
    pub generation: u64,
    pub operation: u64,
    pub stream: u32,
}
pub const FIXTURE: Binding = Binding {
    native: [19; 16],
    attempt: 1,
    generation: 1,
    operation: 7,
    stream: 1,
};
pub struct Codec {
    binding: Binding,
    next: u64,
    bytes: usize,
}
impl Codec {
    pub fn new(binding: Binding) -> Self {
        Self {
            binding,
            next: 0,
            bytes: 0,
        }
    }
    pub fn accept<'a>(&mut self, frame: &'a [u8]) -> Result<&'a [u8], Refusal> {
        if frame.len() < HEADER {
            return Err(Refusal::Partial);
        }
        if frame.len() > MAX_FRAME
            || self.next >= MAX_FRAMES
            || self
                .bytes
                .checked_add(frame.len())
                .filter(|v| *v <= MAX_TOTAL)
                .is_none()
        {
            return Err(Refusal::Bound);
        }
        if frame[0] != 0 {
            return Err(Refusal::Encoding);
        } // Plain only. No decompressor/retry fallback.
        let u64_at = |i| u64::from_be_bytes(frame[i..i + 8].try_into().unwrap());
        if frame[1..17] != self.binding.native
            || u64_at(17) != self.binding.attempt
            || u64_at(25) != self.binding.generation
            || u64_at(33) != self.binding.operation
            || u32::from_be_bytes(frame[49..53].try_into().unwrap()) != self.binding.stream
        {
            return Err(Refusal::Identity);
        }
        if u64_at(41) != self.next {
            return Err(Refusal::Sequence);
        }
        self.next += 1;
        self.bytes += frame.len();
        Ok(&frame[HEADER..])
    }
}
pub fn encode(binding: Binding, sequence: u64, payload: &[u8]) -> Result<Vec<u8>, Refusal> {
    if payload.len() > MAX_FRAME - HEADER {
        return Err(Refusal::Bound);
    }
    let mut out = Vec::with_capacity(4 + HEADER + payload.len());
    out.extend_from_slice(&((HEADER + payload.len()) as u32).to_be_bytes());
    out.push(0);
    out.extend_from_slice(&binding.native);
    for v in [
        binding.attempt,
        binding.generation,
        binding.operation,
        sequence,
    ] {
        out.extend_from_slice(&v.to_be_bytes());
    }
    out.extend_from_slice(&binding.stream.to_be_bytes());
    out.extend_from_slice(payload);
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identity_sequence_encoding_and_aggregate_bounds() {
        let good = encode(FIXTURE, 0, b"PUBLIC").unwrap();
        let mut c = Codec::new(FIXTURE);
        for offset in [1, 17, 25, 33, 49] {
            let mut f = good[4..].to_vec();
            f[offset] ^= 1;
            assert_eq!(c.accept(&f), Err(Refusal::Identity));
        }
        let mut compressed = good[4..].to_vec();
        compressed[0] = 1;
        assert_eq!(c.accept(&compressed), Err(Refusal::Encoding));
        assert_eq!(c.accept(&good[4..]), Ok(&b"PUBLIC"[..]));
        assert_eq!(c.accept(&good[4..]), Err(Refusal::Sequence));
        assert_eq!(c.accept(&[]), Err(Refusal::Partial));
        let mut full = Codec::new(FIXTURE);
        for seq in 0..8 {
            let frame = encode(FIXTURE, seq, &[0; MAX_FRAME - HEADER]).unwrap();
            assert!(full.accept(&frame[4..]).is_ok());
        }
        assert_eq!(
            full.accept(&encode(FIXTURE, 8, b"x").unwrap()[4..]),
            Err(Refusal::Bound)
        );
    }
}
