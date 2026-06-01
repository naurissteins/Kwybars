use std::fs;
use std::path::{Path, PathBuf};

use toml_edit::{DocumentMut, table, value};

use crate::error::ControlError;

const OVERLAY_EXTENSIONS: [&str; 4] = ["png", "webp", "jpg", "jpeg"];

pub fn match_image_overlay(
    config_path: &Path,
    wallpaper: &Path,
    overlay_dir: &Path,
) -> Result<String, ControlError> {
    let overlay_path = find_overlay_image(wallpaper, overlay_dir)?;
    let edit_path = editable_config_path(config_path);
    update_config(&edit_path, &overlay_path)?;

    Ok(format!(
        "matched wallpaper {} -> image overlay {}",
        wallpaper.display(),
        overlay_path.display()
    ))
}

fn find_overlay_image(wallpaper: &Path, overlay_dir: &Path) -> Result<PathBuf, ControlError> {
    let overlay_dir = fs::canonicalize(overlay_dir).map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            ControlError::InvalidTarget(format!(
                "overlay directory does not exist: {}",
                overlay_dir.display()
            ))
        } else {
            ControlError::Io(err)
        }
    })?;

    if !fs::metadata(&overlay_dir)?.is_dir() {
        return Err(ControlError::InvalidTarget(format!(
            "overlay path is not a directory: {}",
            overlay_dir.display()
        )));
    }

    let Some(stem) = wallpaper.file_stem() else {
        return Err(ControlError::InvalidTarget(format!(
            "wallpaper path has no filename stem: {}",
            wallpaper.display()
        )));
    };

    for extension in OVERLAY_EXTENSIONS {
        let mut candidate = overlay_dir.join(stem);
        candidate.set_extension(extension);
        if fs::metadata(&candidate).is_ok_and(|metadata| metadata.is_file()) {
            return fs::canonicalize(candidate).map_err(ControlError::Io);
        }
    }

    Err(ControlError::InvalidTarget(format!(
        "no matching overlay image found for {} in {} (tried: {})",
        wallpaper.display(),
        overlay_dir.display(),
        OVERLAY_EXTENSIONS.join(", ")
    )))
}

fn editable_config_path(config_path: &Path) -> PathBuf {
    fs::canonicalize(config_path).unwrap_or_else(|_| config_path.to_path_buf())
}

fn update_config(config_path: &Path, overlay_path: &Path) -> Result<(), ControlError> {
    let raw = match fs::read_to_string(config_path) {
        Ok(value) => value,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(err) => return Err(ControlError::Io(err)),
    };

    let mut document = raw.parse::<DocumentMut>().map_err(|err| {
        ControlError::InvalidTarget(format!(
            "failed to parse config {}: {err}",
            config_path.display()
        ))
    })?;
    let Some(path_value) = overlay_path.to_str() else {
        return Err(ControlError::InvalidTarget(format!(
            "overlay image path is not valid UTF-8: {}",
            overlay_path.display()
        )));
    };

    if document.as_table().contains_key("image_overlay")
        && !document.as_table().contains_table("image_overlay")
    {
        return Err(ControlError::InvalidTarget(
            "config image_overlay entry is not a table".to_owned(),
        ));
    }
    if !document.as_table().contains_table("image_overlay") {
        document["image_overlay"] = table();
    }
    document["image_overlay"]["enabled"] = value(true);
    document["image_overlay"]["path"] = value(path_value);

    write_atomically(config_path, document.to_string().as_bytes())
}

fn write_atomically(path: &Path, contents: &[u8]) -> Result<(), ControlError> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;

    let temp_path = parent.join(format!(
        ".kwybarsctl-image-overlay-{}.tmp",
        std::process::id()
    ));
    if temp_path.exists() {
        let _ = fs::remove_file(&temp_path);
    }
    fs::write(&temp_path, contents)?;
    fs::rename(&temp_path, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::match_image_overlay;

    #[test]
    fn updates_existing_image_overlay_section() {
        let root = test_dir("updates-existing");
        let overlays = root.join("overlays");
        let config = root.join("config.toml");

        assert!(fs::create_dir_all(&overlays).is_ok());
        assert!(fs::write(overlays.join("forest.png"), b"image").is_ok());
        assert!(
            fs::write(
                &config,
                "[image_overlay]\nenabled = false\npath = \"old.png\"\nopacity = 0.7\n",
            )
            .is_ok()
        );

        let result = match_image_overlay(&config, &PathBuf::from("forest.jpg"), &overlays);
        assert!(result.is_ok());

        let updated = fs::read_to_string(&config);
        assert!(updated.is_ok());
        let updated = updated.unwrap_or_default();
        assert!(updated.contains("enabled = true"));
        assert!(updated.contains("path = "));
        assert!(updated.contains("forest.png"));
        assert!(updated.contains("opacity = 0.7"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn creates_image_overlay_section_when_config_is_missing() {
        let root = test_dir("creates-missing");
        let overlays = root.join("overlays");
        let config = root.join("config.toml");

        assert!(fs::create_dir_all(&overlays).is_ok());
        assert!(fs::write(overlays.join("city.webp"), b"image").is_ok());

        let result = match_image_overlay(&config, &PathBuf::from("city.jpg"), &overlays);
        assert!(result.is_ok());

        let updated = fs::read_to_string(&config);
        assert!(updated.is_ok());
        let updated = updated.unwrap_or_default();
        assert!(updated.contains("[image_overlay]"));
        assert!(updated.contains("enabled = true"));
        assert!(updated.contains("city.webp"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn reports_missing_match() {
        let root = test_dir("missing-match");
        let overlays = root.join("overlays");
        let config = root.join("config.toml");

        assert!(fs::create_dir_all(&overlays).is_ok());

        let result = match_image_overlay(&config, &PathBuf::from("forest.jpg"), &overlays);
        assert!(result.is_err());

        let _ = fs::remove_dir_all(root);
    }

    fn test_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!("kwybarsctl-{name}-{}-{nanos}", std::process::id()))
    }
}
