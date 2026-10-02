//! `[audio]`: how captured audio is turned into bar heights

use serde::Deserialize;

use super::bounds::Bounds;

/// the `[audio]` table
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct AudioConfig {
    /// bar height multiplier; the fixed gain when `auto_sensitivity` is off
    pub sensitivity: f32,
    /// adapt the gain so loud passages just reach the top
    pub auto_sensitivity: bool,
    /// lowest frequency shown, in hz
    pub low_cutoff_hz: f32,
    /// highest frequency shown, in hz
    pub high_cutoff_hz: f32,
    /// `0.0` follows the audio exactly, higher values move more calmly
    pub smoothing: f32,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sensitivity: 1.0,
            auto_sensitivity: true,
            low_cutoff_hz: 50.0,
            high_cutoff_hz: 10_000.0,
            smoothing: 0.7,
        }
    }
}

impl AudioConfig {
    /// fixes out-of-range values in place, recording a warning for each
    pub(crate) fn normalize(&mut self, warnings: &mut Vec<String>) {
        let defaults = Self::default();
        let mut bounds = Bounds::new("audio", warnings);
        let mut check = |key, value: &mut f32, min, max, fallback| {
            let mut checked = Some(*value);
            bounds.within(key, &mut checked, min, max);
            *value = checked.unwrap_or(fallback);
        };
        check(
            "sensitivity",
            &mut self.sensitivity,
            0.1,
            10.0,
            defaults.sensitivity,
        );
        check(
            "low_cutoff_hz",
            &mut self.low_cutoff_hz,
            20.0,
            20_000.0,
            defaults.low_cutoff_hz,
        );
        check(
            "high_cutoff_hz",
            &mut self.high_cutoff_hz,
            40.0,
            48_000.0,
            defaults.high_cutoff_hz,
        );
        check(
            "smoothing",
            &mut self.smoothing,
            0.0,
            0.95,
            defaults.smoothing,
        );

        if self.high_cutoff_hz < self.low_cutoff_hz * 2.0 {
            warnings.push(format!(
                "audio.high_cutoff_hz: must be at least twice low_cutoff_hz ({}), using {}",
                self.low_cutoff_hz,
                self.low_cutoff_hz * 2.0
            ));
            self.high_cutoff_hz = self.low_cutoff_hz * 2.0;
        }
    }
}
