//! Small loopback harness, with an owned executor and read-only bounded waiters.
use crate::provider::{FakeProvider, ProviderRequestMessage};
use crate::*;
use anyhow::{Context, Result, bail};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

const IO_TIMEOUT: Duration = Duration::from_secs(12);
const MAX_FRAME: u64 = 1024 * 1024;
pub fn rpc<T: Serialize, R: DeserializeOwned>(address: SocketAddr, request: &T) -> Result<R> {
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_millis(300))?;
    stream.set_read_timeout(Some(IO_TIMEOUT))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    write_message(&mut stream, request)?;
    let response: std::result::Result<R, String> = read_message(&stream)?;
    response.map_err(anyhow::Error::msg)
}
pub fn write_message(stream: &mut TcpStream, value: &impl Serialize) -> Result<()> {
    serde_json::to_writer(&mut *stream, value)?;
    stream.write_all(b"\n")?;
    stream.flush()?;
    Ok(())
}
pub(crate) fn read_message<T: DeserializeOwned>(stream: &TcpStream) -> Result<T> {
    use std::io::Read;
    let mut line = String::new();
    BufReader::new(stream)
        .take(MAX_FRAME)
        .read_line(&mut line)?;
    if !line.ends_with('\n') {
        bail!("incomplete or oversized frame");
    }
    Ok(serde_json::from_str(&line)?)
}
fn ready(listener: &TcpListener) -> Result<()> {
    println!(
        "{}",
        serde_json::json!({"address": listener.local_addr()?, "pid": std::process::id()})
    );
    std::io::stdout().flush()?;
    Ok(())
}
pub fn run_provider(path: &Path) -> Result<()> {
    let mut provider = FakeProvider::open(path)?;
    let listener = TcpListener::bind("127.0.0.1:0")?;
    ready(&listener)?;
    for stream in listener.incoming() {
        let mut stream = stream?;
        stream.set_read_timeout(Some(Duration::from_secs(2)))?;
        stream.set_write_timeout(Some(Duration::from_secs(2)))?;
        let reply: std::result::Result<Value, String> =
            read_message::<ProviderRequestMessage>(&stream)
                .and_then(|r| provider.handle(r))
                .and_then(|r| Ok(serde_json::to_value(r)?))
                .map_err(|e| e.to_string());
        let _ = write_message(&mut stream, &reply);
    }
    Ok(())
}
type Answer = mpsc::Sender<std::result::Result<Value, String>>;
struct Envelope {
    request: Request,
    answer: Answer,
}
enum PendingKind {
    Wait(Condition),
    Ticket(EffectId, Reply),
}
struct Pending {
    kind: PendingKind,
    answer: Answer,
    until: Instant,
}

fn classify_pending(
    now: Instant,
    until: Instant,
    ready: Option<Value>,
) -> Option<std::result::Result<Value, String>> {
    if now >= until {
        return Some(Err(
            "observation deadline expired; accepted responsibility is unchanged".into(),
        ));
    }
    ready.map(Ok)
}

