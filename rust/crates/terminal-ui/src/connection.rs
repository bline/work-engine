//! One bounded passive socket worker. The UI thread never performs transport I/O.
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, SyncSender, TrySendError},
    thread::{self, JoinHandle},
    time::Duration,
};

use lifecycle_client::{ClientError, ReadClient, SubjectRef, UnixSocketTransport};
use lifecycle_wire::{CursorV1, LifecycleSnapshotV1};

const READ_BUDGET: Duration = Duration::from_millis(600);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadKind {
    Initial,
    Refresh,
    Reconnect,
}

pub struct Request {
    pub generation: u64,
    pub socket: PathBuf,
    pub subject: SubjectRef,
    pub previous: Option<CursorV1>,
    pub kind: ReadKind,
}

pub struct Reply {
    pub generation: u64,
    pub kind: ReadKind,
    pub result: Result<LifecycleSnapshotV1, ClientError>,
}

pub struct Worker {
    requests: Option<SyncSender<Request>>,
    replies: Receiver<Reply>,
    thread: Option<JoinHandle<()>>,
}

impl Worker {
    pub fn start() -> Self {
        let (requests, incoming) = mpsc::sync_channel::<Request>(1);
        let (outgoing, replies) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("passive-lifecycle-read".into())
            .spawn(move || {
                while let Ok(request) = incoming.recv() {
                    let client = ReadClient::new(UnixSocketTransport::new(&request.socket));
                    let result = match &request.previous {
                        Some(cursor) => client.reconnect(&request.subject, cursor, READ_BUDGET),
                        None => client.snapshot(&request.subject, READ_BUDGET),
                    };
                    if outgoing
                        .send(Reply {
                            generation: request.generation,
                            kind: request.kind,
                            result,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })
            .expect("spawn passive lifecycle reader");
        Self {
            requests: Some(requests),
            replies,
            thread: Some(thread),
        }
    }

    pub fn request(&self, request: Request) -> Result<(), &'static str> {
        match self.requests.as_ref().unwrap().try_send(request) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => Err("reader busy; retry after the bounded read finishes"),
            Err(TrySendError::Disconnected(_)) => Err("reader stopped"),
        }
    }

    pub fn try_reply(&self) -> Option<Reply> {
        self.replies.try_recv().ok()
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.requests.take();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
