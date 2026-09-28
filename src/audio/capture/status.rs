//! capture state shared with other threads through atomics

use std::sync::atomic::{AtomicU8, AtomicU32, AtomicU64, Ordering};

/// what the capture thread is doing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureState {
    Starting,
    Unavailable,
    Connecting,
    Idle,
    Streaming,
    Stopped,
    Failed,
}

impl CaptureState {
    const ALL: [Self; 7] = [
        Self::Starting,
        Self::Unavailable,
        Self::Connecting,
        Self::Idle,
        Self::Streaming,
        Self::Stopped,
        Self::Failed,
    ];

    fn to_raw(self) -> u8 {
        Self::ALL
            .iter()
            .position(|state| *state == self)
            .map_or(0, |index| index as u8)
    }

    fn from_raw(raw: u8) -> Self {
        Self::ALL
            .get(usize::from(raw))
            .copied()
            .unwrap_or(Self::Starting)
    }
}

/// latest capture state, written by the capture thread
#[derive(Debug, Default)]
pub struct CaptureStatus {
    state: AtomicU8,
    rate: AtomicU32,
    channels: AtomicU32,
    /// f32 bits of the peak level over the last analysis interval
    peak: AtomicU32,
    frames: AtomicU64,
}

/// a copy of [`CaptureStatus`] at one moment
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatusSnapshot {
    pub state: CaptureState,
    pub rate: u32,
    pub channels: u32,
    /// absolute peak in `0.0..=1.0`, may exceed 1 for clipped input
    pub peak: f32,
    /// frames captured since the stream started
    pub frames: u64,
}

// values are independent readouts, so relaxed ordering is enough
impl CaptureStatus {
    pub fn snapshot(&self) -> StatusSnapshot {
        StatusSnapshot {
            state: CaptureState::from_raw(self.state.load(Ordering::Relaxed)),
            rate: self.rate.load(Ordering::Relaxed),
            channels: self.channels.load(Ordering::Relaxed),
            peak: f32::from_bits(self.peak.load(Ordering::Relaxed)),
            frames: self.frames.load(Ordering::Relaxed),
        }
    }

    pub(super) fn set_state(&self, state: CaptureState) {
        self.state.store(state.to_raw(), Ordering::Relaxed);
        if state != CaptureState::Streaming {
            self.set_peak(0.0);
        }
    }

    pub(super) fn set_format(&self, rate: u32, channels: u32) {
        self.rate.store(rate, Ordering::Relaxed);
        self.channels.store(channels, Ordering::Relaxed);
    }

    pub(super) fn set_peak(&self, peak: f32) {
        self.peak.store(peak.to_bits(), Ordering::Relaxed);
    }

    pub(super) fn set_frames(&self, frames: u64) {
        self.frames.store(frames, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::{CaptureState, CaptureStatus};

    #[test]
    fn states_round_trip() {
        for state in CaptureState::ALL {
            assert_eq!(CaptureState::from_raw(state.to_raw()), state);
        }
    }

    #[test]
    fn leaving_streaming_clears_the_peak() {
        let status = CaptureStatus::default();
        status.set_state(CaptureState::Streaming);
        status.set_peak(0.5);
        assert_eq!(status.snapshot().peak, 0.5);
        status.set_state(CaptureState::Idle);
        assert_eq!(status.snapshot().peak, 0.0);
    }
}
