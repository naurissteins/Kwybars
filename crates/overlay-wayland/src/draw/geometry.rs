use kwybars_common::config::VisualizerConfig;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BarGeometry {
    thickness: f64,
    gap: f64,
}

impl BarGeometry {
    pub fn from_visualizer(visualizer: &VisualizerConfig) -> Self {
        Self {
            thickness: f64::from(visualizer.bar_width.max(1)),
            gap: f64::from(visualizer.gap),
        }
    }
}

pub(super) struct BarSlots {
    pub start: f64,
    pub step: f64,
    pub thickness: f64,
}

impl BarSlots {
    pub fn new(count: usize, available_length: f64, geometry: &BarGeometry) -> Self {
        let count = count as f64;
        let total_nominal = (count * geometry.thickness) + ((count - 1.0).max(0.0) * geometry.gap);
        let scale = if total_nominal > available_length {
            available_length / total_nominal
        } else {
            1.0
        };
        let thickness = (geometry.thickness * scale).max(1.0);
        let gap = geometry.gap * scale;
        let rendered_total = (count * thickness) + ((count - 1.0).max(0.0) * gap);
        let start = (available_length - rendered_total).max(0.0) * 0.5;

        Self {
            start,
            step: thickness + gap,
            thickness,
        }
    }
}
