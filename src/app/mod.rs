//! startup, run, and shutdown of the overlay

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
    // Must run before any thread is spawned; see `logging::init`.
    let log = logging::init(&xdg::process_env);
    info!("kwybars {} starting", env!("CARGO_PKG_VERSION"));
    log.report();

    let config_path = match options.config_path {
        Some(path) => path,
        None => config::default_path(&xdg::process_env)?,
    };
    if config_path.is_file() {
        info!("config path: {} (found)", config_path.display());
    } else {
        info!(
            "config path: {} (not found, using built-in defaults)",
            config_path.display()
        );
    }

    warn!("the overlay is not implemented yet in this build, exiting");
    Ok(())
}
