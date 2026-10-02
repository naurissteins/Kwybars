//! whether pipewire can be reached and captured from

use crate::audio::capture::{CaptureState, StatusSnapshot};
use crate::cli::report::Report;

pub fn pipewire(report: &mut Report, status: Result<StatusSnapshot, String>) {
    let status = match status {
        Ok(status) => status,
        Err(err) => {
            report.error(format!("pipewire: {err}"));
            return;
        }
    };
    let format = if status.rate > 0 {
        format!(" ({} Hz, {} channels)", status.rate, status.channels)
    } else {
        String::new()
    };
    match status.state {
        CaptureState::Streaming => {
            report.line(format!(
                "pipewire: connected, capturing the default output{format}"
            ));
        }
        CaptureState::Idle => report.line(format!(
            "pipewire: connected, the default output is idle{format}"
        )),
        CaptureState::Connecting => {
            report.warning("pipewire: connected, but there is no output device to capture yet")
        }
        CaptureState::Unavailable => report.error(
            "pipewire: not reachable; Kwybars keeps retrying and shows nothing until PipeWire runs",
        ),
        CaptureState::Starting | CaptureState::Stopped | CaptureState::Failed => {
            report.error("pipewire: the capture thread could not set up its PipeWire loop");
        }
    }
}
