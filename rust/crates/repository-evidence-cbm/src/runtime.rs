use crate::{Binding, Error, LIMIT, Result, ToolRequest, executable_digest, parse_json, require};
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    os::{fd::AsRawFd, unix::process::CommandExt},
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

/// One native MCP connection keeps one capture's related observations together.
pub(crate) struct Session {
    child: Child,
    input: Option<ChildStdin>,
    output: ChildStdout,
    diagnostics: ChildStderr,
    buffer: Vec<u8>,
    stderr: Vec<u8>,
    deadline: Instant,
    next_id: u64,
    initialization: String,
    complete: bool,
}
fn nonblocking(fd: std::os::fd::RawFd) -> Result<()> {
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(())
}
fn drain(input: &mut impl Read, bytes: &mut Vec<u8>, limit: usize) -> Result<bool> {
    let mut buffer = [0u8; 8192];
    loop {
        match input.read(&mut buffer) {
            Ok(0) => return Ok(true),
            Ok(n) => {
                require(bytes.len() + n <= limit, "CBM stream exceeds capacity")?;
                bytes.extend_from_slice(&buffer[..n]);
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => return Ok(false),
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error.into()),
        }
    }
}
impl Session {
    pub fn start(binding: &Binding, deadline: Instant) -> Result<Self> {
        require(
            executable_digest(&binding.executable)? == binding.executable_sha256,
            "CBM executable drift",
        )?;
        let mut child = Command::new(&binding.executable)
            .current_dir(&binding.repository)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0)
            .spawn()?;
        // Piped descriptors are guaranteed by Command; own the child before any
        // fallible setup so error paths always collect this invocation.
        let input = child.stdin.take();
        let output = child.stdout.take().expect("piped stdout");
        let diagnostics = child.stderr.take().expect("piped stderr");
        let mut session = Self {
            child,
            input,
            output,
            diagnostics,
            buffer: Vec::new(),
            stderr: Vec::new(),
            deadline,
            next_id: 0,
            initialization: String::new(),
            complete: false,
        };
        nonblocking(session.output.as_raw_fd())?;
        nonblocking(session.diagnostics.as_raw_fd())?;
        nonblocking(
            session
                .input
                .as_ref()
                .ok_or(Error::Invalid("missing MCP input"))?
                .as_raw_fd(),
        )?;
        session.initialization=session.rpc("initialize",json!({"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"work-engine-repository-evidence","version":"1"}}))?;
        let initialized = parse_json(session.initialization.as_bytes())?;
        require(
            initialized["result"]["serverInfo"]["name"] == "codebase-memory-mcp"
                && initialized["result"]["protocolVersion"] == "2024-11-05",
            "unexpected MCP server",
        )?;
        session.send(&json!({"jsonrpc":"2.0","method":"notifications/initialized"}))?;
        Ok(session)
    }
    pub fn initialization(&self) -> &str {
        &self.initialization
    }
    fn send(&mut self, value: &Value) -> Result<()> {
        let mut bytes = serde_json::to_vec(value)?;
        require(bytes.len() <= 32768, "MCP request exceeds capacity")?;
        bytes.push(b'\n');
        let mut offset = 0;
        let deadline = (Instant::now() + Duration::from_secs(60)).min(self.deadline);
        while offset < bytes.len() {
            let input = self
                .input
                .as_mut()
                .ok_or(Error::Invalid("MCP input closed"))?;
            match input.write(&bytes[offset..]) {
                Ok(0) => return Err(Error::Invalid("MCP input closed")),
                Ok(n) => offset += n,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    drain(&mut self.output, &mut self.buffer, LIMIT)?;
                    drain(&mut self.diagnostics, &mut self.stderr, 65536)?;
                    require(Instant::now() < deadline, "MCP input deadline exceeded")?;
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => return Err(error.into()),
            }
        }
        Ok(())
    }
    fn rpc(&mut self, method: &str, params: Value) -> Result<String> {
        self.next_id += 1;
        let id = self.next_id;
        self.send(&json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))?;
        let deadline = (Instant::now() + Duration::from_secs(60)).min(self.deadline);
        loop {
            let eof = drain(&mut self.output, &mut self.buffer, LIMIT)?;
            drain(&mut self.diagnostics, &mut self.stderr, 65536)?;
            while let Some(end) = self.buffer.iter().position(|b| *b == b'\n') {
                let line: Vec<u8> = self.buffer.drain(..=end).collect();
                let value = parse_json(&line)?;
                if value.get("id").is_none() {
                    continue;
                }
                require(
                    value["jsonrpc"] == "2.0"
                        && value["id"].as_u64() == Some(id)
                        && value.get("error").is_none(),
                    "unexpected MCP response",
                )?;
                return String::from_utf8(line)
                    .map_err(|_| Error::Invalid("MCP response is not UTF-8"));
            }
            require(
                !eof && self.child.try_wait()?.is_none(),
                "MCP closed before response",
            )?;
            require(Instant::now() < deadline, "MCP capture deadline exceeded")?;
            thread::sleep(Duration::from_millis(10));
        }
    }
    pub fn observe(&mut self, request: &ToolRequest) -> Result<String> {
        self.rpc(
            "tools/call",
            json!({"name":request.tool,"arguments":request.arguments}),
        )
    }
    pub fn finish(mut self) -> Result<()> {
        self.input.take();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(status) = self.child.try_wait()? {
                self.complete = true;
                return require(status.success(), "MCP frontend closed unsuccessfully");
            }
            // Closing stdin is the supported session close. No shared daemon
            // stop is sent; remaining sessions retain their own service.
            drain(&mut self.output, &mut self.buffer, LIMIT)?;
            drain(&mut self.diagnostics, &mut self.stderr, 65536)?;
            require(Instant::now() < deadline, "MCP frontend close timed out")?;
            thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        if !self.complete {
            self.input.take();
            unsafe {
                libc::kill(-(self.child.id() as i32), libc::SIGKILL);
            }
            let _ = self.child.wait();
        }
    }
}
