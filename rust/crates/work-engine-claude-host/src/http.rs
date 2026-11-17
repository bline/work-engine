//! Minimal HTTP/1.1 framing for the loopback proxy. Bounded throughout.

use std::io::{BufRead, Read, Write};

pub const MAX_HEAD: usize = 64 * 1024;

#[derive(Debug, Clone)]
pub struct Head {
    pub start: String,
    pub headers: Vec<(String, String)>,
}

impl Head {
    pub fn get(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

/// Read one head (start line and headers). `Ok(None)` at clean EOF before any byte.
pub fn read_head<R: BufRead>(r: &mut R) -> std::io::Result<Option<Head>> {
    let mut lines = Vec::new();
    let mut total = 0usize;
    loop {
        let mut line = Vec::new();
        let n = r
            .by_ref()
            .take((MAX_HEAD + 1) as u64)
            .read_until(b'\n', &mut line)?;
        if n == 0 {
            if lines.is_empty() {
                return Ok(None);
            }
            return Err(invalid("eof inside head"));
        }
        total += n;
        if total > MAX_HEAD || !line.ends_with(b"\n") {
            return Err(invalid("head too large"));
        }
        let text = String::from_utf8(line).map_err(|_| invalid("non-utf8 head"))?;
        let text = text.trim_end_matches(['\r', '\n']).to_owned();
        if text.is_empty() {
            break;
        }
        lines.push(text);
    }
    let start = lines.remove(0);
    let mut headers = Vec::new();
    for l in lines {
        let (k, v) = l
            .split_once(':')
            .ok_or_else(|| invalid("malformed header"))?;
        headers.push((k.trim().to_owned(), v.trim().to_owned()));
    }
    Ok(Some(Head { start, headers }))
}

pub fn invalid(m: &'static str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, m)
}

/// Read exactly `len` bytes, refusing more than `max`.
pub fn read_body<R: Read>(r: &mut R, len: usize, max: usize) -> std::io::Result<Vec<u8>> {
    if len > max {
        return Err(invalid("body exceeds bound"));
    }
    let mut body = vec![0; len];
    r.read_exact(&mut body)?;
    Ok(body)
}

/// Decode a chunked body, calling `sink` with each piece. Stops with an error
/// past `max` total bytes.
pub fn read_chunked<R: BufRead>(
    r: &mut R,
    max: usize,
    sink: &mut dyn FnMut(&[u8]) -> std::io::Result<()>,
) -> std::io::Result<usize> {
    let mut total = 0usize;
    loop {
        let mut line = Vec::new();
        r.by_ref().take(1024).read_until(b'\n', &mut line)?;
        let text = std::str::from_utf8(&line).map_err(|_| invalid("chunk size"))?;
        let size_text = text.trim().split(';').next().unwrap_or("");
        let size = usize::from_str_radix(size_text, 16).map_err(|_| invalid("chunk size"))?;
        if size == 0 {
            // Trailers until the empty line.
            loop {
                let mut t = Vec::new();
                let n = r.by_ref().take(8192).read_until(b'\n', &mut t)?;
                if n == 0 || t == b"\r\n" || t == b"\n" {
                    return Ok(total);
                }
            }
        }
        total = total
            .checked_add(size)
            .ok_or_else(|| invalid("chunk overflow"))?;
        if total > max {
            return Err(invalid("response exceeds bound"));
        }
        let mut chunk = vec![0; size];
        r.read_exact(&mut chunk)?;
        let mut crlf = [0u8; 2];
        r.read_exact(&mut crlf)?;
        if &crlf != b"\r\n" {
            return Err(invalid("chunk terminator"));
        }
        sink(&chunk)?;
    }
}

/// Copy a body of known or unknown (until EOF) length in pieces.
pub fn read_plain<R: Read>(
    r: &mut R,
    len: Option<usize>,
    max: usize,
    sink: &mut dyn FnMut(&[u8]) -> std::io::Result<()>,
) -> std::io::Result<usize> {
    let mut total = 0usize;
    let mut buf = [0u8; 16384];
    loop {
        let want = match len {
            Some(l) if total >= l => return Ok(total),
            Some(l) => (l - total).min(buf.len()),
            None => buf.len(),
        };
        let n = r.read(&mut buf[..want])?;
        if n == 0 {
            if len.is_some() {
                return Err(invalid("eof inside body"));
            }
            return Ok(total);
        }
        total += n;
        if total > max {
            return Err(invalid("response exceeds bound"));
        }
        sink(&buf[..n])?;
    }
}

pub fn write_chunk<W: Write>(w: &mut W, data: &[u8]) -> std::io::Result<()> {
    if data.is_empty() {
        return Ok(());
    }
    write!(w, "{:x}\r\n", data.len())?;
    w.write_all(data)?;
    w.write_all(b"\r\n")?;
    w.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn head_and_chunked_round_trip() {
        let raw = b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n6;x=y\r\n world\r\n0\r\nTrailer: a\r\n\r\n";
        let mut r = std::io::BufReader::new(&raw[..]);
        let head = read_head(&mut r).unwrap().unwrap();
        assert_eq!(head.start, "HTTP/1.1 200 OK");
        assert_eq!(head.get("transfer-encoding"), Some("chunked"));
        let mut out = Vec::new();
        let n = read_chunked(&mut r, 100, &mut |c| {
            out.extend_from_slice(c);
            Ok(())
        })
        .unwrap();
        assert_eq!(n, 11);
        assert_eq!(out, b"hello world");
    }

    #[test]
    fn bounds_are_enforced() {
        let raw = b"5\r\nhello\r\n0\r\n\r\n";
        let mut r = std::io::BufReader::new(&raw[..]);
        assert!(read_chunked(&mut r, 4, &mut |_| Ok(())).is_err());
        let mut r = &b"abc"[..];
        assert!(read_body(&mut r, 3, 2).is_err());
        let big = vec![b'a'; MAX_HEAD + 10];
        let mut r = std::io::BufReader::new(&big[..]);
        assert!(read_head(&mut r).is_err());
        let mut r = std::io::BufReader::new(&b""[..]);
        assert!(read_head(&mut r).unwrap().is_none());
    }
}
