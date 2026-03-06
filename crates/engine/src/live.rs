use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use kwybars_common::config::{VisualizerBackend, VisualizerConfig};
use kwybars_common::spectrum::SpectrumFrame;
use tracing::{error, warn};

use crate::pipeline::{DummySineSource, FrameSource};

const CAVA_ATTACK: f32 = 0.8;
const CAVA_DECAY: f32 = 0.84;

#[derive(Debug, Clone, Copy)]
struct PipewireTuning {
    attack: f32,
    decay: f32,
    gain: f32,
    curve: f32,
    neighbor_mix: f32,
}

impl PipewireTuning {
    fn from_config(config: &VisualizerConfig) -> Self {
        Self {
            attack: config.pipewire_attack.clamp(0.01, 1.0),
            decay: config.pipewire_decay.clamp(0.5, 0.9995),
            gain: config.pipewire_gain.clamp(0.1, 6.0),
            curve: config.pipewire_curve.clamp(0.4, 2.5),
            neighbor_mix: config.pipewire_neighbor_mix.clamp(0.0, 0.45),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    Pipewire,
    Cava,
    Dummy,
}

pub struct LiveFrameStream {
    latest: Arc<Mutex<SpectrumFrame>>,
    source_kind: SourceKind,
}

impl LiveFrameStream {
    pub fn spawn(config: VisualizerConfig) -> Self {
        let bar_count = config.bars.max(1);
        let latest = Arc::new(Mutex::new(SpectrumFrame::new(
            vec![0.0; bar_count],
            now_millis(),
        )));
        let framerate = config.framerate.max(1);
        let pipewire_tuning = PipewireTuning::from_config(&config);

        let source_kind = match config.backend {
            VisualizerBackend::Dummy => {
                spawn_dummy_thread(Arc::clone(&latest), bar_count, framerate);
                SourceKind::Dummy
            }
            VisualizerBackend::Pipewire => {
                if spawn_pipewire_thread(Arc::clone(&latest), bar_count, pipewire_tuning).is_ok() {
                    SourceKind::Pipewire
                } else if spawn_cava_thread(Arc::clone(&latest), bar_count, framerate).is_ok() {
                    SourceKind::Cava
                } else {
                    warn!("kwybars: falling back to dummy frame source");
                    spawn_dummy_thread(Arc::clone(&latest), bar_count, framerate);
                    SourceKind::Dummy
                }
            }
            VisualizerBackend::Cava => {
                if spawn_cava_thread(Arc::clone(&latest), bar_count, framerate).is_ok() {
                    SourceKind::Cava
                } else if spawn_pipewire_thread(Arc::clone(&latest), bar_count, pipewire_tuning)
                    .is_ok()
                {
                    SourceKind::Pipewire
                } else {
                    warn!("kwybars: falling back to dummy frame source");
                    spawn_dummy_thread(Arc::clone(&latest), bar_count, framerate);
                    SourceKind::Dummy
                }
            }
            VisualizerBackend::Auto => {
                if spawn_cava_thread(Arc::clone(&latest), bar_count, framerate).is_ok() {
                    SourceKind::Cava
                } else if spawn_pipewire_thread(Arc::clone(&latest), bar_count, pipewire_tuning)
                    .is_ok()
                {
                    SourceKind::Pipewire
                } else {
                    warn!("kwybars: falling back to dummy frame source");
                    spawn_dummy_thread(Arc::clone(&latest), bar_count, framerate);
                    SourceKind::Dummy
                }
            }
        };

        Self {
            latest,
            source_kind,
        }
    }

    pub fn source_kind(&self) -> SourceKind {
        self.source_kind
    }

    pub fn latest_frame(&self) -> SpectrumFrame {
        match self.latest.lock() {
            Ok(frame) => frame.clone(),
            Err(_) => SpectrumFrame::new(Vec::new(), now_millis()),
        }
    }
}

fn spawn_dummy_thread(latest: Arc<Mutex<SpectrumFrame>>, bar_count: usize, framerate: u32) {
    let frame_delay = Duration::from_millis((1000_u64 / u64::from(framerate)).max(1));

    thread::spawn(move || {
        let mut source = DummySineSource::new(bar_count);
        loop {
            let frame = source.next_frame();
            if let Ok(mut target) = latest.lock() {
                *target = frame;
            }
            thread::sleep(frame_delay);
        }
    });
}

fn spawn_pipewire_thread(
    latest: Arc<Mutex<SpectrumFrame>>,
    bar_count: usize,
    tuning: PipewireTuning,
) -> std::io::Result<()> {
    let mut command = Command::new("pw-cat");
    command
        .arg("--record")
        .arg("--raw")
        .arg("--format")
        .arg("f32")
        .arg("--rate")
        .arg("48000")
        .arg("--channels")
        .arg("2")
        .arg("--latency")
        .arg("64")
        .arg("--media-category")
        .arg("Capture")
        .arg("--media-role")
        .arg("Music")
        .arg("-P")
        .arg("stream.capture.sink=true")
        .arg("-")
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    let mut child = command.spawn()?;

    // Detect immediate startup failures so auto mode can fall back quickly.
    thread::sleep(Duration::from_millis(120));
    if let Some(status) = child.try_wait()? {
        return Err(std::io::Error::other(format!(
            "pw-cat exited early with status {status}"
        )));
    }

    thread::spawn(move || {
        let stdout = match child.stdout.take() {
            Some(stdout) => stdout,
            None => {
                error!("kwybars: pw-cat did not provide stdout");
                let _ = child.kill();
                return;
            }
        };

        let mut reader = BufReader::new(stdout);
        let mut read_buf = vec![0_u8; 8192];
        let mut pending = Vec::<u8>::new();
        let mut smoothed = vec![0.0_f32; bar_count];
        let frame_stride = 2 * std::mem::size_of::<f32>();

        loop {
            let read = match reader.read(&mut read_buf) {
                Ok(0) => break,
                Ok(value) => value,
                Err(err) => {
                    error!("kwybars: error reading pw-cat output: {err}");
                    break;
                }
            };

            pending.extend_from_slice(&read_buf[..read]);
            let usable = pending.len() - (pending.len() % frame_stride);
            if usable < frame_stride {
                continue;
            }

            let bars = bars_from_interleaved_f32le(&pending[..usable], 2, bar_count, tuning);
            apply_decay_smoothing(&mut smoothed, &bars, tuning.attack, tuning.decay);
            let frame = SpectrumFrame::new(smoothed.clone(), now_millis());
            if let Ok(mut target) = latest.lock() {
                *target = frame;
            }

            let tail = pending.split_off(usable);
            pending = tail;
        }

        let _ = child.kill();
    });

    Ok(())
}

fn spawn_cava_thread(
    latest: Arc<Mutex<SpectrumFrame>>,
    bar_count: usize,
    framerate: u32,
) -> std::io::Result<()> {
    let config_path = write_cava_config(bar_count, framerate)?;

    thread::spawn(move || {
        let mut command = Command::new("cava");
        command
            .arg("-p")
            .arg(&config_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(err) => {
                error!("kwybars: failed to start cava: {err}");
                let _ = fs::remove_file(&config_path);
                return;
            }
        };

        let stdout = match child.stdout.take() {
            Some(stdout) => stdout,
            None => {
                error!("kwybars: cava did not provide stdout");
                let _ = fs::remove_file(&config_path);
                let _ = child.kill();
                return;
            }
        };

        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        let mut smoothed = vec![0.0_f32; bar_count];
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    if let Some(bars) = parse_cava_line(&line, bar_count) {
                        apply_decay_smoothing(&mut smoothed, &bars, CAVA_ATTACK, CAVA_DECAY);
                        let frame = SpectrumFrame::new(smoothed.clone(), now_millis());
                        if let Ok(mut target) = latest.lock() {
                            *target = frame;
                        }
                    }
                }
                Err(err) => {
                    error!("kwybars: error reading cava output: {err}");
                    break;
                }
            }
        }

        let _ = fs::remove_file(&config_path);
        let _ = child.kill();
    });

    Ok(())
}

