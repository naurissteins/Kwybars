use serde::Deserialize;

use super::{OutputConfig, PlacementOverrides};
use crate::config::image::ImageOverlayOverrides;
use crate::config::types::{Edge, HorizontalAlignment, Layer, VerticalAlignment};
use crate::config::visualizer::VisualizerOverrides;

/// one [output.NAME] section as written; the placement keys beside overlay
/// are the older flat spelling of [output.NAME.overlay]
#[derive(Debug, Deserialize)]
#[serde(default)]
pub(in crate::config) struct OutputSection {
    monitor: String,
    enabled: bool,
    position: Option<Edge>,
    layer: Option<Layer>,
    anchor_margin: Option<u32>,
    margin_left: Option<u32>,
    margin_right: Option<u32>,
    margin_top: Option<u32>,
    margin_bottom: Option<u32>,
    fade_in_ms: Option<u64>,
    fade_out_ms: Option<u64>,
    full_length: Option<bool>,
    width: Option<u32>,
    height: Option<u32>,
    horizontal_alignment: Option<HorizontalAlignment>,
    vertical_alignment: Option<VerticalAlignment>,
    overlay: PlacementOverrides,
    visualizer: VisualizerOverrides,
    image_overlay: ImageOverlayOverrides,
}

impl Default for OutputSection {
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
            overlay: PlacementOverrides::default(),
            visualizer: VisualizerOverrides::default(),
            image_overlay: ImageOverlayOverrides::default(),
        }
    }
}

macro_rules! fill_from_flat_keys {
    ($section:expr, $table:expr, $warnings:expr, [$($field:ident),+ $(,)?]) => {
        $(
            match ($section.overlay.$field, $section.$field) {
                (Some(_), Some(_)) => $warnings.push(format!(
                    "{table}.{key}: also set in [{table}.overlay], which is used",
                    table = $table,
                    key = stringify!($field),
                )),
                (None, flat) => $section.overlay.$field = flat,
                (Some(_), None) => {}
            }
        )+
    };
}

impl OutputSection {
    pub(super) fn monitor(&self) -> &str {
        &self.monitor
    }

    /// table is how warnings name the section, such as output.DP-1
    pub(super) fn into_config(
        mut self,
        monitor: &str,
        table: &str,
        warnings: &mut Vec<String>,
    ) -> OutputConfig {
        fill_from_flat_keys!(
            self,
            table,
            warnings,
            [
                position,
                layer,
                anchor_margin,
                margin_left,
                margin_right,
                margin_top,
                margin_bottom,
                fade_in_ms,
                fade_out_ms,
                full_length,
                width,
                height,
                horizontal_alignment,
                vertical_alignment,
            ]
        );
        OutputConfig {
            monitor: monitor.to_owned(),
            enabled: self.enabled,
            overlay: self.overlay,
            visualizer: self.visualizer,
            image_overlay: self.image_overlay,
        }
    }
}