pub fn run_coordinator(path: &Path, provider: SocketAddr, pause_at: Option<Stage>) -> Result<()> {
    let mut coordinator = Coordinator::open(path, provider, Arc::new(SystemClock), pause_at)?;
    let listener = TcpListener::bind("127.0.0.1:0")?;
    ready(&listener)?;
    let (send, receive) = mpsc::channel();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { break };
            let send = send.clone();
            std::thread::spawn(move || {
                let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
                let response = (|| -> Result<std::result::Result<Value, String>> {
                    let request = read_message(&stream)?;
                    let (answer, result) = mpsc::channel();
                    send.send(Envelope { request, answer })?;
                    Ok(result.recv_timeout(IO_TIMEOUT)?)
                })()
                .unwrap_or_else(|e| Err(e.to_string()));
                let _ = write_message(&mut stream, &response);
            });
        }
    });
    let mut pending: Vec<Pending> = vec![];
    loop {
        match receive.recv_timeout(Duration::from_millis(3)) {
            Ok(Envelope { request, answer }) => {
                let response: Result<Value> = match request {
                    Request::Command(command) => match coordinator.execute(command) {
                        Ok(reply @ Reply::Ticket(_)) => {
                            let Reply::Ticket(id) = &reply else {
                                unreachable!()
                            };
                            pending.push(Pending {
                                kind: PendingKind::Ticket(id.clone(), reply),
                                answer,
                                until: Instant::now() + Duration::from_secs(10),
                            });
                            coordinator.tick().context("executor step")?;
                            continue;
                        }
                        Ok(reply) => Ok(serde_json::to_value(reply)?),
                        Err(error) => Err(error),
                    },
                    Request::Snapshot => Ok(serde_json::to_value(coordinator.snapshot())?),
                    Request::Bootstrap(id) => coordinator
                        .bootstrap(&id)
                        .and_then(|r| Ok(serde_json::to_value(r)?)),
                    Request::Result(id) => coordinator
                        .result(&id)
                        .and_then(|r| Ok(serde_json::to_value(r)?)),
                    Request::ReleaseBarrier => {
                        coordinator.release_barrier();
                        Ok(serde_json::to_value(Reply::Recorded)?)
                    }
                    Request::Wait {
                        condition,
                        timeout_ms,
                    } => {
                        pending.push(Pending {
                            kind: PendingKind::Wait(condition),
                            answer,
                            until: Instant::now() + Duration::from_millis(timeout_ms.min(10_000)),
                        });
                        coordinator.tick().context("executor step")?;
                        continue;
                    }
                };
                let _ = answer.send(response.map_err(|e| e.to_string()));
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => bail!("command listener stopped"),
        }
        coordinator.tick().context("executor step")?;
        let now = Instant::now();
        let mut remaining = vec![];
        for waiter in pending {
            let ready = match &waiter.kind {
                PendingKind::Wait(condition) if coordinator.matches(condition) => {
                    Some(serde_json::to_value(coordinator.snapshot())?)
                }
                PendingKind::Ticket(id, reply) if coordinator.ack_ready(id) => {
                    Some(serde_json::to_value(reply)?)
                }
                _ => None,
            };
            let value = classify_pending(now, waiter.until, ready);
            if let Some(value) = value {
                let _ = waiter.answer.send(value);
            } else {
                remaining.push(waiter);
            }
        }
        pending = remaining;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready_replies() -> Vec<Value> {
        let condition_snapshot = Snapshot {
            revision: Revision(1),
            contexts: vec![],
            transitions: vec![],
            effects: vec![],
            deliveries: vec![],
            exits: vec![],
            barrier: None,
        };
        vec![
            serde_json::to_value(condition_snapshot).expect("snapshot"),
            serde_json::to_value(Reply::Ticket(EffectId::new("completed-effect"))).expect("ticket"),
        ]
    }

    #[test]
    fn pending_condition_and_ticket_ready_at_or_after_expiry_time_out() {
        let until = Instant::now();
        for now in [until, until + Duration::from_millis(1)] {
            for ready in ready_replies() {
                let outcome = classify_pending(now, until, Some(ready));
                assert!(
                    matches!(outcome, Some(Err(_))),
                    "expired wait returned {outcome:?}"
                );
            }
        }
    }

    #[test]
    fn pending_readiness_before_deadline_succeeds_and_unready_wait_remains_pending() {
        let now = Instant::now();
        let until = now + Duration::from_millis(1);
        for ready in ready_replies() {
            assert_eq!(
                classify_pending(now, until, Some(ready.clone())),
                Some(Ok(ready))
            );
        }
        assert_eq!(classify_pending(now, until, None), None);
        assert!(matches!(classify_pending(until, until, None), Some(Err(_))));
    }
}
