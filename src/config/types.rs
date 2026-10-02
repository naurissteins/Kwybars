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

/// which outputs get an overlay, overlay.show_on
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ShowOn {
    /// the first output the compositor advertises
    #[default]
    Primary,
    All,
    Named(Vec<String>),
    Sections,
}

impl<'de> Deserialize<'de> for ShowOn {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ShowOnVisitor;

        impl<'de> Visitor<'de> for ShowOnVisitor {
            type Value = ShowOn;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("\"primary\", \"all\", a monitor name, or a list of monitor names")
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<ShowOn, E> {
                Ok(match value.trim() {
                    "primary" => ShowOn::Primary,
                    "all" => ShowOn::All,
                    name => ShowOn::Named(clean_names([name])),
                })
            }

            fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<ShowOn, A::Error> {
                let mut names: Vec<String> = Vec::new();
                while let Some(name) = seq.next_element::<String>()? {
                    names.push(name);
                }
                Ok(ShowOn::Named(clean_names(names.iter().map(String::as_str))))
            }
        }

        deserializer.deserialize_any(ShowOnVisitor)
    }
}

/// trimmed names without the empty ones
pub(super) fn clean_names<'a>(names: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    names
        .into_iter()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect()
}

/// the deprecated `overlay.monitor_mode`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum MonitorMode {
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

/// how frequencies are laid out along the bars, visualizer.bar_order
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BarOrder {
    #[default]
    LowToHigh,
    /// bass in the middle, treble towards both ends
    BassCenter,
    /// bass at both ends, treble in the middle
    BassEdges,
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
