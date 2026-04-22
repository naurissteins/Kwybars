use std::f32::consts::TAU;

use kwybars_common::config::{VisualizerBackend, VisualizerConfig};
use kwybars_common::spectrum::SpectrumFrame;
use kwybars_engine::live::{LiveFrameStream, SourceKind};

const DEFAULT_SPREAD: f32 = 0.35;

#[derive(Debug)]
pub struct SyntheticFrameSource {
    bar_count: usize,
}

impl SyntheticFrameSource {
    pub fn new(bar_count: usize) -> Self {
        Self { bar_count }
    }

    pub fn frame_at(&self, timestamp_millis: u64) -> SpectrumFrame {
        let time_seconds = timestamp_millis as f32 / 1_000.0;
        let mut bars = Vec::with_capacity(self.bar_count);

        for index in 0..self.bar_count {
            let relative = if self.bar_count <= 1 {
                0.0
            } else {
                index as f32 / (self.bar_count - 1) as f32
            };
            let base = relative * TAU;
            let primary = (base * 1.15 + time_seconds * 2.8).sin() * 0.5 + 0.5;
            let secondary = (base * 0.55 - time_seconds * 1.4 + 0.8).cos() * 0.5 + 0.5;
            let tertiary = (index as f32 * DEFAULT_SPREAD + time_seconds * 1.9).sin() * 0.5 + 0.5;
            let amplitude = primary * 0.55 + secondary * 0.3 + tertiary * 0.15;
            bars.push(0.16 + amplitude * 0.76);
        }

        SpectrumFrame::new(bars, timestamp_millis)
    }
}

pub enum AppFrameSource {
    Synthetic(SyntheticFrameSource),
    Live(LiveFrameSource),
}

impl AppFrameSource {
    pub fn from_visualizer_config(config: &VisualizerConfig) -> Self {
        if matches!(config.backend, VisualizerBackend::Dummy) {
            Self::Synthetic(SyntheticFrameSource::new(config.bars.max(1)))
        } else {
            Self::Live(LiveFrameSource::new(config.clone()))
        }
    }

    pub fn frame_at(&mut self, timestamp_millis: u64) -> SpectrumFrame {
        match self {
            Self::Synthetic(source) => source.frame_at(timestamp_millis),
            Self::Live(source) => source.frame_at(timestamp_millis),
        }
    }

    pub fn description(&self) -> String {
        match self {
            Self::Synthetic(_) => "synthetic".to_owned(),
            Self::Live(source) => source.description(),
        }
    }
}

pub struct LiveFrameSource {
    stream: LiveFrameStream,
}

impl LiveFrameSource {
    fn new(config: VisualizerConfig) -> Self {
        Self {
            stream: LiveFrameStream::spawn(config),
        }
    }

    fn frame_at(&self, _timestamp_millis: u64) -> SpectrumFrame {
        self.stream.latest_frame()
    }

    fn description(&self) -> String {
        format!("live ({})", source_kind_name(self.stream.source_kind()))
    }
}

fn source_kind_name(kind: SourceKind) -> &'static str {
    match kind {
        SourceKind::Pipewire => "pipewire",
        SourceKind::Cava => "cava",
        SourceKind::Dummy => "dummy",
    }
}

#[cfg(test)]
mod tests {
    use kwybars_common::config::{VisualizerBackend, VisualizerConfig};

    use super::{AppFrameSource, SyntheticFrameSource};

    #[test]
    fn synthetic_source_produces_requested_bar_count() {
        let source = SyntheticFrameSource::new(24);
        let frame = source.frame_at(0);
        assert_eq!(frame.bar_count(), 24);
    }

    #[test]
    fn synthetic_source_changes_across_timestamps() {
        let source = SyntheticFrameSource::new(24);
        let first = source.frame_at(0);
        let second = source.frame_at(400);
        assert_ne!(first.bars, second.bars);
    }

    #[test]
    fn dummy_backend_selects_synthetic_source() {
        let config = VisualizerConfig {
            backend: VisualizerBackend::Dummy,
            ..VisualizerConfig::default()
        };

        let source = AppFrameSource::from_visualizer_config(&config);
        assert_eq!(source.description(), "synthetic");
    }
}
