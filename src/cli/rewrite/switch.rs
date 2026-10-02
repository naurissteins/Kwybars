//! `kwybars switch-config`

use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::{RewriteError, atomic};
use crate::app::RunOptions;
use crate::cli::check::ConfigFile;

/// makes the config path a link to `target` and says what happened
pub fn switch(
    options: &RunOptions,
    target: &Path,
    env: &dyn Fn(&str) -> Option<OsString>,
) -> Result<String, RewriteError> {
    let active = ConfigFile::locate(options, env)?.path;
    let target = existing_file(target)?;
    if fs::canonicalize(&active).is_ok_and(|current| current == target) {
        return Ok(format!(
            "active config already points to {}",
            target.display()
        ));
    }

    let backup = back_up(&active)?;
    atomic::link(&target, &active)?;

    let mut message = format!(
        "switched active config {} -> {}",
        active.display(),
        target.display()
    );
    if let Some(backup) = backup {
        message.push_str(&format!("\nprevious config kept as {}", backup.display()));
    }
    Ok(message)
}

/// the absolute path the link will hold
fn existing_file(target: &Path) -> Result<PathBuf, RewriteError> {
    let canonical = fs::canonicalize(target).map_err(|err| match err.kind() {
        io::ErrorKind::NotFound => RewriteError::TargetMissing(target.to_owned()),
        _ => RewriteError::io("read", target)(err),
    })?;
    if canonical.is_file() {
        Ok(canonical)
    } else {
        Err(RewriteError::TargetNotFile(canonical))
    }
}

/// copies a regular file at `active` to `<name>.bak` before a link replaces it
fn back_up(active: &Path) -> Result<Option<PathBuf>, RewriteError> {
    if !fs::symlink_metadata(active).is_ok_and(|metadata| metadata.is_file()) {
        return Ok(None);
    }
    let mut name = active.file_name().unwrap_or_default().to_owned();
    name.push(".bak");
    let backup = active.with_file_name(name);

    if fs::symlink_metadata(&backup).is_ok() {
        let read = |path: &Path| fs::read(path).map_err(RewriteError::io("read", path));
        return if read(active)? == read(&backup)? {
            Ok(Some(backup))
        } else {
            Err(RewriteError::BackupTaken {
                active: active.to_owned(),
                backup,
            })
        };
    }
    fs::copy(active, &backup).map_err(RewriteError::io("write", &backup))?;
    Ok(Some(backup))
}
