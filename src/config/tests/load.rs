use std::os::unix::fs::symlink;

use super::TempDir;
use crate::config::{Config, Loaded, Rgba, Source, ThemeOrigin, load};
use crate::xdg::fake_env;

const RED: &str = "color_rgba = \"rgba(255, 0, 0, 1)\"\n";
const GREEN: &str = "color_rgba = \"rgba(0, 255, 0, 1)\";\n";

fn load_ok(path: &std::path::Path) -> Loaded {
    load(path, &fake_env(&[])).unwrap_or_else(|err| panic!("{err}"))
}

#[test]
fn missing_config_uses_defaults() {
    let dir = TempDir::new("load-missing");
    let loaded = load_ok(&dir.path().join("config.toml"));
    assert_eq!(loaded.source, Source::Defaults);
    assert_eq!(loaded.config, Config::default());
    assert!(loaded.warnings.is_empty() && loaded.theme.is_none());
}

#[test]
fn config_warnings_name_the_file() {
    let dir = TempDir::new("load-warnings");
    let path = dir.write("config.toml", "[overlay]\ntypo = 1\n");
    let loaded = load_ok(&path);
    assert_eq!(
        loaded.warnings,
        vec![format!(
            "{}: overlay.typo: unknown key, ignored",
            path.display()
        )]
    );
}

#[test]
fn colors_toml_beside_the_config_is_applied() {
    let dir = TempDir::new("load-colors");
    let path = dir.write("config.toml", "[visualizer]\nbars = 20\n");
    let colors = dir.write("colors.toml", GREEN);
    let loaded = load_ok(&path);
    assert_eq!(
        loaded.config.visualizer.color_rgba,
        Rgba::new(0.0, 1.0, 0.0, 1.0)
    );
    assert_eq!(loaded.colors_path, Some(colors));
}

#[test]
fn broken_colors_toml_is_a_warning() {
    let dir = TempDir::new("load-bad-colors");
    let path = dir.write("config.toml", "");
    dir.write("colors.toml", "color_rgba = \"rgba(1, 2)\"\n");
    let loaded = load_ok(&path);
    assert_eq!(loaded.config.visualizer, Config::default().visualizer);
    assert_eq!(loaded.colors_path, None);
    assert!(
        loaded.warnings[0].contains("colors.toml: line 1: color_rgba"),
        "{:?}",
        loaded.warnings
    );
}

#[test]
fn switched_config_prefers_colors_beside_the_link() {
    let dir = TempDir::new("load-symlink");
    let target = dir.write("custom/001.toml", "");
    let link = dir.path().join("current.toml");
    if let Err(err) = symlink(&target, &link) {
        panic!("symlink: {err}");
    }

    dir.write("custom/colors.toml", GREEN);
    assert_eq!(
        load_ok(&link).config.visualizer.color_rgba,
        Rgba::new(0.0, 1.0, 0.0, 1.0)
    );

    dir.write("colors.toml", RED);
    assert_eq!(
        load_ok(&link).config.visualizer.color_rgba,
        Rgba::new(1.0, 0.0, 0.0, 1.0)
    );
}

#[test]
fn themes_load_from_the_config_dir_or_builtin() {
    let dir = TempDir::new("load-theme");
    let path = dir.write("config.toml", "theme = \"mine\"\n");
    let theme_path = dir.write(
        "themes/mine.toml",
        "red = \"#000000\"\ngreen = \"#000000\"\nyellow = \"#000000\"\nblue = \"#000000\"\nmagenta = \"#000000\"\ncyan = \"#000000\"\n",
    );
    let loaded = load_ok(&path);
    assert_eq!(
        loaded.theme.map(|theme| theme.origin),
        Some(ThemeOrigin::File(theme_path))
    );

    dir.write("config.toml", "theme = \"gruvbox\"\n");
    let loaded = load_ok(&path);
    assert_eq!(
        loaded.theme.map(|theme| theme.origin),
        Some(ThemeOrigin::BuiltIn)
    );
}

#[test]
fn missing_theme_is_a_warning() {
    let dir = TempDir::new("load-no-theme");
    let path = dir.write("config.toml", "theme = \"nope\"\n");
    let loaded = load_ok(&path);
    assert!(loaded.theme.is_none());
    assert_eq!(
        loaded.warnings,
        vec!["theme `nope` not found, using the configured colors"]
    );
}
