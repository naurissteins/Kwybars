use std::env;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverlayPosition {
    Bottom,
    Top,
    Left,
    Right,
}

impl OverlayPosition {
    fn parse(value: &str) -> Result<Self, ConfigLoadError> {
        match value {
            "bottom" => Ok(Self::Bottom),
            "top" => Ok(Self::Top),
            "left" => Ok(Self::Left),
            "right" => Ok(Self::Right),
            _ => Err(ConfigLoadError::Parse(format!(
                "unknown overlay.position value: {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverlayLayer {
    Background,
    Bottom,
    Top,
}

impl OverlayLayer {
    fn parse(value: &str) -> Result<Self, ConfigLoadError> {
        match value {
            "background" => Ok(Self::Background),
            "bottom" => Ok(Self::Bottom),
            "top" => Ok(Self::Top),
            _ => Err(ConfigLoadError::Parse(format!(
                "unknown overlay.layer value: {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HorizontalAlignment {
    Left,
    Center,
    Right,
}

impl HorizontalAlignment {
    fn parse(value: &str) -> Result<Self, ConfigLoadError> {
        match value {
            "left" => Ok(Self::Left),
            "center" => Ok(Self::Center),
            "right" => Ok(Self::Right),
            _ => Err(ConfigLoadError::Parse(format!(
                "unknown overlay.horizontal_alignment value: {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerticalAlignment {
    Top,
    Center,
    Bottom,
}

impl VerticalAlignment {
    fn parse(value: &str) -> Result<Self, ConfigLoadError> {
        match value {
            "top" => Ok(Self::Top),
            "center" => Ok(Self::Center),
            "bottom" => Ok(Self::Bottom),
            _ => Err(ConfigLoadError::Parse(format!(
                "unknown overlay.vertical_alignment value: {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayConfig {
    pub position: OverlayPosition,
    pub layer: OverlayLayer,
    pub anchor_margin: u32,
    pub margin_left: u32,
    pub margin_right: u32,
    pub margin_top: u32,
    pub margin_bottom: u32,
    pub full_length: bool,
    pub width: u32,
    pub height: u32,
    pub horizontal_alignment: HorizontalAlignment,
    pub vertical_alignment: VerticalAlignment,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            position: OverlayPosition::Bottom,
            layer: OverlayLayer::Background,
            anchor_margin: 12,
            margin_left: 0,
            margin_right: 0,
            margin_top: 0,
            margin_bottom: 0,
            full_length: true,
            width: 800,
            height: 120,
            horizontal_alignment: HorizontalAlignment::Center,
            vertical_alignment: VerticalAlignment::Center,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VisualizerBackend {
    Auto,
    Pipewire,
    Cava,
    Dummy,
}

impl VisualizerBackend {
    fn parse(value: &str) -> Result<Self, ConfigLoadError> {
        match value {
            "auto" => Ok(Self::Auto),
            "pipewire" => Ok(Self::Pipewire),
            "cava" => Ok(Self::Cava),
            "dummy" => Ok(Self::Dummy),
            _ => Err(ConfigLoadError::Parse(format!(
                "unknown visualizer.backend value: {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RgbaColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl RgbaColor {
    fn parse(value: &str) -> Result<Self, ConfigLoadError> {
        let normalized = value.trim();
        let raw_components = if normalized.starts_with("rgba(") && normalized.ends_with(')') {
            &normalized[5..normalized.len() - 1]
        } else {
            normalized
        };

        let parts: Vec<&str> = raw_components.split(',').map(str::trim).collect();
        if parts.len() != 4 {
            return Err(ConfigLoadError::Parse(format!(
                "invalid visualizer.color_rgba value: {value}"
            )));
        }

        let mut r = parse_f32("visualizer.color_rgba.r", parts[0])?;
        let mut g = parse_f32("visualizer.color_rgba.g", parts[1])?;
        let mut b = parse_f32("visualizer.color_rgba.b", parts[2])?;
        let a = parse_f32("visualizer.color_rgba.a", parts[3])?.clamp(0.0, 1.0);

        // Allow rgb either in 0.0..1.0 or 0..255 ranges.
        if r > 1.0 || g > 1.0 || b > 1.0 {
            r = (r / 255.0).clamp(0.0, 1.0);
            g = (g / 255.0).clamp(0.0, 1.0);
            b = (b / 255.0).clamp(0.0, 1.0);
        } else {
            r = r.clamp(0.0, 1.0);
            g = g.clamp(0.0, 1.0);
            b = b.clamp(0.0, 1.0);
        }

        Ok(Self { r, g, b, a })
    }
}

impl Default for RgbaColor {
    fn default() -> Self {
        Self {
            r: 0.12,
            g: 0.88,
            b: 0.68,
            a: 0.9,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualizerColorMode {
    Solid,
    Gradient,
}

impl VisualizerColorMode {
    fn parse(value: &str) -> Result<Self, ConfigLoadError> {
        match value {
            "solid" => Ok(Self::Solid),
            "gradient" => Ok(Self::Gradient),
            _ => Err(ConfigLoadError::Parse(format!(
                "unknown visualizer.color_mode value: {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisualizerConfig {
    pub backend: VisualizerBackend,
    pub bars: usize,
    pub bar_width: u32,
    pub gap: u32,
    pub framerate: u32,
    pub color_mode: VisualizerColorMode,
    pub color_rgba: RgbaColor,
    pub color2_rgba: RgbaColor,
    pub pipewire_attack: f32,
    pub pipewire_decay: f32,
    pub pipewire_gain: f32,
    pub pipewire_curve: f32,
    pub pipewire_neighbor_mix: f32,
}

impl Default for VisualizerConfig {
    fn default() -> Self {
        Self {
            backend: VisualizerBackend::Cava,
            bars: 48,
            bar_width: 6,
            gap: 3,
            framerate: 60,
            color_mode: VisualizerColorMode::Solid,
            color_rgba: RgbaColor::default(),
            color2_rgba: RgbaColor::default(),
            pipewire_attack: 0.14,
            pipewire_decay: 0.975,
            pipewire_gain: 1.20,
            pipewire_curve: 0.95,
            pipewire_neighbor_mix: 0.24,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AppConfig {
    pub overlay: OverlayConfig,
    pub visualizer: VisualizerConfig,
}

#[derive(Debug)]
pub enum ConfigLoadError {
    Io(std::io::Error),
    Parse(String),
}

impl Display for ConfigLoadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "io error: {err}"),
            Self::Parse(msg) => write!(f, "config parse error: {msg}"),
        }
    }
}

impl Error for ConfigLoadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Parse(_) => None,
        }
    }
}

pub fn default_config_path() -> PathBuf {
    if let Ok(override_path) = env::var("KWYBARS_CONFIG") {
        return PathBuf::from(override_path);
    }

    if let Ok(config_home) = env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(config_home).join("kwybars/config.toml");
    }

    if let Ok(home) = env::var("HOME") {
        return PathBuf::from(home).join(".config/kwybars/config.toml");
    }

    PathBuf::from("kwybars.toml")
}

pub fn load_or_default(path: &Path) -> Result<AppConfig, ConfigLoadError> {
    let raw = match fs::read_to_string(path) {
        Ok(value) => value,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(AppConfig::default()),
        Err(err) => return Err(ConfigLoadError::Io(err)),
    };

    parse_config(&raw)
}

fn parse_config(raw: &str) -> Result<AppConfig, ConfigLoadError> {
    let mut config = AppConfig::default();
    let mut section: Option<&str> = None;

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let next = &trimmed[1..trimmed.len() - 1];
            section = Some(next);
            continue;
        }

        let Some((key, value)) = trimmed.split_once('=') else {
            return Err(ConfigLoadError::Parse(format!(
                "invalid key/value line: {trimmed}"
            )));
        };

        let key = key.trim();
        let value = value.trim().trim_matches('"');

        match section {
            Some("overlay") => parse_overlay_key(&mut config.overlay, key, value)?,
            Some("visualizer") => parse_visualizer_key(&mut config.visualizer, key, value)?,
            Some(other) => {
                return Err(ConfigLoadError::Parse(format!("unknown section [{other}]")));
            }
            None => {
                return Err(ConfigLoadError::Parse(
                    "key/value before a section header".to_owned(),
                ));
            }
        }
    }

    Ok(config)
}

fn parse_overlay_key(
    overlay: &mut OverlayConfig,
    key: &str,
    value: &str,
) -> Result<(), ConfigLoadError> {
    match key {
        "position" => overlay.position = OverlayPosition::parse(value)?,
        "layer" => overlay.layer = OverlayLayer::parse(value)?,
        "anchor_margin" => overlay.anchor_margin = parse_u32(key, value)?,
        "margin_left" => overlay.margin_left = parse_u32(key, value)?,
        "margin_right" => overlay.margin_right = parse_u32(key, value)?,
        "margin_top" => overlay.margin_top = parse_u32(key, value)?,
        "margin_bottom" => overlay.margin_bottom = parse_u32(key, value)?,
        "full_length" => overlay.full_length = parse_bool(key, value)?,
        "width" => overlay.width = parse_u32(key, value)?,
        "height" => overlay.height = parse_u32(key, value)?,
        "horizontal_alignment" => overlay.horizontal_alignment = HorizontalAlignment::parse(value)?,
        "vertical_alignment" => overlay.vertical_alignment = VerticalAlignment::parse(value)?,
        _ => {
            return Err(ConfigLoadError::Parse(format!(
                "unknown overlay key: {key}"
            )));
        }
    }
    Ok(())
}

fn parse_visualizer_key(
    visualizer: &mut VisualizerConfig,
    key: &str,
    value: &str,
) -> Result<(), ConfigLoadError> {
    match key {
        "backend" => visualizer.backend = VisualizerBackend::parse(value)?,
        "bars" => visualizer.bars = parse_usize(key, value)?,
        "bar_width" => visualizer.bar_width = parse_u32(key, value)?,
        "gap" => visualizer.gap = parse_u32(key, value)?,
        "framerate" => visualizer.framerate = parse_u32(key, value)?,
        "color_mode" => visualizer.color_mode = VisualizerColorMode::parse(value)?,
        "color_rgba" => visualizer.color_rgba = RgbaColor::parse(value)?,
        "color2_rgba" => visualizer.color2_rgba = RgbaColor::parse(value)?,
        "pipewire_attack" => visualizer.pipewire_attack = parse_f32(key, value)?,
        "pipewire_decay" => visualizer.pipewire_decay = parse_f32(key, value)?,
        "pipewire_gain" => visualizer.pipewire_gain = parse_f32(key, value)?,
        "pipewire_curve" => visualizer.pipewire_curve = parse_f32(key, value)?,
        "pipewire_neighbor_mix" => visualizer.pipewire_neighbor_mix = parse_f32(key, value)?,
        _ => {
            return Err(ConfigLoadError::Parse(format!(
                "unknown visualizer key: {key}"
            )));
        }
    }
    Ok(())
}

fn parse_u32(key: &str, value: &str) -> Result<u32, ConfigLoadError> {
    value
        .parse::<u32>()
        .map_err(|_| ConfigLoadError::Parse(format!("invalid u32 for {key}: {value}")))
}

fn parse_usize(key: &str, value: &str) -> Result<usize, ConfigLoadError> {
    value
        .parse::<usize>()
        .map_err(|_| ConfigLoadError::Parse(format!("invalid usize for {key}: {value}")))
}

fn parse_f32(key: &str, value: &str) -> Result<f32, ConfigLoadError> {
    value
        .parse::<f32>()
        .map_err(|_| ConfigLoadError::Parse(format!("invalid f32 for {key}: {value}")))
}

fn parse_bool(key: &str, value: &str) -> Result<bool, ConfigLoadError> {
    match value {
        "true" | "1" => Ok(true),
        "false" | "0" => Ok(false),
        _ => Err(ConfigLoadError::Parse(format!(
            "invalid bool for {key}: {value}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AppConfig, HorizontalAlignment, OverlayLayer, OverlayPosition, VerticalAlignment,
        VisualizerBackend, VisualizerColorMode, parse_config,
    };

    #[test]
    fn parses_valid_config() {
        let raw = r#"
        [overlay]
        position = "top"
        layer = "top"
        anchor_margin = 20
        margin_left = 11
        margin_right = 13
        margin_top = 7
        margin_bottom = 9
        full_length = false
        width = 1200
        height = 140
        horizontal_alignment = "right"
        vertical_alignment = "bottom"

        [visualizer]
        backend = "dummy"
        bars = 64
        bar_width = 5
        gap = 2
        framerate = 75
        color_mode = "gradient"
        color_rgba = "rgba(255, 255, 255, 0.5)"
        color2_rgba = "rgba(255, 0, 0, 1.0)"
        pipewire_attack = 0.2
        pipewire_decay = 0.9
        pipewire_gain = 1.5
        pipewire_curve = 0.8
        pipewire_neighbor_mix = 0.3
        "#;

        let parsed = match parse_config(raw) {
            Ok(value) => value,
            Err(err) => panic!("valid config should parse, got error: {err}"),
        };
        assert_eq!(parsed.overlay.position, OverlayPosition::Top);
        assert_eq!(parsed.overlay.layer, OverlayLayer::Top);
        assert_eq!(parsed.overlay.anchor_margin, 20);
        assert_eq!(parsed.overlay.margin_left, 11);
        assert_eq!(parsed.overlay.margin_right, 13);
        assert_eq!(parsed.overlay.margin_top, 7);
        assert_eq!(parsed.overlay.margin_bottom, 9);
        assert!(!parsed.overlay.full_length);
        assert_eq!(parsed.overlay.width, 1200);
        assert_eq!(parsed.overlay.height, 140);
        assert_eq!(
            parsed.overlay.horizontal_alignment,
            HorizontalAlignment::Right
        );
        assert_eq!(parsed.overlay.vertical_alignment, VerticalAlignment::Bottom);
        assert_eq!(parsed.visualizer.backend, VisualizerBackend::Dummy);
        assert_eq!(parsed.visualizer.bars, 64);
        assert_eq!(parsed.visualizer.bar_width, 5);
        assert_eq!(parsed.visualizer.gap, 2);
        assert_eq!(parsed.visualizer.framerate, 75);
        assert_eq!(parsed.visualizer.color_mode, VisualizerColorMode::Gradient);
        assert!((parsed.visualizer.color_rgba.r - 1.0).abs() < 1e-5);
        assert!((parsed.visualizer.color_rgba.g - 1.0).abs() < 1e-5);
        assert!((parsed.visualizer.color_rgba.b - 1.0).abs() < 1e-5);
        assert!((parsed.visualizer.color_rgba.a - 0.5).abs() < 1e-5);
        assert!((parsed.visualizer.color2_rgba.r - 1.0).abs() < 1e-5);
        assert!(parsed.visualizer.color2_rgba.g.abs() < 1e-5);
        assert!(parsed.visualizer.color2_rgba.b.abs() < 1e-5);
        assert!((parsed.visualizer.color2_rgba.a - 1.0).abs() < 1e-5);
        assert_eq!(parsed.visualizer.pipewire_attack, 0.2);
        assert_eq!(parsed.visualizer.pipewire_decay, 0.9);
        assert_eq!(parsed.visualizer.pipewire_gain, 1.5);
        assert_eq!(parsed.visualizer.pipewire_curve, 0.8);
        assert_eq!(parsed.visualizer.pipewire_neighbor_mix, 0.3);
    }

    #[test]
    fn returns_default_for_empty_config() {
        let parsed = match parse_config("") {
            Ok(value) => value,
            Err(err) => panic!("empty config should parse, got error: {err}"),
        };
        assert_eq!(parsed, AppConfig::default());
    }
}
