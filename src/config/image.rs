//! `[image_overlay]`

use serde::Deserialize;

use super::bounds::Bounds;
use super::types::ImageFit;

/// the `[image_overlay]` table
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct ImageOverlayConfig {
    pub enabled: bool,
    /// image file; relative paths are relative to the config file
    pub path: Option<String>,
    pub opacity: f32,
    pub fit: ImageFit,
    /// box size in logical pixels, 0 means the surface size
    pub width: u32,
    pub height: u32,
    pub offset_x: f32,
    pub offset_y: f32,
}

impl Default for ImageOverlayConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            path: None,
            opacity: 1.0,
            fit: ImageFit::Contain,
            width: 0,
            height: 0,
            offset_x: 0.0,
            offset_y: 0.0,
        }
    }
}

impl ImageOverlayConfig {
    /// fixes out-of-range values in place, recording a warning for each
    pub(crate) fn normalize(&mut self, warnings: &mut Vec<String>) {
        let mut bounds = Bounds::new("image_overlay", warnings);
        let mut opacity = Some(self.opacity);
        bounds.within("opacity", &mut opacity, 0.0, 1.0);
        self.opacity = opacity.unwrap_or(1.0);
        for (key, value) in [
            ("offset_x", &mut self.offset_x),
            ("offset_y", &mut self.offset_y),
        ] {
            let mut checked = Some(*value);
            bounds.finite(key, &mut checked);
            *value = checked.unwrap_or(0.0);
        }
        self.path = self
            .path
            .as_deref()
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .map(str::to_owned);
    }
}
