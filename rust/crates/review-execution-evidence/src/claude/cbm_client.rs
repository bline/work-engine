//! Reviewer-side CBM stdio adapter. The only reachable object is the pinned,
//! candidate-specific Unix socket. No CBM binary, cache, or daemon is mounted.

use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;

const SOCKET: &str = "/cbm/socket/analysis.sock";
const MAX_LINE: usize = 64 * 1024;

fn run() -> io::Result<()> {
    if std::env::args_os().len() != 1 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "unexpected argument",
        ));
    }
    let stream = UnixStream::connect(SOCKET)?;
    let mut outbound = stream.try_clone()?;
    let mut inbound = BufReader::new(stream);
    let stdin = io::stdin();
    let mut stdin = stdin.lock();
    let mut stdout = io::stdout().lock();
    loop {
        let mut request = Vec::new();
        if stdin
            .by_ref()
            .take((MAX_LINE + 1) as u64)
            .read_until(b'\n', &mut request)?
            == 0
        {
            return Ok(());
        }
        if request.len() > MAX_LINE || request.last() != Some(&b'\n') {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "oversize MCP request",
            ));
        }
        outbound.write_all(&request)?;
        outbound.flush()?;
        let value: serde_json::Value = serde_json::from_slice(&request)?;
        if value.get("id").is_none() {
            continue;
        }
        let mut response = Vec::new();
        if inbound
            .by_ref()
            .take((1024 * 1024 + 1) as u64)
            .read_until(b'\n', &mut response)?
            == 0
        {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "MCP relay closed",
            ));
        }
        if response.len() > 1024 * 1024 || response.last() != Some(&b'\n') {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "oversize MCP response",
            ));
        }
        stdout.write_all(&response)?;
        stdout.flush()?;
    }
}

use std::io::Read;

fn main() {
    if let Err(error) = run() {
        eprintln!("RP0 CBM client refused: {error}");
        std::process::exit(1);
    }
}
