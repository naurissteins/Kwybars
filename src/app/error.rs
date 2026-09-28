//! Top-level error type for running the overlay.

use crate::config::ConfigPathError;

/// A failure that stops Kwybars.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    ConfigPath(#[from] ConfigPathError),
}
