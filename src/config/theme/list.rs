//! the themes a config can name

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use super::{ThemeOrigin, builtin, is_valid_name};

/// a theme found in the search directories or built in
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailableTheme {
    pub name: String,
    pub origin: ThemeOrigin,
}

pub fn available(dirs: &[PathBuf]) -> Vec<AvailableTheme> {
    let mut themes: BTreeMap<String, ThemeOrigin> = BTreeMap::new();
    for dir in dirs {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        for path in entries.flatten().map(|entry| entry.path()) {
            let is_theme = path.extension().is_some_and(|ext| ext == "toml") && path.is_file();
            let name = path.file_stem().and_then(|stem| stem.to_str());
            if let Some(name) = name.filter(|name| is_theme && is_valid_name(name)) {
                themes
                    .entry(name.to_owned())
                    .or_insert(ThemeOrigin::File(path));
            }
        }
    }
    for name in builtin::names() {
        themes
            .entry(name.to_owned())
            .or_insert(ThemeOrigin::BuiltIn);
    }
    themes
        .into_iter()
        .map(|(name, origin)| AvailableTheme { name, origin })
        .collect()
}
