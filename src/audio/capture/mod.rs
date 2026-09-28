//! pipewire capture of the default sink monitor on a dedicated thread

mod format;
mod ring;
mod status;
mod stream;
mod thread;

use std::io;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use pipewire::channel::Sender;
use tracing::error;

pub use status::{CaptureState, StatusSnapshot};

use status::CaptureStatus;
use thread::Command;

/// the capture thread could not be started
#[derive(Debug, thiserror::Error)]
#[error("could not start the audio capture thread: {0}")]
pub struct CaptureError(#[from] io::Error);

/// handle to the capture thread; dropping it stops the thread
pub struct Capture {
    commands: Option<Sender<Command>>,
    thread: Option<JoinHandle<()>>,
    status: Arc<CaptureStatus>,
}

impl Capture {
    /// starts capturing; new audio is analyzed at most once per `interval`
    pub fn spawn(interval: Duration) -> Result<Self, CaptureError> {
        let status = Arc::new(CaptureStatus::default());
        let (commands, receiver) = pipewire::channel::channel();
        let thread_status = Arc::clone(&status);
        let thread = std::thread::Builder::new()
            .name("kwybars-audio".to_owned())
            .spawn(move || thread::run(receiver, thread_status, interval))?;
        Ok(Self {
            commands: Some(commands),
            thread: Some(thread),
            status,
        })
    }

    /// the latest state, format, and level
    pub fn status(&self) -> StatusSnapshot {
        self.status.snapshot()
    }

    /// stops the thread and waits for it to disconnect
    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        if let Some(commands) = self.commands.take() {
            // fails only if the thread already exited
            let _ = commands.send(Command::Stop);
        }
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            error!("the audio capture thread panicked");
        }
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        self.shutdown();
    }
}
