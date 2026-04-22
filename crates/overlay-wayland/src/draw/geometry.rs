use kwybars_common::config::VisualizerConfig;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BarGeometry {
    thickness: f64,
    gap: f64,
    corner_radius: f64,
    segmented: bool,
    segment_length: f64,
    segment_gap: f64,
}

impl BarGeometry {
    pub fn from_visualizer(visualizer: &VisualizerConfig) -> Self {
        Self {
            thickness: f64::from(visualizer.bar_width.max(1)),
            gap: f64::from(visualizer.gap),
            corner_radius: f64::from(visualizer.bar_corner_radius.max(0.0)),
            segmented: visualizer.segmented_bars,
            segment_length: f64::from(visualizer.segment_length.max(1)),
            segment_gap: f64::from(visualizer.segment_gap),
        }
    }

    pub fn scaled(self, scale: f64) -> Self {
        let scale = scale.max(1.0);
        Self {
            thickness: self.thickness * scale,
            gap: self.gap * scale,
            corner_radius: self.corner_radius * scale,
            segmented: self.segmented,
            segment_length: self.segment_length * scale,
            segment_gap: self.segment_gap * scale,
        }
    }

    pub fn rounded_radius(self, width: u32, height: u32) -> f64 {
        self.corner_radius
            .max(0.0)
            .min(f64::from(width) * 0.5)
            .min(f64::from(height) * 0.5)
    }

    pub fn segmented(self) -> bool {
        self.segmented
    }

    pub fn segment_length(self) -> f64 {
        self.segment_length.max(1.0)
    }

    pub fn segment_gap(self) -> f64 {
        self.segment_gap.max(0.0)
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
