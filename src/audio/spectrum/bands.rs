//! log-spaced frequency bands as weights over fft bins

use std::ops::Range;

/// narrowest stretch of bins a bar averages over; bars closer together than
/// this overlap, which keeps the bass bars from each showing one raw bin
const MIN_WIDTH_BINS: f32 = 2.0;

/// how each bar averages the power spectrum
#[derive(Debug)]
pub struct Bands {
    spans: Vec<Span>,
    weights: Vec<f32>,
    centers_hz: Vec<f32>,
}

#[derive(Debug)]
struct Span {
    first_bin: usize,
    weights: Range<usize>,
}

impl Bands {
    /// `bars` bands between the cutoffs, lowest first
    pub fn new(bars: usize, low_hz: f32, high_hz: f32, sample_rate: u32, fft_len: usize) -> Self {
        let bars = bars.max(1);
        let last_bin = fft_len / 2;
        let hz_per_bin = sample_rate as f32 / fft_len as f32;
        let nyquist = sample_rate as f32 / 2.0;
        let high = high_hz.clamp(hz_per_bin * 2.0, nyquist);
        let low = low_hz.clamp(hz_per_bin, high / 2.0);
        let ratio = high / low;
        let edge = |index: usize| low * ratio.powf(index as f32 / bars as f32);

        let mut bands = Self {
            spans: Vec::with_capacity(bars),
            weights: Vec::new(),
            centers_hz: Vec::with_capacity(bars),
        };
        for index in 0..bars {
            let (from, to) = (edge(index), edge(index + 1));
            bands.centers_hz.push((from * to).sqrt());
            bands.push_span(from / hz_per_bin, to / hz_per_bin, last_bin);
        }
        bands
    }

    /// adds the weights of the band `from..to`, in bins
    fn push_span(&mut self, from: f32, to: f32, last_bin: usize) {
        let widen = (MIN_WIDTH_BINS - (to - from)).max(0.0) / 2.0;
        let (from, to) = (from - widen, to + widen);
        let first_bin = (from.floor().max(0.0) as usize).min(last_bin);
        let end_bin = (to.ceil().max(0.0) as usize).min(last_bin);
        let start = self.weights.len();
        // the spectrum between two bins is taken as a straight line, so a
        // bin's share is the part of its triangle that lies inside the band
        self.weights.extend((first_bin..=end_bin).map(|bin| {
            let bin = bin as f32;
            (triangle_area_up_to(to - bin) - triangle_area_up_to(from - bin)) / (to - from)
        }));
        self.spans.push(Span {
            first_bin,
            weights: start..self.weights.len(),
        });
    }

    pub fn len(&self) -> usize {
        self.spans.len()
    }

    /// geometric middle of each band
    pub fn centers_hz(&self) -> &[f32] {
        &self.centers_hz
    }

    /// writes each band's weighted mean of `power`, one value per fft bin
    pub fn mean_power(&self, power: &[f32], out: &mut [f32]) {
        for (value, span) in out.iter_mut().zip(&self.spans) {
            let weights = self.weights.get(span.weights.clone()).unwrap_or_default();
            let bins = power.get(span.first_bin..).unwrap_or_default();
            *value = weights.iter().zip(bins).map(|(w, p)| w * p).sum();
        }
    }
}

/// area of the unit triangle over `-1.0..=1.0` to the left of `x`
fn triangle_area_up_to(x: f32) -> f32 {
    let x = x.clamp(-1.0, 1.0);
    if x <= 0.0 {
        (x + 1.0) * (x + 1.0) / 2.0
    } else {
        1.0 - (1.0 - x) * (1.0 - x) / 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::Bands;

    const SETUPS: [(usize, u32); 5] = [
        (1, 48000),
        (16, 44100),
        (50, 48000),
        (256, 44100),
        (512, 96000),
    ];

    #[test]
    fn every_band_weighs_bins_inside_the_spectrum_and_sums_to_one() {
        for (bars, rate) in SETUPS {
            let bands = Bands::new(bars, 50.0, 10_000.0, rate, 4096);
            assert_eq!(bands.len(), bars);
            for span in &bands.spans {
                let weights = &bands.weights[span.weights.clone()];
                assert!(span.first_bin + weights.len() <= 4096 / 2 + 1);
                let sum: f32 = weights.iter().sum();
                assert!((sum - 1.0).abs() < 1e-3, "{bars} bars @ {rate}: {sum}");
                assert!(weights.iter().all(|weight| *weight >= 0.0));
            }
        }
    }

    #[test]
    fn a_flat_spectrum_reads_the_same_in_every_band() {
        for (bars, rate) in SETUPS {
            let bands = Bands::new(bars, 50.0, 10_000.0, rate, 4096);
            let mut out = vec![0.0; bars];
            bands.mean_power(&vec![0.25; 4096 / 2 + 1], &mut out);
            assert!(out.iter().all(|value| (value - 0.25).abs() < 1e-3));
        }
    }

    #[test]
    fn centers_rise_between_the_cutoffs() {
        let bands = Bands::new(10, 50.0, 10_000.0, 48000, 4096);
        let centers = bands.centers_hz();
        assert!(centers.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(centers.first().is_some_and(|hz| (50.0..70.0).contains(hz)));
        assert!(
            centers
                .last()
                .is_some_and(|hz| (7_500.0..10_000.0).contains(hz))
        );
    }

    #[test]
    fn narrow_neighbours_share_bins() {
        // 256 bars put the lowest bands well under one bin apart
        let bands = Bands::new(256, 50.0, 10_000.0, 48000, 4096);
        let mut power = vec![0.0; 4096 / 2 + 1];
        power[6] = 1.0;
        let mut out = vec![0.0; 256];
        bands.mean_power(&power, &mut out);
        let lit = out.iter().filter(|value| **value > 0.05).count();
        assert!(lit > 4, "{lit}");
        let steps = out.windows(2).map(|pair| (pair[1] - pair[0]).abs());
        assert!(steps.fold(0.0, f32::max) < 0.2);
    }

    #[test]
    fn high_cutoff_is_capped_at_nyquist() {
        let bands = Bands::new(8, 50.0, 40_000.0, 22050, 1024);
        assert!(bands.centers_hz().iter().all(|hz| *hz < 11_025.0));
    }
}
