use std::path::{Path, PathBuf};

use super::{Theme, ThemeError, ThemeOrigin, load, parse_hex, search_dirs};
use crate::config::Rgba;
use crate::config::tests::TempDir;
use crate::xdg::fake_env;

const NORD: &str = "name = \"nord\"\nred = \"#bf616a\"\ngreen = \"#a3be8c\"\nyellow = \"#ebcb8b\"\nblue = \"#81a1c1\"\nmagenta = \"#b48ead\"\ncyan = \"#88c0d0\"\n";

#[test]
fn parses_six_colors_in_order() {
    let theme = match Theme::parse(NORD, "fallback") {
        Ok(theme) => theme,
        Err(err) => panic!("{err}"),
    };
    assert_eq!(theme.name, "nord");
    assert_eq!(
        theme.colors[0],
        Rgba::new(191.0 / 255.0, 97.0 / 255.0, 106.0 / 255.0, 1.0)
    );
    assert_eq!(
        theme.colors[5],
        Rgba::new(136.0 / 255.0, 192.0 / 255.0, 208.0 / 255.0, 1.0)
    );
}

#[test]
fn missing_name_uses_the_file_name() {
    let raw = NORD.replace("name = \"nord\"\n", "");
    assert_eq!(
        Theme::parse(&raw, "mine").map(|theme| theme.name),
        Ok("mine".to_owned())
    );
}

#[test]
fn missing_or_bad_colors_are_errors() {
    let missing = NORD.replace("cyan = \"#88c0d0\"\n", "");
    assert_eq!(
        Theme::parse(&missing, "x").err().as_deref(),
        Some("missing color `cyan`")
    );
    let bad = NORD.replace("#a3be8c", "#a3be8");
    let err = Theme::parse(&bad, "x").err().unwrap_or_default();
    assert!(err.starts_with("line 3: green:"), "{err}");
}

#[test]
fn parses_hex_with_and_without_alpha() {
    assert_eq!(
        parse_hex("#ff000080"),
        Some(Rgba::new(1.0, 0.0, 0.0, 128.0 / 255.0))
    );
    assert_eq!(parse_hex("00ff00"), Some(Rgba::new(0.0, 1.0, 0.0, 1.0)));
    for bad in ["#fff", "#gg0000", "#ff00ff0", "#ééé"] {
        assert_eq!(parse_hex(bad), None, "{bad}");
    }
}

#[test]
fn opacity_scales_alpha() {
    let theme = Theme::parse(NORD, "nord").unwrap_or_else(|err| panic!("{err}"));
    assert!(
        theme
            .with_opacity(0.5)
            .iter()
            .all(|color| (color.a - 0.5).abs() < 1e-6)
    );
}

#[test]
fn file_themes_win_over_builtin_ones() {
    let dir = TempDir::new("theme-file");
    dir.write("nord.toml", &NORD.replace("#bf616a", "#000000"));
    let loaded = load("nord", &[dir.path().to_owned()]).unwrap_or_else(|err| panic!("{err}"));
    assert_eq!(loaded.theme.colors[0], Rgba::new(0.0, 0.0, 0.0, 1.0));
    assert_eq!(
        loaded.origin,
        ThemeOrigin::File(dir.path().join("nord.toml"))
    );
}

#[test]
fn falls_back_to_builtin_themes() {
    let loaded = load("dracula", &[]).unwrap_or_else(|err| panic!("{err}"));
    assert_eq!(loaded.theme.name, "dracula");
    assert_eq!(loaded.origin, ThemeOrigin::BuiltIn);
    assert!(matches!(
        load("no-such-theme", &[]),
        Err(ThemeError::NotFound(_))
    ));
}

#[test]
fn rejects_names_that_are_paths() {
    for name in ["", "../nord", "a/b", ".hidden"] {
        assert!(
            matches!(load(name, &[]), Err(ThemeError::InvalidName(_))),
            "{name:?}"
        );
    }
}

#[test]
fn search_order_is_config_dir_target_dir_user_env_system() {
    let env = fake_env(&[
        ("HOME", "/home/u"),
        ("KWYBARS_THEMES_DIR", "/nix/themes::/opt/themes"),
    ]);
    let dirs = search_dirs(
        Path::new("/home/u/.config/kwybars/current.toml"),
        Some(Path::new("/home/u/.config/kwybars/custom/001.toml")),
        &env,
    );
    let expected: Vec<PathBuf> = [
        "/home/u/.config/kwybars/themes",
        "/home/u/.config/kwybars/custom/themes",
        "/nix/themes",
        "/opt/themes",
        "/usr/share/kwybars/themes",
    ]
    .iter()
    .map(PathBuf::from)
    .collect();
    assert_eq!(dirs, expected);
}
