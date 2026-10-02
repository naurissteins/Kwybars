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
use std::sync::atomic::{AtomicU32, Ordering, fence};
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
    /// activity threshold as f32 bits, 0 until the owner sets one
    threshold: AtomicU32,
}

impl Shared {
    fn new(waker: Option<Ping>) -> Self {
        Self {
            status: CaptureStatus::default(),
            waker,
            threshold: AtomicU32::new(0),
        }
    }

    fn publish(&self, frames: &FrameSlot, values: Option<&[f32]>, level: f32) {
        if self.store(frames, values, level)
            && let Some(waker) = &self.waker
        {
            waker.ping();
        }
    }

    fn store(&self, frames: &FrameSlot, values: Option<&[f32]>, level: f32) -> bool {
        let before = frames.level();
        let wanted = match values {
            Some(values) => frames.publish(values, level),
            None if !frames.is_silent() => frames.publish_silence(),
            None => false,
        };
        let threshold = f32::from_bits(self.threshold.load(Ordering::SeqCst));
        wanted || (before >= threshold) != (frames.level() >= threshold)
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
        let shared = Arc::new(Shared::new(waker));
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

    pub fn set_activity_threshold(&self, threshold: f32) {
        self.shared
            .threshold
            .store(threshold.to_bits(), Ordering::SeqCst);
        fence(Ordering::SeqCst);
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

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;

    use super::Shared;
    use crate::audio::frame::FrameSlot;

    fn shared(threshold: f32) -> Shared {
        let shared = Shared::new(None);
        shared
            .threshold
            .store(threshold.to_bits(), Ordering::SeqCst);
        shared
    }

    #[test]
    fn a_level_crossing_the_threshold_wakes_without_a_request() {
        let (shared, frames) = (shared(0.1), FrameSlot::new(1));
        assert!(shared.store(&frames, Some(&[0.5]), 0.5));
        // steady playback on either side does not wake
        assert!(!shared.store(&frames, Some(&[0.5]), 0.3));
        assert!(shared.store(&frames, Some(&[0.5]), 0.05));
        assert!(!shared.store(&frames, Some(&[0.5]), 0.01));
        assert!(shared.store(&frames, Some(&[0.5]), 0.1));
    }

    #[test]
    fn sound_stopping_wakes_once() {
        let (shared, frames) = (shared(0.1), FrameSlot::new(1));
        shared.store(&frames, Some(&[0.5]), 0.5);
        assert!(shared.store(&frames, None, 0.0));
        assert!(!shared.store(&frames, None, 0.0));
        assert_eq!(frames.frame_number(), 2);
    }

    #[test]
    fn a_requested_wake_is_still_reported() {
        let (shared, frames) = (shared(0.1), FrameSlot::new(1));
        shared.store(&frames, Some(&[0.5]), 0.5);
        assert!(frames.request_wake(1));
        assert!(shared.store(&frames, Some(&[0.5]), 0.5));
        assert!(!shared.store(&frames, Some(&[0.5]), 0.5));
    }

    #[test]
    fn without_a_threshold_nothing_crosses() {
        let (shared, frames) = (Shared::new(None), FrameSlot::new(1));
        assert!(!shared.store(&frames, Some(&[0.5]), 0.5));
        assert!(!shared.store(&frames, None, 0.0));
    }
}
