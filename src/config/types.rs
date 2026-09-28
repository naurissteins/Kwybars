//! enum values used in the config file, spelled as in toml

use std::fmt;

use serde::Deserialize;
use serde::de::{self, Deserializer, Visitor};

/// screen edge
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Edge {
    Bottom,
    Top,
    Left,
    Right,
}

/// layer-shell layer the overlay lives on
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Layer {
    Background,
    Bottom,
    Top,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HorizontalAlignment {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VerticalAlignment {
    Top,
    Center,
    Bottom,
}

/// which outputs get an overlay
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MonitorMode {
    Primary,
    All,
    List,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Layout {
    Line,
    Mirror,
    Wave,
    Frame,
    Radial,
    Polygon,
    Particle,
    Floating,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LineMode {
    Continuous,
    Split,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MirrorOrientation {
    Horizontal,
    Vertical,
}

/// how values are distributed over frame edges
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameMirrorMode {
    Off,
    All,
    Pairs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorMode {
    Solid,
    Gradient,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GradientDirection {
    Vertical,
    Horizontal,
}

/// how the image overlay is scaled into its box
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageFit {
    Contain,
    Cover,
    Stretch,
}

// also accepts the legacy `frame_mirror = true|false` spelling
impl<'de> Deserialize<'de> for FrameMirrorMode {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ModeVisitor;

        impl Visitor<'_> for ModeVisitor {
            type Value = FrameMirrorMode;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("one of `off`, `all`, `pairs`, or a boolean")
            }

            fn visit_bool<E: de::Error>(self, value: bool) -> Result<Self::Value, E> {
                Ok(if value {
                    FrameMirrorMode::All
                } else {
                    FrameMirrorMode::Off
                })
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                match value {
                    "off" => Ok(FrameMirrorMode::Off),
                    "all" => Ok(FrameMirrorMode::All),
                    "pairs" => Ok(FrameMirrorMode::Pairs),
                    _ => Err(E::unknown_variant(value, &["off", "all", "pairs"])),
                }
            }
        }

        deserializer.deserialize_any(ModeVisitor)
    }
}
