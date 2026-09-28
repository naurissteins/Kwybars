//! configuration loading

mod bounds;
mod color;
mod error;
mod image;
mod overlay;
mod parse;
mod path;
mod types;
mod visualizer;

#[cfg(test)]
mod tests;

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

pub use color::{ColorParseError, Rgba};
pub use error::ConfigError;
pub use image::ImageOverlayConfig;
pub use overlay::{OutputConfig, OverlayConfig};
pub use parse::{ParseError, Parsed, parse};
pub use path::{ConfigPathError, default_path};
pub use types::{
    ColorMode, Edge, FrameMirrorMode, GradientDirection, HorizontalAlignment, ImageFit, Layer,
    Layout, LineMode, MirrorOrientation, MonitorMode, VerticalAlignment,
};
pub use visualizer::{VisualizerConfig, VisualizerOverrides, default_frame_edges};

/// the whole config file
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Config {
    pub overlay: OverlayConfig,
    pub visualizer: VisualizerConfig,
    pub image_overlay: ImageOverlayConfig,
}

/// where a loaded config came from
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    File,
    /// the file does not exist, built-in defaults are used
    Defaults,
}

/// result of [`load`]
#[derive(Debug)]
pub struct Loaded {
    pub config: Config,
    pub warnings: Vec<String>,
    pub source: Source,
}

/// reads and parses a config file; a missing file yields the defaults
pub fn load(path: &Path) -> Result<Loaded, ConfigError> {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == ErrorKind::NotFound => {
            return Ok(Loaded {
                config: Config::default(),
                warnings: Vec::new(),
                source: Source::Defaults,
            });
        }
        Err(source) => {
            return Err(ConfigError::Read {
                path: path.to_owned(),
                source,
            });
        }
    };

    let parsed = parse(&raw).map_err(|err| match err {
        ParseError::Toml(source) => ConfigError::Parse {
            path: path.to_owned(),
            source,
        },
        ParseError::Invalid(message) => ConfigError::Invalid {
            path: path.to_owned(),
            message,
        },
    })?;
    Ok(Loaded {
        config: parsed.config,
        warnings: parsed.warnings,
        source: Source::File,
    })
}
