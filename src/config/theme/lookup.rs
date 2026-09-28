//! directories searched for theme files

use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::xdg;

/// extra theme directories, colon separated, used by packages such as nix
const THEMES_DIR_ENV: &str = "KWYBARS_THEMES_DIR";
const SYSTEM_THEMES_DIR: &str = "/usr/share/kwybars/themes";

pub fn search_dirs(
    config_path: &Path,
    canonical: Option<&Path>,
    env: &dyn Fn(&str) -> Option<OsString>,
) -> Vec<PathBuf> {
    let beside_config = std::iter::once(config_path)
        .chain(canonical)
        .filter_map(Path::parent)
        .map(|dir| dir.join("themes"));
    let user = xdg::base_dir(env, "XDG_CONFIG_HOME", ".config")
        .map(|dir| dir.join("kwybars").join("themes"));
    let extra: Vec<PathBuf> = env(THEMES_DIR_ENV)
        .map(|value| {
            env::split_paths(&value)
                .filter(|p| !p.as_os_str().is_empty())
                .collect()
        })
        .unwrap_or_default();

    let mut dirs: Vec<PathBuf> = Vec::new();
    for dir in beside_config
        .chain(user)
        .chain(extra)
        .chain(std::iter::once(PathBuf::from(SYSTEM_THEMES_DIR)))
    {
        if !dirs.contains(&dir) {
            dirs.push(dir);
        }
    }
    dirs
}
