use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::RgbaColor;

const THEME_COLOR_KEYS: [&str; 6] = ["red", "green", "yellow", "blue", "magenta", "cyan"];

#[derive(Debug, Clone, PartialEq)]
pub struct ThemePalette {
    pub name: String,
    pub colors: Vec<RgbaColor>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ThemeSource {
    User,
    System,
    SourceCheckout,
}

impl ThemeSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::System => "system",
            Self::SourceCheckout => "source",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailableTheme {
    pub name: String,
    pub path: PathBuf,
    pub source: ThemeSource,
}

#[derive(Debug)]
pub enum ThemeLoadError {
    Io(std::io::Error),
    Parse(String),
}

impl Display for ThemeLoadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "io error: {err}"),
            Self::Parse(msg) => write!(f, "theme parse error: {msg}"),
        }
    }
}

impl std::error::Error for ThemeLoadError {}

pub fn resolve_theme_path(config_path: &Path, theme_name: &str) -> PathBuf {
    let theme_file = format!("{theme_name}.toml");

    let config_path_candidate = config_path
        .parent()
        .map(|parent| parent.join("themes").join(&theme_file))
        .unwrap_or_else(|| PathBuf::from("themes").join(&theme_file));
    if config_path_candidate.exists() {
        return config_path_candidate;
    }

    let system_path = PathBuf::from("/usr/share/kwybars/themes").join(&theme_file);
    if system_path.exists() {
        return system_path;
    }

    let source_checkout_path = source_checkout_themes_dir().join(&theme_file);
    if source_checkout_path.exists() {
        return source_checkout_path;
    }

    config_path_candidate
}

pub fn load_theme_palette(
    path: &Path,
    theme_name: &str,
    opacity: f32,
) -> Result<ThemePalette, ThemeLoadError> {
    let raw = fs::read_to_string(path).map_err(ThemeLoadError::Io)?;
    parse_theme_palette(&raw, theme_name, opacity)
}

pub fn list_available_themes(config_path: &Path) -> Vec<AvailableTheme> {
    let mut themes = BTreeMap::<String, AvailableTheme>::new();

    let user_dir = config_path
        .parent()
        .map(|parent| parent.join("themes"))
        .unwrap_or_else(|| PathBuf::from("themes"));
    collect_theme_dir(&user_dir, ThemeSource::User, &mut themes);
    collect_theme_dir(
        &PathBuf::from("/usr/share/kwybars/themes"),
        ThemeSource::System,
        &mut themes,
    );
    let source_dir = source_checkout_themes_dir();
    collect_theme_dir(&source_dir, ThemeSource::SourceCheckout, &mut themes);

    themes.into_values().collect()
}

fn parse_theme_palette(
    raw: &str,
    fallback_name: &str,
    opacity: f32,
) -> Result<ThemePalette, ThemeLoadError> {
    let mut parsed_name: Option<String> = None;
    let mut colors = std::collections::HashMap::<String, RgbaColor>::new();
    let alpha_mul = opacity.clamp(0.0, 1.0);

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            continue;
        }

        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };

        let key = key.trim();
        let value = normalize_value(value);
        if key == "name" {
            if !value.is_empty() {
                parsed_name = Some(value);
            }
            continue;
        }

        if !THEME_COLOR_KEYS.contains(&key) {
            continue;
        }

        let mut color = parse_hex_color(&value).map_err(|msg| {
            ThemeLoadError::Parse(format!("invalid color for key `{key}`: {msg}"))
        })?;
        color.a = (color.a * alpha_mul).clamp(0.0, 1.0);
        colors.insert(key.to_owned(), color);
    }

    let mut ordered = Vec::with_capacity(THEME_COLOR_KEYS.len());
    for key in THEME_COLOR_KEYS {
        let Some(color) = colors.get(key) else {
            return Err(ThemeLoadError::Parse(format!(
                "missing required color key: {key}"
            )));
        };
        ordered.push(*color);
    }

    Ok(ThemePalette {
        name: parsed_name.unwrap_or_else(|| fallback_name.to_owned()),
        colors: ordered,
    })
}

fn parse_hex_color(value: &str) -> Result<RgbaColor, String> {
    let hex = value.trim().trim_start_matches('#');
    match hex.len() {
        6 => {
            let r = parse_hex_byte(&hex[0..2])?;
            let g = parse_hex_byte(&hex[2..4])?;
            let b = parse_hex_byte(&hex[4..6])?;
            Ok(RgbaColor {
                r: f32::from(r) / 255.0,
                g: f32::from(g) / 255.0,
                b: f32::from(b) / 255.0,
                a: 1.0,
            })
        }
        8 => {
            let r = parse_hex_byte(&hex[0..2])?;
            let g = parse_hex_byte(&hex[2..4])?;
            let b = parse_hex_byte(&hex[4..6])?;
            let a = parse_hex_byte(&hex[6..8])?;
            Ok(RgbaColor {
                r: f32::from(r) / 255.0,
                g: f32::from(g) / 255.0,
                b: f32::from(b) / 255.0,
                a: f32::from(a) / 255.0,
            })
        }
        _ => Err(format!("expected 6 or 8 hex digits, got `{value}`")),
    }
}

fn collect_theme_dir(
    dir: &Path,
    source: ThemeSource,
    themes: &mut BTreeMap<String, AvailableTheme>,
) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("toml") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        let name = stem.trim().to_owned();
        if name.is_empty() {
            continue;
        }

        themes
            .entry(name.clone())
            .or_insert(AvailableTheme { name, path, source });
    }
}

fn source_checkout_themes_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("assets/themes")
}

fn parse_hex_byte(value: &str) -> Result<u8, String> {
    u8::from_str_radix(value, 16).map_err(|_| format!("invalid hex byte `{value}`"))
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
    use super::{ThemeSource, list_available_themes, parse_theme_palette};
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn parses_six_color_theme() {
        let raw = r##"
name = "catppuccin-mocha"
red = "#f38ba8"
green = "#a6e3a1"
yellow = "#f9e2af"
blue = "#89b4fa"
magenta = "#f5c2e7"
cyan = "#94e2d5"
"##;

        let palette = match parse_theme_palette(raw, "fallback", 0.8) {
            Ok(value) => value,
            Err(err) => panic!("theme should parse: {err}"),
        };

        assert_eq!(palette.name, "catppuccin-mocha");
        assert_eq!(palette.colors.len(), 6);
        assert!((palette.colors[0].a - 0.8).abs() < 1e-5);
    }

    #[test]
    fn lists_user_themes_from_config_directory() {
        let base = PathBuf::from(format!("/tmp/kwybars-theme-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let themes_dir = base.join("themes");
        fs::create_dir_all(&themes_dir).unwrap_or_else(|err| {
            panic!("failed to create theme dir: {err}");
        });
        fs::write(themes_dir.join("custom.toml"), "name = \"custom\"\n").unwrap_or_else(|err| {
            panic!("failed to write theme file: {err}");
        });

        let listed = list_available_themes(&base.join("config.toml"));
        let found = listed.iter().find(|theme| theme.name == "custom");
        assert!(found.is_some(), "expected custom theme to be listed");
        assert_eq!(found.map(|theme| theme.source), Some(ThemeSource::User));

        let _ = fs::remove_dir_all(&base);
    }
}
