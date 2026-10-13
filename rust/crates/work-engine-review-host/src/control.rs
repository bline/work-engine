use crate::worker::{self, Event, Work};
use review_workflow::protocol::{COMMAND_LIMIT, Command, Operation, REPLY_LIMIT};
use serde_json::{Value, json};
use std::{
    fs::File,
    io::{Read, Write},
    os::fd::FromRawFd,
    path::PathBuf,
    time::Duration,
};
use tokio::{io::unix::AsyncFd, sync::mpsc};

const OUTPUT_SLOTS: usize = 8;
struct Pipe(AsyncFd<File>);
impl Pipe {
    fn standard(fd: i32) -> std::io::Result<Self> {
        // Each descriptor is owned exactly once by the stdio control runtime.
        let file = unsafe { File::from_raw_fd(fd) };
        let flags = rustix::fs::fcntl_getfl(&file)?;
        rustix::fs::fcntl_setfl(&file, flags | rustix::fs::OFlags::NONBLOCK)?;
        Ok(Self(AsyncFd::new(file)?))
    }
    async fn read(&self, bytes: &mut [u8]) -> std::io::Result<usize> {
        loop {
            let mut ready = self.0.readable().await?;
            match ready.try_io(|inner| inner.get_ref().read(bytes)) {
                Ok(value) => return value,
                Err(_) => continue,
            }
        }
    }
    async fn write_all(&self, mut bytes: &[u8]) -> std::io::Result<()> {
        while !bytes.is_empty() {
            let mut ready = self.0.writable().await?;
            match ready.try_io(|inner| inner.get_ref().write(bytes)) {
                Ok(Ok(0)) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::WriteZero,
                        "stdout closed",
                    ));
                }
                Ok(Ok(n)) => bytes = &bytes[n..],
                Ok(Err(e)) => return Err(e),
                Err(_) => continue,
            }
        }
        Ok(())
    }
}
fn refusal(id: &str, kind: &str, reason: &str) -> Value {
    json!({"version":1,"requestId":id,"error":{"kind":kind,"message":reason}})
}
fn bounded(value: Value) -> Vec<u8> {
    let mut bytes = serde_json::to_vec(&value).expect("JSON value encodes");
    let limit = if value["result"]["status"] == "host_status" {
        COMMAND_LIMIT
    } else {
        REPLY_LIMIT
    };
    if bytes.len() > limit {
        bytes = serde_json::to_vec(&refusal(
            value["requestId"].as_str().unwrap_or(""),
            "capacity",
            "response exceeds declared envelope limit; recover exact operation IDs",
        ))
        .unwrap();
    }
    bytes.push(b'\n');
    bytes
}

pub async fn serve(path: PathBuf) -> Result<(), String> {
    let input = Pipe::standard(0).map_err(|e| e.to_string())?;
    let output = Pipe::standard(1).map_err(|e| e.to_string())?;
    let (replies, mut output_queue) = mpsc::channel::<Value>(OUTPUT_SLOTS);
    let (events_tx, mut events) = mpsc::channel(2);
    let (commands, commands_rx) = mpsc::channel(1);
    let worker = worker::spawn(path, commands_rx, events_tx);
    let (output_failure, mut output_failed) = mpsc::channel(1);
    let writer = tokio::spawn(async move {
        while let Some(value) = output_queue.recv().await {
            let bytes = bounded(value);
            if tokio::time::timeout(Duration::from_secs(2), output.write_all(&bytes))
                .await
                .map_or(true, |r| r.is_err())
            {
                let _ = output_failure.send(()).await;
                return;
            }
        }
    });
    let mut open = true;
    let mut ready = false;
    let mut busy = true; // startup/reconstruction is owned by the worker.
    let mut eof = false;
    let mut pending = Vec::new();
    let mut chunk = [0u8; 4096];
    let mut observation = json!({"status":"starting","revision":null});
    let mut missed_output = 0u64;
    loop {
        if eof && !busy {
            break;
        }
        tokio::select! {
            event = events.recv(), if busy => match event {
                Some(Event::Ready(value)) | Some(Event::Finished(value)) => { observation = value; ready = true; busy = false; },
                Some(Event::Failed(error)) => { observation = json!({"startupError":error}); busy = false; open = false; },
                None => { open = false; busy = false; ready = false; },
            },
            _ = output_failed.recv(), if !eof => { open = false; eof = true; },
            read = input.read(&mut chunk), if !eof => {
                let size = match read { Ok(n) => n, Err(_) => { open = false; eof = true; continue; } };
                if size == 0 { open = false; eof = true; continue; }
                for byte in &chunk[..size] {
                    if *byte != b'\n' {
                        pending.push(*byte);
                        if pending.len() > COMMAND_LIMIT {
                            let _ = replies.try_send(refusal("","capacity","command exceeds 64 KiB; admission stopped"));
                            open = false; eof = true; break;
                        }
                        continue;
                    }
                    let parsed = Command::parse(&pending);
                    pending.clear();
                    let command = match parsed {
                        Ok(command) => command,
                        Err(error) => {
                            if replies.try_send(json!({"version":1,"requestId":"","error":error})).is_err() { open=false; missed_output+=1; }
                            continue;
                        }
                    };
                    let response = match command.operation {
                        Operation::Status => Some(json!({"version":1,"requestId":command.request_id,"result":{"status":"host_status","admissionOpen":open,"busy":busy,"snapshotStale":busy,"lastObservation":observation,"missedOutput":missed_output,"outputSlots":OUTPUT_SLOTS}})),
                        Operation::StopAdmission => { open=false; Some(json!({"version":1,"requestId":command.request_id,"result":{"status":"admission_stopped","inFlight":busy,"cancelled":false}})) },
                        _ if !open => Some(refusal(&command.request_id,"stopped","admission is closed")),
                        _ if busy || !ready => Some(refusal(&command.request_id,"busy","one workflow operation is already in flight")),
                        _ => {
                            match replies.clone().try_reserve_owned() {
                                Ok(slot) => {
                                    busy=true;
                                    if let Err(error) = commands.try_send(Work { command, reply:slot }) {
                                        let work = error.into_inner();
                                        work.reply.send(refusal(&work.command.request_id,"unresolved","workflow worker unavailable"));
                                        busy=false; open=false;
                                    }
                                    None
                                },
                                Err(_) => { open=false; missed_output+=1; None },
                            }
                        },
                    };
                    if let Some(response) = response && replies.try_send(response).is_err() { open=false; missed_output+=1; }
                }
            }
        }
    }
    drop(commands);
    drop(replies);
    let _ = writer.await;
    worker
        .join()
        .map_err(|_| "workflow worker panicked".to_string())?;
    Ok(())
}
