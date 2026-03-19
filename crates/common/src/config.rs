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
    fn parse(value: &str) -> Result<Self, ConfigLoadError> {
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
    fn parse(value: &str) -> Result<Self, ConfigLoadError> {
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

impl Display for VisualizerColorMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Solid => write!(f, "solid"),
            Self::Gradient => write!(f, "gradient"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualizerLayout {
    Line,
    Frame,
    Radial,
    Polygon,
    Particle,
    Floating,
}

impl VisualizerLayout {
    fn parse(value: &str) -> Result<Self, ConfigLoadError> {
        match value {
            "line" => Ok(Self::Line),
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
            Self::Frame => write!(f, "frame"),
            Self::Radial => write!(f, "radial"),
            Self::Polygon => write!(f, "polygon"),
            Self::Particle => write!(f, "particle"),
            Self::Floating => write!(f, "floating"),
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
    fn parse(value: &str) -> Result<Self, ConfigLoadError> {
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
    pub polygon_rotation: f32,
    pub polygon_rotation_speed: f32,
    pub gap: u32,
    pub framerate: u32,
    pub color_mode: VisualizerColorMode,
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
            polygon_rotation: -90.0,
            polygon_rotation_speed: 0.0,
            gap: 20,
            framerate: 60,
            color_mode: VisualizerColorMode::Gradient,
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

pub fn default_colors_path(config_path: &Path) -> PathBuf {
    match config_path.parent() {
        Some(parent) => parent.join("colors.toml"),
        None => PathBuf::from("colors.toml"),
    }
}

pub fn load_or_default(path: &Path) -> Result<AppConfig, ConfigLoadError> {
    let raw = match fs::read_to_string(path) {
        Ok(value) => value,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(AppConfig::default()),
        Err(err) => return Err(ConfigLoadError::Io(err)),
    };

    parse_config(&raw)
}

pub fn load_color_overrides(path: &Path) -> Result<VisualizerColorOverrides, ConfigLoadError> {
    let raw = match fs::read_to_string(path) {
        Ok(value) => value,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(VisualizerColorOverrides::default());
        }
        Err(err) => return Err(ConfigLoadError::Io(err)),
    };

    parse_color_overrides(&raw)
}

pub fn apply_color_overrides(config: &mut AppConfig, overrides: VisualizerColorOverrides) {
    if let Some(color) = overrides.color_rgba {
        config.visualizer.color_rgba = color;
    }
    if let Some(color) = overrides.color2_rgba {
        config.visualizer.color2_rgba = color;
    }
}

fn parse_config(raw: &str) -> Result<AppConfig, ConfigLoadError> {
    let mut config = AppConfig::default();
    let mut section: Option<&str> = None;

    for (line_idx, line) in raw.lines().enumerate() {
        let line_no = line_idx + 1;
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
                "line {line_no}: invalid key/value line: {trimmed}"
            )));
        };

        let key = key.trim();
        let value = normalize_value(value);

        match section {
            Some("overlay") => parse_overlay_key(&mut config.overlay, key, &value)
                .map_err(|err| with_line_context(err, line_no))?,
            Some("visualizer") => parse_visualizer_key(&mut config.visualizer, key, &value)
                .map_err(|err| with_line_context(err, line_no))?,
            Some("daemon") => parse_daemon_key(&mut config.daemon, key, &value)
                .map_err(|err| with_line_context(err, line_no))?,
            Some(other) => {
                return Err(ConfigLoadError::Parse(format!(
                    "line {line_no}: unknown section [{other}]"
                )));
            }
            None => {
                if !parse_root_key(&mut config, key, &value)
                    .map_err(|err| with_line_context(err, line_no))?
                {
                    return Err(ConfigLoadError::Parse(format!(
                        "line {line_no}: key/value before a section header"
                    )));
                }
            }
        }
    }

    Ok(config)
}

fn parse_color_overrides(raw: &str) -> Result<VisualizerColorOverrides, ConfigLoadError> {
    let mut overrides = VisualizerColorOverrides::default();
    let mut section: Option<&str> = None;

    for (line_idx, line) in raw.lines().enumerate() {
        let line_no = line_idx + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            section = Some(&trimmed[1..trimmed.len() - 1]);
            continue;
        }

        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };

        let section_supported =
            section.is_none() || matches!(section, Some("visualizer") | Some("colors"));
        if !section_supported {
            continue;
        }

        let key = key.trim();
        let value = normalize_value(value);

        match key {
            "color_rgba" => {
                overrides.color_rgba =
                    Some(RgbaColor::parse(&value).map_err(|err| with_line_context(err, line_no))?)
            }
            "color2_rgba" => {
                overrides.color2_rgba =
                    Some(RgbaColor::parse(&value).map_err(|err| with_line_context(err, line_no))?)
            }
            _ => {}
        }
    }

    Ok(overrides)
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
        "monitor_mode" => overlay.monitor_mode = OverlayMonitorMode::parse(value)?,
        "monitors" => overlay.monitors = parse_string_list(value),
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
        "layout" => visualizer.layout = VisualizerLayout::parse(value)?,
        "frame_edges" => visualizer.frame_edges = parse_overlay_position_list(value)?,
        "frame_mirror_mode" => visualizer.frame_mirror_mode = FrameMirrorMode::parse(value)?,
        "frame_mirror" => {
            visualizer.frame_mirror_mode = if parse_bool(key, value)? {
                FrameMirrorMode::All
            } else {
                FrameMirrorMode::Off
            };
        }
        "bars" => visualizer.bars = parse_usize(key, value)?,
        "bar_width" => visualizer.bar_width = parse_u32(key, value)?,
        "bar_corner_radius" => {
            visualizer.bar_corner_radius = parse_f32(key, value)?.max(0.0);
        }
        "segmented_bars" => visualizer.segmented_bars = parse_bool(key, value)?,
        "segment_length" => visualizer.segment_length = parse_u32(key, value)?.max(1),
        "segment_gap" => visualizer.segment_gap = parse_u32(key, value)?,
        "radial_inner_radius" => visualizer.radial_inner_radius = parse_u32(key, value)?.max(1),
        "radial_start_angle" => visualizer.radial_start_angle = parse_f32(key, value)?,
        "radial_arc_degrees" => visualizer.radial_arc_degrees = parse_f32(key, value)?,
        "radial_rotation_speed" => visualizer.radial_rotation_speed = parse_f32(key, value)?,
        "center_offset_x" => visualizer.center_offset_x = parse_f32(key, value)?,
        "center_offset_y" => visualizer.center_offset_y = parse_f32(key, value)?,
        "polygon_sides" => visualizer.polygon_sides = parse_u32(key, value)?.max(3),
        "polygon_radius" => visualizer.polygon_radius = parse_u32(key, value)?.max(1),
        "polygon_rotation" => visualizer.polygon_rotation = parse_f32(key, value)?,
        "polygon_rotation_speed" => visualizer.polygon_rotation_speed = parse_f32(key, value)?,
        "gap" => visualizer.gap = parse_u32(key, value)?,
        "framerate" => visualizer.framerate = parse_u32(key, value)?,
        "color_mode" => visualizer.color_mode = VisualizerColorMode::parse(value)?,
        "color_rgba" => visualizer.color_rgba = RgbaColor::parse(value)?,
        "color2_rgba" => visualizer.color2_rgba = RgbaColor::parse(value)?,
        "theme" => visualizer.theme = parse_optional_string(value),
        "theme_opacity" => visualizer.theme_opacity = parse_f32(key, value)?.clamp(0.0, 1.0),
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

fn parse_daemon_key(
    daemon: &mut DaemonConfig,
    key: &str,
    value: &str,
) -> Result<(), ConfigLoadError> {
    match key {
        "enabled" => daemon.enabled = parse_bool(key, value)?,
        "poll_interval_ms" => daemon.poll_interval_ms = parse_u64(key, value)?.max(16),
        "activity_threshold" => daemon.activity_threshold = parse_f32(key, value)?.clamp(0.0, 1.0),
        "activate_delay_ms" => daemon.activate_delay_ms = parse_u64(key, value)?,
        "deactivate_delay_ms" => daemon.deactivate_delay_ms = parse_u64(key, value)?,
        "stop_on_silence" => daemon.stop_on_silence = parse_bool(key, value)?,
        "notify_on_error" => daemon.notify_on_error = parse_bool(key, value)?,
        "notify_cooldown_seconds" => daemon.notify_cooldown_seconds = parse_u64(key, value)?,
        "overlay_command" => {
            let command = parse_optional_string(value).unwrap_or_default();
            daemon.overlay_command = if command.is_empty() {
                DaemonConfig::default().overlay_command
            } else {
                command
            };
        }
        "overlay_args" => daemon.overlay_args = parse_string_list(value),
        _ => {
            return Err(ConfigLoadError::Parse(format!("unknown daemon key: {key}")));
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

fn parse_u64(key: &str, value: &str) -> Result<u64, ConfigLoadError> {
    value
        .parse::<u64>()
        .map_err(|_| ConfigLoadError::Parse(format!("invalid u64 for {key}: {value}")))
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

fn parse_root_key(config: &mut AppConfig, key: &str, value: &str) -> Result<bool, ConfigLoadError> {
    match key {
        "theme" => {
            config.visualizer.theme = parse_optional_string(value);
            Ok(true)
        }
        "theme_opacity" => {
            config.visualizer.theme_opacity = parse_f32(key, value)?.clamp(0.0, 1.0);
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn parse_overlay_position_list(value: &str) -> Result<Vec<OverlayPosition>, ConfigLoadError> {
    let mut edges = Vec::new();
    for item in parse_string_list(value) {
        let edge = OverlayPosition::parse(&item)?;
        if !edges.contains(&edge) {
            edges.push(edge);
        }
    }

    if edges.is_empty() {
        Ok(VisualizerConfig::default().frame_edges)
    } else {
        Ok(edges)
    }
}

fn with_line_context(error: ConfigLoadError, line_no: usize) -> ConfigLoadError {
    match error {
        ConfigLoadError::Parse(message) => {
            ConfigLoadError::Parse(format!("line {line_no}: {message}"))
        }
        other => other,
    }
}

fn parse_optional_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

fn parse_string_list(value: &str) -> Vec<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let inner = if trimmed.starts_with('[') && trimmed.ends_with(']') && trimmed.len() >= 2 {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    };

    inner
        .split(',')
        .map(str::trim)
        .map(|item| item.trim_matches('"').trim_matches('\'').trim().to_owned())
        .filter(|item| !item.is_empty())
        .collect()
}

fn normalize_value(raw: &str) -> String {
    let mut without_comment = String::new();
    let mut in_quotes = false;
    let mut escaped = false;

    for ch in raw.chars() {
        if ch == '"' && !escaped {
            in_quotes = !in_quotes;
            without_comment.push(ch);
            continue;
        }
        if ch == '#' && !in_quotes {
            break;
        }
        escaped = ch == '\\' && !escaped;
        without_comment.push(ch);
    }

    let mut cleaned = without_comment.trim().trim_end_matches([',', ';']).trim();

    if cleaned.len() >= 2 {
        let quoted_double = cleaned.starts_with('"') && cleaned.ends_with('"');
        let quoted_single = cleaned.starts_with('\'') && cleaned.ends_with('\'');
        if quoted_double || quoted_single {
            cleaned = &cleaned[1..cleaned.len() - 1];
        }
    }

    cleaned.trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::{
        AppConfig, DaemonConfig, FrameMirrorMode, HorizontalAlignment, OverlayLayer,
        OverlayMonitorMode, OverlayPosition, VerticalAlignment, VisualizerBackend,
        VisualizerColorMode, VisualizerColorOverrides, VisualizerLayout, apply_color_overrides,
        parse_color_overrides, parse_config,
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
        monitor_mode = "list"
        monitors = ["DP-1", "HDMI-A-1"]

        [visualizer]
        backend = "dummy"
        layout = "polygon"
        frame_edges = ["top", "bottom"]
        frame_mirror_mode = "pairs"
        bars = 64
        bar_width = 5
        bar_corner_radius = 3.5
        segmented_bars = true
        segment_length = 9
        segment_gap = 4
        radial_inner_radius = 160
        radial_start_angle = -180
        radial_arc_degrees = 180
        radial_rotation_speed = 24
        center_offset_x = 80
        center_offset_y = -40
        polygon_sides = 3
        polygon_radius = 240
        polygon_rotation = -90
        polygon_rotation_speed = 12
        gap = 2
        framerate = 75
        color_mode = "gradient"
        color_rgba = "rgba(255, 255, 255, 0.5)"
        color2_rgba = "rgba(255, 0, 0, 1.0)"
        theme = "catppuccin-mocha"
        theme_opacity = 0.8
        pipewire_attack = 0.2
        pipewire_decay = 0.9
        pipewire_gain = 1.5
        pipewire_curve = 0.8
        pipewire_neighbor_mix = 0.3

        [daemon]
        enabled = true
        poll_interval_ms = 50
        activity_threshold = 0.045
        activate_delay_ms = 120
        deactivate_delay_ms = 1800
        stop_on_silence = false
        notify_on_error = true
        notify_cooldown_seconds = 30
        overlay_command = "cargo"
        overlay_args = ["run", "-p", "kwybars-overlay"]
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
        assert_eq!(parsed.overlay.monitor_mode, OverlayMonitorMode::List);
        assert_eq!(parsed.overlay.monitors, vec!["DP-1", "HDMI-A-1"]);
        assert_eq!(parsed.visualizer.backend, VisualizerBackend::Dummy);
        assert_eq!(parsed.visualizer.layout, VisualizerLayout::Polygon);
        assert_eq!(
            parsed.visualizer.frame_edges,
            vec![OverlayPosition::Top, OverlayPosition::Bottom]
        );
        assert_eq!(parsed.visualizer.frame_mirror_mode, FrameMirrorMode::Pairs);
        assert_eq!(parsed.visualizer.bars, 64);
        assert_eq!(parsed.visualizer.bar_width, 5);
        assert!((parsed.visualizer.bar_corner_radius - 3.5).abs() < 1e-5);
        assert!(parsed.visualizer.segmented_bars);
        assert_eq!(parsed.visualizer.segment_length, 9);
        assert_eq!(parsed.visualizer.segment_gap, 4);
        assert_eq!(parsed.visualizer.radial_inner_radius, 160);
        assert!((parsed.visualizer.radial_start_angle - (-180.0)).abs() < 1e-5);
        assert!((parsed.visualizer.radial_arc_degrees - 180.0).abs() < 1e-5);
        assert!((parsed.visualizer.radial_rotation_speed - 24.0).abs() < 1e-5);
        assert!((parsed.visualizer.center_offset_x - 80.0).abs() < 1e-5);
        assert!((parsed.visualizer.center_offset_y - (-40.0)).abs() < 1e-5);
        assert_eq!(parsed.visualizer.polygon_sides, 3);
        assert_eq!(parsed.visualizer.polygon_radius, 240);
        assert!((parsed.visualizer.polygon_rotation - (-90.0)).abs() < 1e-5);
        assert!((parsed.visualizer.polygon_rotation_speed - 12.0).abs() < 1e-5);
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
        assert_eq!(parsed.visualizer.theme.as_deref(), Some("catppuccin-mocha"));
        assert!((parsed.visualizer.theme_opacity - 0.8).abs() < 1e-5);
        assert_eq!(parsed.visualizer.pipewire_attack, 0.2);
        assert_eq!(parsed.visualizer.pipewire_decay, 0.9);
        assert_eq!(parsed.visualizer.pipewire_gain, 1.5);
        assert_eq!(parsed.visualizer.pipewire_curve, 0.8);
        assert_eq!(parsed.visualizer.pipewire_neighbor_mix, 0.3);
        assert_eq!(
            parsed.daemon,
            DaemonConfig {
                enabled: true,
                poll_interval_ms: 50,
                activity_threshold: 0.045,
                activate_delay_ms: 120,
                deactivate_delay_ms: 1800,
                stop_on_silence: false,
                notify_on_error: true,
                notify_cooldown_seconds: 30,
                overlay_command: "cargo".to_owned(),
                overlay_args: vec![
                    "run".to_owned(),
                    "-p".to_owned(),
                    "kwybars-overlay".to_owned()
                ],
            }
        );
    }

    #[test]
    fn returns_default_for_empty_config() {
        let parsed = match parse_config("") {
            Ok(value) => value,
            Err(err) => panic!("empty config should parse, got error: {err}"),
        };
        assert_eq!(parsed, AppConfig::default());
    }

    #[test]
    fn built_in_defaults_match_expected_no_config_setup() {
        let config = AppConfig::default();

        assert_eq!(config.overlay.monitor_mode, OverlayMonitorMode::Primary);
        assert_eq!(config.overlay.layer, OverlayLayer::Background);
        assert_eq!(config.overlay.position, OverlayPosition::Bottom);
        assert!(config.overlay.full_length);
        assert_eq!(config.overlay.height, 500);
        assert_eq!(config.overlay.anchor_margin, 20);
        assert_eq!(config.overlay.margin_left, 20);
        assert_eq!(config.overlay.margin_right, 20);

        assert_eq!(config.visualizer.backend, VisualizerBackend::Cava);
        assert_eq!(config.visualizer.layout, VisualizerLayout::Line);
        assert_eq!(
            config.visualizer.frame_edges,
            vec![OverlayPosition::Top, OverlayPosition::Bottom]
        );
        assert_eq!(config.visualizer.frame_mirror_mode, FrameMirrorMode::Pairs);
        assert!((config.visualizer.bar_corner_radius - 20.0).abs() < 1e-5);
        assert_eq!(config.visualizer.bars, 50);
        assert_eq!(config.visualizer.bar_width, 8);
        assert_eq!(config.visualizer.radial_inner_radius, 180);
        assert!((config.visualizer.radial_start_angle - (-90.0)).abs() < 1e-5);
        assert!((config.visualizer.radial_arc_degrees - 360.0).abs() < 1e-5);
        assert!(config.visualizer.radial_rotation_speed.abs() < 1e-5);
        assert!(config.visualizer.center_offset_x.abs() < 1e-5);
        assert!(config.visualizer.center_offset_y.abs() < 1e-5);
        assert_eq!(config.visualizer.polygon_sides, 3);
        assert_eq!(config.visualizer.polygon_radius, 220);
        assert!((config.visualizer.polygon_rotation - (-90.0)).abs() < 1e-5);
        assert!(config.visualizer.polygon_rotation_speed.abs() < 1e-5);
        assert_eq!(config.visualizer.gap, 20);
        assert_eq!(config.visualizer.framerate, 60);
        assert_eq!(config.visualizer.color_mode, VisualizerColorMode::Gradient);
        assert!((config.visualizer.color_rgba.r - (175.0 / 255.0)).abs() < 1e-5);
        assert!((config.visualizer.color_rgba.g - (198.0 / 255.0)).abs() < 1e-5);
        assert!((config.visualizer.color_rgba.b - 1.0).abs() < 1e-5);
        assert!((config.visualizer.color_rgba.a - 0.7).abs() < 1e-5);
        assert!((config.visualizer.color2_rgba.r - (191.0 / 255.0)).abs() < 1e-5);
        assert!((config.visualizer.color2_rgba.g - (198.0 / 255.0)).abs() < 1e-5);
        assert!((config.visualizer.color2_rgba.b - (220.0 / 255.0)).abs() < 1e-5);
        assert!((config.visualizer.color2_rgba.a - 0.7).abs() < 1e-5);

        assert!(config.daemon.enabled);
        assert_eq!(config.daemon.poll_interval_ms, 90);
        assert!((config.daemon.activity_threshold - 0.035).abs() < 1e-5);
        assert_eq!(config.daemon.activate_delay_ms, 180);
        assert_eq!(config.daemon.deactivate_delay_ms, 2200);
        assert!(config.daemon.stop_on_silence);
        assert!(config.daemon.notify_on_error);
        assert_eq!(config.daemon.notify_cooldown_seconds, 45);
        assert_eq!(config.daemon.overlay_command, "kwybars-overlay");
        assert!(config.daemon.overlay_args.is_empty());
    }

    #[test]
    fn parses_colors_override_file() {
        let raw = r#"
        [colors]
        color_rgba = "rgba(10, 20, 30, 0.8)"
        color2_rgba = "rgba(100, 110, 120, 0.6)"
        "#;

        let parsed = match parse_color_overrides(raw) {
            Ok(value) => value,
            Err(err) => panic!("colors override should parse, got error: {err}"),
        };

        let Some(color1) = parsed.color_rgba else {
            panic!("missing color_rgba override");
        };
        assert!((color1.r - (10.0 / 255.0)).abs() < 1e-5);
        assert!((color1.g - (20.0 / 255.0)).abs() < 1e-5);
        assert!((color1.b - (30.0 / 255.0)).abs() < 1e-5);
        assert!((color1.a - 0.8).abs() < 1e-5);

        let Some(color2) = parsed.color2_rgba else {
            panic!("missing color2_rgba override");
        };
        assert!((color2.r - (100.0 / 255.0)).abs() < 1e-5);
        assert!((color2.g - (110.0 / 255.0)).abs() < 1e-5);
        assert!((color2.b - (120.0 / 255.0)).abs() < 1e-5);
        assert!((color2.a - 0.6).abs() < 1e-5);
    }

    #[test]
    fn parses_legacy_frame_mirror_alias() {
        let raw = r#"
        [visualizer]
        layout = "frame"
        frame_mirror = true
        "#;

        let parsed = match parse_config(raw) {
            Ok(value) => value,
            Err(err) => panic!("legacy frame_mirror alias should parse, got error: {err}"),
        };

        assert_eq!(parsed.visualizer.layout, VisualizerLayout::Frame);
        assert_eq!(parsed.visualizer.frame_mirror_mode, FrameMirrorMode::All);
    }

    #[test]
    fn display_tokens_round_trip_with_enum_parsers() {
        macro_rules! assert_round_trip {
            ($enum_type:ident [$($variant:ident),+ $(,)?]) => {
                $(
                    let value = $enum_type::$variant;
                    let token = value.to_string();
                    let reparsed = $enum_type::parse(&token)
                        .unwrap_or_else(|err| panic!(
                            "{} token `{}` should parse: {err}",
                            stringify!($enum_type),
                            token
                        ));
                    assert_eq!(reparsed, value);
                )+
            };
        }

        assert_round_trip!(OverlayPosition [Bottom, Top, Left, Right]);
        assert_round_trip!(OverlayLayer [Background, Bottom, Top]);
        assert_round_trip!(HorizontalAlignment [Left, Center, Right]);
        assert_round_trip!(VerticalAlignment [Top, Center, Bottom]);
        assert_round_trip!(OverlayMonitorMode [Primary, All, List]);
        assert_round_trip!(VisualizerBackend [Auto, Pipewire, Cava, Dummy]);
        assert_round_trip!(VisualizerColorMode [Solid, Gradient]);
        assert_round_trip!(VisualizerLayout [Line, Frame, Radial, Polygon, Particle, Floating]);
        assert_round_trip!(FrameMirrorMode [Off, All, Pairs]);
    }

    #[test]
    fn parses_colors_override_with_inline_comment_and_separator() {
        let raw = r#"
        [visualizer]
        color_rgba = "rgba(202, 122, 99, 0.9)" # matugen
        color2_rgba = "rgba(150, 100, 255, 0.7)";
        "#;

        let parsed = match parse_color_overrides(raw) {
            Ok(value) => value,
            Err(err) => panic!("colors override should parse, got error: {err}"),
        };

        let Some(color1) = parsed.color_rgba else {
            panic!("missing color_rgba override");
        };
        assert!((color1.r - (202.0 / 255.0)).abs() < 1e-5);
        assert!((color1.g - (122.0 / 255.0)).abs() < 1e-5);
        assert!((color1.b - (99.0 / 255.0)).abs() < 1e-5);
        assert!((color1.a - 0.9).abs() < 1e-5);

        let Some(color2) = parsed.color2_rgba else {
            panic!("missing color2_rgba override");
        };
        assert!((color2.r - (150.0 / 255.0)).abs() < 1e-5);
        assert!((color2.g - (100.0 / 255.0)).abs() < 1e-5);
        assert!((color2.b - 1.0).abs() < 1e-5);
        assert!((color2.a - 0.7).abs() < 1e-5);
    }

    #[test]
    fn applies_color_overrides_with_precedence() {
        let mut config = AppConfig::default();
        let original_color2 = config.visualizer.color2_rgba;

        let overrides = VisualizerColorOverrides {
            color_rgba: Some(super::RgbaColor {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 0.75,
            }),
            color2_rgba: None,
        };
        apply_color_overrides(&mut config, overrides);

        assert!((config.visualizer.color_rgba.r - 1.0).abs() < 1e-5);
        assert!(config.visualizer.color_rgba.g.abs() < 1e-5);
        assert!(config.visualizer.color_rgba.b.abs() < 1e-5);
        assert!((config.visualizer.color_rgba.a - 0.75).abs() < 1e-5);
        assert_eq!(config.visualizer.color2_rgba, original_color2);
    }

    #[test]
    fn parses_root_theme_keys() {
        let raw = r#"
        theme = "catppuccin-mocha"
        theme_opacity = 0.7

        [overlay]
        position = "bottom"
        "#;

        let parsed = match parse_config(raw) {
            Ok(value) => value,
            Err(err) => panic!("root theme keys should parse, got error: {err}"),
        };

        assert_eq!(parsed.visualizer.theme.as_deref(), Some("catppuccin-mocha"));
        assert!((parsed.visualizer.theme_opacity - 0.7).abs() < 1e-5);
    }
}
