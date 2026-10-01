//! image_overlay

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::bounds::Bounds;
use super::types::ImageFit;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct ImageOverlayConfig {
    pub enabled: bool,
    pub path: Option<String>,
    pub opacity: f32,
    pub fit: ImageFit,
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
    pub fn file(
        &self,
        config_path: &Path,
        env: &dyn Fn(&str) -> Option<OsString>,
    ) -> Option<PathBuf> {
        let raw = self.path.as_deref().filter(|_| self.enabled)?;
        let path = match raw.strip_prefix("~/") {
            Some(rest) => match env("HOME").filter(|home| !home.is_empty()) {
                Some(home) => PathBuf::from(home).join(rest),
                None => PathBuf::from(raw),
            },
            None => PathBuf::from(raw),
        };
        if path.is_absolute() {
            return Some(path);
        }
        Some(match config_path.parent() {
            Some(dir) => dir.join(path),
            None => path,
        })
    }

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
