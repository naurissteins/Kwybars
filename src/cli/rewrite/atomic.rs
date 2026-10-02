//! replaces a file or link by renaming a temporary one made beside it, so a
//! reader sees the old content or the new, never a partial write

use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use super::RewriteError;

/// replaces the file at `path`, keeping its permissions
pub fn write(path: &Path, contents: &[u8]) -> Result<(), RewriteError> {
    let temp = prepare(path)?;
    let written = create(&temp, |temp| {
        OpenOptions::new().write(true).create_new(true).open(temp)
    })
    .and_then(|mut file| fill(&mut file, path, contents))
    .map_err(RewriteError::io("write", &temp));
    finish(written, &temp, path)
}

/// replaces whatever is at `link` with a symbolic link to `target`
pub fn link(target: &Path, link: &Path) -> Result<(), RewriteError> {
    let temp = prepare(link)?;
    let linked =
        create(&temp, |temp| symlink(target, temp)).map_err(RewriteError::io("create", &temp));
    finish(linked, &temp, link)
}

/// creates the directory of `path` and names the temporary file in it
fn prepare(path: &Path) -> Result<PathBuf, RewriteError> {
    let dir = match path.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir,
        _ => Path::new("."),
    };
    fs::create_dir_all(dir).map_err(RewriteError::io("create", dir))?;
    let mut name = OsString::from(".");
    name.push(path.file_name().unwrap_or_default());
    name.push(format!(".kwybars-{}.tmp", std::process::id()));
    Ok(dir.join(name))
}

/// runs `make`, which must fail when `temp` exists, clearing a leftover once
fn create<T>(temp: &Path, make: impl Fn(&Path) -> io::Result<T>) -> io::Result<T> {
    match make(temp) {
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
            fs::remove_file(temp)?;
            make(temp)
        }
        result => result,
    }
}

fn fill(file: &mut File, path: &Path, contents: &[u8]) -> io::Result<()> {
    if let Ok(metadata) = fs::metadata(path) {
        file.set_permissions(metadata.permissions())?;
    }
    file.write_all(contents)?;
    file.sync_all()
}

/// renames `temp` over `path`, removing it when anything failed
fn finish(made: Result<(), RewriteError>, temp: &Path, path: &Path) -> Result<(), RewriteError> {
    let result =
        made.and_then(|()| fs::rename(temp, path).map_err(RewriteError::io("replace", path)));
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}
