use std::collections::VecDeque;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[derive(Debug, Error)]
pub enum FrameError {
    #[error("frame exceeds configured byte capacity")]
    TooLarge,
    #[error("transport ended inside a frame")]
    PartialEof,
    #[error("outbound frame may have been partially written")]
    UncertainWrite,
    #[error("frame contains a reserved newline delimiter")]
    Delimiter,
    #[error("transport I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameDiagnosticCode {
    TooLarge,
    PartialEof,
    UncertainWrite,
    Delimiter,
    Io,
}

#[derive(Clone)]
pub struct FrameDiagnostics {
    enabled: bool,
    max_entries: usize,
    entries: Arc<Mutex<VecDeque<FrameDiagnosticCode>>>,
}

impl FrameDiagnostics {
    pub fn new(enabled: bool, max_entries: usize) -> Self {
        Self {
            enabled,
            max_entries,
            entries: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub fn snapshot(&self) -> Vec<FrameDiagnosticCode> {
        self.entries.lock().unwrap().iter().copied().collect()
    }

    fn record(&self, error: &FrameError) {
        if !self.enabled || self.max_entries == 0 {
            return;
        }
        let code = match error {
            FrameError::TooLarge => FrameDiagnosticCode::TooLarge,
            FrameError::PartialEof => FrameDiagnosticCode::PartialEof,
            FrameError::UncertainWrite => FrameDiagnosticCode::UncertainWrite,
            FrameError::Delimiter => FrameDiagnosticCode::Delimiter,
            FrameError::Io(_) => FrameDiagnosticCode::Io,
        };
        let mut entries = self.entries.lock().unwrap();
        if entries.len() == self.max_entries {
            entries.pop_front();
        }
        entries.push_back(code);
    }
}

/// Owns its partial frame buffer across awaits. The caller must retain this
/// reader instead of starting a fresh multi-step read in each select branch.
pub struct BoundedFrameReader<R> {
    reader: R,
    max_frame: usize,
    pending: Vec<u8>,
    diagnostics: Option<FrameDiagnostics>,
}

impl<R: AsyncRead + Unpin> BoundedFrameReader<R> {
    pub fn new(reader: R, max_frame: usize) -> Result<Self, FrameError> {
        if max_frame == 0 || max_frame.checked_add(1).is_none() {
            return Err(FrameError::TooLarge);
        }
        Ok(Self {
            reader,
            max_frame,
            pending: Vec::new(),
            diagnostics: None,
        })
    }

    pub fn with_diagnostics(mut self, diagnostics: FrameDiagnostics) -> Self {
        self.diagnostics = Some(diagnostics);
        self
    }

    fn report(&self, error: FrameError) -> FrameError {
        if let Some(diagnostics) = &self.diagnostics {
            diagnostics.record(&error);
        }
        error
    }

    pub async fn read_frame(&mut self) -> Result<Option<Vec<u8>>, FrameError> {
        loop {
            if let Some(end) = self.pending.iter().position(|byte| *byte == b'\n') {
                if end > self.max_frame {
                    return Err(self.report(FrameError::TooLarge));
                }
                let mut frame = self.pending.drain(..=end).collect::<Vec<_>>();
                frame.pop();
                return Ok(Some(frame));
            }
            if self.pending.len() > self.max_frame {
                return Err(self.report(FrameError::TooLarge));
            }
            let mut chunk = [0u8; 1024];
            let room = self.max_frame + 1 - self.pending.len();
            let limit = room.min(chunk.len());
            let read = self
                .reader
                .read(&mut chunk[..limit])
                .await
                .map_err(|error| self.report(FrameError::Io(error)))?;
            if read == 0 {
                return if self.pending.is_empty() {
                    Ok(None)
                } else {
                    Err(self.report(FrameError::PartialEof))
                };
            }
            self.pending.extend_from_slice(&chunk[..read]);
            if self.pending.len() > self.max_frame
                && !self.pending[..=self.max_frame].contains(&b'\n')
            {
                return Err(self.report(FrameError::TooLarge));
            }
        }
    }
}

struct InFlight {
    uncertain: Arc<AtomicBool>,
    armed: bool,
}

impl InFlight {
    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for InFlight {
    fn drop(&mut self) {
        if self.armed {
            self.uncertain.store(true, Ordering::Release);
        }
    }
}

/// The owner must await `write_frame` outside a cancellation-losing `select!`.
/// If its future is dropped mid-write, later writes fail as uncertain.
pub struct BoundedFrameWriter<W> {
    writer: W,
    max_frame: usize,
    uncertain: Arc<AtomicBool>,
    diagnostics: Option<FrameDiagnostics>,
}

impl<W: AsyncWrite + Unpin> BoundedFrameWriter<W> {
    pub fn new(writer: W, max_frame: usize) -> Result<Self, FrameError> {
        if max_frame == 0 {
            return Err(FrameError::TooLarge);
        }
        Ok(Self {
            writer,
            max_frame,
            uncertain: Arc::new(AtomicBool::new(false)),
            diagnostics: None,
        })
    }

    pub fn with_diagnostics(mut self, diagnostics: FrameDiagnostics) -> Self {
        self.diagnostics = Some(diagnostics);
        self
    }

    fn report(&self, error: FrameError) -> FrameError {
        if let Some(diagnostics) = &self.diagnostics {
            diagnostics.record(&error);
        }
        error
    }

    pub fn outbound_uncertain(&self) -> bool {
        self.uncertain.load(Ordering::Acquire)
    }

    pub async fn write_frame(&mut self, bytes: &[u8]) -> Result<(), FrameError> {
        if self.outbound_uncertain() {
            return Err(self.report(FrameError::UncertainWrite));
        }
        if bytes.len() > self.max_frame {
            return Err(self.report(FrameError::TooLarge));
        }
        if bytes.contains(&b'\n') {
            return Err(self.report(FrameError::Delimiter));
        }
        let mut guard = InFlight {
            uncertain: self.uncertain.clone(),
            armed: true,
        };
        self.writer
            .write_all(bytes)
            .await
            .map_err(|error| self.report(FrameError::Io(error)))?;
        self.writer
            .write_all(b"\n")
            .await
            .map_err(|error| self.report(FrameError::Io(error)))?;
        self.writer
            .flush()
            .await
            .map_err(|error| self.report(FrameError::Io(error)))?;
        guard.disarm();
        Ok(())
    }
}
