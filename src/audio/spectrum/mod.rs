//! spectrum analysis: mono downmix, hann window, fft, power in log-spaced bands

mod bands;
mod history;
pub mod order;

#[cfg(test)]
mod tests;

use std::f32::consts::TAU;
use std::sync::Arc;

use realfft::num_complex::Complex;
use realfft::{RealFftPlanner, RealToComplex};

use bands::Bands;
use history::History;

use crate::config::{BarOrder, Config};

/// analysis window length the fft size is derived from, in seconds
const WINDOW_SECONDS: f32 = 0.085;

/// what to analyze
#[derive(Debug, Clone, PartialEq)]
pub struct SpectrumConfig {
    pub bars: usize,
    pub order: BarOrder,
    pub low_cutoff_hz: f32,
    pub high_cutoff_hz: f32,
}

impl SpectrumConfig {
    pub fn from_config(config: &Config) -> Self {
        Self {
            bars: config.visualizer.bars.max(1),
            order: config.visualizer.bar_order,
            low_cutoff_hz: config.audio.low_cutoff_hz,
            high_cutoff_hz: config.audio.high_cutoff_hz,
        }
    }
}

impl Default for SpectrumConfig {
    fn default() -> Self {
        Self {
            bars: 50,
            order: BarOrder::LowToHigh,
            low_cutoff_hz: 50.0,
            high_cutoff_hz: 10_000.0,
        }
    }
}

/// turns captured samples into one amplitude per frequency band
pub struct Analyzer {
    fft: Arc<dyn RealToComplex<f32>>,
    history: History,
    window: Vec<f32>,
    input: Vec<f32>,
    spectrum: Vec<Complex<f32>>,
    scratch: Vec<Complex<f32>>,
    power: Vec<f32>,
    bands: Bands,
    bars: Vec<f32>,
    /// maps a bin magnitude so a full-scale sine reads 1.0
    scale: f32,
}

impl Analyzer {
    /// allocates everything the analysis needs; later calls do not allocate
    pub fn new(sample_rate: u32, channels: u32, config: &SpectrumConfig) -> Self {
        let len = fft_len(sample_rate);
        let fft = RealFftPlanner::<f32>::new().plan_fft_forward(len);
        let window = hann(len);
        let scale = 2.0 / window.iter().sum::<f32>();
        let bands = Bands::new(
            order::band_count(config.order, config.bars),
            config.low_cutoff_hz,
            config.high_cutoff_hz,
            sample_rate,
            len,
        );
        Self {
            history: History::new(len, channels as usize),
            input: fft.make_input_vec(),
            spectrum: fft.make_output_vec(),
            scratch: fft.make_scratch_vec(),
            power: vec![0.0; len / 2 + 1],
            bars: vec![0.0; bands.len()],
            bands,
            window,
            scale,
            fft,
        }
    }

    /// adds one interleaved sample
    pub fn push_interleaved(&mut self, sample: f32) {
        self.history.push_interleaved(sample);
    }

    /// rms amplitude per band over the newest window, lowest frequency first;
    /// a full-scale sine reads 1.0 in a band no wider than the sine's peak
    pub fn analyze(&mut self) -> &[f32] {
        self.history.copy_ordered(&mut self.input);
        for (sample, weight) in self.input.iter_mut().zip(&self.window) {
            *sample *= weight;
        }
        let transformed =
            self.fft
                .process_with_scratch(&mut self.input, &mut self.spectrum, &mut self.scratch);
        if transformed.is_err() {
            // buffer lengths come from the planner and never change
            self.bars.fill(0.0);
            return &self.bars;
        }
        for (power, bin) in self.power.iter_mut().zip(&self.spectrum) {
            *power = bin.norm_sqr();
        }
        self.bands.mean_power(&self.power, &mut self.bars);
        for bar in &mut self.bars {
            *bar = bar.sqrt() * self.scale;
        }
        &self.bars
    }

    /// fft length in samples
    pub fn fft_len(&self) -> usize {
        self.input.len()
    }

    /// middle frequency of each band
    pub fn centers_hz(&self) -> &[f32] {
        self.bands.centers_hz()
    }
}

/// power of two covering [`WINDOW_SECONDS`] at `sample_rate`
fn fft_len(sample_rate: u32) -> usize {
    ((sample_rate as f32 * WINDOW_SECONDS) as usize)
        .next_power_of_two()
        .clamp(1024, 16_384)
}

/// periodic hann window
fn hann(len: usize) -> Vec<f32> {
    (0..len)
        .map(|index| 0.5 - 0.5 * (TAU * index as f32 / len as f32).cos())
        .collect()
}
