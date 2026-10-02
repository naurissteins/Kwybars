//! analysis run on the capture thread: spectrum, spectral tilt, bar order, silence gate

use super::spectrum::{Analyzer, SpectrumConfig, order};

/// peak below which audio counts as digital silence and the fft is skipped (-80 dbfs)
pub const SILENCE_FLOOR: f32 = 1e-4;
/// frequency the tilt is neutral at
const TILT_REFERENCE_HZ: f32 = 1_000.0;
/// amplitude doubles every 6.02 db, so this exponent is 4 db per octave
const TILT_EXPONENT: f32 = 4.0 / 6.02;

/// spectrum plus tilt for one negotiated format
pub struct Pipeline {
    analyzer: Analyzer,
    /// per band loudness compensation, +4 db per octave above the reference
    tilt: Vec<f32>,
    /// band shown by each bar
    band_of_bar: Vec<usize>,
    bars: Vec<f32>,
}

impl Pipeline {
    /// allocates everything; later calls do not allocate
    pub fn new(sample_rate: u32, channels: u32, config: &SpectrumConfig) -> Self {
        let analyzer = Analyzer::new(sample_rate, channels, config);
        let tilt: Vec<f32> = analyzer
            .centers_hz()
            .iter()
            .map(|hz| tilt_for(*hz))
            .collect();
        let band_of_bar: Vec<usize> = (0..config.bars)
            .map(|bar| order::band_of(config.order, bar, config.bars))
            .collect();
        Self {
            bars: vec![0.0; band_of_bar.len()],
            band_of_bar,
            tilt,
            analyzer,
        }
    }

    pub fn push_interleaved(&mut self, sample: f32) {
        self.analyzer.push_interleaved(sample);
    }

    /// tilted amplitude per bar over the newest window, or `None` without running
    /// the fft when `peak`, the level of the new samples, is digital silence
    pub fn analyze(&mut self, peak: f32) -> Option<&[f32]> {
        if peak < SILENCE_FLOOR {
            return None;
        }
        let raw = self.analyzer.analyze();
        for (bar, band) in self.bars.iter_mut().zip(&self.band_of_bar) {
            let level = raw.get(*band).copied().unwrap_or_default();
            *bar = level * self.tilt.get(*band).copied().unwrap_or_default();
        }
        Some(&self.bars)
    }
}

/// +4 db per octave relative to [`TILT_REFERENCE_HZ`], since music gets
/// quieter towards the treble
fn tilt_for(center_hz: f32) -> f32 {
    (center_hz.max(1.0) / TILT_REFERENCE_HZ).powf(TILT_EXPONENT)
}

#[cfg(test)]
mod tests {
    use std::f32::consts::TAU;

    use super::{Pipeline, tilt_for};
    use crate::audio::spectrum::SpectrumConfig;
    use crate::config::BarOrder;

    #[test]
    fn tilt_is_four_db_per_octave() {
        let db = |hz: f32| 20.0 * tilt_for(hz).log10();
        assert!(db(1_000.0).abs() < 1e-4);
        assert!((db(4_000.0) - 8.0).abs() < 1e-3);
        assert!((db(250.0) + 8.0).abs() < 1e-3);
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

    #[test]
    fn bass_center_puts_a_low_tone_in_the_middle_of_the_row() {
        let config = SpectrumConfig {
            bars: 21,
            order: BarOrder::BassCenter,
            ..SpectrumConfig::default()
        };
        let mut pipeline = Pipeline::new(48_000, 1, &config);
        for index in 0..8_192 {
            let t = index as f32 / 48_000.0;
            pipeline.push_interleaved(0.5 * (TAU * 60.0 * t).sin());
        }
        let bars = pipeline
            .analyze(0.5)
            .map(<[f32]>::to_vec)
            .unwrap_or_default();
        assert_eq!(bars.len(), 21);
        let loudest = bars.iter().copied().fold(0.0, f32::max);
        assert_eq!(bars.get(10), Some(&loudest));
        let mirrored: Vec<f32> = bars.iter().rev().copied().collect();
        assert_eq!(bars, mirrored);
    }
}
