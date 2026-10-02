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

/// the bar whose middle frequency is closest to `hz`
fn bar_of(analyzer: &Analyzer, hz: f32) -> usize {
    let distance = |center: &f32| (center / hz).ln().abs();
    let centers = analyzer.centers_hz();
    (0..centers.len())
        .min_by(|a, b| distance(&centers[*a]).total_cmp(&distance(&centers[*b])))
        .unwrap_or_default()
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
fn sines_land_in_their_bar() {
    for bars in [8, 32, 50, 100, 256] {
        for hz in [60.0, 440.0, 2_500.0, 8_000.0] {
            let mut analyzer = analyzer(bars, 2);
            feed_sine(&mut analyzer, hz, 0.5, 2);
            let expected = bar_of(&analyzer, hz);
            let values = analyzer.analyze().to_vec();
            let found = loudest(&values);
            assert!(found.abs_diff(expected) <= 1, "{hz} Hz with {bars} bars");
            // a bar shows the mean over its band, so a lone sine reads lower
            // the wider the band is; it never reads above its amplitude
            let hz_per_bin = RATE as f32 / analyzer.fft_len() as f32;
            let width = hz * (200.0_f32.powf(1.0 / bars as f32) - 1.0) / hz_per_bin;
            let lowest = 0.5 * (1.5 / (width + 3.0)).sqrt() * 0.8;
            let level = values[found];
            assert!(
                (lowest..=0.51).contains(&level),
                "{hz} Hz with {bars} bars: {level}, expected at least {lowest}"
            );
        }
    }
}

#[test]
fn neighbouring_bass_bars_move_together() {
    let mut analyzer = analyzer(50, 1);
    feed_sine(&mut analyzer, 62.0, 0.5, 1);
    let values = analyzer.analyze().to_vec();
    let peak = loudest(&values);
    for pair in values[..12].windows(2) {
        assert!((pair[1] - pair[0]).abs() < 0.15, "{values:?}");
    }
    assert!(values[peak] > 0.3, "{values:?}");
}

#[test]
fn distant_bars_stay_quiet() {
    let mut analyzer = analyzer(50, 1);
    feed_sine(&mut analyzer, 1_000.0, 1.0, 1);
    let centers = analyzer.centers_hz().to_vec();
    let values = analyzer.analyze().to_vec();
    for (center, value) in centers.iter().zip(values) {
        if !(800.0..1_250.0).contains(center) {
            assert!(value < 0.03, "{center} Hz leaked {value}");
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
    assert!((0.3..=0.52).contains(&peak), "{peak}");
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
        analyzer.power.as_ptr(),
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
        analyzer.power.as_ptr(),
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

/// prints per-bar raw levels of a recording, for tuning the dynamics
///
/// `pw-cat --record --raw --format f32 --rate 48000 --channels 2 -P stream.capture.sink=true music.f32`
/// then `KWYBARS_RAW_AUDIO=music.f32 cargo test --release --lib -- --ignored --nocapture band_profile`
#[test]
#[ignore = "needs KWYBARS_RAW_AUDIO"]
fn band_profile() {
    let Some(path) = std::env::var_os("KWYBARS_RAW_AUDIO") else {
        panic!("set KWYBARS_RAW_AUDIO to a raw f32 stereo 48 kHz file");
    };
    let bytes = std::fs::read(path).unwrap_or_else(|err| panic!("{err}"));
    let bars: usize = std::env::var("KWYBARS_BARS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(30);
    let mut analyzer = analyzer(bars, 2);
    let mut levels: Vec<Vec<f32>> = vec![Vec::new(); bars];
    for (index, raw) in bytes.as_chunks::<4>().0.iter().enumerate() {
        analyzer.push_interleaved(f32::from_le_bytes(*raw));
        if index % 4096 == 4095 {
            for (bar, value) in analyzer.analyze().iter().enumerate() {
                levels[bar].push(*value);
            }
        }
    }
    let centers = analyzer.centers_hz().to_vec();
    for (bar, values) in levels.iter_mut().enumerate() {
        values.sort_by(f32::total_cmp);
        let at = |q: f32| values[((values.len() - 1) as f32 * q) as usize];
        let db = |v: f32| 20.0 * v.max(1e-9).log10();
        println!(
            "bar {bar:3} {:7.0} Hz  median {:6.1} dB  p90 {:6.1} dB",
            centers[bar],
            db(at(0.5)),
            db(at(0.9))
        );
    }
}

/// pink noise from a fixed seed, peak about 0.5
fn pink_noise(samples: usize) -> Vec<f32> {
    let mut state = 0x2545_f491_4f6c_dd1d_u64;
    let mut rows = [0.0_f32; 3];
    (0..samples)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let white = (state >> 40) as f32 / (1u64 << 23) as f32 - 1.0;
            // paul kellet's economy filter
            rows[0] = 0.99765 * rows[0] + white * 0.099_046;
            rows[1] = 0.963 * rows[1] + white * 0.296_516_4;
            rows[2] = 0.57 * rows[2] + white * 1.052_691_3;
            (rows[0] + rows[1] + rows[2] + white * 0.1848) * 0.1
        })
        .collect()
}

#[test]
#[ignore = "prints a table for tuning"]
fn pink_profile() {
    let bars = 50;
    let mut analyzer = analyzer(bars, 1);
    let mut levels: Vec<Vec<f32>> = vec![Vec::new(); bars];
    for (index, sample) in pink_noise(RATE as usize * 20).into_iter().enumerate() {
        analyzer.push_interleaved(sample);
        if index % 2048 == 2047 && index > 8192 {
            for (bar, value) in analyzer.analyze().iter().enumerate() {
                levels[bar].push(*value);
            }
        }
    }
    let frames = levels[0].len();
    let jump: f32 = levels
        .windows(2)
        .flat_map(|pair| pair[0].iter().zip(&pair[1]))
        .map(|(lower, upper)| (20.0 * (upper / lower).log10()).abs())
        .sum();
    println!(
        "adjacent bars differ by {:.2} dB on average",
        jump / (frames * (bars - 1)) as f32
    );
    for (bar, values) in levels.iter().enumerate().step_by(5) {
        let mean = values.iter().sum::<f32>() / frames as f32;
        let spread =
            (values.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / frames as f32).sqrt();
        println!(
            "bar {bar:2}  mean {:6.1} dB  spread {:.2}",
            20.0 * mean.log10(),
            spread / mean
        );
    }
}
