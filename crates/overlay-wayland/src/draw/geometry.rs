use kwybars_common::config::{LineMode, VisualizerConfig};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BarGeometry {
    thickness: f64,
    gap: f64,
    corner_radius: f64,
    segmented: bool,
    segment_length: f64,
    segment_gap: f64,
    line_mode: LineMode,
    line_split_gap: f64,
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
            line_mode: visualizer.line_mode,
            line_split_gap: f64::from(visualizer.line_split_gap),
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
            line_mode: self.line_mode,
            line_split_gap: self.line_split_gap * scale,
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

    fn line_mode(self) -> LinearSlotMode {
        match self.line_mode {
            LineMode::Continuous => LinearSlotMode::Continuous,
            LineMode::Split => LinearSlotMode::Split {
                center_gap: self.line_split_gap.max(0.0),
            },
        }
    }
}

pub(super) struct BarSlots {
    pub index: usize,
    pub start: f64,
    pub thickness: f64,
}

impl BarSlots {
    pub fn new(count: usize, available_length: f64, geometry: &BarGeometry) -> Vec<Self> {
        let mut slots = Vec::with_capacity(count);
        for_each_linear_slot(
            count,
            available_length,
            geometry.thickness,
            geometry.gap,
            geometry.line_mode(),
            |index, start, thickness| {
                slots.push(Self {
                    index,
                    start,
                    thickness,
                });
            },
        );
        slots
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum LinearSlotMode {
    Continuous,
    Split { center_gap: f64 },
}

fn for_each_linear_slot(
    count: usize,
    available_length: f64,
    thickness: f64,
    gap: f64,
    mode: LinearSlotMode,
    mut slot: impl FnMut(usize, f64, f64),
) {
    match mode {
        LinearSlotMode::Continuous => {
            for_each_continuous_slot(count, available_length, thickness, gap, 0, &mut slot);
        }
        LinearSlotMode::Split { center_gap } if count >= 2 => {
            let left_count = count / 2;
            let right_count = count - left_count;
            let safe_gap = center_gap.min(available_length).max(0.0);
            let half_length = ((available_length - safe_gap) * 0.5).max(1.0);

            for_each_continuous_slot(left_count, half_length, thickness, gap, 0, &mut slot);
            for_each_continuous_slot(
                right_count,
                half_length,
                thickness,
                gap,
                left_count,
                |index, start, slot_thickness| {
                    slot(index, half_length + safe_gap + start, slot_thickness);
                },
            );
        }
        LinearSlotMode::Split { .. } => {
            for_each_continuous_slot(count, available_length, thickness, gap, 0, &mut slot);
        }
    }
}

fn for_each_continuous_slot(
    count: usize,
    available_length: f64,
    thickness: f64,
    gap: f64,
    index_offset: usize,
    mut slot: impl FnMut(usize, f64, f64),
) {
    if count == 0 {
        return;
    }

    let slots = ContinuousSlots::new(count, available_length, thickness, gap);
    for index in 0..count {
        let start = slots.start + (index as f64 * slots.step);
        slot(index_offset + index, start, slots.thickness);
    }
}

struct ContinuousSlots {
    start: f64,
    step: f64,
    thickness: f64,
}

impl ContinuousSlots {
    fn new(count: usize, available_length: f64, thickness: f64, gap: f64) -> Self {
        let count = count as f64;
        let total_nominal = (count * thickness) + ((count - 1.0).max(0.0) * gap);
        let scale = if total_nominal > available_length {
            available_length / total_nominal
        } else {
            1.0
        };
        let thickness = (thickness * scale).max(1.0);
        let gap = gap * scale;
        let rendered_total = (count * thickness) + ((count - 1.0).max(0.0) * gap);
        let start = (available_length - rendered_total).max(0.0) * 0.5;

        Self {
            start,
            step: thickness + gap,
            thickness,
        }
    }
}
