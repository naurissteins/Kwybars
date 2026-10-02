//! reading the config file with its `colors.toml` and theme

use std::ffi::OsString;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::colors;
use super::error::ConfigError;
use super::parse::{ParseError, parse};
use super::theme::{self, AvailableTheme, LoadedTheme};
use super::{Config, ImageOverlayConfig};
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
    /// every file the configured theme is looked for in, found or not
    pub theme_candidates: Vec<PathBuf>,
    pub warnings: Vec<String>,
    pub problems: Vec<String>,
    pub source: Source,
    pub colors_path: Option<PathBuf>,
    pub image: Option<LoadedImage>,
    pub output_images: Vec<Option<LoadedImage>>,
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

    /// every image file in use, each once
    pub fn image_files(&self) -> Vec<PathBuf> {
        let mut files: Vec<PathBuf> = Vec::new();
        for image in self.image.iter().chain(self.output_images.iter().flatten()) {
            if !files.contains(&image.path) {
                files.push(image.path.clone());
            }
        }
        files
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

    let mut theme_candidates = Vec::new();
    let theme = config.visualizer.theme.as_deref().and_then(|name| {
        let dirs = theme::search_dirs(path, canonical, env);
        theme_candidates = theme::candidates(name, &dirs);
        theme::load(name, &dirs)
            .inspect_err(|err| problems.push(format!("{err}, using the configured colors")))
            .ok()
    });

    // each file is decoded once, however many outputs show it
    let mut opened: Vec<LoadedImage> = Vec::new();
    let mut open = |settings: &ImageOverlayConfig| {
        let file = settings.file(path, env)?;
        if let Some(known) = opened.iter().find(|image| image.path == file) {
            return Some(known.clone());
        }
        let source = image::Source::open(&file)
            .map(Arc::new)
            .map_err(|err| err.to_string());
        if let Err(err) = &source {
            problems.push(format!("image overlay {}: {err}", file.display()));
        }
        let image = LoadedImage { path: file, source };
        opened.push(image.clone());
        Some(image)
    };
    let image = open(&config.image_overlay);
    let output_images = config
        .overlay
        .outputs
        .iter()
        .map(|output| open(&config.image(Some(output))))
        .collect();

    Ok(Loaded {
        config,
        theme,
        theme_candidates,
        warnings,
        problems,
        source,
        colors_path,
        image,
        output_images,
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
