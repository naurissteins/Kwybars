//! final settings for one surface: `[overlay]` merged with an output entry

use super::Config;
use super::color::Rgba;
use super::image::ImageOverlayConfig;
use super::overlay::{OutputConfig, OverlayConfig};
use super::theme::Theme;
use super::types::{Edge, HorizontalAlignment, Layer, VerticalAlignment};
use super::visualizer::{VisualizerConfig, VisualizerOverrides};

/// placement and fades for one surface
#[derive(Debug, Clone, PartialEq)]
pub struct OverlaySettings {
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
}

/// everything one surface needs to draw
#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceConfig {
    pub overlay: OverlaySettings,
    pub visualizer: VisualizerConfig,
    /// theme palette with `theme_opacity` applied, replaces the direct colors
    pub theme_colors: Option<[Rgba; 6]>,
}

impl From<&OverlayConfig> for OverlaySettings {
    fn from(overlay: &OverlayConfig) -> Self {
        Self {
            position: overlay.position,
            layer: overlay.layer,
            anchor_margin: overlay.anchor_margin,
            margin_left: overlay.margin_left,
            margin_right: overlay.margin_right,
            margin_top: overlay.margin_top,
            margin_bottom: overlay.margin_bottom,
            fade_in_ms: overlay.fade_in_ms,
            fade_out_ms: overlay.fade_out_ms,
            full_length: overlay.full_length,
            width: overlay.width,
            height: overlay.height,
            horizontal_alignment: overlay.horizontal_alignment,
            vertical_alignment: overlay.vertical_alignment,
        }
    }
}

macro_rules! apply_set_fields {
    ($from:expr, $to:expr, [$($field:ident),+ $(,)?]) => {
        $(
            if let Some(value) = $from.$field {
                $to.$field = value;
            }
        )+
    };
}

impl OutputConfig {
    /// copies every set placement key onto `settings`
    pub fn apply_to(&self, settings: &mut OverlaySettings) {
        apply_set_fields!(
            self,
            settings,
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
    }
}

impl Config {
    /// the image settings of a surface; `output` is its `[output.NAME]` section, if any
    pub fn image(&self, output: Option<&OutputConfig>) -> ImageOverlayConfig {
        match output {
            Some(output) => output.image_overlay.over(&self.image_overlay),
            None => self.image_overlay.clone(),
        }
    }

    /// settings for a surface; `output` is its `[output.NAME]` section, if any
    ///
    /// the theme applies unless the output sets any of its own color keys
    pub fn surface(&self, output: Option<&OutputConfig>, theme: Option<&Theme>) -> SurfaceConfig {
        let mut overlay = OverlaySettings::from(&self.overlay);
        let mut visualizer = self.visualizer.clone();
        let mut use_theme = true;
        if let Some(output) = output {
            output.apply_to(&mut overlay);
            output.visualizer.apply_to(&mut visualizer);
            use_theme = !sets_direct_colors(&output.visualizer);
        }
        let theme_colors = theme
            .filter(|_| use_theme)
            .map(|theme| theme.with_opacity(visualizer.theme_opacity));
        SurfaceConfig {
            overlay,
            visualizer,
            theme_colors,
        }
    }
}

fn sets_direct_colors(overrides: &VisualizerOverrides) -> bool {
    overrides.color_mode.is_some()
        || overrides.gradient_direction.is_some()
        || overrides.color_rgba.is_some()
        || overrides.color2_rgba.is_some()
}
