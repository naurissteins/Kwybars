//! `[visualizer]` settings

mod overrides;

pub use overrides::VisualizerOverrides;

use super::color::Rgba;
use super::types::{
    BarOrder, ColorMode, Edge, FrameMirrorMode, GradientDirection, Layout, LineMode,
    MirrorOrientation,
};

/// bars above this is clamped, every bar sizes the frame slot, bands and painters
pub const MAX_BARS: usize = 1024;

/// fully resolved visualizer settings
#[derive(Debug, Clone, PartialEq)]
pub struct VisualizerConfig {
    pub layout: Layout,
    pub line_mode: LineMode,
    pub line_split_gap: u32,
    pub mirror_orientation: MirrorOrientation,
    pub mirror_gap: u32,
    pub wave_stroke_width: u32,
    pub wave_fill: bool,
    pub wave_glow: bool,
    pub wave_smoothing: f32,
    pub wave_motion_smoothing: f32,
    pub wave_amplitude: f32,
    pub frame_edges: Vec<Edge>,
    pub frame_mirror_mode: FrameMirrorMode,
    pub bars: usize,
    pub bar_order: BarOrder,
    pub bar_width: u32,
    pub bar_corner_radius: f32,
    pub segmented_bars: bool,
    pub segment_length: u32,
    pub segment_gap: u32,
    pub radial_inner_radius: u32,
    pub radial_start_angle: f32,
    pub radial_arc_degrees: f32,
    pub radial_rotation_speed: f32,
    pub center_offset_x: f32,
    pub center_offset_y: f32,
    pub polygon_sides: u32,
    pub polygon_radius: u32,
    pub polygon_bar_length: u32,
    pub polygon_rotation: f32,
    pub polygon_rotation_speed: f32,
    pub gap: u32,
    pub framerate: u32,
    pub color_mode: ColorMode,
    pub gradient_direction: GradientDirection,
    pub color_rgba: Rgba,
    pub color2_rgba: Rgba,
    /// theme name, resolved against user and built-in themes
    pub theme: Option<String>,
    pub theme_opacity: f32,
}

/// edges used by the frame layout when none are configured
pub fn default_frame_edges() -> Vec<Edge> {
    vec![Edge::Top, Edge::Bottom]
}

impl Default for VisualizerConfig {
    fn default() -> Self {
        Self {
            layout: Layout::Line,
            line_mode: LineMode::Continuous,
            line_split_gap: 200,
            mirror_orientation: MirrorOrientation::Horizontal,
            mirror_gap: 0,
            wave_stroke_width: 10,
            wave_fill: true,
            wave_glow: false,
            wave_smoothing: 1.0,
            wave_motion_smoothing: 0.22,
            wave_amplitude: 0.8,
            frame_edges: default_frame_edges(),
            frame_mirror_mode: FrameMirrorMode::Pairs,
            bars: 50,
            bar_order: BarOrder::LowToHigh,
            bar_width: 8,
            bar_corner_radius: 20.0,
            segmented_bars: false,
            segment_length: 14,
            segment_gap: 6,
            radial_inner_radius: 180,
            radial_start_angle: -90.0,
            radial_arc_degrees: 360.0,
            radial_rotation_speed: 0.0,
            center_offset_x: 0.0,
            center_offset_y: 0.0,
            polygon_sides: 3,
            polygon_radius: 220,
            polygon_bar_length: 0,
            polygon_rotation: -90.0,
            polygon_rotation_speed: 0.0,
            gap: 20,
            framerate: 60,
            color_mode: ColorMode::Gradient,
            gradient_direction: GradientDirection::Vertical,
            color_rgba: Rgba::new(175.0 / 255.0, 198.0 / 255.0, 1.0, 0.7),
            color2_rgba: Rgba::new(191.0 / 255.0, 198.0 / 255.0, 220.0 / 255.0, 0.7),
            theme: None,
            theme_opacity: 1.0,
        }
    }
}
