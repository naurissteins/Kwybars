//! `kwybars image-overlay match`

use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use toml_edit::{DocumentMut, Item, TableLike, Value, table};

use super::{RewriteError, atomic};
use crate::app::RunOptions;
use crate::cli::check::ConfigFile;

/// overlay image extensions, in the order they are tried
pub const EXTENSIONS: [&str; 4] = ["png", "webp", "jpg", "jpeg"];

pub fn match_image(
    options: &RunOptions,
    overlay_dir: &Path,
    wallpaper: &Path,
    env: &dyn Fn(&str) -> Option<OsString>,
) -> Result<String, RewriteError> {
    let image = find(wallpaper, overlay_dir)?;
    let config = editable(&ConfigFile::locate(options, env)?.path)?;

    let raw = match fs::read_to_string(&config) {
        Ok(raw) => raw,
        Err(err) if err.kind() == io::ErrorKind::NotFound => String::new(),
        Err(err) => return Err(RewriteError::io("read", &config)(err)),
    };
    let updated = with_image(&raw, &image, &config)?;
    let state = if updated == raw {
        "unchanged"
    } else {
        atomic::write(&config, updated.as_bytes())?;
        "updated"
    };

    Ok(format!(
        "matched wallpaper {} -> image overlay {}\nconfig: {} ({state})",
        wallpaper.display(),
        image.display(),
        config.display()
    ))
}

fn find(wallpaper: &Path, overlay_dir: &Path) -> Result<PathBuf, RewriteError> {
    let dir = fs::canonicalize(overlay_dir).map_err(|err| match err.kind() {
        io::ErrorKind::NotFound => RewriteError::OverlayDirMissing(overlay_dir.to_owned()),
        _ => RewriteError::io("read", overlay_dir)(err),
    })?;
    if !dir.is_dir() {
        return Err(RewriteError::OverlayDirNotDir(dir));
    }
    let Some(stem) = wallpaper.file_stem() else {
        return Err(RewriteError::NoStem(wallpaper.to_owned()));
    };

    for extension in EXTENSIONS {
        let mut name = stem.to_owned();
        name.push(".");
        name.push(extension);
        let candidate = dir.join(name);
        if candidate.is_file() {
            return fs::canonicalize(&candidate).map_err(RewriteError::io("read", candidate));
        }
    }
    Err(RewriteError::NoMatch {
        wallpaper: wallpaper.to_owned(),
        dir,
    })
}

/// the file to rewrite: `path`, or what it links to while that stays inside
/// the directory of `path`
fn editable(path: &Path) -> Result<PathBuf, RewriteError> {
    if !path.is_symlink() {
        return Ok(path.to_owned());
    }
    let target = fs::canonicalize(path).map_err(RewriteError::io("follow the link", path))?;
    let dir = match path.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir,
        _ => Path::new("."),
    };
    let dir = fs::canonicalize(dir).map_err(RewriteError::io("read", dir))?;
    if target.starts_with(&dir) {
        Ok(target)
    } else {
        Err(RewriteError::LinkLeavesDir {
            link: path.to_owned(),
            target,
            dir,
        })
    }
}

/// the config text with the overlay enabled and showing `image`, everything
/// else as it was written
fn with_image(raw: &str, image: &Path, config: &Path) -> Result<String, RewriteError> {
    let Some(image) = image.to_str() else {
        return Err(RewriteError::NotUtf8(image.to_owned()));
    };
    let mut document = raw
        .parse::<DocumentMut>()
        .map_err(|source| RewriteError::Parse {
            path: config.to_owned(),
            source,
        })?;
    let section = document
        .as_table_mut()
        .entry("image_overlay")
        .or_insert_with(table);
    let inline = section.is_inline_table();
    let section = section
        .as_table_like_mut()
        .ok_or_else(|| RewriteError::NotATable(config.to_owned()))?;
    set(section, "enabled", true.into(), inline);
    set(section, "path", image.into(), inline);
    Ok(document.to_string())
}

/// sets a key, keeping the spacing and comment around a value it replaces
fn set(section: &mut dyn TableLike, key: &str, mut new: Value, inline: bool) {
    if let Some(old) = section.get_mut(key).and_then(Item::as_value_mut) {
        *new.decor_mut() = old.decor().clone();
        *old = new;
        return;
    }
    // what stood before an inline table's closing brace stays there
    let last = section
        .iter_mut()
        .last()
        .and_then(|(_, item)| item.as_value_mut());
    if inline && let Some(last) = last {
        let decor = last.decor_mut();
        if let Some(suffix) = decor.suffix().cloned() {
            decor.set_suffix("");
            new.decor_mut().set_suffix(suffix);
        }
    }
    section.insert(key, Item::Value(new));
}
