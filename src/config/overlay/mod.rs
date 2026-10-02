//! `[overlay]` placement and `[output.NAME]` per-output overrides

mod table;

use serde::Deserialize;

use super::image::ImageOverlayOverrides;
use super::types::{Edge, HorizontalAlignment, Layer, ShowOn, VerticalAlignment};
use super::visualizer::VisualizerOverrides;

pub(super) use table::{OverlayTable, resolve};

/// the resolved `[overlay]` table with the per-output sections
#[derive(Debug, Clone, PartialEq)]
pub struct OverlayConfig {
    pub position: Edge,
    pub layer: Layer,
    pub anchor_margin: u32,
    pub margin_left: u32,
    pub margin_right: u32,
    pub margin_top: u32,
    pub margin_bottom: u32,
    pub fade_in_ms: u64,
    pub fade_out_ms: u64,
    pub full_length: bool,
    pub width: u32,
    pub height: u32,
    pub horizontal_alignment: HorizontalAlignment,
    pub vertical_alignment: VerticalAlignment,
    pub show_on: ShowOn,
    /// overrides for the output each one names
    pub outputs: Vec<OutputConfig>,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            position: Edge::Bottom,
            layer: Layer::Background,
            anchor_margin: 20,
            margin_left: 20,
            margin_right: 20,
            margin_top: 0,
            margin_bottom: 0,
            fade_in_ms: 180,
            fade_out_ms: 350,
            full_length: true,
            width: 800,
            height: 500,
            horizontal_alignment: HorizontalAlignment::Center,
            vertical_alignment: VerticalAlignment::Center,
            show_on: ShowOn::Primary,
            outputs: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct OutputConfig {
    /// output name such as DP-1, primary, or a 1-based index
    pub monitor: String,
    pub enabled: bool,
    pub position: Option<Edge>,
    pub layer: Option<Layer>,
    pub anchor_margin: Option<u32>,
    pub margin_left: Option<u32>,
    pub margin_right: Option<u32>,
    pub margin_top: Option<u32>,
    pub margin_bottom: Option<u32>,
    pub fade_in_ms: Option<u64>,
    pub fade_out_ms: Option<u64>,
    pub full_length: Option<bool>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub horizontal_alignment: Option<HorizontalAlignment>,
    pub vertical_alignment: Option<VerticalAlignment>,
    pub visualizer: VisualizerOverrides,
    pub image_overlay: ImageOverlayOverrides,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            monitor: String::new(),
            enabled: true,
            position: None,
            layer: None,
            anchor_margin: None,
            margin_left: None,
            margin_right: None,
            margin_top: None,
            margin_bottom: None,
            fade_in_ms: None,
            fade_out_ms: None,
            full_length: None,
            width: None,
            height: None,
            horizontal_alignment: None,
            vertical_alignment: None,
            visualizer: VisualizerOverrides::default(),
            image_overlay: ImageOverlayOverrides::default(),
        }
    }
}
