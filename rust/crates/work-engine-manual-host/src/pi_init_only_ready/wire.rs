//! Single Moby HTTP/1.1 hijacked attach, non-TTY stdout mux, one B marker.
use super::{Refusal, artifact_contract::CHECKPOINT};

const MAX_UPGRADE: usize = 512;

#[derive(Default)]
pub(super) struct Stream {
    header: Vec<u8>,
    upgraded: bool,
    mux: [u8; 8],
    pending: usize,
    payload: usize,
    offset: usize,
}

impl Stream {
    /// Consume bounded chunks in capture order, including any stdout written
    /// between verified Start issuance and its separately captured reply.
    pub fn feed(&mut self, bytes: &[u8], issued: bool) -> Result<(), Refusal> {
        for &byte in bytes {
            if !self.upgraded {
                if self.header.len() == MAX_UPGRADE {
                    return Err(Refusal::Upgrade);
                }
                self.header.push(byte);
                if self.header.ends_with(b"\r\n\r\n") {
                    check_upgrade(&self.header)?;
                    self.upgraded = true;
                }
                continue;
            }
            if !issued {
                return Err(Refusal::EarlyStream);
            }
            if self.offset == CHECKPOINT.len() {
                return Err(Refusal::ContaminatedStream);
            }
            if self.payload == 0 {
                self.mux[self.pending] = byte;
                self.pending += 1;
                if self.pending == self.mux.len() {
                    let length = u32::from_be_bytes(self.mux[4..].try_into().unwrap());
                    if self.mux[..4] != [1, 0, 0, 0]
                        || length == 0
                        || u64::from(length) > (CHECKPOINT.len() - self.offset) as u64
                    {
                        return Err(Refusal::ContaminatedStream);
                    }
                    self.pending = 0;
                    self.payload = length as usize;
                }
                continue;
            }
            if byte != CHECKPOINT[self.offset] {
                return Err(Refusal::ContaminatedStream);
            }
            self.offset += 1;
            self.payload -= 1;
        }
        Ok(())
    }

    pub(super) fn upgraded(&self) -> bool {
        self.upgraded
    }

    pub(super) fn complete(&self) -> bool {
        self.offset == CHECKPOINT.len() && self.pending == 0 && self.payload == 0
    }
}

fn check_upgrade(header: &[u8]) -> Result<(), Refusal> {
    let text = std::str::from_utf8(header).map_err(|_| Refusal::Upgrade)?;
    let mut lines = text.trim_end_matches("\r\n\r\n").split("\r\n");
    if lines.next() != Some("HTTP/1.1 101 UPGRADED") {
        return Err(Refusal::Upgrade);
    }
    let mut seen = 0u8;
    for line in lines {
        let (name, value) = line.split_once(':').ok_or(Refusal::Upgrade)?;
        let value = value.trim_matches([' ', '\t']);
        let bit =
            if name.eq_ignore_ascii_case("connection") && value.eq_ignore_ascii_case("upgrade") {
                1
            } else if name.eq_ignore_ascii_case("upgrade") && value.eq_ignore_ascii_case("tcp") {
                2
            } else if name.eq_ignore_ascii_case("content-type")
                && value.eq_ignore_ascii_case("application/vnd.docker.multiplexed-stream")
            {
                4
            } else {
                return Err(Refusal::Upgrade);
            };
        if seen & bit != 0 {
            return Err(Refusal::Upgrade);
        }
        seen |= bit;
    }
    (seen == 7).then_some(()).ok_or(Refusal::Upgrade)
}
