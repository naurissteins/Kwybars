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
pub struct OverlayConfig {
    pub position: OverlayPosition,
    pub anchor_margin: u32,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            position: OverlayPosition::Bottom,
            anchor_margin: 12,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisualizerConfig {
    pub bars: usize,
    pub bar_width: u32,
    pub gap: u32,
    pub framerate: u32,
}

impl Default for VisualizerConfig {
    fn default() -> Self {
        Self {
            bars: 48,
            bar_width: 6,
            gap: 3,
            framerate: 60,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
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
        "anchor_margin" => overlay.anchor_margin = parse_u32(key, value)?,
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
        "bars" => visualizer.bars = parse_usize(key, value)?,
        "bar_width" => visualizer.bar_width = parse_u32(key, value)?,
        "gap" => visualizer.gap = parse_u32(key, value)?,
        "framerate" => visualizer.framerate = parse_u32(key, value)?,
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

#[cfg(test)]
mod tests {
    use super::{AppConfig, OverlayPosition, parse_config};

    #[test]
    fn parses_valid_config() {
        let raw = r#"
        [overlay]
        position = "top"
        anchor_margin = 20

        [visualizer]
        bars = 64
        bar_width = 5
        gap = 2
        framerate = 75
        "#;

        let parsed = match parse_config(raw) {
            Ok(value) => value,
            Err(err) => panic!("valid config should parse, got error: {err}"),
        };
        assert_eq!(parsed.overlay.position, OverlayPosition::Top);
        assert_eq!(parsed.overlay.anchor_margin, 20);
        assert_eq!(parsed.visualizer.bars, 64);
        assert_eq!(parsed.visualizer.bar_width, 5);
        assert_eq!(parsed.visualizer.gap, 2);
        assert_eq!(parsed.visualizer.framerate, 75);
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
