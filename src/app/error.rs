//! top-level error type for running the overlay

use crate::audio::capture::CaptureError;
use crate::config::{ConfigError, ConfigPathError};

/// failure that stops kwybars
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    ConfigPath(#[from] ConfigPathError),
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Capture(#[from] CaptureError),
    #[error("event loop: {0}")]
    EventLoop(#[from] calloop::Error),
}
