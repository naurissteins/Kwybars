//! terminal level meter for `kwybars debug audio`

use std::io::{self, Write};

use crate::audio::capture::{CaptureState, StatusSnapshot};

/// meter width in characters
const WIDTH: usize = 40;
/// level shown as an empty meter
const FLOOR_DB: f32 = -60.0;

/// redraws the meter line in place
pub fn show(snapshot: &StatusSnapshot) {
    let mut stdout = io::stdout().lock();
    // a closed stdout is not worth failing over
    let _ = write!(stdout, "\r{}\x1b[K", line(snapshot));
    let _ = stdout.flush();
}

/// ends the meter line so the shell prompt starts on a new one
pub fn finish() {
    println!();
}

fn line(snapshot: &StatusSnapshot) -> String {
    let db = if snapshot.peak > 0.0 {
        20.0 * snapshot.peak.log10()
    } else {
        f32::NEG_INFINITY
    };
    let filled = (((db - FLOOR_DB) / -FLOOR_DB).clamp(0.0, 1.0) * WIDTH as f32).round() as usize;
    let level = if db.is_finite() {
        format!("{db:6.1} dBFS")
    } else {
        "  -inf dBFS".to_owned()
    };
    format!(
        "{:<11} {:>6} Hz {} ch |{}{}| {level}",
        state_label(snapshot.state),
        snapshot.rate,
        snapshot.channels,
        "#".repeat(filled),
        " ".repeat(WIDTH - filled),
    )
}

fn state_label(state: CaptureState) -> &'static str {
    match state {
        CaptureState::Starting => "starting",
        CaptureState::Unavailable => "no pipewire",
        CaptureState::Connecting => "connecting",
        CaptureState::Idle => "idle",
        CaptureState::Streaming => "streaming",
        CaptureState::Stopped => "stopped",
        CaptureState::Failed => "failed",
    }
}

#[cfg(test)]
mod tests {
    use super::{WIDTH, line};
    use crate::audio::capture::{CaptureState, StatusSnapshot};

    fn snapshot(peak: f32) -> StatusSnapshot {
        StatusSnapshot {
            state: CaptureState::Streaming,
            rate: 48000,
            channels: 2,
            peak,
            frames: 0,
        }
    }

    fn filled(text: &str) -> usize {
        text.chars().filter(|c| *c == '#').count()
    }

    #[test]
    fn full_scale_fills_the_meter() {
        let text = line(&snapshot(1.0));
        assert_eq!(filled(&text), WIDTH);
        assert!(text.contains("   0.0 dBFS"), "{text}");
    }

    #[test]
    fn half_way_in_db_fills_half() {
        assert_eq!(
            filled(&line(&snapshot(10f32.powf(-30.0 / 20.0)))),
            WIDTH / 2
        );
    }

    #[test]
    fn silence_is_empty() {
        let text = line(&snapshot(0.0));
        assert_eq!(filled(&text), 0);
        assert!(text.starts_with("streaming    48000 Hz 2 ch |"), "{text}");
        assert!(text.ends_with("-inf dBFS"), "{text}");
    }
}
