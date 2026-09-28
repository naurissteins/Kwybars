//! top-level error type for running the overlay

use crate::config::{ConfigError, ConfigPathError};

/// failure that stops kwybars
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    ConfigPath(#[from] ConfigPathError),
    #[error(transparent)]
    Config(#[from] ConfigError),
}
