use kwybars_common::config::{
    AppConfig, FrameMirrorMode, LineMode, OverlayConfig, OverlayPosition, VisualizerConfig,
    VisualizerLayout,
};

#[derive(Debug, Clone, PartialEq)]
pub struct BarGeometry {
    thickness: f64,
    gap: f64,
    corner_radius: f64,
    segmented: bool,
    segment_length: f64,
    segment_gap: f64,
    line_mode: LineMode,
    line_split_gap: f64,
    layout: VisualizerLayout,
    frame_edges: Vec<OverlayPosition>,
    frame_mirror_mode: FrameMirrorMode,
    frame_top_thickness: f64,
    frame_bottom_thickness: f64,
    frame_left_thickness: f64,
    frame_right_thickness: f64,
    frame_anchor_margin: f64,
    frame_margin_left: f64,
    frame_margin_right: f64,
    frame_margin_top: f64,
    frame_margin_bottom: f64,
}

impl BarGeometry {
    pub fn from_app_config(config: &AppConfig) -> Self {
        Self::from_parts(&config.visualizer, &config.overlay)
    }

    #[cfg(test)]
    pub fn from_visualizer(visualizer: &VisualizerConfig) -> Self {
        Self::from_parts(visualizer, &OverlayConfig::default())
    }

    fn from_parts(visualizer: &VisualizerConfig, overlay: &OverlayConfig) -> Self {
        Self {
            thickness: f64::from(visualizer.bar_width.max(1)),
            gap: f64::from(visualizer.gap),
            corner_radius: f64::from(visualizer.bar_corner_radius.max(0.0)),
            segmented: visualizer.segmented_bars,
            segment_length: f64::from(visualizer.segment_length.max(1)),
            segment_gap: f64::from(visualizer.segment_gap),
            line_mode: visualizer.line_mode,
            line_split_gap: f64::from(visualizer.line_split_gap),
            layout: visualizer.layout,
            frame_edges: normalized_frame_edges(&visualizer.frame_edges),
            frame_mirror_mode: visualizer.frame_mirror_mode,
            frame_top_thickness: f64::from(overlay.height.max(1)),
            frame_bottom_thickness: f64::from(overlay.height.max(1)),
            frame_left_thickness: f64::from(overlay.width.max(1)),
            frame_right_thickness: f64::from(overlay.width.max(1)),
            frame_anchor_margin: f64::from(overlay.anchor_margin),
            frame_margin_left: f64::from(overlay.margin_left),
            frame_margin_right: f64::from(overlay.margin_right),
            frame_margin_top: f64::from(overlay.margin_top),
            frame_margin_bottom: f64::from(overlay.margin_bottom),
        }
    }

    pub fn scaled(&self, scale: f64) -> Self {
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
            layout: self.layout,
            frame_edges: self.frame_edges.clone(),
            frame_mirror_mode: self.frame_mirror_mode,
            frame_top_thickness: self.frame_top_thickness * scale,
            frame_bottom_thickness: self.frame_bottom_thickness * scale,
            frame_left_thickness: self.frame_left_thickness * scale,
            frame_right_thickness: self.frame_right_thickness * scale,
            frame_anchor_margin: self.frame_anchor_margin * scale,
            frame_margin_left: self.frame_margin_left * scale,
            frame_margin_right: self.frame_margin_right * scale,
            frame_margin_top: self.frame_margin_top * scale,
            frame_margin_bottom: self.frame_margin_bottom * scale,
        }
    }

    pub fn rounded_radius(&self, width: u32, height: u32) -> f64 {
        self.corner_radius
            .max(0.0)
            .min(f64::from(width) * 0.5)
            .min(f64::from(height) * 0.5)
    }

    pub fn segmented(&self) -> bool {
        self.segmented
    }

    pub fn segment_length(&self) -> f64 {
        self.segment_length.max(1.0)
    }

    pub fn segment_gap(&self) -> f64 {
        self.segment_gap.max(0.0)
    }

    pub fn is_frame_layout(&self) -> bool {
        self.layout == VisualizerLayout::Frame
    }

    pub fn frame_edges(&self) -> &[OverlayPosition] {
        &self.frame_edges
    }

    pub fn frame_mirror_mode(&self) -> FrameMirrorMode {
        self.frame_mirror_mode
    }

    pub fn frame_metrics(&self, width: u32, height: u32) -> FrameMetrics {
        FrameMetrics {
            width: f64::from(width),
            height: f64::from(height),
            top_thickness: self.frame_top_thickness.max(1.0),
            bottom_thickness: self.frame_bottom_thickness.max(1.0),
            left_thickness: self.frame_left_thickness.max(1.0),
            right_thickness: self.frame_right_thickness.max(1.0),
            anchor_margin: self.frame_anchor_margin.max(0.0),
            margin_left: self.frame_margin_left.max(0.0),
            margin_right: self.frame_margin_right.max(0.0),
            margin_top: self.frame_margin_top.max(0.0),
            margin_bottom: self.frame_margin_bottom.max(0.0),
        }
    }

    fn line_mode(&self) -> LinearSlotMode {
        match self.line_mode {
            LineMode::Continuous => LinearSlotMode::Continuous,
            LineMode::Split => LinearSlotMode::Split {
                center_gap: self.line_split_gap.max(0.0),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameMetrics {
    pub width: f64,
    pub height: f64,
    pub top_thickness: f64,
    pub bottom_thickness: f64,
    pub left_thickness: f64,
    pub right_thickness: f64,
    pub anchor_margin: f64,
    pub margin_left: f64,
    pub margin_right: f64,
    pub margin_top: f64,
    pub margin_bottom: f64,
}

fn normalized_frame_edges(edges: &[OverlayPosition]) -> Vec<OverlayPosition> {
    let mut normalized = Vec::new();
    for edge in edges {
        if !normalized.contains(edge) {
            normalized.push(edge.clone());
        }
    }
    normalized
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

    pub fn continuous(count: usize, available_length: f64, geometry: &BarGeometry) -> Vec<Self> {
        let mut slots = Vec::with_capacity(count);
        for_each_continuous_slot(
            count,
            available_length,
            geometry.thickness,
            geometry.gap,
            0,
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
