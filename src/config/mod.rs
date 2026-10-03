//! configuration loading

mod activity;
mod audio;
mod bounds;
mod color;
mod colors;
mod compat;
mod error;
mod image;
mod load;
mod loose;
mod overlay;
mod parse;
mod path;
mod resolve;
mod theme;
mod types;
mod visualizer;

#[cfg(test)]
pub(crate) mod tests;

pub use activity::ActivityConfig;
pub use audio::AudioConfig;
pub use color::{ColorParseError, Rgba};
pub use colors::{COLORS_FILE, ColorOverrides};
pub use error::ConfigError;
pub use image::{ImageOverlayConfig, ImageOverlayOverrides};
pub use load::{
    Loaded, LoadedImage, Source, colors_candidates, link_target, load, load_reusing, themes,
};
pub use overlay::{OutputConfig, OverlayConfig, PlacementOverrides};
pub use parse::{ParseError, Parsed, parse};
pub use path::{ConfigPathError, default_path, env_path};
pub use resolve::{OverlaySettings, SurfaceConfig};
pub use theme::{AvailableTheme, LoadedTheme, THEME_KEYS, Theme, ThemeError, ThemeOrigin};
pub use types::{
    BarOrder, ColorMode, Edge, FrameMirrorMode, GradientDirection, HorizontalAlignment, ImageFit,
    Layer, Layout, LineMode, MirrorOrientation, ShowOn, VerticalAlignment,
};
pub use visualizer::{VisualizerConfig, VisualizerOverrides, default_frame_edges};

/// the whole config file
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Config {
    pub overlay: OverlayConfig,
    pub visualizer: VisualizerConfig,
    pub image_overlay: ImageOverlayConfig,
    pub activity: ActivityConfig,
    pub audio: AudioConfig,
}
