//! reading the config file with its `colors.toml` and theme

use std::ffi::OsString;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use super::Config;
use super::colors;
use super::error::ConfigError;
use super::parse::{ParseError, parse};
use super::theme::{self, LoadedTheme};

/// where a loaded config came from
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    File,
    /// the file does not exist, built-in defaults are used
    Defaults,
}

/// result of [`load`]
#[derive(Debug)]
pub struct Loaded {
    pub config: Config,
    pub theme: Option<LoadedTheme>,
    pub warnings: Vec<String>,
    pub source: Source,
    pub colors_path: Option<PathBuf>,
}

/// reads the config, then `colors.toml` and the theme; only a broken config
/// file is an error, a broken colors or theme file is a warning
pub fn load(path: &Path, env: &dyn Fn(&str) -> Option<OsString>) -> Result<Loaded, ConfigError> {
    let (mut config, source, mut warnings) = read_config(path)?;
    let canonical = fs::canonicalize(path).ok();
    let canonical = canonical.as_deref().filter(|real| *real != path);

    let colors_path = apply_colors(&mut config, path, canonical, &mut warnings);

    let theme = config.visualizer.theme.as_deref().and_then(|name| {
        let dirs = theme::search_dirs(path, canonical, env);
        theme::load(name, &dirs)
            .inspect_err(|err| warnings.push(format!("{err}, using the configured colors")))
            .ok()
    });

    Ok(Loaded {
        config,
        theme,
        warnings,
        source,
        colors_path,
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

/// applies the first `colors.toml` found, returning its path
fn apply_colors(
    config: &mut Config,
    path: &Path,
    canonical: Option<&Path>,
    warnings: &mut Vec<String>,
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
            warnings.push(format!(
                "{}: {err}, using the config colors",
                colors_path.display()
            ));
            None
        }
    }
}
