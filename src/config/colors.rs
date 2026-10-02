//! colors.toml: overrides for the two direct colors, usually written by matugen

use std::path::{Path, PathBuf};

use super::color::Rgba;
use super::loose;
use super::visualizer::VisualizerConfig;

/// file name looked up next to the config
pub const COLORS_FILE: &str = "colors.toml";

/// colors set in `colors.toml`
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ColorOverrides {
    pub color_rgba: Option<Rgba>,
    pub color2_rgba: Option<Rgba>,
}

impl ColorOverrides {
    /// replaces the base colors with every color that is set
    pub fn apply_to(&self, visualizer: &mut VisualizerConfig) {
        if let Some(color) = self.color_rgba {
            visualizer.color_rgba = color;
        }
        if let Some(color) = self.color2_rgba {
            visualizer.color2_rgba = color;
        }
    }
}

/// parses `colors.toml`; keys outside the root, `[visualizer]`, or `[colors]` are skipped
pub fn parse(raw: &str) -> Result<ColorOverrides, String> {
    let mut overrides = ColorOverrides::default();
    for entry in loose::entries(raw) {
        if !matches!(entry.section, None | Some("visualizer" | "colors")) {
            continue;
        }
        let slot = match entry.key {
            "color_rgba" => &mut overrides.color_rgba,
            "color2_rgba" => &mut overrides.color2_rgba,
            _ => continue,
        };
        let color = Rgba::parse(&entry.value)
            .map_err(|err| format!("line {}: {}: {err}", entry.line, entry.key))?;
        *slot = Some(color);
    }
    Ok(overrides)
}

/// candidate `colors.toml` paths: beside the config as given, then beside its
/// symlink target, so matugen's `~/.config/kwybars/colors.toml` works with a
/// switched config
pub fn candidates(config_path: &Path, canonical: Option<&Path>) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = Vec::with_capacity(2);
    for config in std::iter::once(config_path).chain(canonical) {
        let path = config
            .parent()
            .map_or_else(|| PathBuf::from(COLORS_FILE), |dir| dir.join(COLORS_FILE));
        if !paths.contains(&path) {
            paths.push(path);
        }
    }
    paths
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{ColorOverrides, candidates, parse};
    use crate::config::{Rgba, VisualizerConfig};

    #[test]
    fn parses_both_colors_from_any_supported_section() {
        let parsed = parse(
            "[colors]\ncolor_rgba = \"rgba(10, 20, 30, 0.8)\"\n[visualizer]\ncolor2_rgba = \"rgba(150, 100, 255, 0.7)\";\n",
        );
        assert_eq!(
            parsed,
            Ok(ColorOverrides {
                color_rgba: Some(Rgba::new(10.0 / 255.0, 20.0 / 255.0, 30.0 / 255.0, 0.8)),
                color2_rgba: Some(Rgba::new(150.0 / 255.0, 100.0 / 255.0, 1.0, 0.7)),
            })
        );
    }

    #[test]
    fn skips_other_sections_and_keys() {
        let parsed = parse("[overlay]\ncolor_rgba = \"nope\"\n[visualizer]\nbars = 3\n");
        assert_eq!(parsed, Ok(ColorOverrides::default()));
    }

    #[test]
    fn bad_color_names_line_and_key() {
        let err = parse("\ncolor_rgba = \"rgba(1, 2)\"\n")
            .err()
            .unwrap_or_default();
        assert!(
            err.starts_with("line 2: color_rgba: invalid color"),
            "{err}"
        );
    }

    #[test]
    fn only_set_colors_are_applied() {
        let mut visualizer = VisualizerConfig::default();
        let original = visualizer.color2_rgba;
        let red = Rgba::new(1.0, 0.0, 0.0, 0.75);
        ColorOverrides {
            color_rgba: Some(red),
            color2_rgba: None,
        }
        .apply_to(&mut visualizer);
        assert_eq!(
            (visualizer.color_rgba, visualizer.color2_rgba),
            (red, original)
        );
    }

    #[test]
    fn looks_beside_the_config_then_its_target() {
        let found = candidates(
            Path::new("/cfg/current.toml"),
            Some(Path::new("/cfg/custom/001.toml")),
        );
        assert_eq!(
            found,
            vec![
                PathBuf::from("/cfg/colors.toml"),
                PathBuf::from("/cfg/custom/colors.toml")
            ]
        );
        assert_eq!(
            candidates(
                Path::new("/cfg/config.toml"),
                Some(Path::new("/cfg/config.toml"))
            )
            .len(),
            1
        );
    }
}
