//! pipewire capture of the default sink monitor on a dedicated thread

mod analysis;
mod format;
mod probe;
mod ring;
mod status;
mod stream;
mod thread;
mod tuning;

use std::io;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use calloop::ping::Ping;
use pipewire::channel::Sender;
use tracing::error;

pub use probe::probe;
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
    waker: Option<Ping>,
}

impl Shared {
    fn publish(&self, frames: &FrameSlot, values: Option<&[f32]>, level: f32) {
        let wake = match values {
            Some(values) => frames.publish(values, level),
            None if !frames.is_silent() => frames.publish_silence(),
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
    settings: CaptureSettings,
    frames: Arc<FrameSlot>,
}

impl Capture {
    pub fn spawn(settings: CaptureSettings, waker: Option<Ping>) -> Result<Self, CaptureError> {
        let shared = Arc::new(Shared {
            status: CaptureStatus::default(),
            waker,
        });
        let frames = Arc::new(FrameSlot::new(settings.spectrum.bars.max(1)));
        let (commands, receiver) = pipewire::channel::channel();
        let thread_shared = Arc::clone(&shared);
        let (thread_settings, thread_frames) = (settings.clone(), Arc::clone(&frames));
        let thread = std::thread::Builder::new()
            .name("kwybars-audio".to_owned())
            .spawn(move || thread::run(receiver, thread_shared, thread_settings, thread_frames))?;
        Ok(Self {
            commands: Some(commands),
            thread: Some(thread),
            shared,
            settings,
            frames,
        })
    }

    pub fn reconfigure(&mut self, settings: CaptureSettings) -> bool {
        if settings == self.settings {
            return false;
        }
        let replaced = settings.spectrum.bars != self.settings.spectrum.bars;
        if replaced {
            self.frames = Arc::new(FrameSlot::new(settings.spectrum.bars.max(1)));
        }
        self.settings = settings.clone();
        let command = Command::Reconfigure {
            settings,
            frames: Arc::clone(&self.frames),
        };
        if let Some(commands) = &self.commands
            && commands.send(command).is_err()
        {
            error!("the audio capture thread is gone, new settings not applied");
        }
        replaced
    }

    /// the latest state, format, and level
    pub fn status(&self) -> StatusSnapshot {
        self.shared.status.snapshot()
    }

    /// the latest tilted spectrum, one value per bar
    pub fn frames(&self) -> &Arc<FrameSlot> {
        &self.frames
    }

    /// stops the thread and waits for it to disconnect
    pub fn stop(mut self) {
        self.shutdown();
    }

    /// like [`Self::stop`] for an owner that cannot give the handle away
    pub fn shutdown(&mut self) {
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
