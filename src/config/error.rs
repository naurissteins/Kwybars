//! config loading errors

use std::io;
use std::path::PathBuf;

/// a config file that could not be loaded
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("could not read {}: {source}", .path.display())]
    Read { path: PathBuf, source: io::Error },
    #[error("{}: {source}", .path.display())]
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
    #[error("{}: {message}", .path.display())]
    Invalid { path: PathBuf, message: String },
}
