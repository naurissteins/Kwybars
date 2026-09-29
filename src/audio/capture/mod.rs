//! pipewire capture of the default sink monitor on a dedicated thread

mod analysis;
mod format;
mod ring;
mod status;
mod stream;
mod thread;

use std::io;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use calloop::ping::Ping;
use pipewire::channel::Sender;
use tracing::error;

pub use status::{CaptureState, StatusSnapshot};

use super::frame::FrameSlot;
use super::spectrum::SpectrumConfig;
use status::CaptureStatus;
use thread::Command;

/// how captured audio is analyzed
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CaptureSettings {
    pub interval: Duration,
    pub spectrum: SpectrumConfig,
}

/// state the capture thread publishes to other threads
#[derive(Debug)]
struct Shared {
    status: CaptureStatus,
    frames: Arc<FrameSlot>,
    /// wakes the reader's loop when it asked for the next frame
    waker: Option<Ping>,
}

impl Shared {
    fn publish(&self, values: Option<&[f32]>, level: f32) {
        let wake = match values {
            Some(values) => self.frames.publish(values, level),
            None if !self.frames.is_silent() => self.frames.publish_silence(),
            None => false,
        };
        if wake && let Some(waker) = &self.waker {
            waker.ping();
        }
    }
}

/// the capture thread could not be started
#[derive(Debug, thiserror::Error)]
#[error("could not start the audio capture thread: {0}")]
pub struct CaptureError(#[from] io::Error);

/// handle to the capture thread; dropping it stops the thread
pub struct Capture {
    commands: Option<Sender<Command>>,
    thread: Option<JoinHandle<()>>,
    shared: Arc<Shared>,
}

impl Capture {
    pub fn spawn(settings: CaptureSettings, waker: Option<Ping>) -> Result<Self, CaptureError> {
        let shared = Arc::new(Shared {
            status: CaptureStatus::default(),
            frames: Arc::new(FrameSlot::new(settings.spectrum.bars.max(1))),
            waker,
        });
        let (commands, receiver) = pipewire::channel::channel();
        let thread_shared = Arc::clone(&shared);
        let thread = std::thread::Builder::new()
            .name("kwybars-audio".to_owned())
            .spawn(move || thread::run(receiver, thread_shared, settings))?;
        Ok(Self {
            commands: Some(commands),
            thread: Some(thread),
            shared,
        })
    }

    /// the latest state, format, and level
    pub fn status(&self) -> StatusSnapshot {
        self.shared.status.snapshot()
    }

    /// the latest tilted spectrum, one value per bar
    pub fn frames(&self) -> &Arc<FrameSlot> {
        &self.shared.frames
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
