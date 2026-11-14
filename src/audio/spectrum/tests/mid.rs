use std::f32::consts::TAU;

use crate::audio::pipeline::Pipeline;
use crate::audio::spectrum::{Analyzer, SpectrumConfig};

fn config() -> SpectrumConfig {
    SpectrumConfig {
        bass: Some((30.0, 150.0)),
        mid: Some((250.0, 4000.0)),
        ..SpectrumConfig::default()
    }
}

fn tone(analyzer: &mut Analyzer, rate: u32, hz: f32, amplitude: f32) -> (f32, f32) {
    for index in 0..analyzer.fft_len() {
        analyzer.push_interleaved(amplitude * (TAU * hz * index as f32 / rate as f32).sin());
    }
    analyzer.analyze();
    (analyzer.bass(), analyzer.mid())
}

#[test]
fn middle_transients_are_isolated_from_bass_and_highs_at_negotiated_rates() {
    for rate in [22050, 44100, 48000, 96000] {
        let mut analyzer = Analyzer::new(rate, 1, &config());
        let (bass, mid) = tone(&mut analyzer, rate, 1000.0, 0.5);
        assert!(mid > 0.005, "{rate}: {mid}");
        assert!(bass < mid * 0.001);
        for hz in [60.0, 8000.0] {
            let mut outside = Analyzer::new(rate, 1, &config());
            assert!(
                tone(&mut outside, rate, hz, 0.5).1 < mid * 0.001,
                "{rate}: {hz}"
            );
        }
    }
}

#[test]
fn sustained_and_falling_energy_do_not_repeat_the_initial_hit() {
    let mut analyzer = Analyzer::new(48000, 1, &config());
    let first = tone(&mut analyzer, 48000, 1000.0, 0.5).1;
    for _ in 0..4 {
        assert!(tone(&mut analyzer, 48000, 1000.0, 0.5).1 < first * 0.001);
    }
    assert!(tone(&mut analyzer, 48000, 1000.0, 0.1).1 < first * 0.001);
    assert!(tone(&mut analyzer, 48000, 1000.0, 0.8).1 > first);
}

#[test]
fn reset_and_digital_silence_rearm_identical_hits() {
    let mut analyzer = Analyzer::new(48000, 1, &config());
    let first = tone(&mut analyzer, 48000, 1000.0, 0.5).1;
    analyzer.reset_transients();
    assert_eq!(analyzer.mid(), 0.0);
    assert!((tone(&mut analyzer, 48000, 1000.0, 0.5).1 - first).abs() < 1e-6);

    let mut pipeline = Pipeline::new(48000, 1, &config());
    for index in 0..8192 {
        pipeline.push_interleaved(0.5 * (TAU * 1000.0 * index as f32 / 48000.0).sin());
    }
    let first = pipeline.analyze(0.5).map_or(0.0, |frame| frame.mid);
    assert!(pipeline.analyze(0.0).is_none());
    let next = pipeline.analyze(0.5).map_or(0.0, |frame| frame.mid);
    assert!(first > 0.0 && (next - first).abs() < 1e-6);
}

#[test]
fn disabled_midrange_and_visualizer_settings_do_not_change_background_signals() {
    let mut one = Analyzer::new(48000, 1, &config());
    let expected = tone(&mut one, 48000, 1000.0, 0.5);
    let mut changed = config();
    changed.bars = 9;
    changed.low_cutoff_hz = 5000.0;
    changed.high_cutoff_hz = 10000.0;
    changed.order = crate::config::BarOrder::BassEdges;
    let mut two = Analyzer::new(48000, 1, &changed);
    assert_eq!(tone(&mut two, 48000, 1000.0, 0.5), expected);
    changed.mid = None;
    let mut off = Analyzer::new(48000, 1, &changed);
    assert_eq!(tone(&mut off, 48000, 1000.0, 0.5).1, 0.0);
    assert!(off.mid_previous.is_empty());
}

#[test]
fn burst_timing_uses_the_shared_window_without_waiting_for_a_full_new_window() {
    for (rate, hop) in [(48000, 256), (48000, 2048), (44100, 512), (96000, 1024)] {
        let mut analyzer = Analyzer::new(rate, 1, &config());
        let window = analyzer.fft_len();
        let mut first = None;
        let mut peak = (0, 0.0_f32);
        for frame in 0..(window * 2 / hop) {
            for i in 0..hop {
                let sample = frame * hop + i;
                let amplitude = if sample < rate as usize / 50 {
                    0.7
                } else {
                    0.0
                };
                analyzer.push_interleaved(
                    amplitude * (TAU * 1000.0 * sample as f32 / rate as f32).sin(),
                );
            }
            analyzer.analyze();
            let mid = analyzer.mid();
            let elapsed = (frame + 1) * hop;
            if mid > 0.001 && first.is_none() {
                first = Some(elapsed);
            }
            if mid > peak.1 {
                peak = (elapsed, mid);
            }
        }
        let Some(first) = first else {
            panic!("burst was missed at {rate}/{hop}")
        };
        assert!(
            first < window && peak.0 <= window,
            "{rate}/{hop}: {first} {peak:?}"
        );
        eprintln!(
            "mid burst {rate}/{hop}: first {:.1} ms, peak {:.1} ms ({:.5})",
            first as f32 * 1000.0 / rate as f32,
            peak.0 as f32 * 1000.0 / rate as f32,
            peak.1
        );
    }
}
