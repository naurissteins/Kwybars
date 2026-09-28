//! theme palettes: six colors that replace the direct bar colors

mod builtin;
mod lookup;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub use lookup::search_dirs;

use super::color::Rgba;
use super::loose;

/// palette keys in rendering order
pub const THEME_KEYS: [&str; 6] = ["red", "green", "yellow", "blue", "magenta", "cyan"];

/// a parsed theme palette, alpha not yet scaled by `theme_opacity`
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    pub name: String,
    pub colors: [Rgba; 6],
}

/// where a theme was loaded from
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThemeOrigin {
    File(PathBuf),
    BuiltIn,
}

/// a theme plus where it came from
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedTheme {
    pub theme: Theme,
    pub origin: ThemeOrigin,
}

/// a theme that could not be used
#[derive(Debug, thiserror::Error)]
pub enum ThemeError {
    #[error("invalid theme name {0:?}")]
    InvalidName(String),
    #[error("theme `{0}` not found")]
    NotFound(String),
    #[error("could not read theme {}: {source}", .path.display())]
    Read { path: PathBuf, source: io::Error },
    /// `file` is the theme's path, or `built-in theme <name>`
    #[error("{file}: {message}")]
    Parse { file: String, message: String },
}

impl Theme {
    /// parses a theme file; `name` in the file wins over `fallback_name`
    pub fn parse(raw: &str, fallback_name: &str) -> Result<Self, String> {
        let mut name = None;
        let mut colors: [Option<Rgba>; 6] = [None; 6];
        for entry in loose::entries(raw) {
            if entry.key == "name" {
                name = Some(entry.value).filter(|value| !value.is_empty());
                continue;
            }
            let Some(index) = THEME_KEYS.iter().position(|key| *key == entry.key) else {
                continue;
            };
            let color = parse_hex(&entry.value).ok_or_else(|| {
                format!(
                    "line {}: {}: expected #rrggbb or #rrggbbaa, got {:?}",
                    entry.line, entry.key, entry.value
                )
            })?;
            if let Some(slot) = colors.get_mut(index) {
                *slot = Some(color);
            }
        }

        let mut palette = [Rgba::new(0.0, 0.0, 0.0, 0.0); 6];
        for ((slot, color), key) in palette.iter_mut().zip(colors).zip(THEME_KEYS) {
            *slot = color.ok_or_else(|| format!("missing color `{key}`"))?;
        }
        Ok(Self {
            name: name.unwrap_or_else(|| fallback_name.to_owned()),
            colors: palette,
        })
    }

    /// colors with alpha multiplied by `opacity`
    pub fn with_opacity(&self, opacity: f32) -> [Rgba; 6] {
        self.colors.map(|color| Rgba {
            a: (color.a * opacity).clamp(0.0, 1.0),
            ..color
        })
    }
}

/// finds `<name>.toml` in `dirs`, falling back to the built-in themes
pub fn load(name: &str, dirs: &[PathBuf]) -> Result<LoadedTheme, ThemeError> {
    if name.is_empty() || name.starts_with('.') || name.contains(['/', '\\']) {
        return Err(ThemeError::InvalidName(name.to_owned()));
    }
    let file_name = format!("{name}.toml");
    let parse_error = |file: String| move |message| ThemeError::Parse { file, message };

    if let Some(path) = dirs
        .iter()
        .map(|dir| dir.join(&file_name))
        .find(|path| path.is_file())
    {
        let raw = read(&path)?;
        let theme = Theme::parse(&raw, name).map_err(parse_error(path.display().to_string()))?;
        let origin = ThemeOrigin::File(path);
        return Ok(LoadedTheme { theme, origin });
    }
    let raw = builtin::find(name).ok_or_else(|| ThemeError::NotFound(name.to_owned()))?;
    let theme = Theme::parse(raw, name).map_err(parse_error(format!("built-in theme {name}")))?;
    Ok(LoadedTheme {
        theme,
        origin: ThemeOrigin::BuiltIn,
    })
}

fn read(path: &Path) -> Result<String, ThemeError> {
    fs::read_to_string(path).map_err(|source| ThemeError::Read {
        path: path.to_owned(),
        source,
    })
}

/// parses `#rrggbb` or `#rrggbbaa`, the `#` being optional
fn parse_hex(value: &str) -> Option<Rgba> {
    let hex = value.trim().trim_start_matches('#');
    if !matches!(hex.len(), 6 | 8) || !hex.is_ascii() {
        return None;
    }
    let channel = |index: usize| {
        hex.get(index..index + 2)
            .and_then(|pair| u8::from_str_radix(pair, 16).ok())
            .map(|byte| f32::from(byte) / 255.0)
    };
    let alpha = if hex.len() == 8 { channel(6)? } else { 1.0 };
    Some(Rgba::new(channel(0)?, channel(2)?, channel(4)?, alpha))
}

#[cfg(test)]
mod tests;
