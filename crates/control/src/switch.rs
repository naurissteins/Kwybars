use std::fs;
use std::path::{Path, PathBuf};

use crate::error::ControlError;

pub fn validate_target(path: &Path) -> Result<PathBuf, ControlError> {
    let canonical = fs::canonicalize(path).map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            ControlError::InvalidTarget(format!("target config does not exist: {}", path.display()))
        } else {
            ControlError::Io(err)
        }
    })?;

    let metadata = fs::metadata(&canonical)?;
    if !metadata.is_file() {
        return Err(ControlError::InvalidTarget(format!(
            "target config is not a file: {}",
            canonical.display()
        )));
    }

    Ok(canonical)
}

pub fn switch_config(active_path: &Path, target_path: &Path) -> Result<String, ControlError> {
    if paths_match(active_path, target_path) {
        return Ok(format!(
            "active config already points to {}",
            target_path.display()
        ));
    }

    let Some(parent) = active_path.parent() else {
        return Err(ControlError::InvalidTarget(format!(
            "active config path has no parent directory: {}",
            active_path.display()
        )));
    };
    fs::create_dir_all(parent)?;

    maybe_backup_regular_file(active_path)?;

    let temp_link = parent.join(format!(".kwybarsctl-{}.tmp", std::process::id()));
    if temp_link.exists() {
        let _ = fs::remove_file(&temp_link);
    }

    create_symlink(target_path, &temp_link)?;
    fs::rename(&temp_link, active_path)?;

    Ok(format!(
        "switched active config {} -> {}",
        active_path.display(),
        target_path.display()
    ))
}

fn maybe_backup_regular_file(active_path: &Path) -> Result<(), ControlError> {
    let Ok(metadata) = fs::symlink_metadata(active_path) else {
        return Ok(());
    };

    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Ok(());
    }

    let backup_path = active_path.with_file_name(format!(
        "{}.bak",
        active_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("config.toml")
    ));
    if backup_path.exists() {
        return Ok(());
    }

    fs::copy(active_path, backup_path)?;
    Ok(())
}

fn paths_match(active_path: &Path, target_path: &Path) -> bool {
    fs::canonicalize(active_path)
        .ok()
        .is_some_and(|current| current == target_path)
}

#[cfg(unix)]
fn create_symlink(target: &Path, link: &Path) -> Result<(), ControlError> {
    use std::os::unix::fs::symlink;
    symlink(target, link)?;
    Ok(())
}

#[cfg(not(unix))]
fn create_symlink(_target: &Path, _link: &Path) -> Result<(), ControlError> {
    Err(ControlError::InvalidTarget(
        "kwybarsctl switch-config requires Unix symlink support".to_owned(),
    ))
}
