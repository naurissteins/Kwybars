//! the configs shipped in `assets/examples/`

use std::path::{Path, PathBuf};

use crate::config::{self, Config, Layout};
use crate::xdg::fake_env;

fn examples() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/examples");
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(err) => panic!("cannot read {}: {err}", dir.display()),
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
        .collect();
    files.sort();
    files
}

/// loads a shipped config, which must not warn about anything
fn load(path: &Path) -> Config {
    let loaded = match config::load(path, &fake_env(&[])) {
        Ok(loaded) => loaded,
        Err(err) => panic!("{err}"),
    };
    let messages: Vec<&String> = loaded.messages().collect();
    assert!(messages.is_empty(), "{}: {messages:?}", path.display());
    loaded.config
}

#[test]
fn the_example_config_is_the_built_in_defaults() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/examples/config.toml");
    assert_eq!(load(&path), Config::default());
}

#[test]
fn every_preset_loads_without_warnings_and_shows_its_layout() {
    let presets = [
        ("floating", Layout::Floating),
        ("frame", Layout::Frame),
        ("line", Layout::Line),
        ("mirror", Layout::Mirror),
        ("particle", Layout::Particle),
        ("polygon", Layout::Polygon),
        ("radial", Layout::Radial),
        ("wave", Layout::Wave),
    ];
    let files = examples();
    assert_eq!(files.len(), presets.len() + 1, "{files:?}");
    for (name, layout) in presets {
        let file = files
            .iter()
            .find(|path| path.file_stem().is_some_and(|stem| stem == name));
        let Some(file) = file else {
            panic!("no preset for the {name} layout");
        };
        assert_eq!(load(file).visualizer.layout, layout, "{name}");
    }
}
