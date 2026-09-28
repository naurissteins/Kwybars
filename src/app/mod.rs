//! startup, run, and shutdown of the overlay

pub mod debug;
mod error;
mod logging;

use std::path::PathBuf;

use tracing::{info, warn};

pub use error::AppError;

use crate::{config, xdg};

/// options for running the overlay
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RunOptions {
    /// config path from `--config`; `None` means the default location
    pub config_path: Option<PathBuf>,
}

/// runs the overlay until it exits
pub fn run(options: RunOptions) -> Result<(), AppError> {
    // must run before any thread is spawned, see `logging::init`
    let log = logging::init(&xdg::process_env);
    info!("kwybars {} starting", env!("CARGO_PKG_VERSION"));
    log.report();

    let loaded = load_config(&options)?;
    let visualizer = &loaded.config.visualizer;
    info!(
        "config: layout {:?}, {} bars at {} fps, {} output override(s)",
        visualizer.layout,
        visualizer.bars,
        visualizer.framerate,
        loaded.config.overlay.outputs.len()
    );

    warn!("the overlay is not implemented yet in this build, exiting");
    Ok(())
}

/// resolves and loads the config, logging where everything came from
fn load_config(options: &RunOptions) -> Result<config::Loaded, AppError> {
    let config_path = match &options.config_path {
        Some(path) => path.clone(),
        None => config::default_path(&xdg::process_env)?,
    };
    let loaded = config::load(&config_path, &xdg::process_env)?;
    match loaded.source {
        config::Source::File => info!("config path: {} (found)", config_path.display()),
        config::Source::Defaults => info!(
            "config path: {} (not found, using built-in defaults)",
            config_path.display()
        ),
    }
    for warning in &loaded.warnings {
        warn!("{warning}");
    }
    if let Some(path) = &loaded.colors_path {
        info!("colors: {}", path.display());
    }
    if let Some(loaded_theme) = &loaded.theme {
        match &loaded_theme.origin {
            config::ThemeOrigin::File(path) => info!("theme: {}", path.display()),
            config::ThemeOrigin::BuiltIn => {
                info!("theme: {} (built-in)", loaded_theme.theme.name);
            }
        }
    }
    Ok(loaded)
}
