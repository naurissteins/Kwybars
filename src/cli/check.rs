//! the config check `validate-config` and `doctor` share

use std::ffi::OsString;
use std::path::PathBuf;

use super::report::Report;
use crate::app::RunOptions;
use crate::config::{self, ConfigPathError, Loaded, LoadedImage, Source, ThemeOrigin};

/// the config file a subcommand looks at
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigFile {
    pub path: PathBuf,
    pub named: bool,
}

impl ConfigFile {
    pub fn locate(
        options: &RunOptions,
        env: &dyn Fn(&str) -> Option<OsString>,
    ) -> Result<Self, ConfigPathError> {
        if let Some(path) = &options.config_path {
            return Ok(Self {
                path: path.clone(),
                named: true,
            });
        }
        Ok(Self {
            named: config::env_path(env).is_some(),
            path: config::default_path(env)?,
        })
    }
}

pub fn config(
    report: &mut Report,
    file: &ConfigFile,
    env: &dyn Fn(&str) -> Option<OsString>,
) -> Option<Loaded> {
    let path = &file.path;
    let loaded = match config::load(path, env) {
        Ok(loaded) => loaded,
        Err(err) => {
            report.error(err);
            return None;
        }
    };
    match loaded.source {
        Source::File => report.line(format!("config: {} (ok)", path.display())),
        Source::Defaults if file.named => {
            report.error(format!("config does not exist: {}", path.display()));
            return Some(loaded);
        }
        Source::Defaults => report.line(format!(
            "config: {} (not found, built-in defaults are used)",
            path.display()
        )),
    }
    // a relative path also differs from its canonical form
    if let Some(target) = config::link_target(path).filter(|_| path.is_symlink()) {
        report.line(format!("resolved config path: {}", target.display()));
    }
    for warning in &loaded.warnings {
        report.warning(warning);
    }
    colors(report, file, &loaded);
    theme(report, &loaded);
    image(report, &loaded);
    // each file that failed above has its reason here
    for problem in &loaded.problems {
        report.error(problem);
    }
    Some(loaded)
}

fn colors(report: &mut Report, file: &ConfigFile, loaded: &Loaded) {
    if let Some(path) = &loaded.colors_path {
        report.line(format!("colors: {} (ok)", path.display()));
        return;
    }
    let candidates = config::colors_candidates(&file.path);
    if !candidates.iter().any(|candidate| candidate.is_file())
        && let Some(first) = candidates.first()
    {
        report.line(format!("colors: {} (not found)", first.display()));
    }
}

fn theme(report: &mut Report, loaded: &Loaded) {
    match (&loaded.config.visualizer.theme, &loaded.theme) {
        (None, _) => report.line("theme: none"),
        (Some(_), Some(found)) => report.line(match &found.origin {
            ThemeOrigin::File(path) => format!("theme: {} ({})", found.theme.name, path.display()),
            ThemeOrigin::BuiltIn => format!("theme: {} (built-in)", found.theme.name),
        }),
        (Some(_), None) => {}
    }
}

fn image(report: &mut Report, loaded: &Loaded) {
    match &loaded.image {
        None if loaded.config.image_overlay.enabled => {
            report.error("image_overlay is enabled but image_overlay.path is empty");
        }
        None => report.line("image overlay: disabled"),
        Some(LoadedImage {
            path,
            source: Ok(source),
        }) => {
            let (width, height) = source.size();
            report.line(format!(
                "image overlay: {} (ok, {width}x{height})",
                path.display()
            ));
        }
        Some(_) => {}
    }
}
