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
        loaded.problems[0].contains("colors.toml: line 1: color_rgba"),
        "{:?}",
        loaded.problems
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
    assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    assert_eq!(
        loaded.problems,
        vec!["theme `nope` not found, using the configured colors"]
    );
}

#[test]
fn the_overlay_image_is_found_beside_the_config_or_under_home() {
    use crate::config::ImageOverlayConfig;
    use std::path::{Path, PathBuf};
    let image = |enabled: bool, path: &str| ImageOverlayConfig {
        enabled,
        path: Some(path.to_owned()),
        ..ImageOverlayConfig::default()
    };
    let env = fake_env(&[("HOME", "/home/u")]);
    let config = Path::new("/cfg/kwybars/current.toml");
    let file = |image: ImageOverlayConfig| image.file(config, &env);
    assert_eq!(
        file(image(true, "overlays/a.png")),
        Some(PathBuf::from("/cfg/kwybars/overlays/a.png"))
    );
    assert_eq!(
        file(image(true, "/abs/a.png")),
        Some(PathBuf::from("/abs/a.png"))
    );
    assert_eq!(
        file(image(true, "~/pics/a.png")),
        Some(PathBuf::from("/home/u/pics/a.png"))
    );
    assert_eq!(file(image(false, "/abs/a.png")), None);
    assert_eq!(
        file(ImageOverlayConfig {
            enabled: true,
            ..ImageOverlayConfig::default()
        }),
        None
    );
}

#[test]
fn the_overlay_image_is_decoded_and_a_broken_one_is_a_warning() {
    let dir = TempDir::new("load-image");
    let good = dir.path().join("art.png");
    let saved = image::RgbaImage::from_pixel(4, 3, image::Rgba([1, 2, 3, 255])).save(&good);
    assert!(saved.is_ok(), "{saved:?}");
    let path = dir.write(
        "config.toml",
        "[image_overlay]\nenabled = true\npath = \"art.png\"\n",
    );
    let loaded = load_ok(&path);
    let Some(image) = loaded.image else {
        panic!("the image is loaded");
    };
    assert_eq!(image.path, good);
    assert_eq!(image.source.map(|source| source.size()).ok(), Some((4, 3)));
    assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    assert!(loaded.problems.is_empty(), "{:?}", loaded.problems);

    dir.write("art.png", "not an image");
    let loaded = load_ok(&path);
    assert!(loaded.image.is_some_and(|image| image.source.is_err()));
    assert_eq!(loaded.problems.len(), 1, "{:?}", loaded.problems);
    assert!(loaded.problems[0].starts_with(&format!("image overlay {}: ", good.display())));

    let off = dir.write(
        "off.toml",
        "[image_overlay]\nenabled = false\npath = \"art.png\"\n",
    );
    assert!(load_ok(&off).image.is_none());
}