fn apply_decay_smoothing(smoothed: &mut [f32], input: &[f32], attack: f32, decay: f32) {
    for (current, next) in smoothed.iter_mut().zip(input.iter()) {
        let target = next.clamp(0.0, 1.0);
        if target > *current {
            *current = (*current * (1.0 - attack)) + (target * attack);
        } else {
            *current *= decay;
            if *current < target {
                *current = target;
            }
        }
    }
}

fn bars_from_interleaved_f32le(
    bytes: &[u8],
    channels: usize,
    bar_count: usize,
    tuning: PipewireTuning,
) -> Vec<f32> {
    if bar_count == 0 {
        return Vec::new();
    }

    let channels = channels.max(1);
    let bytes_per_frame = channels * std::mem::size_of::<f32>();
    if bytes.len() < bytes_per_frame {
        return vec![0.0; bar_count];
    }

    let frame_count = bytes.len() / bytes_per_frame;
    if frame_count == 0 {
        return vec![0.0; bar_count];
    }

    let mut bin_energy = vec![0.0_f32; bar_count];
    let mut bin_count = vec![0_u32; bar_count];

    for frame_idx in 0..frame_count {
        let frame_base = frame_idx * bytes_per_frame;
        let mut sample_sq_sum = 0.0_f32;
        let mut channel_count = 0_u32;

        for channel in 0..channels {
            let sample_offset = frame_base + (channel * std::mem::size_of::<f32>());
            let sample = f32::from_le_bytes([
                bytes[sample_offset],
                bytes[sample_offset + 1],
                bytes[sample_offset + 2],
                bytes[sample_offset + 3],
            ]);
            if sample.is_finite() {
                sample_sq_sum += sample * sample;
                channel_count += 1;
            }
        }

        if channel_count == 0 {
            continue;
        }

        let amplitude_rms = (sample_sq_sum / channel_count as f32).sqrt();
        let bin = frame_idx * bar_count / frame_count;
        if let Some(value) = bin_energy.get_mut(bin) {
            *value += amplitude_rms * amplitude_rms;
        }
        if let Some(count) = bin_count.get_mut(bin) {
            *count += 1;
        }
    }

    let mut bars = vec![0.0_f32; bar_count];
    for (idx, value) in bars.iter_mut().enumerate() {
        let count = bin_count[idx];
        if count > 0 {
            *value = (bin_energy[idx] / count as f32).sqrt();
        }
    }

    // Neighbor blend smoothes sharp isolated spikes that feel too aggressive.
    if bar_count > 1 && tuning.neighbor_mix > 0.0 {
        let mut blended = bars.clone();
        let center_weight = (1.0 - (2.0 * tuning.neighbor_mix)).max(0.05);
        for idx in 0..bar_count {
            let mut sum = bars[idx] * center_weight;
            let mut weight = center_weight;

            if idx > 0 {
                sum += bars[idx - 1] * tuning.neighbor_mix;
                weight += tuning.neighbor_mix;
            }
            if idx + 1 < bar_count {
                sum += bars[idx + 1] * tuning.neighbor_mix;
                weight += tuning.neighbor_mix;
            }

            blended[idx] = sum / weight;
        }
        bars = blended;
    }

    for value in &mut bars {
        let boosted = *value * tuning.gain;
        *value = boosted.powf(tuning.curve).clamp(0.0, 1.0);
    }

    bars
}

