//! analysis run on the capture thread: spectrum, spectral tilt, silence gate

use super::spectrum::{Analyzer, SpectrumConfig};

/// peak below which audio counts as digital silence and the fft is skipped (-80 dbfs)
pub const SILENCE_FLOOR: f32 = 1e-4;
/// frequency the tilt is neutral at
const TILT_REFERENCE_HZ: f32 = 1_000.0;

/// spectrum plus tilt for one negotiated format
pub struct Pipeline {
    analyzer: Analyzer,
    /// per bar loudness compensation, +3 db per octave above the reference
    tilt: Vec<f32>,
    tilted: Vec<f32>,
}

impl Pipeline {
    /// allocates everything; later calls do not allocate
    pub fn new(sample_rate: u32, channels: u32, config: &SpectrumConfig) -> Self {
        let analyzer = Analyzer::new(sample_rate, channels, config);
        let hz_per_bin = sample_rate as f32 / analyzer.fft_len() as f32;
        let tilt: Vec<f32> = analyzer
            .bands()
            .iter()
            .map(|band| tilt_for((band.start.max(1) as f32 * band.end as f32).sqrt() * hz_per_bin))
            .collect();
        Self {
            tilted: vec![0.0; tilt.len()],
            tilt,
            analyzer,
        }
    }

    pub fn push_interleaved(&mut self, sample: f32) {
        self.analyzer.push_interleaved(sample);
    }

    /// tilted band amplitudes of the newest window, or `None` without running
    /// the fft when `peak`, the level of the new samples, is digital silence
    pub fn analyze(&mut self, peak: f32) -> Option<&[f32]> {
        if peak < SILENCE_FLOOR {
            return None;
        }
        let raw = self.analyzer.analyze();
        for ((out, value), tilt) in self.tilted.iter_mut().zip(raw).zip(&self.tilt) {
            *out = value * tilt;
        }
        Some(&self.tilted)
    }
}

/// +3 db per octave relative to [`TILT_REFERENCE_HZ`], since music gets
/// quieter towards the treble
fn tilt_for(center_hz: f32) -> f32 {
    (center_hz.max(1.0) / TILT_REFERENCE_HZ).sqrt()
}

#[cfg(test)]
mod tests {
    use std::f32::consts::TAU;

    use super::{Pipeline, tilt_for};
    use crate::audio::spectrum::SpectrumConfig;

    #[test]
    fn tilt_is_three_db_per_octave() {
        assert!((tilt_for(1_000.0) - 1.0).abs() < 1e-6);
        assert!((tilt_for(4_000.0) - 2.0).abs() < 1e-6);
        assert!((tilt_for(250.0) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn sound_is_analyzed_and_silence_is_skipped() {
        let config = SpectrumConfig::default();
        let mut pipeline = Pipeline::new(48_000, 1, &config);
        for index in 0..8_192 {
            let t = index as f32 / 48_000.0;
            pipeline.push_interleaved(0.5 * (TAU * 440.0 * t).sin());
        }
        let bars = pipeline.analyze(0.5).map(<[f32]>::to_vec);
        assert_eq!(bars.as_ref().map(Vec::len), Some(config.bars));
        assert!(bars.is_some_and(|bars| bars.iter().any(|value| *value > 0.1)));
        assert!(pipeline.analyze(0.0).is_none());
    }
}
