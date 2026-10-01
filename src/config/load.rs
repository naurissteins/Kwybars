//! reading the config file with its `colors.toml` and theme

use std::ffi::OsString;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::Config;
use super::colors;
use super::error::ConfigError;
use super::parse::{ParseError, parse};
use super::theme::{self, AvailableTheme, LoadedTheme};
use crate::render::image;

/// where a loaded config came from
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    File,
    Defaults,
}

/// result of [`load`]
#[derive(Debug)]
pub struct Loaded {
    pub config: Config,
    pub theme: Option<LoadedTheme>,
    pub warnings: Vec<String>,
    pub problems: Vec<String>,
    pub source: Source,
    pub colors_path: Option<PathBuf>,
    pub image: Option<LoadedImage>,
}

/// the image overlay's file and its pixels, or why it cannot be shown
#[derive(Debug, Clone)]
pub struct LoadedImage {
    pub path: PathBuf,
    pub source: Result<Arc<image::Source>, String>,
}

impl Loaded {
    /// warnings, then problems
    pub fn messages(&self) -> impl Iterator<Item = &String> {
        self.warnings.iter().chain(&self.problems)
    }
}

/// the file a symlinked config points at
pub fn link_target(path: &Path) -> Option<PathBuf> {
    fs::canonicalize(path).ok().filter(|real| real != path)
}

/// where `colors.toml` is looked for, in order
pub fn colors_candidates(path: &Path) -> Vec<PathBuf> {
    colors::candidates(path, link_target(path).as_deref())
}

/// every theme the config at path can name
pub fn themes(path: &Path, env: &dyn Fn(&str) -> Option<OsString>) -> Vec<AvailableTheme> {
    theme::available(&theme::search_dirs(path, link_target(path).as_deref(), env))
}

pub fn load(path: &Path, env: &dyn Fn(&str) -> Option<OsString>) -> Result<Loaded, ConfigError> {
    let (mut config, source, warnings) = read_config(path)?;
    let mut problems = Vec::new();
    let canonical = link_target(path);
    let canonical = canonical.as_deref();

    let colors_path = apply_colors(&mut config, path, canonical, &mut problems);

    let theme = config.visualizer.theme.as_deref().and_then(|name| {
        let dirs = theme::search_dirs(path, canonical, env);
        theme::load(name, &dirs)
            .inspect_err(|err| problems.push(format!("{err}, using the configured colors")))
            .ok()
    });

    let image = config.image_overlay.file(path, env).map(|file| {
        let source = image::Source::open(&file)
            .map(Arc::new)
            .map_err(|err| err.to_string());
        if let Err(err) = &source {
            problems.push(format!("image overlay {}: {err}", file.display()));
        }
        LoadedImage { path: file, source }
    });

    Ok(Loaded {
        config,
        theme,
        warnings,
        problems,
        source,
        colors_path,
        image,
    })
}

fn read_config(path: &Path) -> Result<(Config, Source, Vec<String>), ConfigError> {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == ErrorKind::NotFound => {
            return Ok((Config::default(), Source::Defaults, Vec::new()));
        }
        Err(source) => {
            return Err(ConfigError::Read {
                path: path.to_owned(),
                source,
            });
        }
    };
    let parsed = parse(&raw).map_err(|err| match err {
        ParseError::Toml(source) => ConfigError::Parse {
            path: path.to_owned(),
            source,
        },
        ParseError::Invalid(message) => ConfigError::Invalid {
            path: path.to_owned(),
            message,
        },
    })?;
    let warnings = parsed
        .warnings
        .into_iter()
        .map(|warning| format!("{}: {warning}", path.display()))
        .collect();
    Ok((parsed.config, Source::File, warnings))
}

/// applies the first colors.toml found, returning its path
fn apply_colors(
    config: &mut Config,
    path: &Path,
    canonical: Option<&Path>,
    problems: &mut Vec<String>,
) -> Option<PathBuf> {
    let colors_path = colors::candidates(path, canonical)
        .into_iter()
        .find(|candidate| candidate.is_file())?;
    let overrides = fs::read_to_string(&colors_path)
        .map_err(|err| err.to_string())
        .and_then(|raw| colors::parse(&raw));
    match overrides {
        Ok(overrides) => {
            overrides.apply_to(&mut config.visualizer);
            Some(colors_path)
        }
        Err(err) => {
            problems.push(format!(
                "{}: {err}, using the config colors",
                colors_path.display()
            ));
            None
        }
    }
}
