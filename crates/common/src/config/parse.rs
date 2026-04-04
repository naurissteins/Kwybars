use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use super::model::{
    AppConfig, ConfigLoadError, DaemonConfig, FrameMirrorMode, HorizontalAlignment, LineMode,
    MirrorOrientation, OverlayConfig, OverlayLayer, OverlayMonitorMode, OverlayPosition, RgbaColor,
    VerticalAlignment, VisualizerBackend, VisualizerColorMode, VisualizerColorOverrides,
    VisualizerConfig, VisualizerLayout,
};

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

pub(crate) fn parse_config(raw: &str) -> Result<AppConfig, ConfigLoadError> {
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

pub(crate) fn parse_color_overrides(
    raw: &str,
) -> Result<VisualizerColorOverrides, ConfigLoadError> {
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
        "line_mode" => visualizer.line_mode = LineMode::parse(value)?,
        "line_split_gap" => visualizer.line_split_gap = parse_u32(key, value)?,
        "mirror_orientation" => visualizer.mirror_orientation = MirrorOrientation::parse(value)?,
        "mirror_gap" => visualizer.mirror_gap = parse_u32(key, value)?,
        "wave_stroke_width" => visualizer.wave_stroke_width = parse_u32(key, value)?.max(1),
        "wave_fill" => visualizer.wave_fill = parse_bool(key, value)?,
        "wave_glow" => visualizer.wave_glow = parse_bool(key, value)?,
        "wave_smoothing" => visualizer.wave_smoothing = parse_f32(key, value)?.max(0.0),
        "wave_motion_smoothing" => {
            visualizer.wave_motion_smoothing = parse_f32(key, value)?.max(0.0);
        }
        "wave_amplitude" => visualizer.wave_amplitude = parse_f32(key, value)?.max(0.0),
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
