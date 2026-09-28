use std::f32::consts::TAU;

use super::{Analyzer, SpectrumConfig, fft_len};

const RATE: u32 = 48_000;

fn analyzer(bars: usize, channels: u32) -> Analyzer {
    let config = SpectrumConfig {
        bars,
        ..SpectrumConfig::default()
    };
    Analyzer::new(RATE, channels, &config)
}

/// feeds one window of a sine with the same value on every channel
fn feed_sine(analyzer: &mut Analyzer, hz: f32, amplitude: f32, channels: u32) {
    for index in 0..analyzer.fft_len() {
        let value = amplitude * (TAU * hz * index as f32 / RATE as f32).sin();
        for _ in 0..channels {
            analyzer.push_interleaved(value);
        }
    }
}

fn bin_of(analyzer: &Analyzer, hz: f32) -> usize {
    (hz * analyzer.fft_len() as f32 / RATE as f32).round() as usize
}

fn loudest(bars: &[f32]) -> usize {
    bars.iter()
        .enumerate()
        .fold((0, f32::MIN), |best, (index, value)| {
            if *value > best.1 {
                (index, *value)
            } else {
                best
            }
        })
        .0
}

#[test]
fn fft_length_follows_the_sample_rate() {
    assert_eq!(fft_len(22_050), 2048);
    assert_eq!(fft_len(44_100), 4096);
    assert_eq!(fft_len(48_000), 4096);
    assert_eq!(fft_len(96_000), 8192);
    assert_eq!(fft_len(8_000), 1024);
}

#[test]
fn sines_land_in_their_bar_at_their_amplitude() {
    for bars in [8, 32, 50, 100, 256] {
        for hz in [60.0, 440.0, 2_500.0, 8_000.0] {
            let mut analyzer = analyzer(bars, 2);
            feed_sine(&mut analyzer, hz, 0.5, 2);
            let bin = bin_of(&analyzer, hz);
            let expected = analyzer.bands().iter().position(|band| band.contains(&bin));
            let values = analyzer.analyze().to_vec();
            let found = loudest(&values);
            assert_eq!(Some(found), expected, "{hz} Hz with {bars} bars");
            let level = values[found];
            assert!(
                (0.4..=0.52).contains(&level),
                "{hz} Hz with {bars} bars: {level}"
            );
        }
    }
}

#[test]
fn distant_bars_stay_quiet() {
    let mut analyzer = analyzer(50, 1);
    feed_sine(&mut analyzer, 1_000.0, 1.0, 1);
    let bin = bin_of(&analyzer, 1_000.0);
    let bands = analyzer.bands().to_vec();
    let values = analyzer.analyze().to_vec();
    for (band, value) in bands.iter().zip(values) {
        let distance = band.start.abs_diff(bin).min(band.end.abs_diff(bin + 1));
        if distance > 3 {
            assert!(value < 0.03, "band {band:?} leaked {value}");
        }
    }
}

#[test]
fn silence_is_zero() {
    let mut analyzer = analyzer(50, 2);
    for _ in 0..analyzer.fft_len() * 2 {
        analyzer.push_interleaved(0.0);
    }
    assert!(analyzer.analyze().iter().all(|value| *value == 0.0));
}

#[test]
fn stereo_is_averaged_to_mono() {
    let mut analyzer = analyzer(50, 2);
    for index in 0..analyzer.fft_len() {
        let left = (TAU * 440.0 * index as f32 / RATE as f32).sin();
        analyzer.push_interleaved(left);
        analyzer.push_interleaved(0.0);
    }
    let values = analyzer.analyze().to_vec();
    let peak = values[loudest(&values)];
    assert!((0.4..=0.52).contains(&peak), "{peak}");
}

#[test]
fn one_value_per_bar() {
    for bars in [1, 7, 64] {
        assert_eq!(analyzer(bars, 2).analyze().len(), bars);
    }
}

#[test]
fn analyzing_does_not_reallocate() {
    let mut analyzer = analyzer(64, 2);
    let before = (
        analyzer.input.as_ptr(),
        analyzer.spectrum.as_ptr(),
        analyzer.scratch.as_ptr(),
        analyzer.bars.as_ptr(),
        analyzer.bars.capacity(),
    );
    for round in 0..50 {
        feed_sine(&mut analyzer, 100.0 + round as f32 * 50.0, 0.8, 2);
        analyzer.analyze();
    }
    let after = (
        analyzer.input.as_ptr(),
        analyzer.spectrum.as_ptr(),
        analyzer.scratch.as_ptr(),
        analyzer.bars.as_ptr(),
        analyzer.bars.capacity(),
    );
    assert_eq!(before, after);
}

/// `cargo test --release --lib -- --ignored --nocapture analyze_cost`
#[test]
#[ignore = "timing, run in release"]
fn analyze_cost() {
    for (rate, bars) in [(48_000, 50), (48_000, 256), (96_000, 50)] {
        let config = SpectrumConfig {
            bars,
            ..SpectrumConfig::default()
        };
        let mut analyzer = Analyzer::new(rate, 2, &config);
        feed_sine(&mut analyzer, 440.0, 0.5, 2);
        let rounds = 2_000;
        let start = std::time::Instant::now();
        for _ in 0..rounds {
            std::hint::black_box(analyzer.analyze());
        }
        let micros = start.elapsed().as_secs_f64() * 1e6 / f64::from(rounds);
        println!(
            "{rate} Hz, {bars} bars, fft {}: {micros:.1} us per analyze",
            analyzer.fft_len()
        );
    }
}
