//! Narrow Linux access-lease launcher and loopback to Unix relay.
//!
//! Build with `RUSTFLAGS='-C target-feature=+crt-static' cargo build --locked
//! --offline -p lifecycle-codex --bin access_lease_launcher`. The product pins
//! the resulting executable bytes in its selected access-lease profile.

use std::{
    env,
    io::{self, BufRead as _, Write as _},
    net::{Shutdown, TcpListener, TcpStream},
    os::unix::{net::UnixStream, process::CommandExt as _},
    path::PathBuf,
    process::{Command, Stdio},
    thread,
};

fn pipe(mut tcp: TcpStream, mut unix: UnixStream) -> io::Result<()> {
    let mut tcp_read = tcp.try_clone()?;
    let mut unix_write = unix.try_clone()?;
    let inbound = thread::spawn(move || {
        let result = io::copy(&mut tcp_read, &mut unix_write);
        let _ = unix_write.shutdown(Shutdown::Write);
        result
    });
    let outbound = io::copy(&mut unix, &mut tcp);
    let _ = tcp.shutdown(Shutdown::Write);
    let _ = inbound.join();
    outbound.map(|_| ())
}

fn relay(socket: PathBuf, port: u16, expected: &str) -> io::Result<()> {
    if port == 0 || !socket.is_absolute() {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port))?;
    // An invalid method must be denied by the selected host CONNECT policy
    // without opening an upstream socket. A mere successful connect does not
    // establish that the intended proxy owns this socket.
    let mut probe = UnixStream::connect(&socket)?;
    probe.set_read_timeout(Some(std::time::Duration::from_secs(2)))?;
    probe.write_all(b"GET /__work_engine_ready__ HTTP/1.1\r\nHost: invalid\r\n\r\n")?;
    let mut line = String::new();
    io::BufReader::new(&probe).read_line(&mut line)?;
    if line != expected {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    io::stdout().write_all(b"READY\n")?;
    io::stdout().flush()?;
    for incoming in listener.incoming() {
        let tcp = incoming?;
        let socket = socket.clone();
        thread::spawn(move || {
            if let Ok(unix) = UnixStream::connect(socket) {
                let _ = pipe(tcp, unix);
            }
        });
    }
    Ok(())
}

fn launch(native: PathBuf, socket: PathBuf, port: u16) -> io::Result<()> {
    if !native.is_absolute() || !socket.is_absolute() || port == 0 {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let own_exe = env::current_exe()?;
    let mut relay = Command::new(own_exe)
        .args([
            "--relay",
            socket.to_str().ok_or(io::ErrorKind::InvalidInput)?,
            &port.to_string(),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let ready = relay.stdout.take().ok_or(io::ErrorKind::BrokenPipe)?;
    let mut line = String::new();
    let count = io::BufReader::new(ready).read_line(&mut line)?;
    if count != 6 || line != "READY\n" || relay.try_wait()?.is_some() {
        let _ = relay.kill();
        let _ = relay.wait();
        return Err(io::ErrorKind::ConnectionRefused.into());
    }
    // Exercise the actual loopback-to-Unix bridge before Codex can receive
    // access material. The invalid request must stay local and be denied.
    let bridged_check = (|| -> io::Result<()> {
        let mut bridged = TcpStream::connect_timeout(
            &std::net::SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, port)),
            std::time::Duration::from_secs(2),
        )?;
        bridged.set_read_timeout(Some(std::time::Duration::from_secs(2)))?;
        bridged.write_all(b"GET /__work_engine_ready__ HTTP/1.1\r\nHost: invalid\r\n\r\n")?;
        let mut response = String::new();
        io::BufReader::new(&bridged).read_line(&mut response)?;
        if response != "HTTP/1.1 403 Forbidden\r\n" {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        Ok(())
    })();
    if let Err(error) = bridged_check {
        let _ = relay.kill();
        let _ = relay.wait();
        return Err(error);
    }
    // Replaces the launcher at the same host PID. Bubblewrap's private PID
    // namespace owns the relay's lifetime if Codex exits or is killed.
    let error = Command::new(native)
        .args(["app-server", "--strict-config", "--listen", "stdio://"])
        .exec();
    let _ = relay.kill();
    let _ = relay.wait();
    Err(error)
}

#[cfg(feature = "synthetic-access-test")]
fn launch_synthetic(
    native: PathBuf,
    proxy_socket: PathBuf,
    proxy_port: u16,
    http_socket: PathBuf,
    http_port: u16,
) -> io::Result<()> {
    if !http_socket.is_absolute() || http_port == 0 || http_port == proxy_port {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let own_exe = env::current_exe()?;
    let mut http = Command::new(own_exe)
        .args([
            "--relay-http",
            http_socket.to_str().ok_or(io::ErrorKind::InvalidInput)?,
            &http_port.to_string(),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let started = (|| -> io::Result<()> {
        let ready = http.stdout.take().ok_or(io::ErrorKind::BrokenPipe)?;
        let mut line = String::new();
        let count = io::BufReader::new(ready).read_line(&mut line)?;
        if count != 6 || line != "READY\n" || http.try_wait()?.is_some() {
            return Err(io::ErrorKind::ConnectionRefused.into());
        }
        let mut bridged = TcpStream::connect_timeout(
            &std::net::SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, http_port)),
            std::time::Duration::from_secs(2),
        )?;
        bridged.set_read_timeout(Some(std::time::Duration::from_secs(2)))?;
        bridged.write_all(b"GET /__work_engine_ready__ HTTP/1.1\r\nHost: invalid\r\n\r\n")?;
        let mut response = String::new();
        io::BufReader::new(&bridged).read_line(&mut response)?;
        if response != "HTTP/1.1 200 OK\r\n" {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        Ok(())
    })();
    if let Err(error) = started {
        let _ = http.kill();
        let _ = http.wait();
        return Err(error);
    }
    // Both relays are namespace children. The production launch is unchanged;
    // this mode exists only in a separately compiled synthetic test helper.
    let result = launch(native, proxy_socket, proxy_port);
    let _ = http.kill();
    let _ = http.wait();
    result
}

fn main() {
    let arguments = env::args().collect::<Vec<_>>();
    let result = match arguments.as_slice() {
        [_, mode, native, socket, port] if mode == "--launch" => port
            .parse::<u16>()
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))
            .and_then(|port| launch(native.into(), socket.into(), port)),
        [_, mode, socket, port] if mode == "--relay" => port
            .parse::<u16>()
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))
            .and_then(|port| relay(socket.into(), port, "HTTP/1.1 403 Forbidden\r\n")),
        #[cfg(feature = "synthetic-access-test")]
        [_, mode, socket, port] if mode == "--relay-http" => port
            .parse::<u16>()
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))
            .and_then(|port| relay(socket.into(), port, "HTTP/1.1 200 OK\r\n")),
        #[cfg(feature = "synthetic-access-test")]
        [
            _,
            mode,
            native,
            proxy_socket,
            proxy_port,
            http_socket,
            http_port,
        ] if mode == "--launch-synthetic" => proxy_port
            .parse::<u16>()
            .and_then(|proxy_port| {
                http_port
                    .parse::<u16>()
                    .map(|http_port| (proxy_port, http_port))
            })
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))
            .and_then(|(proxy_port, http_port)| {
                launch_synthetic(
                    native.into(),
                    proxy_socket.into(),
                    proxy_port,
                    http_socket.into(),
                    http_port,
                )
            }),
        _ => Err(io::ErrorKind::InvalidInput.into()),
    };
    if result.is_err() {
        std::process::exit(111);
    }
}