fn write_cava_config(bar_count: usize, framerate: u32) -> std::io::Result<PathBuf> {
    let timestamp = now_millis();
    let path = env::temp_dir().join(format!(
        "kwybars-cava-{}-{timestamp}.conf",
        std::process::id()
    ));

    let config = format!(
        "[general]
bars = {bar_count}
framerate = {framerate}

[input]
method = pulse
source = auto

[output]
method = raw
raw_target = /dev/stdout
data_format = ascii
ascii_max_range = 1000
bar_delimiter = 59
frame_delimiter = 10
"
    );

    fs::write(&path, config)?;
    Ok(path)
}

fn parse_cava_line(line: &str, expected_bars: usize) -> Option<Vec<f32>> {
    let mut bars = Vec::with_capacity(expected_bars);

    for token in line.trim().split(';') {
        let trimmed = token.trim();
        if trimmed.is_empty() {
            continue;
        }

        let raw = match trimmed.parse::<f32>() {
            Ok(value) => value,
            Err(_) => return None,
        };
        bars.push((raw / 1000.0).clamp(0.0, 1.0));
    }

    if bars.is_empty() {
        return None;
    }

    if bars.len() > expected_bars {
        bars.truncate(expected_bars);
    } else if bars.len() < expected_bars {
        bars.resize(expected_bars, 0.0);
    }

    Some(bars)
}

fn now_millis() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_millis().min(u64::MAX as u128) as u64,
        Err(_) => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::{PipewireTuning, bars_from_interleaved_f32le, parse_cava_line};

    #[test]
    fn parses_ascii_bar_line() {
        let parsed = parse_cava_line("50;125;1000\n", 3);
        assert_eq!(parsed, Some(vec![0.05, 0.125, 1.0]));
    }

    #[test]
    fn pads_short_line_to_expected_count() {
        let parsed = parse_cava_line("900;450\n", 4);
        assert_eq!(parsed, Some(vec![0.9, 0.45, 0.0, 0.0]));
    }

    #[test]
    fn builds_bars_from_interleaved_f32le() {
        let samples: [f32; 8] = [0.1, -0.1, 0.8, -0.8, 0.2, 0.2, 0.9, 0.9];
        let mut bytes = Vec::new();
        for sample in samples {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }

        let tuning = PipewireTuning {
            attack: 0.2,
            decay: 0.9,
            gain: 1.0,
            curve: 1.0,
            neighbor_mix: 0.2,
        };
        let bars = bars_from_interleaved_f32le(&bytes, 2, 2, tuning);
        assert_eq!(bars.len(), 2);
        assert!(bars[0] > 0.0);
        assert!(bars[1] > 0.0);
        assert!(bars[1] >= bars[0]);
    }
}
