use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverlayPosition {
    Bottom,
    Top,
    Left,
    Right,
}

impl OverlayPosition {
    pub(crate) fn parse(value: &str) -> Result<Self, ConfigLoadError> {
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

impl Display for OverlayPosition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bottom => write!(f, "bottom"),
            Self::Top => write!(f, "top"),
            Self::Left => write!(f, "left"),
            Self::Right => write!(f, "right"),
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
    pub(crate) fn parse(value: &str) -> Result<Self, ConfigLoadError> {
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

impl Display for OverlayLayer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Background => write!(f, "background"),
            Self::Bottom => write!(f, "bottom"),
            Self::Top => write!(f, "top"),
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
    pub(crate) fn parse(value: &str) -> Result<Self, ConfigLoadError> {
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

impl Display for HorizontalAlignment {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Left => write!(f, "left"),
            Self::Center => write!(f, "center"),
            Self::Right => write!(f, "right"),
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
    pub(crate) fn parse(value: &str) -> Result<Self, ConfigLoadError> {
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

impl Display for VerticalAlignment {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Top => write!(f, "top"),
            Self::Center => write!(f, "center"),
            Self::Bottom => write!(f, "bottom"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverlayMonitorMode {
    Primary,
    All,
    List,
}

impl OverlayMonitorMode {
    pub(crate) fn parse(value: &str) -> Result<Self, ConfigLoadError> {
        match value {
            "primary" => Ok(Self::Primary),
            "all" => Ok(Self::All),
            "list" => Ok(Self::List),
            _ => Err(ConfigLoadError::Parse(format!(
                "unknown overlay.monitor_mode value: {value}"
            ))),
        }
    }
}

impl Display for OverlayMonitorMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Primary => write!(f, "primary"),
            Self::All => write!(f, "all"),
            Self::List => write!(f, "list"),
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
    pub monitor_mode: OverlayMonitorMode,
    pub monitors: Vec<String>,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            position: OverlayPosition::Bottom,
            layer: OverlayLayer::Background,
            anchor_margin: 20,
            margin_left: 20,
            margin_right: 20,
            margin_top: 0,
            margin_bottom: 0,
            full_length: true,
            width: 800,
            height: 500,
            horizontal_alignment: HorizontalAlignment::Center,
            vertical_alignment: VerticalAlignment::Center,
            monitor_mode: OverlayMonitorMode::Primary,
            monitors: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageOverlayFit {
    Contain,
    Cover,
    Stretch,
}

impl ImageOverlayFit {
    pub(crate) fn parse(value: &str) -> Result<Self, ConfigLoadError> {
        match value {
            "contain" => Ok(Self::Contain),
            "cover" => Ok(Self::Cover),
            "stretch" => Ok(Self::Stretch),
            _ => Err(ConfigLoadError::Parse(format!(
                "unknown image_overlay.fit value: {value}"
            ))),
        }
    }
}

impl Display for ImageOverlayFit {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Contain => write!(f, "contain"),
            Self::Cover => write!(f, "cover"),
            Self::Stretch => write!(f, "stretch"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImageOverlayConfig {
    pub enabled: bool,
    pub path: Option<String>,
    pub opacity: f32,
    pub fit: ImageOverlayFit,
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
            fit: ImageOverlayFit::Contain,
            width: 0,
            height: 0,
            offset_x: 0.0,
            offset_y: 0.0,
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
    pub(crate) fn parse(value: &str) -> Result<Self, ConfigLoadError> {
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

impl Display for VisualizerBackend {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Auto => write!(f, "auto"),
            Self::Pipewire => write!(f, "pipewire"),
            Self::Cava => write!(f, "cava"),
            Self::Dummy => write!(f, "dummy"),
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
    pub(crate) fn parse(value: &str) -> Result<Self, ConfigLoadError> {
        let normalized = value.trim();
        let raw_components = if let Some(inner) = normalized.strip_prefix("rgba(") {
            inner.strip_suffix(')').unwrap_or(inner)
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
    pub(crate) fn parse(value: &str) -> Result<Self, ConfigLoadError> {
        match value {
            "solid" => Ok(Self::Solid),
            "gradient" => Ok(Self::Gradient),
            _ => Err(ConfigLoadError::Parse(format!(
                "unknown visualizer.color_mode value: {value}"
            ))),
        }
    }
}

impl Display for VisualizerColorMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Solid => write!(f, "solid"),
            Self::Gradient => write!(f, "gradient"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualizerGradientDirection {
    Vertical,
    Horizontal,
}

impl VisualizerGradientDirection {
    pub(crate) fn parse(value: &str) -> Result<Self, ConfigLoadError> {
        match value {
            "vertical" => Ok(Self::Vertical),
            "horizontal" => Ok(Self::Horizontal),
            _ => Err(ConfigLoadError::Parse(format!(
                "unknown visualizer.gradient_direction value: {value}"
            ))),
        }
    }
}

impl Display for VisualizerGradientDirection {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Vertical => write!(f, "vertical"),
            Self::Horizontal => write!(f, "horizontal"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualizerLayout {
    Line,
    Mirror,
    Wave,
    Frame,
    Radial,
    Polygon,
    Particle,
    Floating,
}

impl VisualizerLayout {
    pub(crate) fn parse(value: &str) -> Result<Self, ConfigLoadError> {
        match value {
            "line" => Ok(Self::Line),
            "mirror" => Ok(Self::Mirror),
            "wave" => Ok(Self::Wave),
            "frame" => Ok(Self::Frame),
            "radial" => Ok(Self::Radial),
            "polygon" => Ok(Self::Polygon),
            "particle" => Ok(Self::Particle),
            "floating" => Ok(Self::Floating),
            _ => Err(ConfigLoadError::Parse(format!(
                "unknown visualizer.layout value: {value}"
            ))),
        }
    }
}

impl Display for VisualizerLayout {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Line => write!(f, "line"),
            Self::Mirror => write!(f, "mirror"),
            Self::Wave => write!(f, "wave"),
            Self::Frame => write!(f, "frame"),
            Self::Radial => write!(f, "radial"),
            Self::Polygon => write!(f, "polygon"),
            Self::Particle => write!(f, "particle"),
            Self::Floating => write!(f, "floating"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineMode {
    Continuous,
    Split,
}

impl LineMode {
    pub(crate) fn parse(value: &str) -> Result<Self, ConfigLoadError> {
        match value {
            "continuous" => Ok(Self::Continuous),
            "split" => Ok(Self::Split),
            _ => Err(ConfigLoadError::Parse(format!(
                "unknown visualizer.line_mode value: {value}"
            ))),
        }
    }
}

impl Display for LineMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Continuous => write!(f, "continuous"),
            Self::Split => write!(f, "split"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MirrorOrientation {
    Horizontal,
    Vertical,
}

impl MirrorOrientation {
    pub(crate) fn parse(value: &str) -> Result<Self, ConfigLoadError> {
        match value {
            "horizontal" => Ok(Self::Horizontal),
            "vertical" => Ok(Self::Vertical),
            _ => Err(ConfigLoadError::Parse(format!(
                "unknown visualizer.mirror_orientation value: {value}"
            ))),
        }
    }
}

impl Display for MirrorOrientation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Horizontal => write!(f, "horizontal"),
            Self::Vertical => write!(f, "vertical"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameMirrorMode {
    Off,
    All,
    Pairs,
}

impl FrameMirrorMode {
    pub(crate) fn parse(value: &str) -> Result<Self, ConfigLoadError> {
        match value {
            "off" => Ok(Self::Off),
            "all" => Ok(Self::All),
            "pairs" => Ok(Self::Pairs),
            _ => Err(ConfigLoadError::Parse(format!(
                "unknown visualizer.frame_mirror_mode value: {value}"
            ))),
        }
    }
}

impl Display for FrameMirrorMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Off => write!(f, "off"),
            Self::All => write!(f, "all"),
            Self::Pairs => write!(f, "pairs"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisualizerConfig {
    pub backend: VisualizerBackend,
    pub layout: VisualizerLayout,
    pub line_mode: LineMode,
    pub line_split_gap: u32,
    pub mirror_orientation: MirrorOrientation,
    pub mirror_gap: u32,
    pub wave_stroke_width: u32,
    pub wave_fill: bool,
    pub wave_glow: bool,
    pub wave_smoothing: f32,
    pub wave_motion_smoothing: f32,
    pub wave_amplitude: f32,
    pub frame_edges: Vec<OverlayPosition>,
    pub frame_mirror_mode: FrameMirrorMode,
    pub bars: usize,
    pub bar_width: u32,
    pub bar_corner_radius: f32,
    pub segmented_bars: bool,
    pub segment_length: u32,
    pub segment_gap: u32,
    pub radial_inner_radius: u32,
    pub radial_start_angle: f32,
    pub radial_arc_degrees: f32,
    pub radial_rotation_speed: f32,
    pub center_offset_x: f32,
    pub center_offset_y: f32,
    pub polygon_sides: u32,
    pub polygon_radius: u32,
    pub polygon_bar_length: u32,
    pub polygon_rotation: f32,
    pub polygon_rotation_speed: f32,
    pub gap: u32,
    pub framerate: u32,
    pub color_mode: VisualizerColorMode,
    pub gradient_direction: VisualizerGradientDirection,
    pub color_rgba: RgbaColor,
    pub color2_rgba: RgbaColor,
    pub theme: Option<String>,
    pub theme_opacity: f32,
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
            layout: VisualizerLayout::Line,
            line_mode: LineMode::Continuous,
            line_split_gap: 200,
            mirror_orientation: MirrorOrientation::Horizontal,
            mirror_gap: 0,
            wave_stroke_width: 10,
            wave_fill: true,
            wave_glow: false,
            wave_smoothing: 1.0,
            wave_motion_smoothing: 0.22,
            wave_amplitude: 0.8,
            frame_edges: vec![OverlayPosition::Top, OverlayPosition::Bottom],
            frame_mirror_mode: FrameMirrorMode::Pairs,
            bars: 50,
            bar_width: 8,
            bar_corner_radius: 20.0,
            segmented_bars: false,
            segment_length: 14,
            segment_gap: 6,
            radial_inner_radius: 180,
            radial_start_angle: -90.0,
            radial_arc_degrees: 360.0,
            radial_rotation_speed: 0.0,
            center_offset_x: 0.0,
            center_offset_y: 0.0,
            polygon_sides: 3,
            polygon_radius: 220,
            polygon_bar_length: 0,
            polygon_rotation: -90.0,
            polygon_rotation_speed: 0.0,
            gap: 20,
            framerate: 60,
            color_mode: VisualizerColorMode::Gradient,
            gradient_direction: VisualizerGradientDirection::Vertical,
            color_rgba: RgbaColor {
                r: 175.0 / 255.0,
                g: 198.0 / 255.0,
                b: 1.0,
                a: 0.7,
            },
            color2_rgba: RgbaColor {
                r: 191.0 / 255.0,
                g: 198.0 / 255.0,
                b: 220.0 / 255.0,
                a: 0.7,
            },
            theme: None,
            theme_opacity: 1.0,
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
    pub image_overlay: ImageOverlayConfig,
    pub daemon: DaemonConfig,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DaemonConfig {
    pub enabled: bool,
    pub poll_interval_ms: u64,
    pub activity_threshold: f32,
    pub activate_delay_ms: u64,
    pub deactivate_delay_ms: u64,
    pub stop_on_silence: bool,
    pub notify_on_error: bool,
    pub notify_cooldown_seconds: u64,
    pub overlay_command: String,
    pub overlay_args: Vec<String>,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            poll_interval_ms: 90,
            activity_threshold: 0.035,
            activate_delay_ms: 180,
            deactivate_delay_ms: 2200,
            stop_on_silence: true,
            notify_on_error: true,
            notify_cooldown_seconds: 45,
            overlay_command: "kwybars-overlay".to_owned(),
            overlay_args: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct VisualizerColorOverrides {
    pub color_rgba: Option<RgbaColor>,
    pub color2_rgba: Option<RgbaColor>,
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

pub fn apply_color_overrides(config: &mut AppConfig, overrides: VisualizerColorOverrides) {
    if let Some(color) = overrides.color_rgba {
        config.visualizer.color_rgba = color;
    }
    if let Some(color) = overrides.color2_rgba {
        config.visualizer.color2_rgba = color;
    }
}

fn parse_f32(key: &str, value: &str) -> Result<f32, ConfigLoadError> {
    value
        .parse::<f32>()
        .map_err(|_| ConfigLoadError::Parse(format!("invalid f32 for {key}: {value}")))
}
