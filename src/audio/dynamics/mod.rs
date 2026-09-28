//! turns band amplitudes into bar heights in `0.0..=1.0`

#[cfg(test)]
mod tests;

use crate::config::Config;

/// acceleration of a falling bar, in bar heights per second squared
const GRAVITY: f32 = 9.0;
/// gain change per second while bars overshoot the top
const GAIN_FALL_PER_SECOND: f32 = 1.2;
/// gain change per second while audio stays below the top
const GAIN_RISE_PER_SECOND: f32 = 0.08;
/// faster rise until the first overshoot, so the first seconds are not flat
const GAIN_WARMUP_PER_SECOND: f32 = 3.0;
const GAIN_RANGE: (f32, f32) = (0.01, 5_000.0);
/// smoothing values are defined at this rate and converted to real elapsed time
const SMOOTHING_RATE_HZ: f32 = 60.0;

/// tuning for [`Dynamics`]
#[derive(Debug, Clone, PartialEq)]
pub struct DynamicsConfig {
    /// multiplier on top of the gain; the whole gain when `auto_sensitivity` is off
    pub sensitivity: f32,
    pub auto_sensitivity: bool,
    /// `0.0..1.0`, fraction of the previous height kept per 1/60 s
    pub smoothing: f32,
}

impl DynamicsConfig {
    pub fn from_config(config: &Config) -> Self {
        Self {
            sensitivity: config.audio.sensitivity,
            auto_sensitivity: config.audio.auto_sensitivity,
            smoothing: config.audio.smoothing,
        }
    }
}

impl Default for DynamicsConfig {
    fn default() -> Self {
        Self {
            sensitivity: 1.0,
            auto_sensitivity: true,
            smoothing: 0.7,
        }
    }
}

/// per-bar state that makes the bars move musically
#[derive(Debug)]
pub struct Dynamics {
    config: DynamicsConfig,
    smoothed: Vec<f32>,
    fall_from: Vec<f32>,
    fall_time: Vec<f32>,
    heights: Vec<f32>,
    gain: f32,
    warming_up: bool,
}

impl Dynamics {
    pub fn new(bars: usize, config: DynamicsConfig) -> Self {
        Self {
            smoothed: vec![0.0; bars],
            fall_from: vec![0.0; bars],
            fall_time: vec![0.0; bars],
            heights: vec![0.0; bars],
            gain: 1.0,
            warming_up: config.auto_sensitivity,
            config,
        }
    }

    /// advances by `dt` seconds; `raw` is one amplitude per bar, `None` for
    /// silence, which lets the bars fall without moving the gain
    pub fn update(&mut self, raw: Option<&[f32]>, dt: f32) -> &[f32] {
        let dt = dt.clamp(0.0, 0.25);
        let keep = self.config.smoothing.powf(dt * SMOOTHING_RATE_HZ);
        let gain = self.effective_gain();
        let mut loudest = 0.0_f32;

        for index in 0..self.heights.len() {
            let target = raw
                .and_then(|values| values.get(index))
                .map_or(0.0, |value| value * gain);
            let Some(smoothed) = self.smoothed.get_mut(index) else {
                continue;
            };
            *smoothed = *smoothed * keep + target * (1.0 - keep);
            let smoothed = *smoothed;
            let fallen = self.fall(index, smoothed, dt);
            loudest = loudest.max(fallen);
            if let Some(height) = self.heights.get_mut(index) {
                *height = fallen.min(1.0);
            }
        }

        if raw.is_some() && self.config.auto_sensitivity {
            self.adapt_gain(loudest, dt);
        }
        &self.heights
    }

    /// current bar heights without advancing
    pub fn heights(&self) -> &[f32] {
        &self.heights
    }

    /// whether every bar has come to rest at zero
    pub fn at_rest(&self) -> bool {
        self.heights.iter().all(|height| *height < 1e-4)
    }

    /// gain applied before falloff, including `sensitivity`
    pub fn effective_gain(&self) -> f32 {
        if self.config.auto_sensitivity {
            self.gain * self.config.sensitivity
        } else {
            self.config.sensitivity
        }
    }

    /// rising targets are taken at once, falling ones follow gravity
    fn fall(&mut self, index: usize, target: f32, dt: f32) -> f32 {
        let (Some(from), Some(time)) =
            (self.fall_from.get_mut(index), self.fall_time.get_mut(index))
        else {
            return target;
        };
        let falling = *from - GRAVITY * *time * *time;
        if target >= falling {
            *from = target;
            *time = 0.0;
            return target;
        }
        *time += dt;
        (*from - GRAVITY * *time * *time).max(target)
    }

    fn adapt_gain(&mut self, loudest: f32, dt: f32) {
        let rate = if loudest > 1.0 {
            self.warming_up = false;
            -GAIN_FALL_PER_SECOND
        } else if self.warming_up {
            GAIN_WARMUP_PER_SECOND
        } else {
            GAIN_RISE_PER_SECOND
        };
        self.gain = (self.gain * (rate * dt).exp()).clamp(GAIN_RANGE.0, GAIN_RANGE.1);
    }
}
