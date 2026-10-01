//! a one-off connection attempt, for `kwybars doctor`

use std::thread;
use std::time::{Duration, Instant};

use super::{Capture, CaptureError, CaptureSettings, CaptureState, StatusSnapshot};
use crate::audio::spectrum::SpectrumConfig;

/// how long pipewire gets to answer
const PATIENCE: Duration = Duration::from_secs(2);
const FORMAT_PATIENCE: Duration = Duration::from_millis(300);
const STEP: Duration = Duration::from_millis(20);

pub fn probe() -> Result<StatusSnapshot, CaptureError> {
    let settings = CaptureSettings {
        interval: Duration::from_millis(16),
        spectrum: SpectrumConfig::default(),
    };
    let capture = Capture::spawn(settings, None)?;
    let mut deadline = Instant::now() + PATIENCE;
    let mut connected = false;
    let status = loop {
        let status = capture.status();
        let linked = matches!(status.state, CaptureState::Idle | CaptureState::Streaming);
        if linked && !connected {
            connected = true;
            deadline = deadline.min(Instant::now() + FORMAT_PATIENCE);
        }
        let waiting = match status.state {
            CaptureState::Starting | CaptureState::Connecting => true,
            CaptureState::Idle | CaptureState::Streaming => status.rate == 0,
            CaptureState::Unavailable | CaptureState::Stopped | CaptureState::Failed => false,
        };
        if !waiting || Instant::now() >= deadline {
            break status;
        }
        thread::sleep(STEP);
    };
    capture.stop();
    Ok(status)
}
