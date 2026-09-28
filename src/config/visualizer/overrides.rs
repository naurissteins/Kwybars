//! visualizer keys as written in a toml table, all optional

use serde::Deserialize;

use super::{VisualizerConfig, default_frame_edges};
use crate::config::bounds::Bounds;
use crate::config::color::Rgba;
use crate::config::types::{
    ColorMode, Edge, FrameMirrorMode, GradientDirection, Layout, LineMode, MirrorOrientation,
};

/// a `[visualizer]` or `[overlay.outputs.visualizer]` table; `None` means unset
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct VisualizerOverrides {
    pub layout: Option<Layout>,
    pub line_mode: Option<LineMode>,
    pub line_split_gap: Option<u32>,
    pub mirror_orientation: Option<MirrorOrientation>,
    pub mirror_gap: Option<u32>,
    pub wave_stroke_width: Option<u32>,
    pub wave_fill: Option<bool>,
    pub wave_glow: Option<bool>,
    pub wave_smoothing: Option<f32>,
    pub wave_motion_smoothing: Option<f32>,
    pub wave_amplitude: Option<f32>,
    pub frame_edges: Option<Vec<Edge>>,
    #[serde(alias = "frame_mirror")]
    pub frame_mirror_mode: Option<FrameMirrorMode>,
    pub bars: Option<usize>,
    pub bar_width: Option<u32>,
    pub bar_corner_radius: Option<f32>,
    pub segmented_bars: Option<bool>,
    pub segment_length: Option<u32>,
    pub segment_gap: Option<u32>,
    pub radial_inner_radius: Option<u32>,
    pub radial_start_angle: Option<f32>,
    pub radial_arc_degrees: Option<f32>,
    pub radial_rotation_speed: Option<f32>,
    pub center_offset_x: Option<f32>,
    pub center_offset_y: Option<f32>,
    pub polygon_sides: Option<u32>,
    pub polygon_radius: Option<u32>,
    pub polygon_bar_length: Option<u32>,
    pub polygon_rotation: Option<f32>,
    pub polygon_rotation_speed: Option<f32>,
    pub gap: Option<u32>,
    pub framerate: Option<u32>,
    pub color_mode: Option<ColorMode>,
    pub gradient_direction: Option<GradientDirection>,
    pub color_rgba: Option<Rgba>,
    pub color2_rgba: Option<Rgba>,
    pub theme: Option<String>,
    pub theme_opacity: Option<f32>,
}

macro_rules! apply_set_fields {
    ($from:expr, $to:expr, [$($field:ident),+ $(,)?]) => {
        $(
            if let Some(value) = &$from.$field {
                $to.$field = value.clone();
            }
        )+
    };
}

impl VisualizerOverrides {
    /// copies every set key onto `target`
    pub fn apply_to(&self, target: &mut VisualizerConfig) {
        apply_set_fields!(
            self,
            target,
            [
                layout,
                line_mode,
                line_split_gap,
                mirror_orientation,
                mirror_gap,
                wave_stroke_width,
                wave_fill,
                wave_glow,
                wave_smoothing,
                wave_motion_smoothing,
                wave_amplitude,
                frame_edges,
                frame_mirror_mode,
                bars,
                bar_width,
                bar_corner_radius,
                segmented_bars,
                segment_length,
                segment_gap,
                radial_inner_radius,
                radial_start_angle,
                radial_arc_degrees,
                radial_rotation_speed,
                center_offset_x,
                center_offset_y,
                polygon_sides,
                polygon_radius,
                polygon_bar_length,
                polygon_rotation,
                polygon_rotation_speed,
                gap,
                framerate,
                color_mode,
                gradient_direction,
                color_rgba,
                color2_rgba,
                theme_opacity,
            ]
        );
        if self.theme.is_some() {
            target.theme.clone_from(&self.theme);
        }
    }

    /// fixes out-of-range values in place, recording a warning for each
    pub(crate) fn normalize(&mut self, table: &str, warnings: &mut Vec<String>) {
        let mut bounds = Bounds::new(table, warnings);
        bounds.at_least("wave_stroke_width", &mut self.wave_stroke_width, 1);
        bounds.at_least("segment_length", &mut self.segment_length, 1);
        bounds.at_least("radial_inner_radius", &mut self.radial_inner_radius, 1);
        bounds.at_least("polygon_sides", &mut self.polygon_sides, 3);
        bounds.at_least("polygon_radius", &mut self.polygon_radius, 1);
        bounds.at_least("bars", &mut self.bars, 1);
        bounds.at_least("framerate", &mut self.framerate, 1);
        for (key, value) in [
            ("wave_smoothing", &mut self.wave_smoothing),
            ("wave_motion_smoothing", &mut self.wave_motion_smoothing),
            ("wave_amplitude", &mut self.wave_amplitude),
            ("bar_corner_radius", &mut self.bar_corner_radius),
        ] {
            bounds.within(key, value, 0.0, f32::MAX);
        }
        for (key, value) in [
            ("radial_start_angle", &mut self.radial_start_angle),
            ("radial_arc_degrees", &mut self.radial_arc_degrees),
            ("radial_rotation_speed", &mut self.radial_rotation_speed),
            ("center_offset_x", &mut self.center_offset_x),
            ("center_offset_y", &mut self.center_offset_y),
            ("polygon_rotation", &mut self.polygon_rotation),
            ("polygon_rotation_speed", &mut self.polygon_rotation_speed),
        ] {
            bounds.finite(key, value);
        }
        bounds.within("theme_opacity", &mut self.theme_opacity, 0.0, 1.0);

        if let Some(edges) = &mut self.frame_edges {
            let mut unique: Vec<Edge> = Vec::with_capacity(edges.len());
            for edge in edges.iter() {
                if !unique.contains(edge) {
                    unique.push(*edge);
                }
            }
            *edges = if unique.is_empty() {
                default_frame_edges()
            } else {
                unique
            };
        }

        self.theme = self
            .theme
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_owned);
    }

    /// drops keys that only make sense for the whole visualizer
    pub(crate) fn drop_global_keys(&mut self, table: &str, warnings: &mut Vec<String>) {
        let mut bounds = Bounds::new(table, warnings);
        let reason = "cannot be set per output";
        bounds.unsupported("bars", &mut self.bars, reason);
        bounds.unsupported("framerate", &mut self.framerate, reason);
        bounds.unsupported("theme", &mut self.theme, reason);
        bounds.unsupported("theme_opacity", &mut self.theme_opacity, reason);
    }
}
