//! the subcommands that change user files: `switch-config` and
//! `image-overlay match`

use std::io;
use std::path::PathBuf;

use crate::config::ConfigPathError;

mod atomic;
mod image;
mod switch;
#[cfg(test)]
mod tests;

pub use image::match_image;
pub use switch::switch;

/// why a file-changing subcommand left the files as they were
#[derive(Debug, thiserror::Error)]
pub enum RewriteError {
    #[error(transparent)]
    ConfigPath(#[from] ConfigPathError),
    #[error("could not {action} {}: {source}", path.display())]
    Io {
        action: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    #[error("target config does not exist: {}", .0.display())]
    TargetMissing(PathBuf),
    #[error("target config is not a file: {}", .0.display())]
    TargetNotFile(PathBuf),
    #[error(
        "{} is a regular file and its backup {} already holds something else; move one of them away first",
        active.display(),
        backup.display()
    )]
    BackupTaken { active: PathBuf, backup: PathBuf },
    #[error(
        "{} is a link to {}, outside {}; pass --config {} to change that file",
        link.display(),
        target.display(),
        dir.display(),
        target.display()
    )]
    LinkLeavesDir {
        link: PathBuf,
        target: PathBuf,
        dir: PathBuf,
    },
    #[error("overlay directory does not exist: {}", .0.display())]
    OverlayDirMissing(PathBuf),
    #[error("overlay path is not a directory: {}", .0.display())]
    OverlayDirNotDir(PathBuf),
    #[error("wallpaper path has no file name: {}", .0.display())]
    NoStem(PathBuf),
    #[error(
        "no overlay image named like {} in {} (tried: {})",
        wallpaper.display(),
        dir.display(),
        image::EXTENSIONS.join(", ")
    )]
    NoMatch { wallpaper: PathBuf, dir: PathBuf },
    #[error("overlay image path is not valid UTF-8: {}", .0.display())]
    NotUtf8(PathBuf),
    #[error("{}: {source}", path.display())]
    Parse {
        path: PathBuf,
        source: toml_edit::TomlError,
    },
    #[error("{}: image_overlay is not a table", .0.display())]
    NotATable(PathBuf),
}

impl RewriteError {
    /// wraps an io error with what was being done to which path
    fn io(action: &'static str, path: impl Into<PathBuf>) -> impl FnOnce(io::Error) -> Self {
        let path = path.into();
        move |source| Self::Io {
            action,
            path,
            source,
        }
    }
}
